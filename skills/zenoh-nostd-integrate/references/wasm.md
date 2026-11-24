# Browser/WASM integration

Reuse the application's browser executor and UI framework. Select compatible core
and `zenoh-wasm` revisions and inspect that revision's manifest for the `yawc` Git
patch used by the upstream browser example. Cargo dependency patches do not
propagate to the consuming application's root manifest.

Provide a WASM-compatible Embassy time driver and entropy backend. The native
`embassy-time/std` driver is not a browser driver; use the setup matching the SDK
revision's browser example. Compile for `wasm32-unknown-unknown` from the consumer's
project directory, preserving its existing bindgen/bundler configuration. Do not
copy the example CLI or its HTML assets merely to use the session API.

Use `WasmLinkManager` with a router WebSocket endpoint such as
`ws/127.0.0.1:7446`. For a remote router, choose an address the browser can
reach. Check endpoint syntax and secure WebSocket support in the adapter Rustdoc
for the deployment.

Hand owned callback data to the UI. When diagnosing reconnect failures, inspect
the pinned WebSocket dependency's close/drop behavior and retain close/error
details for comparison with the router's logs.

## Acceptance

Build the consumer's WASM bundle, then observe publication/subscription and a
finalized query response through the router. Exercise repeated cancellation and
reopen from the browser. A successful bundle build or WebSocket upgrade does not
prove the Zenoh handshake, message exchange or teardown succeeded. Inspect browser
and router evidence together and report unavailable runtime coverage explicitly.
