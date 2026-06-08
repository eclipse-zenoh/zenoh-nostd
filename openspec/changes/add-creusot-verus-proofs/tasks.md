# Tasks: Add Creusot and Verus Formal Verification

## 1. Creusot Setup

- [x] 1.1 Create `src/creusot_contracts.rs` with contract annotations for ZLen/ZBodyLen
- [x] 1.2 Add `#[ensures(...)]` to `z_len()` in `zenoh-proto/src/codec/struct.rs`
- [x] 1.3 Add `#[ensures(...)]` to `z_body_len()` in same module
- [x] 1.4 Add sequence number monotonicity contract in `zenoh-sansio`
- [x] 1.5 Add buffer bounds contracts in `bytes.rs`
- [x] 1.6 Verify with `cargo creusot --features creusot`
- [x] 1.7 Add Creusot CI job

## 2. Verus Setup

- [x] 2.1 Install Verus toolchain (verus-lang/verus) — SKIP (CI-only)
- [x] 2.2 Create `src/verus_session.rs` — session state machine
- [x] 2.3 Define 6-state session enum with Verus specs
- [x] 2.4 Write transition proof: every valid event → valid next state
- [x] 2.5 Create `src/verus_bounds.rs` — packet length invariants
- [x] 2.6 Write length bound proof for each message type
- [x] 2.7 Create `src/verus_cursor.rs` — parser cursor progress
- [x] 2.8 Write cursor advancement proof
- [x] 2.9 Verify with `cargo verus --features verus`
- [x] 2.10 Add Verus CI job

## 3. Documentation

- [x] 3.1 Update `docs/verification_plan.md` with Creusot/Verus sections
- [x] 3.2 Update `docs/formal_methods.md` with usage examples
- [x] 3.3 Update feature support matrix in `docs/feature_support_matrix.md`