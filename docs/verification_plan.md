# Verification Plan

## Overview

This document describes the verification strategy for `zenoh-nostd`, a `no_std` embedded-safe implementation of the Zenoh protocol. Verification spans multiple techniques targeting different assurance levels.

---

## Verification Layers

| Layer | Technique | Scope | Tool |
|-------|-----------|-------|------|
| 1 | Unit tests | Individual functions, edge cases | `cargo test` |
| 2 | Property tests | Protocol invariants, no-panic, roundtrip | `proptest` |
| 3 | Fuzz testing | Malformed wire input, arbitrary bytes | `cargo fuzz` |
| 4 | Model checking | Bounded state spaces, buffer bounds | `Kani` |
| 5 | Deductive verification | Contract-heavy pure functions | `Creusot` |
| 6 | Formal verification | Critical protocol kernels | `Verus` |

---

## Layer 1: Unit Tests

### Coverage targets (≥90% line coverage)

- Packet encoding/decoding for every message type
- Key expression parsing and validation
- Session state transitions (all valid and invalid paths)
- Sequence number arithmetic (wrap, epoch, monotonicity)
- Fragmentation and reassembly
- Reliability window management
- Buffer boundary conditions
- Error type conversions
- Timeout and lease management
- All `From`/`TryFrom` implementations

### Location

- `crates/zenoh-proto/src/tests/` — protocol codec unit tests
- `crates/zenoh-sansio/src/tests/` — transport and session unit tests
- `crates/zenoh-sansio/src/verus_proofs.rs` — Verus-style proof-annotated tests

---

## Layer 2: Property Tests

### Properties verified with `proptest`

| Property | Location | Status |
|----------|----------|--------|
| VLE encode/decode roundtrip | `proptest_tests.rs` | ✅ |
| VLE truncated decode rejected safely | `proptest_tests.rs` | ✅ |
| Keyexpr parser never panics on any input | `proptest_tests.rs` | ✅ |
| Keyexpr rejects non-UTF-8 input | `proptest_tests.rs` | ✅ |
| Ext decoder never panics on any bytes | `proptest_tests.rs` | ✅ |
| Batch decoder never panics | `batch_decode_fuzz.rs` | ✅ |
| Fragment wrap safety | `fragment_wrap_fuzz.rs` | ✅ |
| Message ID audit properties | `message_id_audit.rs` | ✅ |
| Transport state invariants | `transport.rs` | ✅ |

### Desired additions

- Encode(Decode(bytes)) == original for all message types
- Serialized length never exceeds known maximum
- Session epoch increments monotonically
- Sequence numbers never produce duplicate (epoch, sn) keys
- Malformed frames never produce valid decoded messages

---

## Layer 3: Fuzz Testing

### Active fuzz targets

| Target | File | Purpose |
|--------|------|---------|
| `transport_decode` | `fuzz_targets/transport_decode.rs` | Transport-level decode with raw and prefixed paths |
| `establishment` | `fuzz_targets/establishment.rs` | Server-side handshake fuzz |
| `establishment_client` | `fuzz_targets/establishment_client.rs` | Client-side handshake fuzz |
| `keyexpr` | `fuzz_targets/keyexpr.rs` | Key expression parser — arbitrary bytes |
| `decode_frame` | `fuzz_targets/decode_frame.rs` | Frame-level decoder — prefixed and raw decode paths |
| `decode_message` | `fuzz_targets/decode_message.rs` | Individual message type decoders (Put, Del, Declare, KeepAlive, Close, InitSyn, InitAck, OpenSyn, OpenAck, Push, Request) |

### Planned targets

*None — all identified targets are implemented.*

### Fuzz harness requirements

Every fuzz harness must:
1. Never panic on any input
2. Never exhibit undefined behavior
3. Reject invalid input without allocating
4. Return a deterministic result for a given input

---

## Layer 4: Model Checking with Kani

### Current proof count: 69

| Module | Proofs | What is verified |
|--------|--------|------------------|
| `kani_proofs.rs` | 16 | VLE encode/decode bounds, keyexpr bounds, message encode/decode bounds, ext header bounds, body length bounds |
| `transport/kani_state_machine.rs` | 24 | Transport state machine transitions: InitSyn/InitAck, OpenSyn/OpenAck, Close handling, invariants, fragment reassembly |
| `transport/kani_proofs.rs` | 29 | Batch encode/decode bounds, sequence number handling, fragment framing, transport-level buffer bounds |

### Kani proof checklist

Every proof must:
- [ ] Specify bounded inputs (typically `kani::any()` with size constraints)
- [ ] Assert safety properties (no panics, no overflow)
- [ ] Assert functional correctness where possible
- [ ] Use `#[kani::unwind(N)]` for bounded loops
- [ ] Complete within reasonable time (<60s per proof)

---

## Layer 5: Deductive Verification with Creusot

### Target functions

- `ZLen::z_len()` implementations — length bound invariants
- `ZBodyLen::z_body_len()` — body-only length contracts
- Sequence number increment and wrap arithmetic
- Buffer offset and remaining capacity calculations
- Enum discriminant extraction and conversion

### What is verified

