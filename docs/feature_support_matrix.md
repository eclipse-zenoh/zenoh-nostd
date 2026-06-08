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
| Reliability (NACK) | ✅ | 🔶 | 🔶 | ❌ | ❌ | In progress |
| Liveliness probe | ✅ | ✅ | ✅ | ❌ | ❌ | |
| Keep-alive | ✅ | ✅ | ✅ | ❌ | ❌ | |
| Close handshake | ✅ | ✅ | ✅ | ✅ | ❌ | |

---

## Session

| Feature | Upstream | nostd | Tests | Fuzz | Kani | Notes |
|---------|----------|-------|-------|------|------|-------|
| Client handshake | ✅ | ✅ | ✅ | ✅ | ❌ | |
| Server handshake | ✅ | ✅ | ✅ | ✅ | ❌ | |
| Peer handshake | ✅ | ❌ | ❌ | ❌ | ❌ | Not yet implemented |
| Session state machine | ✅ | ✅ | ✅ | ❌ | ✅ | Kani on transport SM |
| Lease management | ✅ | ✅ | ✅ | ❌ | ❌ | |
| Mesh routing | ✅ | ❌ | ❌ | ❌ | ❌ | Not yet implemented |

---

## Advanced Features

| Feature | Upstream | nostd | Tests | Fuzz | Kani | Notes |
|---------|----------|-------|-------|------|------|-------|
| Access control | ✅ | ✅ | ✅ | ❌ | ❌ | Static rules, FromStr parsing, integration |
| Authentication | ✅ | ❌ | ❌ | ❌ | ❌ | Not yet implemented |
| Liveliness API | ✅ | ✅ | ✅ | ✅ | ❌ | Token declare/undeclare, subscriber, lease |
| Query API | ✅ | ✅ | ✅ | ✅ | ❌ | Request/Response/ResponseFinal, roundtrip |
| Admin space | ✅ | ❌ | ❌ | ❌ | ❌ | Not yet implemented |
| Plugins | ✅ | ❌ | ❌ | ❌ | ❌ | Not applicable (no_std) |
| EVAL | ✅ | ❌ | ❌ | ❌ | ❌ | Not yet implemented |

---

## Platform Support

| Platform | nostd | Status | Notes |
|----------|-------|--------|-------|
| std (Linux/Mac/Windows) | ✅ | Working | TCP/UDP via wtx |
| STM32 (thumbv7m) | ✅ | Verified | QEMU + hardware |
| ESP32 (riscv32) | ✅ | Verified | ESP-IDF + esp-wifi |
| WASM | ✅ | Working | WebSocket via yawc |

## Priority for Next Implementation

1. Query/Reply (Request, Response, ResponseFinal)
2. Liveliness API
3. Reliability (NACK-based)
4. Peer-to-peer handshake
5. Golden vectors for all message types
6. Access control