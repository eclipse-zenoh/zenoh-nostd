//! Add Zenoh commands to an `embedded-cli` console and send requests to a Zenoh task.
//!
//! The application supplies the session, the supported operations, and the
//! conversion from borrowed command arguments to owned requests. The inbox
//! works on native, WebAssembly, and embedded targets without an allocator.
//!
//! The default `commands` feature exports `ZenohCommand` for applications that
//! want a shared Zenoh command group. Disable default features to use the inbox
//! with a custom schema alone. Use [`CommandInbox::forward`] when the inbox
//! receives raw input directly, or [`CommandInbox::try_send`] when the existing
//! console has already parsed the command.

#![cfg_attr(feature = "commands", doc = include_str!("../README.md"))]
#![no_std]
#![warn(missing_docs)]

#[cfg(feature = "commands")]
mod commands;
#[cfg(feature = "commands")]
pub use commands::{LegacyZenohCommand, ZenohCommand};

use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    channel::{Channel, TryReceiveError, TrySendError},
};
use embedded_cli::{
    command::RawCommand,
    service::{FromRaw, ParseError},
};

/// An error encountered while parsing or converting a console command.
#[derive(Debug)]
pub enum CommandError<'input, E> {
    /// The application-defined command schema rejected the input.
    Parse(ParseError<'input>),
    /// The application rejected the parsed command during conversion.
    Invalid(E),
}

/// An error encountered before a request reached the Zenoh task.
#[derive(Debug)]
pub enum ForwardError<'input, E, T> {
    /// Parsing or conversion failed.
    Command(CommandError<'input, E>),
    /// The inbox is full. Contains the unsent request.
    Full(T),
}

/// A bounded channel shared by a synchronous CLI and an async Zenoh task.
pub struct CommandInbox<M: RawMutex, T, const N: usize> {
    requests: Channel<M, T, N>,
}

impl<M: RawMutex, T, const N: usize> CommandInbox<M, T, N> {
    /// Create an empty inbox with capacity `N`.
    pub const fn new() -> Self {
        Self {
            requests: Channel::new(),
        }
    }

    /// Parse, convert, and forward a command without waiting for capacity.
    ///
    /// The conversion runs only after parsing succeeds. It must copy any data
    /// that the task needs after the editor reuses its input buffer.
    /// If your command processor already parsed the outer command enum, convert
    /// its typed Zenoh variant into a request and call [`Self::try_send`].
    pub fn forward<'input, C, E>(
        &self,
        raw: RawCommand<'input>,
        convert: impl FnOnce(C) -> Result<T, E>,
    ) -> Result<(), ForwardError<'input, E, T>>
    where
        C: FromRaw<'input>,
    {
        let request = parse_command(raw, convert).map_err(ForwardError::Command)?;
        self.try_send(request)
            .map_err(|TrySendError::Full(request)| ForwardError::Full(request))
    }

    /// Forward an existing request without waiting for capacity.
    ///
    /// This is also useful for requests created by a timer or local application
    /// logic. On failure the error contains the unsent request.
    pub fn try_send(&self, request: T) -> Result<(), TrySendError<T>> {
        self.requests.try_send(request)
    }

    /// Wait for the next request in the Zenoh task.
    pub async fn receive(&self) -> T {
        self.requests.receive().await
    }

    /// Receive the next request if one is immediately available.
    pub fn try_receive(&self) -> Result<T, TryReceiveError> {
        self.requests.try_receive()
    }

    /// Discard requests still waiting in the inbox, such as after cancellation.
    pub fn clear(&self) {
        self.requests.clear();
    }
}

impl<M: RawMutex, T, const N: usize> Default for CommandInbox<M, T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse and convert a command without forwarding it.
///
/// Use this when the console must finish output or apply admission policy
/// before handing the request to the task.
pub fn parse_command<'input, C, T, E>(
    raw: RawCommand<'input>,
    convert: impl FnOnce(C) -> Result<T, E>,
) -> Result<T, CommandError<'input, E>>
where
    C: FromRaw<'input>,
{
    let command = C::parse(raw).map_err(CommandError::Parse)?;
    convert(command).map_err(CommandError::Invalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use embassy_sync::blocking_mutex::raw::NoopRawMutex;
    use embedded_cli::{Command, cli::CliBuilder};

    #[derive(Command)]
    enum TestCommand {
        Set { value: u8 },
    }

    fn convert(TestCommand::Set { value }: TestCommand) -> Result<u8, ()> {
        (value != 0).then_some(value).ok_or(())
    }

    fn feed(input: &[u8], mut handle: impl FnMut(RawCommand<'_>)) {
        let mut cli = CliBuilder::default().build().unwrap();
        let mut processor = RawCommand::processor(|_, raw| {
            handle(raw);
            Ok(())
        });
        for &byte in input {
            cli.process_byte::<TestCommand, _>(byte, &mut processor)
                .unwrap();
        }
    }

    #[test]
    fn failed_commands_never_reach_the_task() {
        let inbox = CommandInbox::<NoopRawMutex, _, 1>::new();
        let mut failures = ["parse", "conversion"].into_iter();
        feed(b"set x\nset 0\n", |raw| {
            let failure = match inbox.forward(raw, convert) {
                Err(ForwardError::Command(CommandError::Parse(_))) => "parse",
                Err(ForwardError::Command(CommandError::Invalid(()))) => "conversion",
                result => panic!("expected invalid input to fail: {result:?}"),
            };
            assert_eq!(failures.next(), Some(failure));
        });
        assert_eq!(failures.next(), None);
        assert!(inbox.try_receive().is_err());
    }

    #[test]
    fn forwarding_to_a_full_inbox_returns_the_unsent_request() {
        let inbox = CommandInbox::<NoopRawMutex, _, 1>::new();
        feed(b"set 1\n", |raw| inbox.forward(raw, convert).unwrap());
        feed(b"set 2\n", |raw| {
            assert!(matches!(
                inbox.forward(raw, convert),
                Err(ForwardError::Full(2))
            ));
        });
        assert_eq!(inbox.try_receive(), Ok(1));
        assert!(inbox.try_receive().is_err());
    }
}
