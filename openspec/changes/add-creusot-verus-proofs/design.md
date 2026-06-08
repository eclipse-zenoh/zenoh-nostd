# Design: Creusot and Verus Formal Verification

## Architecture

### Creusot Contracts

Target functions for Creusot contracts:

```
ZLen::z_len()        → ensures result ≥ actual encoded bytes
ZBodyLen::z_body_len() → ensures result ≤ total len
SN increment         → ensures monotonicity, correct wrap
Buffer remaining()   → ensures remaining ≤ capacity
```

Contracts are annotated inline:

```rust
#[ensures(result >= self.z_body_len())]
fn z_len(&self) -> usize;
```

Crate-level feature flag: `creusot = ["dep:creusot-contracts"]`

### Verus Verification

Target kernels for Verus:

```
1. Session state machine → 6 states, transition table verified
2. Packet length bounds → max encoded size ≤ declared max
3. Parser cursor progress → cursor advances by ≥1 per step
4. Bounded queue → push/pop preserve invariants
```

Each kernel gets a Verus proof module:

```
src/verus_session.rs   → session state machine
src/verus_bounds.rs    → length invariants
src/verus_cursor.rs    → parser progress
```

Feature flag: `verus = []`

### CI Integration

```yaml
creusot:
  name: Creusot
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - run: cargo creusot -p zenoh-sansio --features creusot

verus:
  name: Verus
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - run: cargo verus -p zenoh-sansio --features verus
```

## Trade-offs

| Approach | Pros | Cons |
|----------|------|------|
| Creusot contracts only | Lighter weight, fast CI | No full program verification |
| Verus proofs only | Full functional correctness | Heavy toolchain, slower CI |
| Both (recommended) | Layered assurance | More toolchain maintenance |

## Risk

- Creusot requires a nightly toolchain with specific patches
- Verus requires its own Rust toolchain fork
- Proofs may need updating as code evolves