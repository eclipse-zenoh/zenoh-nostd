# Change: Serial/UART and USB CDC Transport

## Why

zenoh-pico supports serial links under `Z_FEATURE_LINK_SERIAL` for UART on Zephyr, ESP-IDF, Arduino, and RPi Pico platforms, plus experimental USB CDC on RPi Pico. zenoh-nostd has zero serial transport support.

Many embedded targets (STM32, ESP32, RPi Pico, nRF52) communicate over UART in industrial and automation contexts. USB CDC enables plug-and-play connectivity over a virtual COM port — critical for field deployment where ethernet/WiFi is unavailable. Without this, nostd is excluded from a large class of embedded use cases.

The current link architecture centers on `ZLinkManager`/`ZLink`/`EmbeddedIOLink`, which abstracts over `embedded_io_async::Read`/`Write`. This is already suitable for UART via the `embedded-hal` ecosystem, but no serial-specific endpoint parsing, configuration (baud rate, parity, flow control), or platform HAL bindings exist.

## What Changes

### 1. zenoh-sansio — Feature flag `transport-serial`

Add to `crates/zenoh-sansio/Cargo.toml`:

```toml
[features]
transport-serial = []
```

No codec changes needed — serial transports use the same Zenoh session handshake (`InitSyn`/`InitAck`/`OpenSyn`/`OpenAck`) and produce the same `Transport<Buff>`. The sansio layer only needs the feature flag for build gating; all logic lives in the link layer.

### 2. zenoh-proto — Endpoint serial URI scheme

Add `serial` and `usb-cdc` URI scheme parsing in `zenoh-proto`'s `Endpoint` type:

- `serial:///dev/ttyUSB0?baud=115200&parity=none&bits=8&stop=1`
- `serial:///dev/ttyAMA0?baud=9600`
- `usb-cdc:///dev/ttyACM0`
- `serial://usart2?baud=115200&tx=PA9&rx=PA10` (embedded, platform-specific pin config)

Add `LinkType::Serial` and `LinkType::UsbCdc` variants. Add serial-specific configuration fields to `Endpoint` or a new `SerialConfig` struct:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct SerialConfig {
    pub baud_rate: u32,
    pub data_bits: u8,       // 5, 6, 7, 8
    pub parity: SerialParity, // None, Even, Odd
    pub stop_bits: u8,       // 1 or 2
    pub flow_control: SerialFlowControl, // None, RtsCts, XonXoff
}

#[derive(Clone, Debug, PartialEq)]
pub enum SerialParity { None, Even, Odd }

#[derive(Clone, Debug, PartialEq)]
pub enum SerialFlowControl { None, RtsCts, XonXoff }
```

Add golden vectors for serial endpoint URI parsing.

### 3. zenoh-nostd — `SerialLink` type

Create `crates/zenoh-nostd/src/io/link/serial.rs`:

```rust
pub struct SerialLink<Tx: embedded_io_async::Write, Rx: embedded_io_async::Read> {
    inner: EmbeddedIOLink<Tx, Rx>,
    config: SerialConfig,
}
```

Serial/UART is a stream-oriented transport (`is_streamed = true`), so the read path uses `read_exact`. MTU for UART is typically small (64–256 bytes) or derived from the underlying buffer size. The link must handle:

- **Framing**: UART provides no message framing. Zenoh transport framing uses length-prefixed messages (the `prefixed` encoding path via `ZTransportRx::decode_prefixed` and `ZTransportTx::flush_prefixed`). This is already handled by the sansio layer — `Handshake` takes `prefixed: bool` and `TransportBuilder` methods expose `.with_prefixed()`.
- **Partial reads**: UART drivers may return fewer bytes than requested. The `SerialLink` read implementation must loop on `read_exact` or handle `WouldBlock` gracefully.
- **Buffering**: UART may need intermediate framing buffering for platforms that cannot do `read_exact` at the HAL level. Propose a `heapless::Vec<u8, N>` accumulation buffer inside `SerialLink` when the HAL does not guarantee full reads.

Implement `ZLink`, `ZLinkTx`, `ZLinkRx` for `SerialLink<Tx, Rx>`.

### 4. zenoh-nostd — `SerialLinkManager`

Create `crates/zenoh-nostd/src/io/link/serial_manager.rs`:

```rust
pub trait SerialHal {
    type Tx: embedded_io_async::Write;
    type Rx: embedded_io_async::Read;

    fn open(
        endpoint: &Endpoint<'_>,
        config: &SerialConfig,
    ) -> impl Future<Output = Result<(Self::Tx, Self::Rx), LinkError>>;
}

