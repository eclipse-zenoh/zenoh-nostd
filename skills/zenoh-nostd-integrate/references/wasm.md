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
`ws/127.0.0.1:7446`. A browser cannot use the native TCP adapter. The current
adapter accepts numeric socket addresses and supports neither listening nor
broker mode. `127.0.0.1` identifies the browser's machine, not a remote router.
A page served over HTTPS may require secure WebSockets; verify transport support
in the actual adapter revision before promising that deployment.

Poll the session driver while UI operations run. Keep callbacks short and bounded;
hand owned data to the UI after returning from receive callbacks. Closing a UI
operation must drop the connection scope on cancellation/error. Verify the pinned
WebSocket dependency's close/drop behavior when diagnosing reconnect failures;
retain close/error details rather than treating generic link errors as proof of a
particular cause. Do not suppress failures or work around them by retrying forever.

## Acceptance

Build the consumer's WASM bundle, then observe publication/subscription and a
finalized query response through the router. Exercise repeated cancellation and
reopen from the browser. A successful bundle build or WebSocket upgrade does not
prove the Zenoh handshake, message exchange or teardown succeeded. Inspect browser
and router evidence together and report unavailable runtime coverage explicitly.
