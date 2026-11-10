# Browser example application

From the repository root, enter `nix develop` and run `just wasm`. Open
`http://127.0.0.1:8000`, enter commands in the text field, and use Stop or Ctrl-C
to cancel an operation. The console shares parsing and operations with native
and embedded applications.

If port 8000 is already in use, run `WASM_PORT=8001 just wasm` and open
`http://127.0.0.1:8001`. The same override works with `just wasm-headless`.

A Zenoh router must offer a WebSocket endpoint. The compiled default is
`ws/127.0.0.1:7446`; use `z_open ws/host:port` to override it. Native TCP endpoints
are unavailable in the browser. Listening and broker mode report unsupported
capability; they do not require separate browser binaries.

`just build-wasm` builds the single `zenoh-app.wasm` and generates `z_example.js`
using the Nix-pinned wasm-bindgen CLI. `just wasm alloc` enables allocated Zenoh
storage. Both storage configurations are checked. Terminal output is bounded
in Rust, and the browser view retains the last 65,536 characters.

`COUNT=3 just wasm-headless pub` builds without the default `cli` feature and
starts a compiled scenario automatically, hiding command controls.
`just build-wasm-headless get alloc` only builds the assets. See the
[headless guide](../../docs/applications.md#headless) for startup settings.

See the [command guide](../../docs/applications.md#cli).