| Function | Location | Contract |
|----------|----------|----------|
| `ZBodyLen::z_body_len()` | `zenoh-proto/src/codec/struct.rs:60` | `ensures(result >= 0)` |
| `ZLen::z_len()` | `zenoh-proto/src/codec/struct.rs:147` | `ensures(result >= self.z_body_len())` |
| `sn_increment_monotonic_property` | `zenoh-sansio/src/transport/creusot_contracts.rs:32` | `ensures(next == sn.wrapping_add(1))` |
| `sn_decode_with_epoch` | `zenoh-sansio/src/transport/creusot_contracts.rs:65` | epoch increment iff SN wrap |
| `epoch_monotonic_property` | `zenoh-sansio/src/transport/creusot_contracts.rs:81` | `ensures(result >= *epoch)` |
| `batch_size_negotiation_property` | `zenoh-sansio/src/transport/creusot_contracts.rs:92` | `ensures(result <= mine && result <= theirs)` |
| `zid_comparison_total_property` | `zenoh-sansio/src/transport/creusot_contracts.rs:100` | total order for distinct ZIDs |
| `resolution_negotiation_property` | `zenoh-sansio/src/transport/creusot_contracts.rs:117` | `ensures(result <= mine && result <= theirs)` |
| `write_u8_bounded` | `zenoh-sansio/src/transport/creusot_contracts.rs:131` | `requires(remaining > 0)` → `ensures(remaining == old - 1)` |
| `write_exact_bounded` | `zenoh-sansio/src/transport/creusot_contracts.rs:142` | `requires(remaining >= len)` → `ensures(remaining == old - len)` |
| `read_u8_bounded` | `zenoh-sansio/src/transport/creusot_contracts.rs:151` | `requires(remaining > 0)` → `ensures(remaining == old - 1)` |
| `read_exact_bounded` | `zenoh-sansio/src/transport/creusot_contracts.rs:160` | `requires(remaining >= len)` → `ensures(remaining == old - len)` |
| `read_slice_bounded` | `zenoh-sansio/src/transport/creusot_contracts.rs:169` | `requires(remaining >= len)` → `ensures(remaining == old - len)` |
| `write_slot_bounded` | `zenoh-sansio/src/transport/creusot_contracts.rs:178` | `requires(remaining >= len)` → bounded decrease |

### Running

```bash
# Check contract annotations compile
cargo check -p zenoh-sansio -p zenoh-proto --features creusot

# Full Creusot verification (requires cargo-creusot)
cargo creusot -p zenoh-sansio --features creusot
```

### Status: ✅ Active (18 contract annotations)

---

## Layer 6: Formal Verification with Verus

### Target kernels

- Session state machine — verified lifecycle transitions
- Packet length invariants — total encoded length never exceeds declared maximum
- Bounded queue operations — push/pop never corrupt state
- Parser cursor progress — every decode step advances the cursor

### What is verified

| Module | File | What is proven |
|--------|------|----------------|
| Session state machine | `verus_session.rs` | 6-state enum, 14 events, all valid transitions; |
|                    |       | every valid (state, event) → Ok(next_state) |
| Packet length invariants | `verus_bounds.rs` | VLE encoding ≤10 bytes for u64; z_len() ≥ z_body_len(); |
|                        |       | MAX_ENCODED_SIZE ≤ u16::MAX |
| Transport split | `verus_transport_split.rs` | Tx/Rx disjoint after split; no data race; |
|                |       | link unusable during split |
| Session proofs | `verus_proofs.rs` | ZID non-zero, total order, determinism; |
|               |       | epoch monotonic across wraps; reassembly key uniqueness |
| Parser cursor progress | `verus_cursor.rs` | Cursor struct with pos/limit; |
|                      |       | advance(n) increases pos by n on success; |
|                      |       | no over-read (pos never exceeds limit); |
|                      |       | read_u8/slice/VLE all advance cursor; |
|                      |       | on failure, cursor unchanged |

### Running

```bash
# Check all Verus-gated modules compile
cargo check -p zenoh-sansio --features verus

# Run property tests
cargo test -p zenoh-sansio --features verus

# Full SMT verification (requires Verus toolchain)
# cargo verus verify -p zenoh-sansio --features verus
```

### Status: ✅ Active (6 modules, 40+ property tests, Verus SMT proofs behind #[cfg(any())])

---

## CI Integration

All verification layers must pass in CI:

```bash
cargo fmt --check                          # Code style
cargo clippy --workspace -- -D warnings    # Lint
cargo test --workspace --all-features      # Unit + property tests
cargo test --workspace --no-default-features  # no_std compatibility
cargo llvm-cov --workspace --fail-under-lines 90  # Coverage gate
cargo deny check                           # License + advisory check
cargo audit                                # Vulnerability check
cargo kani                                 # Model checking
```

---

## Coverage Targets

| Metric | Target | Current |
|--------|--------|---------|
| Line coverage | ≥90% | TBD |
| Function coverage | ≥90% | TBD |
| Kani proof count | ≥100 | 69 |
| Fuzz targets | ≥5 | 6 |
| Proptest properties | ≥20 | 28 |
| Golden vectors | ≥50 | 0 |

---

## Residual Risk Documentation

Each feature must document:

```text
Feature: <name>
  Upstream behavior: <reference>
  nostd behavior: <current>
  Unit tests: [passing/failing/missing]
  Proptest: [passing/failing/missing]
  Fuzz target: [present/absent]
  Kani proof: [present/absent]
  Known gaps: <description>
```