pub struct SerialLinkManager<Hal: SerialHal> {
    hal: Hal,
}
```

Implement `ZLinkManager` for `SerialLinkManager<Hal>`:

- `connect()` — calls `Hal::open()` with the parsed endpoint/config, returns `SerialLink`
- `listen()` — serial is inherently point-to-point; `listen` and `connect` behave identically (open the port in read/write mode). `listen` returns an acceptor that yields the single open serial link.

### 5. zenoh-nostd — std implementation (`transport-serial`, `std`)

Add a std serial HAL implementation using the `serialport` crate (or direct `termios` syscalls for `no_std`-adjacent targets):

```rust
#[cfg(feature = "transport-serial")]
#[cfg(feature = "std")]
mod serial_hal_std;

impl SerialHal for StdSerialHal { ... }
```

This opens `/dev/tty*` ports with the configured baud rate and serial parameters. The `std+transport-serial` path is for:

- Linux/Mac host testing
- Integration tests with virtual serial ports (`socat` PTY pairs)
- Debug and golden vector generation against real hardware

### 6. zenoh-nostd — embedded-hal implementation (`transport-serial`, `no_std`)

For embedded targets, the HAL trait is implemented per-platform:

```rust
#[cfg(feature = "transport-serial")]
mod serial_hal_embassy;  // embassy-rs UART integration
mod serial_hal_esp32;    // esp-hal UART
mod serial_hal_stm32;    // stm32-hal UART via embassy or bare-metal
```

The embedded HAL implementation maps to `embedded-io-async::Read`/`Write` from the platform's UART peripheral driver. The `SerialHal` trait's `open()` method takes pin configuration from the endpoint URI (e.g., `tx=PA9&rx=PA10`) and initializes the peripheral.

For embassy-based platforms, this integrates directly with `embassy-rs`'s `Uart` type which already implements `embedded_io_async::Read`/`Write`.

### 7. zenoh-nostd — USB CDC transport (`transport-usb-cdc`)

USB CDC ACM (serial over USB) is architecturally similar to UART but:

- The HAL trait is different: USB device stack (e.g., `usb-device` + `usbd-serial` on embassy, or TinyUSB on bare-metal)
- The read path must handle USB packet framing (endpoint size, 64-byte packets for full-speed)
- The configuration includes USB descriptors (vendor ID, product ID, serial string)

Add a separate feature flag `transport-usb-cdc` and trait:

```rust
#[cfg(feature = "transport-usb-cdc")]
pub trait UsbCdcHal {
    type Tx: embedded_io_async::Write;
    type Rx: embedded_io_async::Read;

    fn open(
        endpoint: &Endpoint<'_>,
    ) -> impl Future<Output = Result<(Self::Tx, Self::Rx), LinkError>>;
}
```

For RPi Pico (RP2040), integrate with `embassy-rp` USB driver. For STM32, integrate with `embassy-stm32` USB driver or TinyUSB.

### 8. zenoh-nostd — Feature flags

Add to `crates/zenoh-nostd/Cargo.toml`:

```toml
[features]
transport-serial = ["zenoh-sansio/transport-serial", "dep:serialport"]
transport-serial-std = ["transport-serial", "std"]
transport-usb-cdc = ["zenoh-sansio/transport-serial"]

# Platform-specific serial HAL implementations (gated):
serial-embassy = ["transport-serial", "embassy-executor"]
serial-stm32 = ["transport-serial", "dep:embassy-stm32"]
serial-esp32 = ["transport-serial", "dep:esp-hal"]
serial-rp2040 = ["transport-serial", "dep:embassy-rp"]
usb-cdc-rp2040 = ["transport-usb-cdc", "dep:embassy-rp"]
usb-cdc-stm32 = ["transport-usb-cdc", "dep:embassy-stm32"]
```

### 9. zenoh-nostd — Session API wiring

Add to `TransportLinkManager`:

```rust
impl<LinkManager> TransportLinkManager<LinkManager> {
    pub async fn serial_connect<Buff>(
        &self,
        endpoint: Endpoint<'_>,
        config: &SerialConfig,
        buff: Buff,
    ) -> Result<TransportLink<SerialLink<Tx, Rx>, Buff>, TransportLinkError>
    where
        LinkManager: SerialHal,
        Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
    { ... }

