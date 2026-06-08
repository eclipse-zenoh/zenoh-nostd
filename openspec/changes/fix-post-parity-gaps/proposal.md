# Change: Fix 19 Post-Parity Gaps

## Why

The zenoh-pico parity audit identified 19 gaps between nostd's current implementation and production-quality code. Five of these are **production panic paths** that will crash on real inputs. Eight cause **incorrect behavior** (wrong stream semantics, dead code, unused config). Six are **missing functionality** (no listen on TLS/WS, BLE stub, platform quality issues).

These gaps are the difference between "feature-complete on paper" and "actually works in production." Fixing them before shipping prevents crashes in the field and ensures correct protocol behavior.

## What Changes

### Group 1: Fix 5 Production Panic Paths (CRITICAL)

#### 1.1 TCP `expect()` in split — `platforms/zenoh-std/src/tcp.rs`

Replace `expect()` with proper error handling:

```rust
// Current (panics):
self.stream.lock().expect("lock")

// Fixed:
self.stream.lock().map_err(|_| LinkError::LinkTxFailed)?
```

The `Mutex` is used for splitting TX/RX from the same `TcpStream`. The `expect()` will panic if the mutex is poisoned (e.g., a panic in another thread holding the lock). Replace with `map_err` returning `LinkError`.

#### 1.2 UDP `unimplemented!()` in read_exact — `platforms/zenoh-std/src/udp.rs`

UDP is datagram-based. `read_exact` cannot be implemented for datagram sockets — each `recv` returns one datagram. Replace `unimplemented!()` with a loop that calls `read()` repeatedly until the buffer is full or the socket times out:

```rust
fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
    let mut offset = 0;
    while offset < buffer.len() {
        let n = self.read(&mut buffer[offset..])?;
        if n == 0 {
            return Err(LinkError::LinkRxFailed);
        }
        offset += n;
    }
    Ok(())
}
```

#### 1.3 BLE `unimplemented!()` in split — `platforms/zenoh-std/src/bt.rs`

Replace with a working BLE implementation using `bluer` (sync BlueZ D-Bus API) or mark the entire module behind `#[cfg(feature = "bt")]` so it's not compiled in by default:

```rust
impl ZLink for BtStream {
    type Tx<'a> = BtStreamTx where Self: 'a;
    type Rx<'a> = BtStreamRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        // BtStream is Arc<Mutex<...>> — split clones the Arc
        (BtStreamTx { inner: self.inner.clone(), mtu: self.mtu },
         BtStreamRx { inner: self.inner.clone(), mtu: self.mtu })
    }
}
```

Add feature-gating: `bt` feature flag default-off, or fix the split properly.

#### 1.4 `expect()` in sansio decode path — `crates/zenoh-sansio/src/transport/rx.rs:252`

Replace with proper error propagation:

```rust
// Current:
let sn = sn.expect("SN should be set");

// Fixed:
let sn = sn.ok_or(TransportLinkError::InvalidAttribute)?;
```

#### 1.5 `unreachable!()` in fields codec — `crates/zenoh-proto/src/msgs/fields.rs:806`

Replace with a proper error return or default value. Analyze the specific context — it's likely a match arm that shouldn't be reached in normal operation but COULD be reached with malformed input:

```rust
// Current:
_ => unreachable!(),

// Fixed:
_ => return Err(CodecError::UnsupportedVariant),
```

Or add a `CodecError::UnsupportedVariant` variant if it doesn't exist.

### Group 2: Fix 8 Wrong Behaviors

#### 2.1 Serial `is_streamed=false` — `platforms/zenoh-std/src/serial.rs`

Serial UART is a stream protocol (byte-oriented, ordered, reliable within link). Change to `true`:

```rust
fn is_streamed(&self) -> bool { true }
```

#### 2.2 USB CDC `is_streamed=false` — `platforms/zenoh-std/src/usb_cdc.rs`

USB CDC ACM is also streamed. Same fix:

```rust
fn is_streamed(&self) -> bool { true }
```

#### 2.3 mcast not registered — `platforms/zenoh-std/src/lib.rs`

Add `mod mcast;` and wire `Mcast(mcast::McLink)` variant to the StdLink enum. Currently the mcast.rs file exists with full implementation but is never included.

#### 2.4 Auto-reconnect not wired — `crates/zenoh-nostd/src/api/auto_reconnect.rs`

The `AutoReconnectConfig` and `is_retryable()` function exist but `Session::run()` doesn't use them. Wire auto-reconnect into the session's poll loop:

```rust
pub(crate) fn run(&mut self) {
    loop {
        match self.driver.poll(...) {
            Ok(_) => {},
            Err(e) if is_retryable(&e) && retries < config.max_retries => {
                // backoff and reconnect
            },
            Err(_) => break,
        }
    }
}
```

#### 2.5 `multicast-declarations` dead feature flag — `crates/zenoh-nostd/Cargo.toml`

Add `#[cfg(feature = "multicast-declarations")]` guards to the DeclarePublisher/UndeclarePublisher code, or remove the feature flag if it's always-on.

#### 2.6 No `// SAFETY:` comments on unsafe blocks — all platform crates

Add safety comments to all ~250+ unsafe blocks. Each must explain:
- Why the invariants hold
- What caller obligations are
- Why safe Rust cannot express this

#### 2.7 IPv6 bracket parsing — 5 POSIX platform crates

Fix `to_sockaddr()` to strip brackets from IPv6 addresses before parsing:

```rust
let addr_str = addr.trim_start_matches('[').trim_end_matches(']');
```

#### 2.8 No TCP_NODELAY on embedded — 7 embedded platforms

Add setsockopt call after connect on all POSIX-based platforms:

```rust
let nodelay: u32 = 1;
unsafe {
    setsockopt(fd, IPPROTO_TCP as i32, TCP_NODELAY as i32,
        &nodelay as *const _ as *const u8, core::mem::size_of::<u32>() as u32);
}
```

Define `IPPROTO_TCP` and `TCP_NODELAY` constants in sockets.rs.

### Group 3: Fix 6 Missing Features

#### 3.1 TLS listen path — `platforms/zenoh-std/src/tls.rs`

Add a `listen()` method that accepts a TCP connection, then wraps it with TLS:

```rust
pub fn accept(listener: &TcpListener, domain: &str) -> Result<Self, LinkError> {
    let (tcp, _) = listener.accept().map_err(|_| LinkError::CouldNotListen)?;
    let connector = native_tls::TlsAcceptor::builder(
        native_tls::Identity::from_pkcs12(...) // requires identity
    ).build().map_err(|_| LinkError::CouldNotListen)?;
    let stream = connector.accept(domain, tcp).map_err(|_| LinkError::CouldNotListen)?;
    Ok(Self { stream: Arc::new(Mutex::new(stream)), mtu: u16::MAX - 60 })
}
```

#### 3.2 WebSocket listen path — `platforms/zenoh-std/src/ws.rs`

Add a `listen()` method using tungstenite's `accept()`:

```rust
pub fn accept(tcp: TcpStream) -> Result<Self, LinkError> {
    let stream = tungstenite::accept(tcp).map_err(|_| LinkError::CouldNotListen)?;
    Ok(Self::new(stream, u16::MAX))
}
```

#### 3.3 BLE real implementation — `platforms/zenoh-std/src/bt.rs`

Replace the 37-line stub with a real BlueZ-based implementation using `bluer` crate (sync D-Bus API):

```rust
pub struct BtStream {
    socket: bluer::rfcomm::Socket,
    mtu: u16,
}
```

Or mark the module as `#[cfg(feature = "bt")]` to avoid dead code.

#### 3.4 STM32 serial — `platforms/zenoh-stm32/src/serial.rs`

Implement STM32 UART via the STM32 HAL's USART peripheral. Use `stm32-f7` or `stm32h7` HAL traits:

```rust
pub struct Stm32SerialLink {
    usart: USART,
    mtu: u16,
}
```

#### 3.5 Transport integration tests

Add end-to-end tests:
1. TCP connect+listen roundtrip (std)
2. UDP send+recv roundtrip (std)
3. Serial loopback test (std, requires virtual serial port or loopback)
4. Peer-to-peer handshake test (sansio)

#### 3.6 Deduplicate POSIX platform code

Extract the shared POSIX socket code (sockets.rs, tcp.rs, udp.rs) into a shared crate `zenoh-platform-posix` that all 7 embedded platforms depend on:

```toml
[dependencies]
zenoh-platform-posix = { path = "../../platforms/zenoh-platform-posix" }
```

This eliminates ~3,500 lines of duplicated code and ensures all platforms get the same fixes.

## Capabilities

### Modified

- `platform-std` — Fix panic paths in TCP/UDP/BT, fix is_streamed on serial/USB CDC, wire mcast, add TLS/WS listen, implement BLE
- `platform-zephyr` — Fix IPv6 parsing, add TCP_NODELAY, add SAFETY comments, deduplicate
- `platform-mbedos` — Same
- `platform-freertos` — Same
- `platform-opencr` — Same, plus fix STM32 serial
- `platform-arduino` — Same
- `platform-rp2040` — Same
- `platform-flipper` — Same
- `zenoh-sansio` — Fix expect() in rx.rs decode path
- `zenoh-proto` — Fix unreachable!() in fields.rs codec
- `zenoh-nostd` — Wire auto-reconnect into session run, fix dead feature flag

### New

- `zenoh-platform-posix` — Shared POSIX platform crate for embedded platforms

## Impact

| Area | Change | Risk |
|------|--------|------|
| std/tcp.rs | Replace expect() with error | Low — mechanical change |
| std/udp.rs | Replace unimplemented!() with loop | Low — mechanical change |
| std/bt.rs | Fix split() or feature-gate | Medium — BLE was already broken |
| sansio/rx.rs | Replace expect() with error | Low — better error handling |
| proto/fields.rs | Replace unreachable!() with error | Low — defensive coding |
| std/serial.rs | is_streamed true | Low — matches actual protocol |
| std/usb_cdc.rs | is_streamed true | Low |
| std/lib.rs | Add mod mcast | Low — wires existing code |
| All embedded | IPv6 fix, TCP_NODELAY, SAFETY | Low — targeted fixes |
| nostd/auto_reconnect.rs | Wire into run() | Medium — session behavior change |
| nostd/Cargo.toml | Fix dead feature flag | Low |
| std/tls.rs | Add listen() | Medium — requires TlsAcceptor |
| std/ws.rs | Add listen() | Low |
| std/bt.rs | Real BLE or feature gate | High — needs BlueZ integration |
| stm32/serial.rs | Implement | Medium — needs STM32 HAL |
| New crate | POSIX dedup | Medium — refactoring |
| Tests | Add transport tests | Low — testing only |