//! Bounded command interface for example applications. No board policy lives here.
mod frontend;
mod parser;
mod runner;
pub use crate::AppConfig;
pub use parser::{Command, KEY_CAPACITY, Kind, LINE_CAPACITY, Line, PAYLOAD_CAPACITY};

use crate::scenarios::{Context, Ready, Reporter};
use core::{
    cell::{Cell, RefCell},
    fmt::{self, Write as _},
};
use embassy_futures::{
    join::join,
    select::{Either, select},
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel, signal::Signal};
use heapless::{String, Vec};

pub trait ConsoleRead {
    fn read(&mut self, bytes: &mut [u8]) -> impl core::future::Future<Output = Result<usize, ()>>;
}
pub trait ConsoleWrite {
    fn write(&mut self, bytes: &[u8]) -> impl core::future::Future<Output = Result<(), ()>>;
}
pub struct IoRead<R>(pub R);
pub struct IoWrite<W>(pub W);
impl<R: embedded_io_async::Read> ConsoleRead for IoRead<R> {
    async fn read(&mut self, bytes: &mut [u8]) -> Result<usize, ()> {
        self.0.read(bytes).await.map_err(|_| ())
    }
}
impl<W: embedded_io_async::Write> ConsoleWrite for IoWrite<W> {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), ()> {
        self.0.write_all(bytes).await.map_err(|_| ())
    }
}

pub struct PendingRead;
impl ConsoleRead for PendingRead {
    async fn read(&mut self, _: &mut [u8]) -> Result<usize, ()> {
        core::future::pending().await
    }
}

// Inline buffers keep the default console allocation-free.
#[allow(clippy::large_enum_variant)]
enum Output {
    Bytes(Vec<u8, 384>),
    End(i32),
}

pub struct Cli {
    commands: Channel<NoopRawMutex, Command, 2>,
    initial: RefCell<Option<Line>>,
    output: Channel<NoopRawMutex, Output, 16>,
    scenario: Context,
    cancel: Signal<NoopRawMutex, ()>,
    network_address: RefCell<String<48>>,
    ready: Cell<bool>,
    busy: Cell<bool>,
    connected: Cell<bool>,
    pub(crate) shutdown: Cell<bool>,
    one_shot: Cell<bool>,
    dropped: Cell<u64>,
}

impl Default for Cli {
    fn default() -> Self {
        Self::new()
    }
}
impl Cli {
    pub const fn new() -> Self {
        Self {
            commands: Channel::new(),
            initial: RefCell::new(None),
            output: Channel::new(),
            scenario: Context::new(),
            cancel: Signal::new(),
            network_address: RefCell::new(String::new()),
            ready: Cell::new(false),
            busy: Cell::new(false),
            connected: Cell::new(false),
            shutdown: Cell::new(false),
            one_shot: Cell::new(false),
            dropped: Cell::new(0),
        }
    }
    pub fn set_network_address(&self, address: impl fmt::Display) {
        let mut stored = self.network_address.borrow_mut();
        stored.clear();
        let _ = write!(stored, "{address}");
    }
    pub fn set_one_shot(&self, command: Line) -> Result<(), &'static str> {
        self.one_shot.set(true);
        if self.initial.borrow().is_some() {
            return Err("initial command already set");
        }
        *self.initial.borrow_mut() = Some(command);
        Ok(())
    }
    async fn bytes(&self, bytes: &[u8]) {
        for chunk in bytes.chunks(384) {
            self.output
                .send(Output::Bytes(Vec::from_slice(chunk).unwrap()))
                .await;
        }
    }

    pub(crate) fn drop_event(&self) {
        self.dropped.set(self.dropped.get().saturating_add(1));
    }
    fn raw(&self, bytes: &[u8]) {
        if let Ok(bytes) = Vec::from_slice(bytes) {
            if self.output.try_send(Output::Bytes(bytes)).is_err() {
                self.drop_event();
            }
        } else {
            self.drop_event();
        }
    }
    pub(crate) fn emit(&self, args: fmt::Arguments<'_>) {
        let mut text = String::<384>::new();
        if text.write_fmt(args).is_err() {
            self.drop_event();
            self.raw(b"[output exceeded display limit]\r\n");
        } else {
            self.raw(text.as_bytes());
        }
    }
    pub(crate) async fn report(&self, args: fmt::Arguments<'_>) {
        let mut text = String::<384>::new();
        if text.write_fmt(args).is_ok() {
            self.text(&text).await;
        } else {
            self.text("[output exceeded display limit]\r\n").await;
        }
    }
    pub(crate) async fn text(&self, text: &str) {
        // Split on UTF-8 boundaries, including long help output.
        let mut remaining = text;
        while !remaining.is_empty() {
            let mut n = remaining.len().min(384);
            while !remaining.is_char_boundary(n) {
                n -= 1;
            }
            self.output
                .send(Output::Bytes(
                    Vec::from_slice(&remaining.as_bytes()[..n]).unwrap(),
                ))
                .await;
            remaining = &remaining[n..];
        }
    }
    fn status(&self) {
        self.emit(format_args!("\r\nnetwork={} session={} command={} alloc={} dropped={} address={}\r\nlimits: line=512 key=128 payload=256 events=8 output=16\r\n",
            if self.ready.get() { "ready" } else { "starting" }, if self.connected.get() { "open" } else { "closed" },
            if self.busy.get() { "running" } else { "idle" }, cfg!(feature="alloc"), self.dropped.get().saturating_add(self.scenario.dropped()), self.network_address.borrow()));
    }
    fn reset_command(&self) {
        self.scenario.reset();
        self.busy.set(true);
    }
    async fn prompt(&self) {
        self.busy.set(false);
        if !self.one_shot.get() {
            self.text("zenoh> ").await;
        }
    }
}

