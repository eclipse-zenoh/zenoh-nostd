# Change: Production Hardening — From Feature Parity to Production Quality

## Why

zenoh-nostd has achieved feature parity with zenoh-pico across all transports, protocols, auth, resilience, and platforms. However, parity is not production. The codebase has never been tested against real upstream Zenoh traffic, never had its auth verified, never been run on actual embedded hardware, and never passed the quality gates required by the project's own AGENTS.md.

Each gap below represents a real risk in production:

| Risk | If Unaddressed |
|------|----------------|
| No auth verification | Anyone can connect to a production bridge |
| No wire-format compat | nostd nodes silently misinterpret upstream messages |
| No serial parity/baud config | Serial transport unusable on half the targets |
| No background lease management | Sessions drop if `poll()` is delayed |
| No cross-compilation CI | Embedded platforms silently broken at build time |
| No fuzz execution | Protocol parsers crash on malformed input |
| No Kani execution | Bounded correctness proofs are stale |
| No hardware testing | Transport implementations fail on real devices |

## What Changes

### 1. Production Auth Verification

The current auth implementation sends credentials but never verifies them server-side. Add:

#### 1a. Server-side auth callback

In `zenoh-sansio`, add an auth verification callback to `TransportBuilder`:

```rust
pub fn with_auth_verifier(mut self, verifier: impl Fn(&[u8]) -> bool) -> Self
```

The `verifier` receives the auth payload from the remote peer's InitSyn/InitAck and returns `true` if accepted, `false` if rejected (closes the connection).

#### 1b. Default verifier for user/password

Provide a built-in user/password verifier:

```rust
pub fn with_user_pass_verifier(mut self, expected_username: &str, expected_password: &str) -> Self
```

This verifies tag byte `0x01` followed by `username\0password`.

#### 1c. Rejection handshake

When auth fails, send a Close message with reason `AuthRejected` instead of silently dropping.

#### 1d. Auth in zenoh-nostd config

Wire the auth verifier through `ZSessionConfig` so users can configure it:

```rust
#[cfg(feature = "auth")]
fn auth_verifier(&self) -> Option<&dyn Fn(&[u8]) -> bool> { None }
```

### 2. Wire-Format Compatibility Tests

#### 2a. Golden vector generation from upstream

Use the reference Rust zenoh implementation (`zenoh::codec`) to generate golden vectors:

```bash
# Run a script that creates real InitSyn/InitAck/Put/Sub messages
# using upstream zenoh, captures the encoded bytes, and stores them
# as golden vectors in tests/golden_vectors/
```

For each message type in `tests/golden_vectors/`:
- `init_syn_peer.json` — InitSyn with WhatAmI::Peer
- `init_ack_client.json` — InitAck response
- `put_simple.json` — Put with keyexpr and value
- `sub_declare.json` — DeclareSubscriber
- `query_request.json` — Request message
- `query_response.json` — Response message
- `scout_message.json` — Scout message
- `auth_init_syn.json` — InitSyn with auth payload

#### 2b. Cross-decoder tests

For each golden vector:
1. nostd encodes → upstream decodes (via subprocess calling zenohd or a test harness)
2. upstream encodes → nostd decodes

These tests verify that nostd's wire format is byte-for-byte identical to upstream.

#### 2c. Property test: encode→decode→re-encode

Add a `proptest` that:
1. Generates a random protocol message
2. Encodes with nostd
3. Decodes the output
4. Re-encodes the decoded value
5. Asserts byte-for-byte equality between steps 2 and 4

This catches codec bugs where decode changes the meaning.

### 3. Serial Parity/Baud Configuration

#### 3a. Endpoint URI query parameter parsing

Add query string parsing to `Endpoint` in `zenoh-proto`:

```
serial:///dev/ttyUSB0?baud=115200&parity=even&bits=7&stop=1
serial:///dev/ttyAMA0?baud=9600
usb-cdc:///dev/ttyACM0
```

