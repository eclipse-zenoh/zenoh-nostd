# Change: Peer-to-Peer Unicast TCP Transport Mode

## Why

Zenoh-Pico supports `Z_FEATURE_UNICAST_PEER` (v1.1.0+) where two peers establish a bidirectional TCP connection with mutual session negotiation. zenoh-nostd is missing this capability. The branch `feat/peer-transport-mode` exists but the peer-to-peer handshake path is not wired through the session API.

Without peer mode, nostd only supports client-server topology (client connects to a router/broker, or a router accepts client connections). Peer-to-peer unicast TCP is required for mesh and decentralized topologies where each node acts symmetrically — both initiating connections to discovered peers and accepting incoming connections.

The underlying transport state machine in `zenoh-sansio` (`transport/establishment.rs`) already implements simultaneous open resolution (ZID-based tie-breaking when both sides send `InitSyn` as `WhatAmI::Peer`). The codec `WhatAmI::Peer` variant exists in `zenoh-proto`. What remains is wiring peer mode through the session configuration and API layers.

## What Changes

### 1. zenoh-proto — Handshake message verification

No new message variants needed. The existing `InitSyn`/`InitAck` messages carry `identifier.whatami` which already supports `WhatAmI::Peer`. Add:
- Property tests verifying `InitSyn`/`InitAck` roundtrip with `WhatAmI::Peer`
- Golden vector for a peer-mode InitSyn exchange

### 2. zenoh-sansio — Transport builder peer helpers

The `TransportBuilder` currently has `connect()` (starts in `WaitingInitAck`) and `listen()` (starts in `WaitingInitSyn`). Both accept `with_whatami()`. Add:
- `peer_connect()` — alias that calls `self.with_whatami(WhatAmI::Peer).connect(...)` (convenience, explicit role: initiating side)
- `peer_listen()` — alias that calls `self.with_whatami(WhatAmI::Peer).listen(...)` (convenience, accepting side)

The establishment state machine already handles the peer simultaneous-open case correctly:
- `WaitingInitAck` receiving `InitSyn` from a `WhatAmI::Peer`: compares ZIDs, lower ZID wins (sends InitAck), higher ZID drops its own InitSyn and waits for OpenSyn

### 3. zenoh-sansio — Test coverage

Expand `tcp_peer.rs` to cover:
- Normal peer connect/listen (existing test uses `with_whatami` manually)
- Simultaneous open (both sides call `connect` at the same time, require ZID collision handling)
- Timeout during peer handshake
- Large ZID comparison edge cases

### 4. zenoh-nostd — TransportLinkManager peer mode

Add to `TransportLinkManager`:
```rust
pub fn with_whatami(mut self, whatami: WhatAmI) -> Self  // exists but add peer convenience
pub async fn peer_connect<Buff>(...) -> Result<TransportLink<...>, TransportLinkError>
```

`peer_connect()` behaves like `connect()` but sets `WhatAmI::Peer` internally. The `listen()` method already accepts `WhatAmI::Peer` via `with_whatami()` — no change needed there.

### 5. zenoh-nostd — Session API peer entry points

Add:
```rust
pub async fn session_peer_connect<'res, Config>(...) -> Result<Session<'res, Config>, TransportLinkError>
pub async fn session_peer_listen<'res, Config>(...) -> Result<Session<'res, Config>, TransportLinkError>
```

The peer connect variant sets `WhatAmI::Peer` on the transport manager and calls `connect()` with the peer identity. The peer listen variant does the same for `listen()`.

Add macros:
```rust
__session_peer_connect!($CONFIG: $config, $endpoint)
__session_peer_listen!($CONFIG: $config, $endpoint)
```

Export from `zenoh-nostd::session::zenoh::peer_connect` and `peer_listen`.

### 6. zenoh-nostd — Feature flag `peer-mode`

Add `peer-mode` feature to `Cargo.toml`. Gate the peer-specific API surface behind this flag. The underlying transport handshake (`zenoh-sansio`) is not gated — it already works with `WhatAmI::Peer`. The flag only guards the session-level convenience API and prevents accidental peer-mode usage when the user hasn't opted in.

### 7. Config and ZSessionConfig

`ZSessionConfig` already has `transports()` returning `&TransportLinkManager`. The `WhatAmI` is configured on the manager. For convenience, add a `ZSessionConfig::whatami()` method or encourage users to configure it through the manager builder. Since `TransportLinkManager` is constructed from `ZLinkManager`, the config struct should expose `with_whatami()` in its builder.

### 8. Protocol features spec update

Remove "Peer-to-peer handshake (simplified, defer to future)" from the `REMOVED Requirements` section of `openspec/specs/protocol-features/spec.md`. Add an `ADDED` requirement for peer mode.

## Capabilities

### New

- `peer-transport` — Peer-to-peer TCP unicast transport with mutual session negotiation and simultaneous open resolution

### Modified

- None (new capability; no existing capability is changed)

## Impact

| Crate | Change | Risk |
|-------|--------|------|
| `zenoh-proto` | Property tests + golden vector for peer-mode InitSyn/InitAck | None (messages already support Peer) |
| `zenoh-sansio` | `peer_connect()`/`peer_listen()` convenience methods on `TransportBuilder`; expanded test suite | Low (wraps existing paths) |
| `zenoh-nostd` | New session entry points, macros, feature flag, config builder extension | Low (additive, gated) |
| `openspec/protocol-features` | Promote from removed to added | None |

## Phasing

| Phase | Scope | Deliverable |
|-------|-------|-------------|
| 1 | zenoh-proto tests + golden vectors | Verify peer-mode InitSyn/InitAck roundtrip |
| 2 | zenoh-sansio convenience methods + tests | `peer_connect()`/`peer_listen()`, simultaneous open test |
| 3 | zenoh-nostd API + feature flag | `session_peer_connect()`, macros, config, `peer-mode` flag |
| 4 | Protocol features spec update | Move peer handshake from REMOVED to ADDED |