impl Reporter for Cli {
    fn emit(&self, args: fmt::Arguments<'_>) {
        Cli::emit(self, args);
    }
    async fn ready(&self, ready: Ready) {
        self.report(format_args!("{}; Ctrl-C to stop\r\n", ready.message()))
            .await;
    }
}

async fn reader(mut rx: impl ConsoleRead, cli: &Cli) {
    let render = RefCell::new(frontend::RenderState::new());
    let mut editor = frontend::Frontend::new(&render).expect("empty CLI writer");
    let mut bytes = [0u8; 32];
    loop {
        let n = match rx.read(&mut bytes).await {
            Ok(0) | Err(()) => {
                cli.shutdown.set(true);
                cli.cancel.signal(());
                let _ = cli.commands.try_send(Command::quit());
                core::future::pending::<()>().await;
                unreachable!()
            }
            Ok(n) => n,
        };
        for &byte in &bytes[..n] {
            let frame = match editor
                .set_idle(!cli.busy.get())
                .and_then(|_| editor.feed(byte))
            {
                Ok(frame) => frame,
                Err(_) => {
                    // A bounded render error must never enqueue a partial command.
                    render.borrow_mut().bytes.clear();
                    cli.text("error: CLI render buffer full\r\n").await;
                    cli.shutdown.set(true);
                    cli.cancel.signal(());
                    let _ = cli.commands.try_send(Command::quit());
                    core::future::pending::<()>().await;
                    unreachable!()
                }
            };
            let length = render.borrow().bytes.len();
            for offset in (0..length).step_by(384) {
                let chunk = {
                    let state = render.borrow();
                    Vec::from_slice(&state.bytes[offset..(offset + 384).min(length)]).unwrap()
                };
                cli.output.send(Output::Bytes(chunk)).await;
            }
            render.borrow_mut().bytes.clear();
            if frame.cancelled {
                cli.cancel.signal(());
                cli.text("^C\r\n").await;
                if !cli.busy.get() {
                    cli.prompt().await;
                }
            } else if let Some(command) = frame.command {
                if command.kind == Kind::Status {
                    cli.status();
                    if !cli.busy.get() {
                        cli.prompt().await;
                    }
                } else if !cli.ready.get() {
                    cli.text("network starting; help and status are available\r\n")
                        .await;
                    cli.prompt().await;
                } else if cli.busy.get() {
                    cli.text("command running; press Ctrl-C first\r\n").await;
                } else {
                    // Clear cancellation when accepting work, before a later
                    // byte in the same paste can cancel that queued command.
                    cli.cancel.reset();
                    if cli.commands.try_send(command).is_err() {
                        cli.text("command queue full\r\n").await;
                        cli.prompt().await;
                    } else {
                        cli.busy.set(true);
                    }
                }
            } else if frame.completed && !cli.busy.get() {
                cli.prompt().await;
            }
        }
    }
}
async fn writer(mut tx: impl ConsoleWrite, cli: &Cli) -> i32 {
    loop {
        match cli.output.receive().await {
            Output::Bytes(bytes) => {
                if tx.write(&bytes).await.is_err() {
                    return 1;
                }
            }
            Output::End(code) => return code,
        }
    }
}

/// Console I/O stays alive while networking initializes or a command executes.
/// The configuration must be initialized once and outlive the application.
pub async fn run<C: AppConfig + 'static>(
    cli: &'static Cli,
    rx: impl ConsoleRead,
    tx: impl ConsoleWrite,
    init: impl core::future::Future<Output = &'static C>,
    default_endpoint: &str,
    can_listen: bool,
) -> i32 {
    let work = async {
        if !cli.one_shot.get() {
            cli.text("zenoh-nostd command console; type help\r\n").await;
            cli.prompt().await;
        }
        let code = match select(init, async {
            while !cli.shutdown.get() {
                embassy_time::Timer::after_millis(20).await;
            }
        })
        .await
        {
            Either::First(config) => {
                cli.ready.set(true);
                runner::application(config, cli, default_endpoint, can_listen).await
            }
            Either::Second(_) => 0,
        };
        cli.output.send(Output::End(code)).await;
        core::future::pending::<()>().await;
    };
    match select(writer(tx, cli), join(reader(rx, cli), work)).await {
        Either::First(code) => code,
        Either::Second(_) => 1,
    }
}
