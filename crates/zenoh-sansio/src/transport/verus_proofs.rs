// Verus proofs for transport protocol verification.
//
// Verus (https://github.com/verus-lang/verus) is a SMT-based Rust verifier
// that uses a Rust-like syntax extended with proof annotations.
//
// To use:
//   1. Install Verus: follow instructions at https://github.com/verus-lang/verus
//   2. Add Verus dependencies: builtin, builtin_macros, vstd
//   3. Write proofs using `verus! { ... }` macro or `.vrs` files
//   4. Run: verus crate --features verus
//
// Example proof skeleton:
//   verus! {
//     proof fn add_commutative(x: u32, y: u32) {
//       ensures add(x, y) == add(y, x);
//     }
//   }
