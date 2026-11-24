## Add Zenoh to an existing `embedded-cli` console

Add `zenoh-cli` from GitHub with its default `commands` feature. Replace the
`rev` placeholder with a commit that contains `zenoh-cli` and use the same Git
URL and revision for your other Zenoh crates from that repository. Keep your
existing CLI features and use an `embedded-cli` version compatible with `0.2.1`.
The Cargo package is `zenoh-cli` and the Rust import is `zenoh_cli`.

```toml
[dependencies]
zenoh-cli = { git = "https://github.com/eclipse-zenoh/zenoh-nostd", rev = "<commit-sha>" }
embedded-cli = { version = "0.2.1", default-features = false, features = ["macros", "help"] }
embassy-sync = "0.7"
heapless = "0.8"
```

Add `Zenoh(ZenohCommand<'a>)` with `#[command(subcommand)]` to your existing
derived command enum. Pass that enum to `process_byte` as usual. Its `parse`
method handles the outer command and its nested arguments in one call. Match the
typed command in your existing processor, keeping local commands in the console.
When `embedded-cli`'s `help` feature is enabled, `help zenoh` shows the shared
command schema. Tell users which operations your application actually supports.

```rust
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embedded_cli::{
    Command,
    cli::CliBuilder,
    command::RawCommand,
    service::{FromRaw, ParseError},
};
use heapless::String;
use zenoh_cli::{CommandInbox, ZenohCommand};

#[derive(Command)]
enum AppCommand<'a> {
    Uptime,
    #[command(subcommand)]
    Zenoh(ZenohCommand<'a>),
}

#[derive(Debug, PartialEq, Eq)]
enum ZenohRequest {
    Put { key: String<64>, payload: String<128> },
}

// Both ends of this inbox run in the same executor/thread context.
type Inbox = CommandInbox<NoopRawMutex, ZenohRequest, 2>;

fn dispatch<'a>(raw: RawCommand<'a>, inbox: &Inbox) -> Result<&'static str, ParseError<'a>> {
    let command = AppCommand::parse(raw)?;
    match command {
        AppCommand::Uptime => Ok("run the local uptime handler"),
        AppCommand::Zenoh(ZenohCommand::Put { key, payload }) => {
            let Ok(key) = String::try_from(key) else {
                return Ok("key too long");
            };
            let Ok(payload) = String::try_from(payload) else {
                return Ok("payload too long");
            };
            match inbox.try_send(ZenohRequest::Put { key, payload }) {
                Ok(()) => Ok("request queued"),
                Err(_) => Ok("Zenoh command queue full"),
            }
        }
        AppCommand::Zenoh(_) => Ok("this application supports zenoh put"),
    }
}

let inbox = Inbox::new();
let mut editor = CliBuilder::default().command_buffer([0; 256]).build().unwrap();
let mut processor = RawCommand::processor(|_, raw| {
    assert_eq!(dispatch(raw, &inbox).unwrap(), "request queued");
    Ok(())
});
for &byte in b"zenoh put office/note \"window open\"\n" {
    editor.process_byte::<AppCommand<'_>, _>(byte, &mut processor).unwrap();
}

// The worker uses inbox.receive().await. This check needs no network.
assert_eq!(inbox.try_receive(), Ok(ZenohRequest::Put {
    key: String::try_from("office/note").unwrap(),
    payload: String::try_from("window open").unwrap(),
}));
```

In your `CommandProcessor::process` implementation, map parse errors to
`ProcessError::ParseError` and report validation failures, unsupported commands,
and a full inbox on the console. Size the editor buffer, request fields, and
inbox for your application. Use a mutex appropriate for the tasks sharing it.

Have the task that owns the session await `inbox.receive()` alongside its
existing work. For the example request, validate the publication key, then
call `session.put(key, payload.as_bytes()).finish().await`, while `session.run()`
continues to be polled. Queue acceptance and operation completion are separate
events. Report the operation result from the worker. Measure or bound how many
requests and replies can accumulate, especially for long-running operations.
