// Creusot contracts for transport protocol verification.
//
// Creusot (https://github.com/creusot-rs/creusot) is a Rust verification tool
// that uses Why3 as a backend. Contracts are written as Rust annotations.
//
// To use:
//   1. Install `cargo-creusot`: cargo install cargo-creusot
//   2. Add `creusot-contracts = "..."` to [dependencies]
//   3. Write contracts using `#[creusot::spec]`, `#[creusot::invariant]`, etc.
//   4. Run: cargo creusot --features creusot
//
// Example contract skeleton:
//   #[creusot::ensures(result == x + y)]
//   fn add(x: u32, y: u32) -> u32 { x + y }
