# High-Assurance `zenoh-nostd` Embedded Rust Agent

## Mission

Improve `ZettaScaleLabs/zenoh-nostd` as a high-assurance, embedded-safe, `no_std` Rust implementation of Zenoh.

The goal is not just "make it work." The goal is:

* protocol correctness
* compatibility with upstream Zenoh behavior
* deterministic embedded behavior
* bounded memory use
* no uncontrolled panics
* robust parsing
* verifiable state machines
* excellent test evidence

`zenoh-nostd` is a `no_std` implementation of Zenoh, while the main Eclipse Zenoh Rust crate is the reference implementation of the protocol. Zenoh supports pub/sub and query/reply communication over arbitrary topologies such as mesh, star, or clique.

---

## Repository-Specific Priorities

1. `no_std` correctness
2. upstream Zenoh compatibility
3. parser and codec robustness
4. session/state-machine correctness
5. bounded queues and buffers
6. transport abstraction safety
7. deterministic behavior on constrained devices
8. fuzzing against malformed wire input
9. property tests against protocol invariants
10. formal verification for critical state transitions and buffer logic

---

## Hard Rules

* Keep `no_std` compatibility.
* Do not introduce `std` into core crates.
* Do not introduce heap allocation unless explicitly justified behind a feature flag.
* Do not introduce uncontrolled `panic!`, `unwrap`, or `expect`.
* Do not add blocking behavior without explicit timeout semantics.
* Do not silently drop protocol messages unless the behavior is documented and tested.
* Do not weaken protocol compatibility to simplify implementation.
* Do not hide unsupported Zenoh features; expose them as explicit errors or feature-gated TODOs.
* Do not merge code without tests.

---

## Required Crate Policy

Use strict linting:

```rust
#![no_std]
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
```

Prefer:

```rust
#![forbid(unsafe_code)]
```

If `unsafe` is absolutely required for embedded/HAL integration:

```rust
// SAFETY:
// - explain invariant
// - explain caller obligations
// - explain why safe Rust cannot express this
// - link to test or verification evidence
```

---

## Agent Workflow

For every change:

1. Inspect the existing implementation.
2. Identify affected protocol behavior.
3. Compare against upstream Zenoh expectations.
4. Add unit tests.
5. Add property tests.
6. Add fuzz cases if parsing/decoding is touched.
7. Add Kani checks for bounded logic.
8. Add Creusot or Verus specs for critical invariants where practical.
9. Run all quality gates.
10. Document residual risk.

---

## Compatibility Strategy

Use the reference Zenoh Rust implementation as the behavioral oracle where possible.

Test classes:

```text
zenoh-nostd encode → upstream decode
upstream encode → zenoh-nostd decode
zenoh-nostd state transition → expected upstream behavior
zenoh-nostd error handling → documented protocol behavior
```

Golden vectors should be stored in:

```text
tests/golden_vectors/
```

Each vector should include:

```text
name
protocol feature
input bytes
expected decoded structure
expected error if invalid
source / upstream version
```

---

## Required Test Categories

### Unit Tests

Required for:

* packet encoding
* packet decoding
* key expression handling
* session state transitions
* sequence numbers
* fragmentation/reassembly if implemented
* reliability behavior if implemented
* liveliness behavior if implemented
* buffer boundaries
* error conversion
* timeout handling

### Property Tests with `proptest`

Use `proptest` for:

* parser never panics
* decode rejects invalid input safely
* encode/decode roundtrip
* serialized length is bounded
* state transitions preserve invariants
* counters do not overflow silently
* malformed frames never produce valid privileged states

### Fuzzing

Use `cargo fuzz` for:

* wire decoder
* frame parser
* transport packet parser
* key expression parser
* config parser if present
* session handshake parser

Fuzz targets:

```text
fuzz_targets/decode_frame.rs
fuzz_targets/decode_message.rs
fuzz_targets/keyexpr.rs
fuzz_targets/session_handshake.rs
```

### Kani

Use Kani for bounded verification of:

* ring buffers
* fixed-capacity queues
* sequence arithmetic
* state transitions
* parser cursor logic
* bounds-checked serialization

### Creusot

Use Creusot for contract-heavy pure functions:

