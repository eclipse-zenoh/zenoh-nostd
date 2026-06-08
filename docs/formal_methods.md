# Formal Methods

## Overview

`zenoh-nostd` employs three formal verification tools targeting different assurance levels:

| Tool | Technique | Scope | Status |
|------|-----------|-------|--------|
| Kani | Bounded model checking | Buffer bounds, state transitions, arithmetic | ✅ Active (69 proofs) |
| Creusot | Deductive verification | Contract-heavy pure functions | ✅ Active (18 contracts) |
| Verus | Full program verification | Critical protocol kernels | ✅ Active (6 modules, 40+ tests) |

---

## Kani (Bounded Model Checking)

Kani is a Rust model checker that verifies safety properties for bounded program executions. It checks for:

- Panic freedom
- Integer overflow safety
- Array bounds safety
- Assertion violations
- Specific functional correctness properties

### Current Proofs: 69

| Module | Proofs | Coverage |
|--------|--------|----------|
| `kani_proofs.rs` | 16 | VLE, keyexpr, message encode/decode, ext header, body length |
| `transport/kani_state_machine.rs` | 24 | State machine transitions, invariants, fragment reassembly |
| `transport/kani_proofs.rs` | 29 | Batch encode/decode, SN handling, fragment framing, buffer bounds |

### Running Kani

```bash
# Run all proofs for a specific crate
cargo kani -p zenoh-sansio

# Run with verbose output
cargo kani -p zenoh-sansio -- --verbose
```

### Kani Proof Template

```rust
#[kani::proof]
#[kani::unwind(10)]
fn my_proof() {
    // Setup
    let input: [u8; 8] = kani::any();

    // Exercise
    let result = my_function(&input);

    // Assert safety
    if let Ok(val) = result {
        assert!(val <= MAX_VALUE);
    }
}
```

### Proof Checklist

- [ ] Bounded input (use `kani::any()` with arrays or sized types)
- [ ] No assumptions about input validity (proof must hold for all inputs)
- [ ] Assertions for both success and failure paths
- [ ] `#[kani::unwind(N)]` for loops with known upper bound
- [ ] Completion time <60s per proof

---

## Creusot (Deductive Verification)

Creusot verifies Rust programs against formal contracts (preconditions, postconditions, invariants). It is ideal for pure functions with clear mathematical properties.

### Target Functions

- `ZLen::z_len()` — prove returned length ≥ number of bytes to encode
- `ZBodyLen::z_body_len()` — prove body length ≤ total length
- SN arithmetic — prove monotonicity and wrap correctness
- Buffer capacity — prove remaining ≤ total capacity

### Creusot Contract Template

```rust
#[ensures(result >= 0)]
#[ensures(result <= MAX_LEN)]
fn z_len(&self) -> usize {
    // ...
}
```

### Status: ✅ Active — 18 contract annotations

Location: `crates/zenoh-sansio/src/transport/creusot_contracts.rs`

Contract annotations cover:
- Sequence number monotonicity and wrap arithmetic
- Epoch increment correctness (exactly once per SN wrap)
- Batch size and resolution negotiation using `min()`
- ZID total order and determinism
- Buffer bounds (write_u8, write_exact, read_u8, read_exact, read_slice, write_slot) — all verify `remaining` decreases correctly and never underflows

To enable Creusot:
```bash
cargo creusot -p zenoh-sansio --features creusot
```

To check contracts compile:
```bash
cargo check -p zenoh-sansio -p zenoh-proto --features creusot
```

### Contract Reference

```rust
// SN monotonicity: wrapping_add(1) always advances by exactly 1
#[cfg_attr(feature = "creusot", ensures(next == (*sn).wrapping_add(1u32)))]
pub fn sn_increment_monotonic_property(sn: &u32) -> u32;

// Epoch increment iff SN wrap
#[cfg_attr(feature = "creusot", ensures(result.0 == epoch_inc + *epoch && result.1 == new_sn))]
pub fn sn_decode_with_epoch(sn: &mut u32, epoch: &mut u32) -> (u32, u32);

// Buffer bounds: no over-read or over-write
#[cfg_attr(feature = "creusot", requires(*remaining > 0))]
#[cfg_attr(feature = "creusot", ensures(*remaining == *old(remaining) - 1))]
pub fn write_u8_bounded(remaining: &mut usize);

#[cfg_attr(feature = "creusot", requires(*remaining >= len))]
#[cfg_attr(feature = "creusot", ensures(*remaining == *old(remaining) - len))]
pub fn write_exact_bounded(remaining: &mut usize, len: usize);
```

