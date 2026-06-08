# Fuzzing Guide

## Overview

Fuzz testing is a core component of `zenoh-nostd`'s verification strategy. Fuzz targets exercise protocol decoders with arbitrary byte sequences to find panics, hangs, or memory safety violations.

Fuzz targets use [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) with the `libfuzzer-sys` backend.

---

## Fuzz Targets

| Target | File | Purpose |
|--------|------|---------|
| `transport_decode` | `fuzz_targets/transport_decode.rs` | Transport-level prefixed and raw decode |
| `establishment` | `fuzz_targets/establishment.rs` | Server-side handshake |
| `establishment_client` | `fuzz_targets/establishment_client.rs` | Client-side handshake |
| `keyexpr` | `fuzz_targets/keyexpr.rs` | Key expression parser — arbitrary bytes |
| `decode_frame` | `fuzz_targets/decode_frame.rs` | Frame-level decoder — prefixed and raw decode paths |
| `decode_message` | `fuzz_targets/decode_message.rs` | Individual message type decoders (Put, Del, Declare, KeepAlive, Close, InitSyn, InitAck, OpenSyn, OpenAck, Push, Request) |

---

## Running Fuzz Targets

```bash
# Build all fuzz targets (no execution)
cargo fuzz build

# Run a specific target (indefinitely)
cargo fuzz run transport_decode
cargo fuzz run decode_frame
cargo fuzz run decode_message
cargo fuzz run keyexpr
cargo fuzz run establishment
cargo fuzz run establishment_client

# Run with a specific corpus
cargo fuzz run transport_decode fuzz/corpus/transport_decode/

# Run for N iterations (CI use)
cargo fuzz run transport_decode -- -runs=100000

# List all targets
cargo fuzz list
```

---

## CI Integration

Fuzz targets are NOT run in CI by default (they require significant CPU time). Instead:

1. **Fuzz targets must compile** in CI (`cargo fuzz build`)
2. **Fuzz targets are run** in scheduled nightly jobs or on demand
3. **Crashes** are checked into `fuzz/artifacts/` and tracked as bugs

---

## Adding a New Fuzz Target

1. Create `fuzz/fuzz_targets/my_target.rs`:

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz harness - must not panic
    let _ = my_decoder::decode(data);
});
```

2. Register in `fuzz/Cargo.toml`:

```toml
[[bin]]
name = "my_target"
path = "fuzz_targets/my_target.rs"
test = false
doc = false
bench = false
```

3. Build and run:

```bash
cargo fuzz run my_target
```

---

## Fuzz Harness Requirements

Every fuzz harness MUST:

1. **Never panic** — all errors must be handled with `let _ = ...` or `if let Ok(...)`
2. **Never hang** — no unbounded loops or allocations based on input size
3. **Be deterministic** — same input always produces same result
4. **Be fast** — target should process ≥10K inputs/second
5. **Cover all decode paths** — exercise every decoder exposed to untrusted input

---

## Corpus Management

- Initial corpora should contain valid protocol messages to guide the fuzzer
- Corpora are stored in `fuzz/corpus/<target_name>/`
- Crash artifacts are stored in `fuzz/artifacts/<target_name>/`
- Minimize corpora periodically: `cargo fuzz tmin <target> corpus_dir`