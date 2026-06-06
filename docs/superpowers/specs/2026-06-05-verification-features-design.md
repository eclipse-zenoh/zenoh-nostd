# Verification Feature Consolidation

## Overview

Add `kani`, `proptest`, `creusot`, and `verus` Cargo features across the crate hierarchy, allowing users to gate formal verification and property-based testing code. Features propagate from `zenoh-std` → `zenoh-nostd` → `zenoh-sansio`.

## Feature propagation

```
zenoh-std ──→ zenoh-nostd ──→ zenoh-sansio
  kani          kani            kani     ("doc" feature; real gate is cfg(kani))
  proptest      proptest        proptest (optional dev-dep, gates tests)
  creusot       creusot         creusot  (scaffolding: stub module, no dep yet)
  verus         verus           verus    (scaffolding: stub module, no dep yet)
```

## Per-crate changes

### zenoh-sansio (defines features, hosts verification code)

**Cargo.toml:**
- Add features: `kani = []`, `proptest = []`, `creusot = []`, `verus = []`
- Move proptest: `[dev-dependencies] proptest = { version = "1.6", optional = true }`

**src/transport.rs:**
- Gate kani_proofs on `#[cfg(kani)]` (unchanged)
- Add: `#[cfg(feature = "creusot")] mod creusot_contracts;`
- Add: `#[cfg(feature = "verus")] mod verus_proofs;`

**src/tests/transport.rs:**
- Change proptest gate from `cfg(feature = "std")` to `cfg(feature = "proptest")`

**New files:**
- `src/transport/creusot_contracts.rs`: stub module
- `src/transport/verus_proofs.rs`: stub module

### zenoh-nostd (pass-through)

**Cargo.toml:**
```
kani = ["zenoh-sansio/kani"]
proptest = ["zenoh-sansio/proptest"]
creusot = ["zenoh-sansio/creusot"]
verus = ["zenoh-sansio/verus"]
```

### zenoh-std (pass-through + platform verification)

**Cargo.toml:**
- Add features forwarding to `zenoh-nostd/*`
- Add optional dev-dep: `proptest = { version = "1.6", optional = true }`

**src/lib.rs:**
- Gate new modules on features

**New files:**
- `src/kani_proofs.rs`: stub
- `src/creusot_contracts.rs`: stub
- `src/verus_proofs.rs`: stub
- `src/proptests.rs`: stub

## Design decisions

| Decision | Rationale |
|---|---|
| Kani uses `cfg(kani)` not `cfg(feature = "kani")` | `cfg(kani)` is auto-set by `cargo kani`; Cargo feature is doc/propagation only |
| Proptest is always-compiled dev-dep, gated via feature flag | Cargo doesn't allow optional dev-dependencies; feature flag gates test modules instead |
| Creusot/Verus are scaffolding only | No existing contracts; features + stubs let users start writing |
| zenoh-std hosts own verification modules | Platform layer (TCP/UDP/WS) needs its own verification |
