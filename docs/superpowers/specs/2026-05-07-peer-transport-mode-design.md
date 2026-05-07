# Peer Transport Mode

Add `WhatAmI::Peer` transport mode to enable direct peer-to-peer zenoh sessions without a router. This is the foundational layer for drone mesh networking on STM32.

## Motivation

`zenoh-nostd` currently only operates in `WhatAmI::Client` mode — every session requires a router to broker communication. For embedded mesh scenarios (drones, sensor networks), nodes need to talk directly to each other.

The Zenoh protocol already supports peer mode at the wire level. `WhatAmI` is a field in `InitIdentifier` carried by every `InitSyn`/`InitAck` message. The codec already handles it. Only the transport state machine lacks peer awareness.

## Protocol Compliance

Per [Zenoh spec §Roles](https://spec.zenoh.io/spec/1.0.0/architecture/roles.html):

> Peer (WhatAmI = 0b01): Connects directly to other Peers and/or Routers. Participates in peer–peer scouting on local network segments.

`WhatAmI` appears in INIT SYN/ACK — the handshake message sequence is role-agnostic. The only protocol addition for peer mode is handling **simultaneous open**: both peers send `InitSyn` at once. The spec resolves this by ZID comparison — the initiator with the lower ZID wins; the higher ZID yields and becomes the acceptor.

## Design

### Architecture

No new traits. Peer mode is a configuration value threaded through the existing builder chain:

```
User configures TransportLinkManager.with_whatami(Peer)
  → TransportLinkManager passes whatami to TransportBuilder
    → TransportBuilder sets WhatAmI in InitIdentifier
      → Establishment state machine handles simultaneous open
```

### Component Changes

#### 1. `TransportBuilder` (zenoh-sansio/src/transport.rs)

| Field | Type | Default |
|-------|------|---------|
| `whatami` | `WhatAmI` | `Client` |

Methods:
- `with_whatami(self, whatami: WhatAmI) -> Self`

`connect()`/`connect_async()`: construct `InitIdentifier { zid, whatami, .. }` instead of using `..Default::default()`.

`listen()`/`listen_async()`: pass `mine_whatami` to `State::WaitingInitSyn`.

#### 2. Establishment State Machine (zenoh-sansio/src/transport/establishment.rs)

Add `mine_whatami: WhatAmI` to states: `WaitingInitSyn`, `WaitingOpenSyn`, `WaitingInitAck`.

Fix `InitAck` creation — currently uses `InitIdentifier { zid: mine_zid, ..Default::default() }` which always resolves to `WhatAmI::Client`. Must set `whatami: mine_whatami`.

Simultaneous open in `State::poll()` — when `WaitingInitAck` receives `InitSyn`:

```
mine_zid > theirs  → yield: send InitAck, transition to WaitingOpenSyn
mine_zid < theirs  → win:   discard InitSyn, stay in WaitingInitAck
mine_zid == theirs → error: identical ZIDs (should never happen)
```

The comparison uses `ZenohIdProto: Ord` (derived) — no magic constants.

#### 3. TransportLinkManager (zenoh-nostd/src/io/transport.rs)

| Field | Type | Default |
|-------|------|---------|
| `whatami` | `WhatAmI` | `Client` |

Methods:
- `with_whatami(self, whatami: WhatAmI) -> Self`

Every `connect`/`listen`/`bridge_connect`/`bridge_listen` method chains `.with_whatami(self.whatami)` on the `TransportBuilder`.

#### 4. Session API (zenoh-nostd/src/api/session.rs)

No changes. `session_connect()` and `session_listen()` delegate to `config.transports()`, which already knows its `whatami`. Peer behavior is determined by how the `TransportLinkManager` is configured, not by which API function you call.

### Error Handling

| Scenario | Error |
|----------|-------|
| Identical ZIDs in simultaneous open | `TransportError::InvalidAttribute` |
| Invalid state transition | `TransportError::InvalidState` (existing) |

### Testing

| Test | Crate | Scope |
|------|-------|-------|
| Peer connect+listen handshake | `zenoh-sansio` | Transport handshake completes with `WhatAmI::Peer` in `InitIdentifier` |
| Simultaneous connect — lower ZID wins | `zenoh-sansio` | Both sides `connect()`, lower ZID continues as initiator |
| Simultaneous connect — equal ZIDs error | `zenoh-sansio` | Identical ZIDs produce `InvalidAttribute` |
| Peer pub/sub integration | `zenoh-nostd` | Two peer sessions exchange data end-to-end |

### Example

`examples/z_peer.rs` — one peer listens on `tcp/127.0.0.1:7444`, another connects. Pub/sub between peers without a router. Uses the spawn pattern (session in background task, operations in main).

## Out of Scope

- Scouting/discovery (SCOUT/HELLO messages)
- Multi-peer session management (connecting to 3+ peers)
- Message forwarding between peers (routing)
- Fragmentation, liveliness, connection recovery

These are follow-up PRs that build on this foundation.
