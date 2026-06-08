## Context

zenoh-nostd has achieved structural feature parity with zenoh-pico but has never been tested against real upstream traffic, never had its auth verified server-side, and never had its quality gates executed. This change addresses all production-hardening gaps.

## Decisions

### Auth: Callback-based verifier

A callback-based verifier is chosen over a config struct because it allows the user to implement arbitrary auth logic (LDAP, OAuth tokens, custom databases). The built-in user/password verifier is a convenience wrapper.

### Wire-format: Golden vectors + proptest

Golden vectors are the primary compatibility mechanism (fixed, repeatable, human-reviewable). The proptest encode-decode-reencode identity test is the secondary mechanism (exhaustive random coverage).

### Serial config: URI query parameters

Query parameters in the endpoint URI are chosen over a separate config object because they follow existing Zenoh conventions and keep the API surface minimal.

### Lease management: Timed poll with `Instant::now()`

Rather than adding a separate background thread in the core crate, `poll()` tracks elapsed time using `Instant::now()`. The background thread is a std-only convenience on top.

### CI: Cargo-deny first, cross-compile second, Kani last

Priority order reflects cost: cargo-deny is fast (30s), cross-compile is medium (2min), Kani is expensive (30min+). Fast checks fail fast.

## Risks

- Kani proofs may need significant updates due to the async removal and new message types — this is expected and acceptable.
- Fuzz targets may find real bugs — each crash should be fixed before moving on.
- Cross-compilation for embedded targets may require nightly Rust for some targets.
- `cargo geiger` may flag unsafe blocks that are necessary for FFI — these should be documented with skip annotations.

## Open Questions

- Wire format tests: do we add upstream zenoh as a dev-dependency, or generate golden vectors offline and commit the byte files?
- Kani: the 69 proof harnesses were written before the async removal — how many need updating?
- Background thread: should `run_background()` return a handle that can be used to stop the thread?