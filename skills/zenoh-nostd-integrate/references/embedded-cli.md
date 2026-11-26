# Add Zenoh commands to an existing `embedded-cli` console

Use `zenoh-cli` (`zenoh_cli` in Rust) with its default `commands` feature and
keep the application's `embedded-cli` dependency compatible with the version
used by that revision. Read the `zenoh-cli` crate Rustdoc or its
`crates/zenoh-cli/README.md` for a runnable example.

1. Add `#[command(subcommand)] Zenoh(zenoh_cli::ZenohCommand<'a>)` to the
   existing derived command enum. Keep using that enum in `process_byte`.
2. In the existing processor, use `embedded_cli::service::FromRaw` and parse
   once with `AppCommand::parse(raw).map_err(ProcessError::ParseError)?`.
   Match the typed `AppCommand::Zenoh(command)` variant and keep the existing
   handlers for local commands. For raw input dedicated to Zenoh,
   `CommandInbox::forward` can parse, convert, and enqueue in one call.
3. Convert supported Zenoh variants into application-owned requests before
   the editor reuses its buffer. Validate arguments and copy borrowed keys,
   payloads, or endpoints into bounded storage on fixed-memory targets.
   Enqueue with `CommandInbox::try_send`, and report unsupported commands,
   invalid arguments, and full queues on the console. Size the editor and the
   inbox for the application's input limits and use a mutex suited to the tasks
   sharing it.
4. Let the existing Zenoh task await `inbox.receive()` and execute each request
   alongside the session driver. Keep connection and transport policy in that
   task. Report when an operation completes separately from queue acceptance.

Test an existing local command, a namespaced command with a quoted payload,
malformed input, and queue saturation. With `embedded-cli` help enabled,
`help zenoh` displays the shared schema. Document the subset the application
executes. Verify the command's effect against a peer when possible.
