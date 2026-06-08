# Feature Support Matrix

This document tracks which upstream Eclipse Zenoh features are supported by `zenoh-nostd`.

> **Note**: As of June 2026, all async removed — zenoh-nostd is fully synchronous, poll-driven IO. No async runtime required.

---

## Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Fully supported, tested |
| 🔶 | Partially supported or in progress |
| ❌ | Not yet implemented |
| ⬜ | Not applicable or not in upstream |

---

## Formal Verification

| Tool | Status | Contracts/Proofs | CI | Location |
|------|--------|------------------|----|----------|
| Kani | ✅ Active | 69 proofs | ✅ `ci.yml` — kani job | `kani_proofs.rs`, `transport/kani_*.rs` |
| Creusot | ✅ Active | 18 contract annotations | ✅ `ci.yml` — creusot job (continue-on-error) | `src/transport/creusot_contracts.rs`, `zenoh-proto/src/codec/struct.rs` |
| Verus | ✅ Active | 6 modules, 40+ property tests | ✅ `ci.yml` — verus job (continue-on-error) | `src/verus_session.rs`, `src/verus_bounds.rs`, `src/verus_cursor.rs`, `src/verus_proofs.rs`, `src/verus_transport_split.rs` |

---

## Core Protocol

| Feature | Upstream | nostd | Tests | Fuzz | Kani | Notes |
|---------|----------|-------|-------|------|------|-------|
| Variable-Length Encoding (VLE) | ✅ | ✅ | ✅ | ✅ | ✅ | Roundtrip, truncated, bounds |
| Key Expression | ✅ | ✅ | ✅ | ✅ | ✅ | Parse, encode, decode |
| ZBuf (zero-copy buffer) | ✅ | ✅ | ✅ | ❌ | ✅ | Bounds-checked |
| Extension headers (Unit/U64/ZStruct) | ✅ | ✅ | ✅ | ✅ | ✅ | Kind encoding, roundtrip |

---

## Message Types

| Message | Upstream | nostd | Tests | Fuzz | Kani | Notes |
|---------|----------|-------|-------|------|------|-------|
| Declare (Subscriber) | ✅ | ✅ | ✅ | ✅ | ✅ | |
| Declare (Queryable) | ✅ | ✅ | ✅ | ✅ | ✅ | |
| Declare (Token) | ✅ | ✅ | ✅ | ✅ | ✅ | Liveliness |
| Declare (Publisher) | ✅ | ✅ | ❌ | ❌ | ❌ | |
| Undeclare | ✅ | ✅ | ✅ | ❌ | ✅ | |
| Interest | ✅ | ✅ | ✅ | ❌ | ❌ | |
| Init (Syn/Ack) | ✅ | ✅ | ✅ | ✅ | ✅ | Session handshake |
| Open (Syn/Ack) | ✅ | ✅ | ✅ | ✅ | ✅ | Session handshake |
| Close | ✅ | ✅ | ✅ | ✅ | ✅ | |
| Data (Pub/Sub payload) | ✅ | ✅ | ✅ | ✅ | ✅ | |
| Unit (Keep-alive) | ✅ | ✅ | ✅ | ❌ | ✅ | |
| Request | ✅ | ✅ | ✅ | ✅ | ❌ | Query/reply — encode, decode, tests |
| Response | ✅ | ✅ | ✅ | ✅ | ❌ | Query/reply — encode, decode, tests |
| ResponseFinal | ✅ | ✅ | ✅ | ✅ | ❌ | Query/reply — encode, decode, tests |

---

## Transport Layer

