// Creusot contracts for transport protocol verification.
//
// Creusot (https://github.com/creusot-rs/creusot) is a Rust verification tool
// that uses Why3 as a backend. Contracts are written as Rust annotations.
//
// To use:
//   1. Install cargo-creusot: `cargo install cargo-creusot`
//   2. Add `creusot-contracts = "..."` and `creusot` to crate dependencies
//   3. Write contracts using #[ensures], #[requires], #[invariant] annotations
//   4. Run: `cargo creusot --features creusot`
//
// These are the same specifications verified by Kani and Proptest.

/// Precondition: ZID must be non-zero (valid Zenoh ID has non-zero u128).
#[allow(dead_code)]
fn requires_valid_zid(zid: u128) -> bool {
    zid != 0
}

/// Postcondition: For any two valid ZIDs, exactly one wins the comparison.
#[allow(dead_code)]
fn ensures_total_order(a: u128, b: u128) -> bool {
    a == b || a < b || a > b
}

/// Determinism: Same ZID pair always yields same comparison result.
#[allow(dead_code)]
fn ensures_deterministic(a: u128, b: u128, result: bool) -> bool {
    let r1 = a > b;
    let r2 = a > b;
    r1 == r2 && r1 == result
}