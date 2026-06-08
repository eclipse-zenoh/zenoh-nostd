# Protocol, Auth, and Resilience Audit: zenoh-nostd vs zenoh-pico

## 1. Publish / Subscribe

### `put.rs`
- **Status**: Clean
- **unwraps/expects/panics**: None in production code
- Uses `?` for all error propagation
- Constructs correct `Push`/`Put` wire messages
- No feature completeness gaps

### `sub.rs`
- **Status**: Clean
- **unwraps/expects/panics**: None in production code
- Undeclare sends `UndeclareSubscriber` with correct ID
- Callback system uses fallible `insert()` with `?`
- No gaps vs zenoh-pico subscriber API

## 2. Query / Reply

### `get.rs`
- **Status**: Clean, with one minor concern
- `get.rs:151`: `self.parameters.unwrap_or_default()` — not a panic (provides default), but unwrap-adjacent pattern
- Uses `?` for error propagation everywhere else
- Correctly constructs `Request` → `Query` wire format
- `drop_timedout()` called before insert

### `queryable.rs`
- **Status**: Clean
- No unwraps/expects/panics in production code
- Correctly sends `DeclareQueryable` / `UndeclareQueryable`
- Callback registration is fallible
- No feature completeness gaps

## 3. Liveliness

### `liveliness.rs`
- **Status**: Clean
- No unwraps/expects/panics in production code
- `declare_liveliness_token` sends `DeclareToken`
- `undeclare_liveliness_token` sends `UndeclareToken`
- Liveliness subscriber sends `Interest` with `InterestMode::CurrentFuture`
- Undeclare uses `InterestMode::Final`
- Matches zenoh-pico behavior

## 4. Scouting

### `scout.rs:82`
- `unwrap_or_default()` for ZenohIdProto conversion — safe but unwrap-adjacent
- No panic paths in production
- Tests cover: encode, decode, buffer-too-small, invalid input, default

## 5. Session State

### `session.rs:has_subscriber()` / `has_queryable()`
- **Status**: Correct
- Uses `ke.intersects()` for key expression matching
- Feature-gated behind `session-check`
- Tests verify empty-state returns false

## 6. Multicast Declarations

### Feature flag: `multicast-declarations`
- **Status**: ❌ DEAD FEATURE FLAG
- Defined in `crates/zenoh-nostd/Cargo.toml:35` as `multicast-declarations = []`
- **Never referenced** in any `#[cfg(feature = "multicast-declarations")]` attribute in any `.rs` file
- There is a `platforms/zenoh-std/src/mcast.rs` for UDP multicast links, but it is unrelated to the protocol-level `DeclarePublisher`/`UndeclarePublisher` messages
- **Remediation**: Either implement the feature or remove the dead flag
- Note: `DeclarePublisher` and `UndeclarePublisher` message types are defined in `zenoh-proto` but there is no API in zenoh-nostd to use them

## 7. NACK

### `nack.rs` (message definition)
- **Status**: Clean
- Simple struct with `sn: u32`, correct wire header ID `0x12`
- No unwraps/panics

### `tx.rs:handle_nack()`
- **Status**: Correct
- Feature-gated under `reliability`
- Searches `send_buf` for matching SN using `iter().find()`
- Returns `false` if not found (no panic)
- Returns `false` if buffer insufficient for retransmission
- Copies frame data and clears `last_frame`

### `rx.rs:drain_nacks()`
- **Status**: Correct
- Returns bounded `heapless::Vec<u32, MAX_PENDING_NACKS>`
- Drains all pending NACKs in FIFO order

### NACK generation in `rx.rs:decode()`
- When `SN` jump detected (`diff > 0` and `diff < 2^31`), all missing SNs enqueued
- Bounded by `MAX_PENDING_NACKS` (8) — drops if full rather than unbounded growth
- Correct behavior

## 8. Authentication

### `TransportBuilder::with_auth()`
- **Status**: Implemented and correctly wired
- Format: `0x01` prefix + username bytes + `0x00` separator + password bytes
- Stored as `heapless::Vec<u8, 256>` (bounded)
- Feature-gated under `auth`

### Auth flow: Config → Wire
1. `TransportBuilder.connect()` → constructs `InitSyn { auth: Some(Auth { payload }) }` (transport.rs:319-323)
2. Uses `unsafe { core::mem::transmute }` for lifetime extension — SAFETY comment documents the invariant
3. `Handshake::PendingInit` variant stores `auth_bytes: Option<heapless::Vec<u8, 256>>`
4. `establishment.rs:State::poll(InitSyn)` → constructs `InitAck` with `ack_auth` from `mine_auth` (line 122-130)
5. Another `unsafe { core::mem::transmute }` in establishment.rs:128

### Auth verification
- **No actual authentication validation is performed**
- Auth payload is passed through the handshake but never verified by either side
- This matches zenoh-pico behavior (pass-through)

### Credential exposure in `with_auth_payload`
- `io/transport.rs:165,205,247,289,331,373`: `builder = builder.with_auth_payload(p.clone());`
- Clones the auth payload on every bridge_connect/listen call — no dedup or zeroing
- Acceptable for embedded use but should be noted

