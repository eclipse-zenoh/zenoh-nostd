# No_std Constraints

This document describes the `no_std` constraints that apply to all core crates in `zenoh-nostd`.

---

## Core Constraint

The crates `zenoh-proto`, `zenoh-sansio`, and `zenoh-nostd` MUST compile without the Rust standard library (`std`). They are annotated with:

```rust
#![cfg_attr(not(feature = "std"), no_std)]
```

This means they can be used in embedded environments (Cortex-M, RISC-V, etc.) where `std` is unavailable.

---

## Synchronous Architecture

zenoh-nostd is fully synchronous. No async runtime is required. IO is driven by `Session::run()` or `Session::poll()` (via `Driver`) in the application's main loop. This makes it suitable for bare-metal embedded systems where async runtimes are unavailable or undesirable.

Key design principles:

- All IO is non-blocking poll-driven
- `Session::run()` processes available messages and returns when idle
- Callbacks are invoked synchronously during `run()`
- No hidden threads, tasks, or coroutines
- Deterministic execution model

---

## Allowed Dependencies

### Always available (no feature gate)

| Crate | Purpose | Heap? |
|-------|---------|-------|
| `core` | Rust core library | No |
| `heapless` | Fixed-capacity containers (`Vec`, `Deque`, `String`) | No |
| `uhlc` | Hybrid Logical Clocks | No |
| `sha3` | SHA-3 hashing | No |

### Behind `std` feature

| Crate | Purpose |
|-------|---------|
| `log` | Logging (requires `std` for global logger) |
| `web-sys` | Web console (requires `std` + WASM) |
| `defmt` | Embedded logging (no `std` needed, separate feature) |

### Behind `alloc` feature

| Crate | Purpose |
|-------|---------|
| (none currently) | Heap allocation not yet required |

---

## Banned Patterns

### Unconditional `std` usage

❌ `std::collections::HashMap` → use `heapless::FnvIndexMap`
❌ `std::vec::Vec` → use `heapless::Vec`
❌ `std::string::String` → use `heapless::String`
❌ `std::net::TcpStream` → behind `std` feature in platform crates
❌ `std::time::Instant` → use `core::time::Duration` with platform-specific timer abstraction
❌ `Box::new()` → use fixed-capacity buffers or `heapless`
❌ `format!()` → use manual `write!` or `defmt`

### Async patterns

❌ `async fn` / `.await` → use synchronous `fn` calls and `Session::run()` / `Session::poll()`
❌ `async` runtimes (tokio, embassy) → not required; IO is poll-driven from the application's main loop
❌ `embassy_time` → use `core::time::Duration` and platform-specific timer peripherals
❌ `embassy_sync::channel` → use callback-based API (`.callback_sync()`) for subscription handlers

### Unconditional panics

❌ `panic!()` → return `Err(...)` with typed error
❌ `unreachable!()` → return `Err(...)` with typed error
❌ `todo!()` → use `#[cfg(feature = "...")]` gated implementation
❌ `unimplemented!()` → use explicit error variant
❌ `.unwrap()` → use `?` operator or match with error conversion
❌ `.expect("...")` → use `?` operator or match with error conversion

---

## Feature Flag Structure

```text
default = []
std     = []     # Enables std-dependent functionality
alloc   = []     # Enables heap allocation features
log     = [std]  # Enables logging
defmt   = []     # Enables defmt logging (no_std compatible)
proptest = []    # Enables property tests (dev only)
kani    = []     # Enables Kani model checking (verification only)
verus   = []     # Enables Verus proofs (verification only)
creusot = []     # Enables Creusot contracts (verification only)
```

Core protocol logic must compile with **no features enabled** (pure `no_std`).

---

## Verification-Specific Features

The following features are for verification tooling and MUST NOT affect production behavior:

- `kani` — compiles Kani proof harnesses
- `verus` — compiles Verus proof annotations and test proofs
- `creusot` — compiles Creusot contract annotations
- `proptest` — compiles property test harnesses

These features should be declared in `Cargo.toml` as:

```toml
kani = []
verus = []
creusot = ["dep:creusot-contracts"]
proptest = []
```

And gated in code with:

```rust
#[cfg(feature = "kani")]
mod kani_proofs;
```

---

## CI Verification

```bash
# Verify no_std compilation
cargo test --workspace --no-default-features

# Verify full std compilation
cargo test --workspace --all-features

# Verify embedded target compilation
cargo build --target thumbv7m-none-eabi --no-default-features
cargo build --target thumbv7em-none-eabihf --no-default-features
cargo build --target riscv32imac-unknown-none-elf --no-default-features
```