Add a `SerialConfig` struct:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct SerialConfig {
    pub baud_rate: u32,
    pub data_bits: SerialDataBits,  // 5, 6, 7, 8
    pub parity: SerialParity,       // None, Even, Odd, Mark, Space
    pub stop_bits: SerialStopBits,  // 1, 1.5, 2
}
```

#### 3b. Parse serial config from endpoint

In `zenoh-proto`'s endpoint parser, extract query parameters for serial endpoints:

```rust
fn parse_serial_config(uri: &str) -> Result<SerialConfig, ParseError> {
    // Extract ?baud=...&parity=... from URI
    // Default: 115200 8N1
}
```

#### 3c. Pass config through link creation

In `StdLinkManager::connect()` for `"serial"` and `"usb-cdc"` protocols, parse the config from the endpoint and pass it to `StdSerialLink::new()`:

```rust
"serial" => {
    let (path, config) = parse_serial_endpoint(address)?;
    let link = serial::StdSerialLink::new(path, config.baud_rate)?;
    // Apply parity, data bits, stop bits to serial port
    Ok(Self::Link::Serial(link))
}
```

#### 3d. Update serialport configuration

Use the `serialport` crate's configuration API to set parity, data bits, stop bits:

```rust
let mut port = serialport::new(path, config.baud_rate)
    .timeout(Duration::from_millis(10))
    .data_bits(match config.data_bits { 5 => DataBits::Five, 7 => DataBits::Seven, 8 => DataBits::Eight, _ => DataBits::Eight })
    .parity(match config.parity { SerialParity::None => Parity::None, SerialParity::Even => Parity::Even, SerialParity::Odd => Parity::Odd, ... })
    .stop_bits(match config.stop_bits { 1 => StopBits::One, 2 => StopBits::Two, _ => StopBits::One })
    .open()?;
```

### 4. Background Lease/Keepalive Management

#### 4a. Session health query API

Add to `Session`:

```rust
pub fn is_connected(&self) -> bool {
    !self.driver.rx.transport().closed()
}

pub fn time_since_last_activity(&self) -> Duration {
    // Return time since last successful send/recv
}
```

#### 4b. Lease timeout detection in poll()

The current `poll()` implementation uses `Duration::ZERO` for all timer checks. Fix to track elapsed time:

```rust
pub fn poll(&mut self) -> Result<bool, Error> {
    let elapsed = self.last_poll.elapsed();
    self.rx.transport_mut().sync(None, elapsed);
    self.tx.transport_mut().sync(Some(self.rx.transport()), elapsed);
    // ... rest of poll
    self.last_poll = Instant::now();
    Ok(did_work)
}
```

#### 4c. Background poll task for std

Add `Session::run_background()` that spawns a thread to call `poll()` at a configurable interval:

```rust
#[cfg(feature = "std")]
pub fn run_background(&self, poll_interval: Duration) {
    std::thread::spawn(move || {
        loop {
            self.driver.poll(...);
            std::thread::sleep(poll_interval);
        }
    });
}
```

### 5. CI Quality Gates

#### 5a. Cross-compilation CI matrix

Add to `.github/workflows/ci.yml`:

```yaml
cross-compile:
  strategy:
    matrix:
      target:
        - aarch64-unknown-none
        - thumbv7em-none-eabihf  # STM32
        - riscv32imac-unknown-none-elf  # ESP32
        - wasm32-unknown-unknown
  steps:
    - run: cargo check --target ${{ matrix.target }} --no-default-features