## 9. TLS Client Certificate

### `platforms/zenoh-std/src/tls.rs:connect_with_cert()`
- **Status**: Works correctly
- Uses `native_tls::Identity::from_pkcs12(der, password)`
- Proper error handling (all errors mapped to `LinkError::CouldNotConnect`)
- No unwraps/panics
- **Limitations**: 
  - std-only (requires `native_tls`, `std::net::TcpStream`)
  - No embedded TLS support
  - No feature flag in zenoh-nostd itself
  - Uses `Arc<Mutex<TlsStream>>` — single-threaded bottleneck

## 10. Auto-Reconnect

### `auto_reconnect.rs`
- **Status**: Implemented as standalone utility, **NOT integrated into session lifecycle**
- `run_with_retry()` is exported but `Session::run()` in `run.rs` does not use it
- Exponential backoff: `base * 2^min(attempt-1, 16)` capped at `max_delay_ms`
- Jitter: LCG-based pseudo-random ±25%
- `is_retryable()` correctly identifies retryable errors
- **Remediation**: Consider integrating into `Session::run()` or providing a higher-level wrapper

## 11. Keepalive

### `tx.rs:should_send_keepalive()`
- **Status**: Correct
- Sends when `now > last_sent + lease/4` (matches zenoh-pico)
- `tx.rs:keepalive()` sends `TransportMessage::KeepAlive`

### `rx.rs:should_close()`
- **Status**: Correct
- Closes when `now > last_received + lease`
- Matches zenoh-pico lease expiry behavior

## 12. NACK Retransmission Flow

### State machine integration
- `tx.rs:handle_nack()` is part of the transport state machine
- NACKs received → decoded as `NetworkBody::Nack` → routed to tx `handle_nack()`
- Flow works end-to-end

## 13. Feature Flag Connectivity

| Flag | Defined | Used in `#[cfg()]` | Status |
|------|---------|-------------------|--------|
| `auth` | zenoh-nostd + zenoh-sansio | Yes | ✅ Wired |
| `peer-mode` | zenoh-nostd | Yes (`session.rs`) | ✅ Wired |
| `session-check` | zenoh-nostd | Yes (`session.rs`) | ✅ Wired |
| `multicast-declarations` | zenoh-nostd | **No** | ❌ Dead |
| `reliability` | zenoh-nostd + zenoh-sansio | Yes (tx.rs, rx.rs) | ✅ Wired |
| `scouting` | zenoh-nostd | Yes (scout.rs) | ✅ Wired |

## 14. Panic/Unwrap/Expect Hotlist

### Production code `unreachable!()`:
| Location | Context |
|----------|---------|
| `zenoh-sansio/src/transport/handshake.rs:70` | After `core::mem::replace` in `HandshakeReady::open()` — safe in practice |
| `zenoh-sansio/src/transport/handshake.rs:143` | After `core::mem::replace` in `poll()` — safe in practice |
| `zenoh-sansio/src/transport/handshake.rs:168` | After `core::mem::replace` in `poll()` — safe in practice |
| `zenoh-sansio/src/transport/handshake.rs:241` | After `core::mem::replace` in `poll_async()` — safe in practice |
| `zenoh-sansio/src/transport/handshake.rs:266` | After `core::mem::replace` in `poll_async()` — safe in practice |
| `zenoh-proto/src/msgs/fields.rs:806` | Wildcard in enum match — could panic on unexpected variant |

### Production code `expect()`:
| Location | Context |
|----------|---------|
| `zenoh-sansio/src/transport/rx.rs:252` | `reader.read_u8().expect("reader should not be empty at this stage")` — **could panic on malformed input** |

### `unwrap_or_default()` patterns:
| Location | Context |
|----------|---------|
| `scout.rs:82` | `ZenohIdProto::try_from(z).unwrap_or_default()` |
| `get.rs:151` | `self.parameters.unwrap_or_default()` |

## Summary

### Strengths
- All production pub/sub, query/reply, liveliness paths are panic-free
- NACK and keepalive are correctly implemented
- Auth flows from config through wire handshake
- Bounded buffers everywhere (heapless-based)
- Error propagation uses `?` consistently

### Critical Issues
1. **`rx.rs:252`**: `expect()` in production decode path — will panic on malformed input if reader is empty
2. **`fields.rs:806`**: `unreachable!()` in production code — panic on unexpected enum variant
3. **`multicast-declarations`**: Dead feature flag — defined but never used

### Moderate Issues
4. **Auto-reconnect not integrated**: `run_with_retry()` is standalone, `Session::run()` doesn't use it
5. **5x `unreachable!()` in handshake.rs**: Safe in practice but violates the "no panics" policy
6. **Auth is pass-through**: No verification performed (matches zenoh-pico baseline)
7. **No TLS for embedded**: TLS only exists in `platforms/zenoh-std`

### Minor Issues
8. Auth payload cloned per connect/listen invocation
9. Some `unwrap()`/`expect()` in test code (acceptable, but many instances)