* length calculations
* bounds checks
* serialization preconditions
* monotonic counters
* invariant-preserving state updates

### Verus

Use Verus for high-value protocol kernels:

* verified state machines
* verified packet length invariants
* verified bounded queues
* verified session lifecycle transitions
* verified parser cursor progress

---

## Required CI Commands

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo llvm-cov --workspace --fail-under-lines 90
cargo deny check
cargo audit
cargo geiger
cargo machete
cargo kani
```

If embedded targets are configured:

```bash
cargo build --target thumbv7em-none-eabihf --no-default-features
cargo build --target thumbv7m-none-eabi --no-default-features
cargo build --target riscv32imac-unknown-none-elf --no-default-features
```

---

## Protocol Safety Checklist

For every protocol feature, verify:

```text
[ ] malformed input rejected
[ ] truncated input rejected
[ ] oversized input rejected
[ ] unknown field handled safely
[ ] unsupported feature returns explicit error
[ ] no panic path
[ ] no unchecked arithmetic
[ ] no unbounded allocation
[ ] deterministic runtime behavior
[ ] compatibility test against upstream Zenoh
[ ] fuzz target added
[ ] property test added
[ ] formal check considered
```

---

## Embedded Runtime Checklist

```text
[ ] no_std build passes
[ ] panic strategy documented
[ ] allocator not required, or explicitly feature-gated
[ ] all buffers are bounded
[ ] all queues have backpressure or explicit drop policy
[ ] timeouts are explicit
[ ] retry loops are bounded
[ ] transport failure is recoverable
[ ] session failure has safe reset path
[ ] watchdog integration possible
[ ] logging/tracing is feature-gated
[ ] binary size checked
```

---

## Specific Areas to Improve

### 1. Decoder Hardening

* no panics on arbitrary bytes
* no cursor overrun
* no integer overflow in length fields
* no invalid message promoted to valid state

### 2. Encoder Correctness

* exact wire format
* deterministic output
* bounded buffer writes
* clear error on insufficient capacity

### 3. State Machine Safety

Represent session states explicitly:

```rust
enum SessionState {
    Closed,
    Opening,
    Open,
    Degraded,
    Closing,
    Failed(SessionError),
}
```

Invalid transitions must return typed errors.

### 4. Transport Abstraction

Transport traits should expose:

```rust
trait Transport {
    type Error;

    fn send(&mut self, buf: &[u8]) -> Result<(), Self::Error>;
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}
```

Avoid hidden allocation, hidden blocking, and implicit retries.

### 5. Bounded Buffers

Use fixed-capacity buffers:

```rust
heapless::Vec<u8, N>
heapless::Deque<T, N>
heapless::String<N>
```

All capacity failures must be explicit errors.

### 6. Feature Flags

Separate:

```text
default = []
alloc = []
std = []
defmt = []
serde = []
transport-serial = []
transport-udp = []
transport-tcp = []
```

Core protocol logic should remain `no_std`.

---

## Documentation to Add

Create or update:

```text
docs/protocol_compatibility.md
docs/no_std_constraints.md
docs/verification_plan.md
docs/safety_case.md
docs/fuzzing.md
docs/formal_methods.md
docs/traceability.md
```

Each feature should include:

```text
Feature:
Status:
Supported upstream behavior:
Current nostd behavior:
Tests:
Fuzzing:
Formal verification:
Known gaps:
```

---

## Definition of Done

A change is done only when:

```text
[ ] no_std build passes
[ ] std test build passes if available
[ ] clippy passes with denied warnings
[ ] no unwrap/expect/panic in core logic
[ ] unit tests added
[ ] proptest properties added
[ ] fuzz target added for parsers
[ ] Kani harness added for bounded logic
[ ] compatibility with upstream Zenoh tested or documented
[ ] unsupported behavior explicitly documented
[ ] residual risk documented
```

---

## Agent Response Format

After each task, report:

```text
Changed files:
- ...

Protocol behavior affected:
- ...

Compatibility evidence:
- ...

Tests added:
- ...

Fuzzing added:
- ...

Formal verification added:
- ...

Commands run:
- ...

Known gaps:
- ...

Next hardening step:
- ...
```