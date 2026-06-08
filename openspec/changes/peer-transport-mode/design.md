## Context

zenoh-nostd currently only supports client-server topology. The peer-to-peer unicast TCP mode is required for mesh and decentralized topologies. The underlying infrastructure already supports it:

- `WhatAmI::Peer` variant exists in `zenoh-proto`
- `TransportBuilder::with_whatami(WhatAmI::Peer)` works but is manual
- The establishment state machine in `zenoh-sansio` already handles simultaneous open (ZID-based tie-breaking)
- The `TransportLinkManager` in `zenoh-nostd` already has `with_whatami()` but no peer-specific API

After the async removal (completed June 2026), all methods are synchronous and poll-driven. The peer API must follow the same pattern.

## Goals / Non-Goals

**Goals:**
- Add `peer_connect()` and `peer_listen()` convenience methods to `TransportBuilder` in `zenoh-sansio`
- Add `peer_connect()` and `peer_listen()` to `TransportLinkManager` in `zenoh-nostd`
- Add `session_peer_connect()` and `session_peer_listen()` free functions
- Add `__session_peer_connect!` and `__session_peer_listen!` macros
- Add `peer-mode` feature flag to `zenoh-nostd`
- Add property tests and golden vectors for peer InitSyn/InitAck
- Update protocol features spec

**Non-Goals:**
- Mesh routing (separate feature)
- Peer-to-peer multicast (uses different transport path)
- Peer discovery (scouting is separate)

## Decisions

The design mirrors the existing `connect()`/`listen()` patterns exactly, with `WhatAmI::Peer` pre-set. No new state machine logic is needed — the sansio layer already handles peer simultaneous open correctly.

All methods are synchronous following the recent async removal.

## Migration Plan

Additive only — no existing behavior changes. New API is feature-gated behind `peer-mode`.

## Open Questions

None.