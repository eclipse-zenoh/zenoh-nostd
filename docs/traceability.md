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
| `session/handshake.rs` | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ |
| `session/state.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
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