    pub async fn serial_listen<Buff>(
        &self,
        endpoint: Endpoint<'_>,
        config: &SerialConfig,
        buff: Buff,
    ) -> Result<TransportLink<SerialLink<Tx, Rx>, Buff>, TransportLinkError>
    { ... }
}
```

The serial connect/listen path follows the same pattern as `EmbeddedIOLink`-based `bridge_connect`/`bridge_listen`:
1. Open the serial port via `SerialHal::open()` → get `(Tx, Rx)`
2. Wrap in `SerialLink`
3. Run Zenoh handshake via `Transport::builder(...).connect_async(...)` with `prefixed: true`
4. Return `TransportLink<SerialLink<Tx, Rx>, Buff>`

### 10. Platform crates — HAL bindings

Create or update platform crates:

| Platform crate | UART HAL | USB CDC HAL |
|---|---|---|
| `zenoh-embassy` | `embassy-rs` UART via `embedded-io-async` | `embassy-usb-serial` |
| `zenoh-stm32` | `embassy-stm32` USART | `embassy-stm32` USB |
| New: `zenoh-rp2040` | `embassy-rp` UART | `embassy-rp` USB |
| New: `zenoh-esp32` | `esp-hal` UART via `embedded-io-async` | TBD (ESP USB support) |

Each platform crate exports its `SerialHal` and `UsbCdcHal` implementations as feature-gated modules.

### 11. Tests

| Test | Scope | Method |
|---|---|---|
| Serial endpoint URI parsing | `zenoh-proto` | Unit test + golden vectors |
| Serial config roundtrip | `zenoh-proto` | Encode/decode `SerialConfig` |
| SerialLink read/write | `zenoh-nostd` | Loopback with `embedded-hal-mock` or channel-based fake |
| SerialLink handshake | `zenoh-nostd` | Full `TransportLinkManager` connect/listen over virtual serial pair |
| Std serial integration | Host | `socat` PTY pair, real handshake |
| USB CDC endpoint parsing | `zenoh-proto` | Unit test |
| Serial fuzz target | `zenoh-proto` | Fuzz endpoint URI with serial params |

### 12. Fuzzing

Add fuzz target for serial endpoint parsing:
```rust
// fuzz_targets/serial_endpoint.rs
fuzz_serial_endpoint!();
```

### 13. Property tests

```rust
proptest! {
    #[test]
    fn serial_config_roundtrip(config: SerialConfig) {
        let encoded = config.encode();
        let decoded = SerialConfig::decode(&encoded);
        assert_eq!(config, decoded);
    }
}
```

### 14. Kani harnesses

Add bounded verification for:
- `SerialLink` internal framing buffer (no overrun, no panic on partial read)
- Serial configuration parsing (no integer overflow in baud rate, data bits)
- `SerialHal::open()` preconditions (valid baud rate range, valid pin config)

## Capabilities

### New

- `serial-transport` — Serial/UART transport via `embedded-io-async` Read/Write abstraction with platform HAL bindings
- `usb-cdc-transport` — USB CDC ACM transport for platforms with USB device stack

### Modified

- `zenoh-proto`: `Endpoint` gains `LinkType::Serial`/`LinkType::UsbCdc` and `SerialConfig`
- `zenoh-sansio`: New feature flag `transport-serial`
- `zenoh-nostd`: New link types, managers, feature flags, platform crate integrations

### Unchanged

- Transport codec (`TransportTx`, `TransportRx`, `Transport` in `zenoh-sansio`) — serial uses existing handshake
- Session state machine — serial transports yield the same `TransportLink<_, Buff>`
- Driver, keepalive, lease management — identical to existing transports

## Impact

| Crate | Change | Risk |
|---|---|---|
| `zenoh-proto` | `Endpoint` URI parsing for `serial://` and `usb-cdc://`; `SerialConfig` struct | Low — additive, no existing code modified |
| `zenoh-sansio` | New feature flag `transport-serial` | None |
| `zenoh-nostd` | `SerialLink`, `SerialLinkManager`, `SerialHal` trait, serial HAL implementations, USB CDC modules, feature flags | Medium — new abstractions, platform HAL bindings vary |
| Platform crates | New HAL implementations per target | Medium — platform-specific driver integration |
| `docs/` | Updated compatibility matrix, embedded runtime checklist | None |

## Phasing

| Phase | Scope | Deliverable |
|---|---|---|
| 1 | `zenoh-proto` serial endpoint + config | URI parsing, `SerialConfig`, golden vectors, fuzz target, property tests |
| 2 | `zenoh-nostd` `SerialLink` + `SerialLinkManager` + `SerialHal` trait | Core link abstraction, no_std + std HAL traits, unit tests with mock HAL |
| 3 | Std serial HAL implementation | `serialport` crate integration, `socat` integration test, golden vector generation |
| 4 | embedded-hal UART HAL (embassy) | `embassy-rs` UART integration, `zenoh-embassy` serial support |
| 5 | Platform-specific HAL bindings | STM32, RP2040, ESP32 UART HAL implementations |
| 6 | USB CDC transport | `UsbCdcHal` trait, embassy USB integration, RP2040 + STM32 USB CDC |
| 7 | Verification | Kani harnesses for serial framing buffer, fuzz targets for endpoint parsing, integration test suite |
| 8 | Documentation | Compatibility matrix update, serial transport guide, embedded runtime checklist update |

## Known Gaps

- USB CDC on ESP32 (ESP USB device stack maturity varies by chip — Xtensa vs RISC-V)
- Flow control (RTS/CTS) is platform-dependent and may not be available on all embedded UART HALs
- Parity and stop bit configuration varies across embedded HAL implementations
- Some platforms may not support runtime reconfiguration of UART pins via `SerialHal::open()`
- USB CDC requires a USB device stack (not available on all embedded targets or feature flags)