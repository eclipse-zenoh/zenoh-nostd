# Traceability Matrix

This document traces requirements through implementation, testing, and verification evidence.

---

## Specification to Implementation

| Req ID | Requirement | Implementation | Tests | Verification |
|--------|-------------|----------------|-------|-------------|
| R1 | VLE encode/decode | `codec/struct/uint.rs` | Unit + Proptest | Kani |
| R2 | Key expression parse | `ke.rs` | Unit + Proptest | Kani |
| R3 | Transport batch encode | `transport/` | Unit + Proptest | Kani |
| R4 | Transport batch decode | `transport/` | Unit + Proptest | Kani + Fuzz |
| R5 | Session handshake | `session/` | Unit | Fuzz |
| R6 | Fragment/reassemble | `transport/fragment.rs` | Unit | Kani |
| R7 | Sequence numbers | `transport/sn.rs` | Unit + Verus test | Kani |
| R8 | Extension headers | `codec/ext.rs` | Unit + Proptest | Kani |
| R9 | Error types | `zerror.rs` | Unit | — |
| R10 | no_std compatibility | All crates | CI gate | — |
| R11 | Query/Reply protocol | `zenoh-proto/src/messages/` | Unit + Proptest | — |
| R12 | Liveliness API | `session/liveliness.rs` | Unit + Proptest | — |
| R13 | Access control | `session/acl.rs` | Unit | — |
| R14 | Peer-to-peer handshake | `session/peer_handshake.rs` | Unit | Fuzz |
| R15 | Serial/UART transport | `transport/serial.rs` | Unit | — |
| R16 | BLE transport (BlueZ) | `transport/ble.rs` | Unit | — |
| R17 | TLS transport | `transport/tls.rs` | Unit | — |
| R18 | Raw Ethernet transport | `transport/ethernet.rs` | Unit | — |
| R19 | Auto-reconnect | `session/reconnect.rs` | Unit | — |
| R20 | Session check (ping/pong) | `session/session_check.rs` | Unit | — |
| R21 | Multicast declarations | `session/multicast_declare.rs` | Unit | — |
| R22 | User/password authentication | `session/auth.rs` | Unit | — |
| R23 | TLS client certificate auth | `transport/tls.rs` | Unit | — |
| R24 | NACK reliability | `transport/nack.rs` | Unit | — |

---

## Verification Coverage by Module

| Module | Unit | Proptest | Fuzz | Kani | Creusot | Verus |
|--------|------|----------|------|------|---------|-------|
| `codec/struct/` | ✅ | ✅ | ❌ | ✅ | ❌ | ❌ |
| `codec/ext/` | ✅ | ✅ | ❌ | ✅ | ❌ | ❌ |
| `ke.rs` | ✅ | ✅ | ❌ | ✅ | ❌ | ❌ |
| `transport/decode.rs` | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| `transport/encode.rs` | ✅ | ✅ | ❌ | ✅ | ❌ | ❌ |
| `transport/fragment.rs` | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ |
| `transport/sn.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `transport/nack.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `transport/serial.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `transport/ble.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `transport/tls.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `transport/ethernet.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `session/handshake.rs` | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ |
| `session/state.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `session/peer_handshake.rs` | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ |
| `session/reconnect.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `session/session_check.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `session/multicast_declare.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `session/auth.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `session/acl.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `session/liveliness.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `bytes.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |

---

## Requirements to Evidence

| Requirement | Unit Test | Property Test | Fuzz | Kani | Document |
|-------------|-----------|---------------|------|------|----------|
| No panics on any input | ✅ | ✅ | ✅ | ✅ | `fuzzing.md` |
| No buffer overflow | ✅ | ❌ | ❌ | ✅ | `safety_case.md` |
| Deterministic encode | ✅ | ✅ | ❌ | ✅ | `no_std_constraints.md` |
| Deterministic decode | ✅ | ✅ | ✅ | ✅ | `verification_plan.md` |
| State machine safety | ✅ | ❌ | ❌ | ✅ | `safety_case.md` |
| SN monotonicity | ✅ | ❌ | ❌ | ❌ | `safety_case.md` |
| Wire compatibility | ❌ | ❌ | ❌ | ❌ | `protocol_compatibility.md` |
| Query/Reply roundtrip | ✅ | ✅ | ❌ | ❌ | `protocol_compatibility.md` |
| Liveliness declaration | ✅ | ✅ | ❌ | ❌ | — |
| Access control enforcement | ✅ | ❌ | ❌ | ❌ | `safety_case.md` |
| Peer handshake | ✅ | ❌ | ✅ | ❌ | `verification_plan.md` |
| NACK retransmission | ✅ | ❌ | ❌ | ❌ | — |
| Auth credential validation | ✅ | ❌ | ❌ | ❌ | — |
| Transport reconnect | ✅ | ❌ | ❌ | ❌ | — |
| Session ping/pong | ✅ | ❌ | ❌ | ❌ | — |
| Multicast declaration | ✅ | ❌ | ❌ | ❌ | — |

---

## Risk to Evidence

| Risk | Evidence Type | Evidence Location | Status |
|------|---------------|-------------------|--------|
| Malformed input → panic | Fuzz + Proptest | Fuzz targets, proptest_tests.rs | Mitigated |
| Buffer overflow | Kani | kani_proofs.rs | Mitigated |
| State machine violation | Kani | kani_state_machine.rs | Mitigated |
| SN collision | Analysis + Test | verus_proofs.rs | Documented |
| Protocol incompatibility | Golden vectors | (planned) | Open |
| Integer overflow | Kani | kani_proofs.rs | Mitigated |
| Auth bypass | Unit test | session/auth.rs | Mitigated |
| Peer handshake failure | Fuzz | Fuzz targets | Mitigated |
| NACK infinite retransmit | Unit test | transport/nack.rs | Documented |
| Reconnect storm | Unit test | session/reconnect.rs | Mitigated |