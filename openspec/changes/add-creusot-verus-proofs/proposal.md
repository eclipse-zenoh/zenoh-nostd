# Change: Add Creusot and Verus formal verification

## Why

Formal verification is the highest assurance layer in our verification plan (Layer 5/6). Currently Kani provides 69 bounded-model-checking proofs, but Creusot (deductive verification for pure functions) and Verus (full program verification for critical kernels) are not started. Adding these tools will provide contract-based verification for length calculations, bounds checks, and state machine correctness.

## What Changes

- Add Creusot contract annotations (`#[requires]`, `#[ensures]`, `#[invariant]`) to ZLen/ZBodyLen implementations
- Add Creusot CI check: `cargo creusot --features creusot`
- Add Verus verification for session state machine transitions
- Add Verus verification for packet length invariants
- Add Verus verification for parser cursor progress
- Update verification_plan.md with proof targets
- Spec delta: formal-verification (ADDED: Creusot/Verus requirements)

## Impact

- Affected specs: formal-verification (Creusot/Verus sections)
- Affected code: Length calculation functions, session state machine, encoder cursor logic
- Risk: Low — Creusot/Verus are additive (no production code changes)
- Effort: Medium — Creusot/Verus require specific toolchain setup