### CI Integration

The Creusot CI job runs with `continue-on-error: true` since the toolchain may not be available in CI:
```bash
cargo creusot --features creusot 2>/dev/null || echo "Creusot verification skipped (tool not available)"
```

---

## Verus (Program Verification)

Verus uses SMT-based verification for full functional correctness of Rust programs. It requires a Verus-aware Rust toolchain.

### Target Kernels

- **Session state machine**: Prove that all valid transitions preserve the state machine invariant
- **Packet length invariants**: Prove that encoded packets always fit within declared bounds
- **Bounded queues**: Prove that push/pop operations preserve queue invariants
- **Parser cursor progress**: Prove that every decode step advances the cursor position

### Verus Proof Template

```rust
verus! {
    proof fn session_transition_preserves_invariant(
        state: SessionState,
        event: Event,
    )
        requires state.is_valid()
        ensures transition(state, event).is_valid()
    {
        // Proof body
    }
}
```

### Status: ✅ Active — 6 modules, 40+ property tests, SMT proof targets

Verus verification modules:

| Module | File | What is proven |
|--------|------|----------------|
| Session state machine | `src/verus_session.rs` | 6-state session lifecycle, transition function, exhaustive test over 84 (state, event) pairs, SMT proof block |
| Packet length bounds | `src/verus_bounds.rs` | VLE encoding ≤10 bytes, z_len() ≥ z_body_len(), MAX_ENCODED_SIZE ≤ u16::MAX, SMT proof for u64 body length bound |
| Transport split | `src/verus_transport_split.rs` | Tx/Rx disjoint fields after split, no data race, link unusable during split |
| Session proofs | `src/verus_proofs.rs` | ZID non-zero, total order, determinism; epoch monotonicity; reassembly key uniqueness across SN wraps |
| Parser cursor progress | `src/verus_cursor.rs` | Cursor pos/limit invariants; advance(n), read_u8, read_slice, read_vle all advance cursor; no over-read; on failure cursor unchanged; VLE malformed detection |
| Transport layer | `src/verus_transport_split.rs` | ZLink::split() ownership and disjointness proofs |

All Verus code is behind `#[cfg(feature = "verus")]` or `#[cfg(any())]` (SMT proof blocks).

To run Verus proofs:
```bash
cargo verus -p zenoh-sansio --features verus
```

To check compilation and run property tests:
```bash
cargo check -p zenoh-sansio --features verus
cargo test -p zenoh-sansio --features verus
```

### Verus Proof Template

```rust
verus! {
    pub proof fn cursor_advance_progress()
        ensures(
            forall(|c: Cursor|
                match c.advance(n) {
                    Ok(_) => c.position() <= c.limit(),
                    Err(_) => true,  // error leaves cursor unchanged
                }
            )
        )
    {
        // Proof body: exhaustive case analysis or induction
    }
}
```

### CI Integration

The Verus CI job runs with `continue-on-error: true`:
```yaml
verus:
    continue-on-error: true
    steps:
      - run: cargo check -p zenoh-sansio --features verus
      - run: cargo verus -p zenoh-sansio --features verus 2>/dev/null || echo "skipped"
```

---

## When to Use Each Tool

| Scenario | Tool | Why |
|----------|------|-----|
| Buffer bounds for encode/decode | Kani | Bounded input, clear safety property |
| State machine transitions | Kani | Finite states, bounded transitions |
| Integer arithmetic (no overflow) | Kani | Bounded integer types |
| Length calculation contracts | Creusot | Mathematical contract, pure function |
| Cursor progress invariant | Verus | Complex loop invariant |
| Full protocol correctness | Verus | Multi-step protocol property |

---

## Integration with CI

```bash
# Kani — runs in CI on nightly
cargo kani -p zenoh-sansio

# Creusot — 18 contract annotations, checked in CI (continue-on-error)
cargo check -p zenoh-sansio -p zenoh-proto --features creusot
cargo creusot -p zenoh-sansio --features creusot

# Verus — 6 modules, 40+ tests, checked in CI (continue-on-error)
cargo check -p zenoh-sansio --features verus
cargo test -p zenoh-sansio --features verus
cargo verus -p zenoh-sansio --features verus
```