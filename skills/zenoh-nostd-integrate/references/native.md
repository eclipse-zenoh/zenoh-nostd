# Native application recipe

Use `zenoh-std` and a compatible `zenoh-nostd` revision in the consuming project's
manifest. Reuse its executor and build environment; the example application
framework and CLI are optional references, not dependencies.

## Dependencies and runtime

- Provide the native Embassy time driver. With a non-Embassy executor such as
  `futures-lite`, also select a generic timer queue. For the current API:

  ```toml
  embassy-time = { version = "0.5.0", features = ["std", "generic-queue-16"] }
  ```

  The default integrated queue expects Embassy executor wakers. A missing
  `__embassy_time_queue_item_from_waker` symbol at link time indicates this
  mismatch; a successful `cargo check` alone does not verify the configuration.
- Inspect the matching upstream native example's manifest and lockfile. Cargo
  dependency patches are not inherited by consumers: carry its required
  `async-net` compatibility patch into the application's root manifest.
- With the adapter's `wtx` 0.39 dependency, fresh resolution can combine
  `sha1` 0.11.0-rc.2 with incompatible stable RustCrypto APIs. A verified
  combination is `digest` 0.11.0-rc.9 and `crypto-common` 0.2.0-rc.13. Check
  the resolved graph and preserve the application's own lockfile; reassess
  these constraints when changing adapter revisions.
- Account for native networking allocations when budgeting fixed Zenoh registries.

Run Cargo from the application directory, including a complete binary link:

```sh
cargo build --locked
```

For a newly created project, generate its lockfile before using `--locked`.

## Verify actual communication

Use the endpoint and connection direction selected for the application. If the
agreed test setup uses an isolated local router, start it in a separate terminal:

```sh
zenohd --listen tcp/127.0.0.1:7447
```

Configure the endpoint through the application's existing configuration.
Use a second session or peer to exercise the requested operation. For pub/sub,
allow declarations to propagate, then verify the received key and payload. For
queries, observe the expected reply and completion. Clean up successful work
and flush as appropriate before exiting.

An unreachable peer must produce a bounded failure. Cancellation or transport
write errors require dropping the connection before diagnostics or reconnecting.
Report build results separately from observed exchanges; a socket connection or
successful write alone does not establish message delivery. Preserve the existing
application layout and runtime when adapting these steps.
