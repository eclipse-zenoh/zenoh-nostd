# Safety Case for zenoh-nostd

## Scope

This safety case documents the assurance arguments for `zenoh-nostd` as an embedded-safe, `no_std` implementation of the Zenoh protocol. It covers the core crates `zenoh-proto`, `zenoh-sansio`, and `zenoh-nostd`.

---

## Safety Goals

| ID | Goal | Evidence |
|----|------|----------|
| SG1 | No uncontrolled panics in production code | Lint config denies `panic!`, `unwrap`, `expect` |
| SG2 | No buffer overflows on encode/decode | Kani proofs for buffer bounds, `heapless` for bounded buffers |
| SG3 | Malformed wire input never produces valid protocol state | Fuzz targets, proptest no-panic properties |
| SG4 | Session state transitions are deterministic and safe | State machine verification with Kani, explicit state enum |
| SG5 | Sequence numbers provide total ordering | Monotonicity proofs, epoch wrap analysis |

---

## Argument Structure

```
Safety of zenoh-nostd
├── SG1: No panics in production code
│   ├── Lint enforcement denies unwrap/expect/panic/todo/unimplemented
│   ├── Fallible operations use Result/Option with typed errors
│   └── Buffer operations return explicit capacity errors
├── SG2: No buffer overflows
│   ├── All buffers are fixed-capacity (heapless)
│   ├── Kani proves encode/decode stays within bounds
│   └── Write operations return BytesError::DstIsTooSmall on overflow
├── SG3: Malformed input safety
│   ├── Fuzz targets exercise all decoders with arbitrary bytes
│   ├── Proptest properties assert decode never panics
│   └── Decoders return CodecError for invalid input (never panic)
├── SG4: Session state safety
│   ├── Session states modeled as explicit enum
│   ├── Invalid transitions return typed errors
│   └── Kani verifies transition coverage
└── SG5: Sequence number correctness
    ├── Epoch monotonicity verified across SN wraps
    ├── Collision analysis shows 2^64 frames required
    └── Kani proofs for SN arithmetic
```

---

## Claimed Properties

### P1: Deterministic Decoding

```
∀ input: &[u8] . decode(input) ∈ { Ok(msg) | Err(CodecError) }
```

The decoder:
- Never panics (proven by proptest + fuzz)
- Never produces a valid message from invalid input (proven by fuzz)
- Is safe under truncation (proven by truncated-decode tests)

### P2: Bounded Encoding

```
∀ msg: Message . msg.z_len() ≤ MAX_ENCODED_SIZE
```

The encoder:
- Always reports an upper bound via `z_len()` (proven by Kani)
- Never writes beyond the reported bound (proven by Kani)
- Returns `Err(DstIsTooSmall)` on insufficient capacity (proven by Kani)

### P3: State Machine Progress

```
∀ s: SessionState, e: Event .
    transition(s, e) ≠ s  (unless s ∈ {Closed, Failed})
```

The session state machine:
- Always makes progress on valid events
- Returns typed errors for invalid events
- Never blocks indefinitely (all operations have timeout semantics)

---

## Evidence Catalog

| Evidence Type | Count | Coverage |
|---------------|-------|----------|
| Unit tests | 180+ | All public API functions |
| Property tests | 28 | Parse, encode/decode, state invariants |
| Fuzz targets | 3 | Transport decode, handshake (client + server) |
| Kani proofs | 69 | Buffer bounds, state transitions, encode/decode |
| Verus proofs | 0 | (planned) |
| Creusot specs | 0 | (planned) |

---

## Residual Risks

| Risk | Impact | Mitigation | Status |
|------|--------|------------|--------|
| Buffer exhaustion on constrained device | Message drop | Explicit capacity errors, documented drop policy | Accepted |
| SN epoch collision after 2^64 frames | Duplicate (epoch, sn) key | Documented as unreachable in practice | Accepted |
| Panic in proc-macro generated code | Compile-time failure | Generated code audited manually | Mitigated |
| Undefined behavior in unsafe code | Memory corruption | `#![forbid(unsafe_code)]` in all crates | Mitigated |
| Time-dependent race in session timeout | False timeout | Configurable lease, keep-alive mechanism | Accepted |

---

## Verification Boundary

The following are OUT of scope for the safety case:
- Platform-specific I/O (TCP, UDP, serial) — handled by platform crates
- Operating system scheduling guarantees
- Hardware fault tolerance (ECC, radiation hardening)
- Network partition behavior beyond timeout handling