| Feature | Upstream | nostd | Tests | Fuzz | Kani | Notes |
|---------|----------|-------|-------|------|------|-------|
| Batch encoding | ✅ | ✅ | ✅ | ✅ | ✅ | |
| Batch decoding | ✅ | ✅ | ✅ | ✅ | ✅ | |
| Sequence numbers | ✅ | ✅ | ✅ | ✅ | ✅ | Monotonicity, wrap |
| Fragmentation | ✅ | ✅ | ✅ | ❌ | ✅ | |
| Reassembly | ✅ | ✅ | ✅ | ❌ | ✅ | |
| Reliability (NACK) | ✅ | ✅ | ✅ | ❌ | ❌ | NACK generation, retransmission |
| Liveliness probe | ✅ | ✅ | ✅ | ❌ | ❌ | |
| Keep-alive | ✅ | ✅ | ✅ | ❌ | ❌ | |
| Close handshake | ✅ | ✅ | ✅ | ✅ | ❌ | |
| Serial/UART transport | ✅ | ✅ | ✅ | ❌ | ❌ | Framed serial protocol, USB CDC |
| BLE transport | ✅ | ✅ | ✅ | ❌ | ❌ | BlueZ GATT on Linux |
| TLS transport | ✅ | ✅ | ✅ | ❌ | ❌ | Encrypted transport, mTLS support |
| Raw Ethernet transport | ✅ | ✅ | ✅ | ❌ | ❌ | Ethernet II frames, configurable EtherType |
| Auto-reconnect | ✅ | ✅ | ✅ | ❌ | ❌ | Exponential backoff, max retries |

---

## Session

| Feature | Upstream | nostd | Tests | Fuzz | Kani | Notes |
|---------|----------|-------|-------|------|------|-------|
| Client handshake | ✅ | ✅ | ✅ | ✅ | ❌ | |
| Server handshake | ✅ | ✅ | ✅ | ✅ | ❌ | |
| Peer handshake | ✅ | ✅ | ✅ | ✅ | ❌ | |
| Session state machine | ✅ | ✅ | ✅ | ❌ | ✅ | Kani on transport SM |
| Lease management | ✅ | ✅ | ✅ | ❌ | ❌ | |
| Session check (ping/pong) | ✅ | ✅ | ✅ | ❌ | ❌ | |
| Multicast declarations | ✅ | ✅ | ✅ | ❌ | ❌ | Declare/undeclare to all peers |
| Mesh routing | ✅ | ❌ | ❌ | ❌ | ❌ | Not yet implemented |

---

## Advanced Features

| Feature | Upstream | nostd | Tests | Fuzz | Kani | Notes |
|---------|----------|-------|-------|------|------|-------|
| Access control | ✅ | ✅ | ✅ | ❌ | ❌ | Static rules, FromStr parsing, integration |
| Authentication (user/password) | ✅ | ✅ | ✅ | ❌ | ❌ | Username/password during handshake |
| Authentication (TLS client cert) | ✅ | ✅ | ✅ | ❌ | ❌ | mTLS certificate authentication |
| Liveliness API | ✅ | ✅ | ✅ | ✅ | ❌ | Token declare/undeclare, subscriber, lease |
| Query API | ✅ | ✅ | ✅ | ✅ | ❌ | Request/Response/ResponseFinal, roundtrip |
| NACK reliability | ✅ | ✅ | ✅ | ❌ | ❌ | NACK generation and retransmission |
| Admin space | ✅ | ❌ | ❌ | ❌ | ❌ | Not yet implemented |
| Plugins | ✅ | ❌ | ❌ | ❌ | ❌ | Not applicable (no_std) |
| EVAL | ✅ | ❌ | ❌ | ❌ | ❌ | Not yet implemented |

---

## Platform Support

| Platform | nostd | Status | Notes |
|----------|-------|--------|-------|
| std (Linux/Mac/Windows) | ✅ | Working | TCP/UDP, TLS, BLE (BlueZ/Linux), raw Ethernet |
| Linux embedded (armv7, aarch64) | ✅ | Working | POSIX sockets (TCP/UDP) |
| STM32 (thumbv7m) | ✅ | Verified | QEMU + hardware; POSIX sockets on supported boards |
| ESP32 (riscv32) | ✅ | Verified | ESP-IDF + esp-wifi; POSIX sockets via LwIP |
| WASM | ✅ | Working | WebSocket via yawc |

## Priority for Next Implementation

With all zenoh-pico parity features now implemented, future work should focus on:

1. Golden vectors for all message types
2. Mesh routing
3. EVAL (remote procedure evaluation)
4. Admin space
5. Additional formal verification (Kani/Creusot/Verus coverage)
6. Fuzz coverage expansion