```

#### 5b. `cargo deny` check

Run `cargo deny check licenses advisories sources` in CI.
Fix any advisories in dependencies.

#### 5c. `cargo audit` check

Run `cargo audit` in CI.
Fix any reported vulnerabilities.

#### 5d. `cargo geiger` check

Run `cargo geiger` to audit unsafe usage.
Ensure all unsafe blocks have SAFETY comments and are justified.

#### 5e. `cargo machete` check

Run `cargo machete` to detect unused dependencies.
Remove any unused deps.

### 6. Fuzz Harness Execution

#### 6a. Run existing fuzz targets

```bash
cargo fuzz run decode_frame -- -runs=100000
cargo fuzz run decode_message -- -runs=100000
cargo fuzz run establish_ment -- -runs=100000
cargo fuzz run keyexpr -- -runs=100000
```

#### 6b. Add new fuzz targets

- `fuzz_targets/decode_auth.rs` — Fuzz auth extension parsing
- `fuzz_targets/decode_nack.rs` — Fuzz NACK message parsing
- `fuzz_targets/decode_serial_endpoint.rs` — Fuzz serial endpoint URI parsing

#### 6c. Fix any crashes found

Run all fuzz targets for 100k iterations each. Fix any panics or hangs.

### 7. Kani Proof Verification

#### 7a. Run existing Kani proof harnesses

```bash
cargo kani --harness transport_state_machine  # Verify transport state machine transitions
cargo kani --harness sequence_number_arithmetic  # Verify SN wrap handling
cargo kani --harness ring_buffer  # Verify bounded buffer operations
cargo kani --harness parser_cursor  # Verify cursor bounds checking
```

#### 7b. Fix proof failures

Update any proofs broken by recent changes (auth, NACK, peer mode, etc.).

### 8. Hardware Testing

#### 8a. std TCP loopback test

Build and run `examples/z_pub.rs` and `examples/z_sub.rs` against a local zenohd router. Verify messages are delivered correctly.

#### 8b. STM32 QEMU test

Run the STM32 example in QEMU:
```bash
cargo run --target thumbv7em-none-eabihf --example stm32_qemu
```
Verify that codec tests pass (as documented in commit 2c72864).

#### 8c. Serial loopback test

On Linux/Mac, create a virtual serial port pair (socat) and run serial transport tests.

### 9. Update Protocol Features Spec

Remove "Auth verification" and "Wire-format compatibility" from the known gaps section. Add all completed verification work as evidence.

## Capabilities

### New

- `auth-verification` — Server-side auth credential verification with configurable verifier
- `wire-compat` — Cross-implementation golden vector testing
- `serial-config` — URI-based serial port configuration (baud, parity, data bits, stop bits)
- `lease-management` — Background lease/keepalive with session health API
- `ci-cross-compile` — Cross-compilation CI for all embedded targets
- `ci-deny-audit` — `cargo deny` and `cargo audit` in CI
- `fuzz-execution` — Automated fuzz testing in CI
- `kani-verification` — Formal bounded verification in CI
- `hardware-test` — Real device testing (QEMU, loopback)

### Modified

- `user-pass-auth` — Extended with server-side verification
- `serial-transport` — Extended with configurable parity/baud/stop bits
- `session` — Extended with health API and background poll
- `ci` — Extended with cross-compilation, deny, audit, fuzz, kani

## Impact

| Area | Change | Risk |
|------|--------|------|
| zenoh-sansio | Auth verification callback | Low — additive, feature-gated |
| zenoh-proto | Serial URI config parsing | Low — new parsing code |
| zenoh-nostd | Session health API, background poll | Low — additive |
| zenoh-std | Serial port configuration | Low — uses serialport crate API |
| CI | 5 new workflows | Low — config only |
| Fuzz | Existing targets + new | Low — testing only |
| Kani | Update proofs | Medium — proofs may need updates |
| Hardware tests | Requires devices or QEMU | Medium — env setup |

## Phasing

| Phase | Scope | Deliverable |
|-------|-------|-------------|
| 1 | Auth verification | Server-side callback, user/pass verifier, Close on reject |
| 2 | Wire-format compat | Upstream golden vectors, cross-decoder tests, proptest |
| 3 | Serial config | URI parsing, SerialConfig, serialport API usage |
| 4 | Lease management | Session health API, timed poll(), background thread for std |
| 5 | CI gates | Cross-compilation matrix, deny, audit, geiger, machete |
| 6 | Fuzz execution | Run all targets, fix crashes, add new targets |
| 7 | Kani verification | Run and fix all proof harnesses |
| 8 | Hardware testing | QEMU, serial loopback, std TCP against zenohd |

## Open Questions

- Should auth verification reject at the transport level (close connection) or session level (return error on operations)?
- What priority order should the CI gates run in if some are slow (Kani can take hours)?
- For wire-format tests, do we have access to an upstream Zenoh Rust binary to run as a subprocess, or do we need to write a standalone test harness?