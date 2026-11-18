# Native example application

One host executable provides all supported scenarios. From the repository root:

```sh
nix develop
just std
just std z_put demo/example "hello world"
just std z_get 'demo/example/**'
just std-alloc z_broker
```

Or run `cargo run --locked --bin zenoh-app -- z_get 'demo/example/**'` from this
directory. With no command arguments it opens an interactive console. Add
`--features alloc` before `--` for broker capability. Native `ENDPOINT` is read
at process startup and defaults to `tcp/127.0.0.1:7447`. Use `z_open <endpoint>`
to choose an address interactively, and `z_close` before changing it.

Ctrl-C stops the foreground operation in an interactive Unix terminal; `quit`
exits. The application restores terminal settings on normal exit. One-command
invocations return a useful exit status. See the
[command guide](../../docs/applications.md#cli) for syntax and capacities.

Disable the default `cli` feature for startup scenarios:
`SCENARIO=pub COUNT=3 just std-headless`, optionally adding `alloc`. This mode
reads typed runtime settings and needs no stdin. See the
[headless guide](../../docs/applications.md#headless).

`just test-cli` runs the lifecycle integration suite against an isolated router
provided by Nix. The suite does not modify existing routers or read `.envrc`.
