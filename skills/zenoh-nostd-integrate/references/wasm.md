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

Use `WasmLinkManager` with a compatible WebSocket endpoint such as
`ws/127.0.0.1:7446`. For a remote endpoint, choose an address the browser can
reach. Check endpoint syntax and secure WebSocket support in the adapter Rustdoc
for the deployment.

Hand owned callback data to the UI. When diagnosing reconnect failures, inspect
the pinned WebSocket dependency's close/drop behavior and retain close/error
details for comparison with the remote endpoint's logs.

## Acceptance

Build the consumer's WASM bundle, then observe the requested exchange through
the selected WebSocket endpoint. Exercise repeated cancellation and reopening
from the browser when the application supports those paths. Check the browser
and remote endpoint logs for the Zenoh handshake and teardown. Report any
runtime coverage that was unavailable.
