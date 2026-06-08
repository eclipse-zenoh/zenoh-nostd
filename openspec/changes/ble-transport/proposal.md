# Change: Bluetooth Low Energy (BLE) Transport

## Why

zenoh-pico supports BLE via `Z_FEATURE_LINK_BLUETOOTH` on Arduino (nRF52840, Arduino Nano 33 BLE). BLE is the dominant wireless protocol for low-power embedded devices — wearables, environmental sensors, medical devices, and consumer IoT. Without BLE transport, zenoh-nostd cannot serve battery-operated or coin-cell-powered devices.

BLE offers:
- **Ultra-low power**: standby current in µA, active transmission in mA (vs. WiFi's 100s of mA)
- **Widespread availability**: nRF52, ESP32, STM32WB, RPi Pico W, and most modern MCUs have BLE radios
- **Connection-oriented and connectionless modes**: L2CAP CoC for reliable data, GATT for service discovery and attribute-based communication
- **Built-in MTU negotiation**: ATT MTU up to 512 bytes typically, L2CAP CoC MTU up to 65533 bytes

The existing link architecture (`ZLink`/`ZLinkManager`/`EmbeddedIOLink` in `zenoh-nostd`) is built around `embedded_io_async::Read`/`Write`, which is compatible with L2CAP CoC channels. No transport codec changes are needed — BLE is a stream-oriented transport using the same Zenoh session handshake (`InitSyn`/`InitAck`/`OpenSyn`/`OpenAck`) with prefixed framing.

## What Changes

### 1. zenoh-proto — `ble` URI scheme and `BleConfig`

Add `ble` URI scheme to `ProtocolId`:

```rust
// In endpoint.rs
pub enum ProtocolId {
    Tcp,
    Udp,
    WebSocket,
    Serial,
    Ble,
}

impl TryFrom<Protocol<'_>> for ProtocolId {
    fn try_from(value: Protocol<'_>) -> Result<Self, Self::Error> {
        Ok(match value.as_str() {
            "tcp" => Self::Tcp,
            "udp" => Self::Udp,
            "ws" => Self::WebSocket,
            "serial" => Self::Serial,
            "ble" => Self::Ble,
            _ => crate::zbail!(crate::EndpointError::CouldNotParseProtocol),
        })
    }
}
```

Endpoint URI format:

```
ble://<device_address>?service=<uuid>&characteristic=<uuid>&mtu=<bytes>&interval=<ms>
ble://<device_address>?l2cap_psm=<psm>&mtu=<bytes>
ble://<device_address>?advertisement=<service_uuid>
```

Two sub-protocols (distinguished by query parameters):
- **GATT-based** (`?service=<uuid>&characteristic=<uuid>`): uses GATT notifications for data transfer
- **L2CAP CoC** (`?l2cap_psm=<psm>`): uses L2CAP connection-oriented channels (preferred, default)

Device address format: MAC address (`AA:BB:CC:DD:EE:FF`) or BLE public/random address with type suffix (`AA:BB:CC:DD:EE:FF?type=random`).

Add `BleConfig`:

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum BleTransport {
    Gatt {
        service_uuid: Uuid128,  // 128-bit service UUID
        characteristic_uuid: Uuid128,  // 128-bit characteristic UUID for notifications
    },
    L2capCoC {
        psm: u16,  // Protocol Service Multiplexer
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct BleConfig {
    pub transport: BleTransport,
    pub mtu: u16,              // Requested ATT/L2CAP MTU (23-65533)
    pub conn_interval: u16,    // Connection interval in 1.25ms units (6-3200, 7.5ms-4s)
    pub conn_latency: u16,     // Slave latency (0-499)
    pub supervision_timeout: u16, // Supervision timeout in 10ms units (10-3200)
}
```

Add golden vectors for BLE endpoint URI parsing:

| URI | Expected Config |
|---|---|
| `ble://AA:BB:CC:DD:EE:FF?l2cap_psm=0x1001&mtu=512` | L2CAP CoC, PSM 0x1001, MTU 512 |
| `ble://AA:BB:CC:DD:EE:FF?service=180D&characteristic=2A37&mtu=128` | GATT, heart rate service, MTU 128 |
| `ble://AA:BB:CC:DD:EE:FF?type=random&l2cap_psm=0x1001` | L2CAP CoC, random address type |

### 2. zenoh-sansio — Feature flag `transport-ble`

```toml
[features]
transport-ble = []
```

No codec changes — BLE is a stream-oriented transport (`is_streamed = true`). The sansio layer uses the same prefixed framing as TCP/serial. The feature flag controls build inclusion of BLE-specific endpoint parsing and transport construction paths.

### 3. zenoh-nostd — `BleLink` type (L2CAP CoC path)

Create `crates/zenoh-nostd/src/io/link/ble.rs`:

```rust
pub struct BleLink<Tx: embedded_io_async::Write, Rx: embedded_io_async::Read> {
    inner: EmbeddedIOLink<Tx, Rx>,
    config: BleConfig,
    actual_mtu: u16,  // MTU negotiated during connection
}
```

BLE as L2CAP CoC is a stream-oriented transport (`is_streamed = true`). The read path uses `read_exact`. Key properties:

- **MTU**: L2CAP CoC MTU is negotiated during connection establishment. The effective MTU is `min(local_mtu, peer_mtu)`. On embedded targets, typical values are 247 (nRF52 default) to 512 (with MTU exchange request). On Linux/BlueZ, up to 65533.
- **Connection interval**: Affects throughput and latency. A shorter interval (20-50ms) yields lower latency but higher power consumption. The `BleConfig.conn_interval` field allows the application to balance power vs. performance.
- **Framing**: L2CAP CoC is stream-oriented = prefixed Zenoh framing. The length-prefix frame fits comfortably even in minimal MTU (a 2-byte length prefix is sufficient for frames up to 65535 bytes).

Implement `ZLink`, `ZLinkTx`, `ZLinkRx` for `BleLink<Tx, Rx>`, delegating to `EmbeddedIOLink` read/write.

### 4. zenoh-nostd — `BleLinkManager` and `BleHal` trait

```rust
#[cfg(feature = "transport-ble")]
pub mod ble;

pub trait BleHal {
    type Tx: embedded_io_async::Write;
    type Rx: embedded_io_async::Read;
    type Error: Display;

    /// Open a BLE connection (central/client role)
    async fn connect(
        endpoint: &Endpoint<'_>,
        config: &BleConfig,
    ) -> Result<(Self::Tx, Self::Rx), Self::Error>;

    /// Advertise and accept a BLE connection (peripheral/server role)
    async fn accept(
        endpoint: &Endpoint<'_>,
        config: &BleConfig,
    ) -> Result<(Self::Tx, Self::Rx), Self::Error>;
}
```

The `BleHal` trait captures two roles:
- **Central** (`connect`): scans for advertiser, initiates connection, negotiates MTU, opens L2CAP CoC channel
- **Peripheral** (`accept`): starts advertising, waits for incoming connection, negotiates MTU, opens L2CAP CoC channel

The `BleLinkManager` wraps `BleHal` and implements `ZLinkManager`:

```rust
pub struct BleLinkManager<Hal: BleHal> {
    hal: Hal,
}

impl<Hal: BleHal> ZLinkManager for BleLinkManager<Hal> {
    type Link<'a> = BleLink<Hal::Tx, Hal::Rx>;

    async fn connect(&self, endpoint: Endpoint<'_>) -> Result<Self::Link<'_>, LinkError> {
        let config = parse_ble_config(&endpoint)?;
        let (tx, rx) = Hal::connect(&endpoint, &config).await?;
        let mtu = config.mtu.max(23).min(65533);
        Ok(BleLink {
            inner: EmbeddedIOLink::new(tx, rx, mtu, true),
            config,
            actual_mtu: mtu,
        })
    }

    async fn listen(&self, endpoint: Endpoint<'_>) -> Result<Self::Link<'_>, LinkError> {
        let config = parse_ble_config(&endpoint)?;
        let (tx, rx) = Hal::accept(&endpoint, &config).await?;
        let mtu = config.mtu.max(23).min(65533);
        Ok(BleLink {
            inner: EmbeddedIOLink::new(tx, rx, mtu, true),
            config,
            actual_mtu: mtu,
        })
    }
}
```

### 5. zenoh-nostd — GATT-based transport (alternative path)

For platforms without L2CAP CoC support (some older BLE controllers, or GATT-only stacks), provide a GATT-based transport:

```rust
pub enum BleTransportMode {
    L2capCoC(BleL2capLink),
    Gatt(BleGattLink),
}
```

GATT transport uses:
- **Write Without Response** (ATT opcode 0x52) for TX direction — max payload = MTU - 3 (opcode + handle)
- **Notifications** (ATT opcode 0x1B) for RX direction — max payload = MTU - 3

GATT introduces additional constraints:
- Each Zenoh frame must fit in a single ATT payload, OR the link must provide its own mini-framing (4-byte length prefix within ATT payloads)
- ATT MTU is typically 23 bytes default, up to 512 bytes after MTU exchange
- GATT is NOT stream-oriented (`is_streamed = false`) — each notification/write is an atomic datagram
- The transport must use `decode_raw`/`flush_raw` (datagram mode) instead of `decode_prefixed`/`flush_prefixed`

GATT mode is a fallback for platforms without L2CAP CoC. The protocol choice (`L2capCoC` vs `Gatt`) is selected automatically based on endpoint URI or explicit `?transport=gatt` query param.

### 6. zenoh-nostd — std BLE implementation via BlueZ D-Bus

For host-side testing and integration with Linux gateways, implement `BleHal` via BlueZ D-Bus API:

```rust
#[cfg(feature = "transport-ble")]
#[cfg(feature = "std")]
mod ble_hal_bluez;
```

BlueZ D-Bus integration:
- Use `zbus` crate (pure Rust D-Bus, `no_std`-compatible core) for D-Bus IPC with BlueZ
- `org.bluez.Device1` for connection management
- `org.bluez.LEConnectionManager1` for LE connection parameters
- `org.bluez.LEAdvertisement1` for advertising (peripheral role)
- L2CAP socket via `org.bluez.Network1` for L2CAP CoC channels (BlueZ exposes L2CAP as network sockets)

On macOS, use CoreBluetooth via `objc2` FFI bindings (lower priority, Phase 3).

### 7. zenoh-nostd — Embedded BLE HAL implementations

| Platform | BLE Stack | L2CAP CoC | GATT |
|---|---|---|---|
| nRF52 (embassy-nrf) | `nrf-softdevice` or `embassy-nrf` + `nrf-hal` | Yes (via softdevice L2CAP) | Yes |
| ESP32 (esp-hal) | `esp-hal` + NimBLE (via `esp-wifi` or `esp-ble`) | Yes (NimBLE L2CAP CoC) | Yes |
| STM32WB | `embassy-stm32` + STM32WB BLE stack | Yes | Yes |
| RPi Pico W | `embassy-rp` + Infineon CYW43439 | Limited | Yes |

Each platform implements `BleHal`:

```rust
#[cfg(feature = "transport-ble")]
#[cfg(feature = "embassy-nrf")]
mod ble_hal_nrf;

#[cfg(feature = "transport-ble")]
#[cfg(feature = "esp32")]
mod ble_hal_esp32;
```

For embassy-nrf, the implementation uses `nrf-softdevice` for L2CAP CoC:

```rust
pub struct NrfBleHal;

impl BleHal for NrfBleHal {
    type Tx = L2capCoCTx;
    type Rx = L2capCoCRx;
    type Error = SoftdeviceError;

    async fn connect(endpoint: &Endpoint<'_>, config: &BleConfig) -> Result<(Self::Tx, Self::Rx), Self::Error> {
        // 1. Initialize softdevice with SD_BLE_API_VERSION
        // 2. Start scanning for peer matching endpoint address
        // 3. Initiate connection with configured interval/latency/timeout
        // 4. Exchange MTU request (up to config.mtu)
        // 5. Open L2CAP CoC channel on specified PSM (or 0x1001 default Zenoh PSM)
        // 6. Return Tx/Rx pair wrapping L2CAP channel SDU buffers
    }
}
```

For ESP32, the implementation uses NimBLE (bundled with ESP-IDF):

```rust
pub struct EspBleHal;

impl BleHal for EspBleHal {
    // NimBLE provides GAP API for scanning/advertising
    // and L2CAP API for connection-oriented channels
}
```

### 8. zenoh-nostd — Feature flags

Add to `crates/zenoh-nostd/Cargo.toml`:

```toml
[features]
transport-ble = ["zenoh-sansio/transport-ble"]

# GATT fallback transport (datagram mode)
transport-ble-gatt = ["transport-ble"]

# L2CAP CoC transport (stream mode, default when transport-ble is enabled)
transport-ble-l2cap = ["transport-ble"]

# Platform-specific BLE HAL implementations
ble-bluez = ["transport-ble", "std", "dep:zbus"]
ble-embassy-nrf = ["transport-ble", "dep:embassy-nrf", "dep:nrf-softdevice"]
ble-esp32 = ["transport-ble", "dep:esp-hal", "dep:esp-wifi"]

# Recommended convenience alias
ble-nrf52 = ["transport-ble-l2cap", "ble-embassy-nrf"]
ble-esp32s3 = ["transport-ble-l2cap", "ble-esp32"]
```

### 9. zenoh-proto — `Uuid128` type for BLE UUIDs

Add a 128-bit UUID type for BLE service/characteristic identification:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Uuid128(pub [u8; 16]);

impl Uuid128 {
    /// Create from 16-byte big-endian representation
    pub const fn from_bytes(bytes: [u8; 16]) -> Self { Self(bytes) }

    /// Parse from canonical UUID string format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    pub fn from_str(s: &str) -> Result<Self, EndpointError> { ... }
}
```

BLE 16-bit UUIDs (Bluetooth SIG-assigned) are promoted to 128-bit using the BLE base UUID: `0000xxxx-0000-1000-8000-00805F9B34FB`.

### 10. zenoh-nostd — Session API wiring

```rust
impl<LinkManager> TransportLinkManager<LinkManager> {
    pub async fn ble_connect<Buff>(
        &self,
        endpoint: Endpoint<'_>,
        config: &BleConfig,
        buff: Buff,
    ) -> Result<TransportLink<BleLink<Hal::Tx, Hal::Rx>, Buff>, TransportLinkError>
    where
        LinkManager: BleHal,
        Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
    {
        let link_manager = BleLinkManager::new(LinkManager::new());
        let link = link_manager.connect(endpoint).await?;
        // ... handshake via Transport::builder(...).connect_async(...) with prefixed=true
    }
}
```

### 11. BLE-specific concerns

**MTU Negotiation**:
- ATT MTU exchange happens during connection establishment, before data transfer
- L2CAP CoC MTU is negotiated when the channel is opened
- Default ATT MTU is 23 bytes; typical negotiated MTU is 247 (nRF52) to 512
- L2CAP CoC MTU default is 65533, but controller/peer may limit to 512 or 1024
- The effective MTU from `BleLink`'s `mtu()` must reflect the negotiated value, not the requested value

**Connection Interval**:
- Controls throughput and power consumption
- `conn_interval = 6` → 7.5ms, high throughput, high power
- `conn_interval = 100` → 125ms, moderate throughput
- `conn_interval = 400` → 500ms, low throughput, low power
- The link should allow dynamic renegotiation of connection parameters

**Power Saving**:
- The transport should expose a power-saving mode (e.g., `set_power_mode(PowerMode::LowPower)`)
- Low-power mode: increase connection interval, enable slave latency
- High-throughput mode: minimize connection interval, disable slave latency
- Idle detection: after `N` seconds without data, automatically increase interval

**Connection Supervision**:
- BLE connections have a supervision timeout (default 10s on nRF, 20s on BlueZ)
- The Zenoh lease/keepalive mechanism must be shorter than the supervision timeout
- For a 10s supervision timeout, Zenoh lease should be ≤4s with keepalive every 2s
- The `BleConfig.supervision_timeout` must not be exceeded by the Zenoh lease duration

**Security**:
- BLE supports pairing/bonding and link-layer encryption (LESC, LE Legacy)
- The transport should support `?secure=true` to require encryption
- MITM protection via passkey or numeric comparison (`?mitm=true`)
- The BLE security architecture is additive: plaintext connection is always available, security is opt-in

### 12. Protocol Service Multiplexer (PSM) allocation

Define a Zenoh-specific BLE PSM for L2CAP CoC:

| PSM | Assignment |
|---|---|
| `0x1001` | Zenoh L2CAP CoC transport (fixed, non-SIG-assigned, in the dynamic range 0x1001-0xFFFF) |

This PSM value must be documented and consistent across all implementations. The PSM is used for:
1. L2CAP CoC channel establishment
2. Service discovery (SDP-like via `l2cap_psm` in endpoint URI)
3. Multiple concurrent Zenoh sessions over separate L2CAP channels

### 13. Tests

| Test | Scope | Method |
|---|---|---|
| BLE endpoint URI parsing | `zenoh-proto` | Unit test + golden vectors |
| `BleConfig` roundtrip | `zenoh-proto` | Encode/decode `BleConfig` |
| `Uuid128` parse/format | `zenoh-proto` | Unit test with canonical UUID strings |
| BLE MTU negotiation logic | `zenoh-nostd` | Mock `BleHal` with configurable peer MTU |
| `BleLink` read/write (L2CAP CoC) | `zenoh-nostd` | Channel-based mock Tx/Rx pair |
| `BleLink` read/write (GATT) | `zenoh-nostd` | Channel-based mock with ATT-sized buffers |
| BLE handshake (L2CAP CoC) | `zenoh-nostd` | Full `BleLinkManager` connect/listen over channel pair |
| BLE handshake (GATT) | `zenoh-nostd` | Full GATT connect/listen over ATT-sized buffers |
| BlueZ integration | Host | Real BLE adapter pair (two dongles via btlejack or hardware loopback) |
| Connection interval control | `zenoh-nostd` | Verify mock `BleHal` applies configured interval |
| MTU negotiation | `zenoh-nostd` | Verify actual_mtu reflects min(local_mtu, peer_mtu) |

### 14. Fuzzing

Add fuzz target for BLE endpoint parsing:
```rust
// fuzz_targets/ble_endpoint.rs
fuzz_ble_endpoint!();  // Fuzz URI parsing with BLE params: address, PSM, MTU, UUIDs
```

### 15. Property tests

```rust
proptest! {
    #[test]
    fn ble_config_roundtrip(config: BleConfig) {
        let encoded = config.encode();
        let decoded = BleConfig::decode(&encoded);
        assert_eq!(config, decoded);
    }

    #[test]
    fn uuid128_roundtrip(uuid: Uuid128) {
        let s = uuid.to_string();
        let parsed = Uuid128::from_str(&s).unwrap();
        assert_eq!(uuid, parsed);
    }

    #[test]
    fn ble_mtu_bounds(mtu in 23u16..=65533) {
        let actual = mtu.min(peer_mtu);
        assert!(actual >= 23);
        assert!(actual <= mtu);
        assert!(actual <= peer_mtu);
    }
}
```

### 16. Kani harnesses

Add bounded verification for:
- `BleConfig` parsing (no integer overflow in MTU, interval, latency, timeout fields)
- MTU negotiation logic (min/max bounds, peer limit)
- Connection interval validation (within spec bounds 6–3200)
- GATT buffer assembly (no overrun when reassembling Zenoh frames from ATT notifications)

## Capabilities

### New

- `ble-transport` — BLE transport via L2CAP CoC (stream mode) with fallback to GATT notifications (datagram mode)
- `ble-config` — `BleConfig` with MTU, connection interval, latency, and supervision timeout parameters
- `ble-endpoint` — `ble://` URI scheme with MAC address, PSM, UUID, and BLE-specific query parameters
- `Uuid128` — 128-bit BLE UUID type for service/characteristic identification

### Modified

- `zenoh-proto`: `ProtocolId` gains `Ble` variant; new `BleConfig` and `Uuid128` types
- `zenoh-sansio`: New feature flag `transport-ble`
- `zenoh-nostd`: New `BleLink`, `BleLinkManager`, `BleHal` trait; feature flags; platform BLE HAL implementations

### Unchanged

- Transport codec (`TransportTx`, `TransportRx`, `Transport` in `zenoh-sansio`) — BLE uses existing handshake with prefixed framing
- Session state machine — BLE yields the same `TransportLink<_, Buff>`
- Driver, keepalive, lease management — identical to TCP/serial, with additional constraint that lease < BLE supervision timeout
- `EmbeddedIOLink` — reused as `BleLink` inner type

## Impact

| Crate | Change | Risk |
|---|---|---|
| `zenoh-proto` | New `ProtocolId::Ble`, `BleConfig`, `Uuid128`; `ble://` URI parsing | Low — additive, no existing code modified |
| `zenoh-sansio` | New feature flag `transport-ble` | None |
| `zenoh-nostd` | `BleLink`, `BleLinkManager`, `BleHal` trait, platform BLE HAL implementations, feature flags | Medium — new abstractions, BLE HAL varies significantly across platforms |
| `zenoh-nostd` (std) | BlueZ D-Bus integration via `zbus` for `BleHal` on Linux | Medium — D-Bus async integration, `zbus` dependency |
| `zenoh-nostd` (embassy-nrf) | `nrf-softdevice` L2CAP CoC integration | Medium — softdevice driver complexity, interrupt handling |
| `zenoh-nostd` (esp32) | NimBLE L2CAP CoC via `esp-hal`/`esp-wifi` | Medium — ESP BLE stack maturity varies by target |
| `docs/` | Updated compatibility matrix, BLE transport guide | None |

## Phasing

| Phase | Scope | Deliverable |
|---|---|---|
| 1 | `zenoh-proto` BLE endpoint + config | URI parsing, `BleConfig`, `Uuid128`, golden vectors, fuzz target, property tests |
| 2 | `zenoh-nostd` `BleLink` + `BleLinkManager` + `BleHal` trait | Core link abstraction, mock HAL for testing, unit tests |
| 3 | Std BLE HAL — BlueZ/Linux | BlueZ D-Bus `BleHal` via `zbus`, L2CAP socket integration, integration test with real adapter |
| 4 | GATT fallback transport | `BleGattLink` (datagram mode), ATT MTU handling, notification/indication plumbing |
| 5 | embassy-nrf BLE HAL | `nrf-softdevice` L2CAP CoC integration, `BleHal` for nRF52, power mode control |
| 6 | ESP32 BLE HAL | NimBLE L2CAP CoC integration, `BleHal` for ESP32-S3/C3/H2 |
| 7 | Connection parameter management | Dynamic interval negotiation, power saving modes, supervision timeout ↔ lease interaction |
| 8 | BLE security | Pairing, LESC encryption, MITM protection, `?secure=true` and `?mitm=true` support |
| 9 | Verification | Kani harnesses for BLE config/mtu/interval, fuzz targets for endpoint parsing, integration test suite with BLE mock |
| 10 | Documentation | Compatibility matrix update, BLE transport guide, power management guide, embedded runtime checklist update |

## Known Gaps

- L2CAP CoC support is not universal across BLE controllers. Some older BLE 4.0 controllers only support GATT. The GATT fallback path is provided but has lower throughput (MTU up to 512 bytes vs 65533 for L2CAP CoC).
- BLE on macOS currently has no plan beyond Phase 3 (BlueZ/Linux). A CoreBluetooth implementation via `objc2` would require a separate phase.
- ESP32 BLE support varies by chip: ESP32 (original) uses Bluedroid, ESP32-S3/C3/H2 use NimBLE. The HAL must handle both.
- BLE connection interval negotiation is asymmetric — the Peripheral (advertiser) typically suggests parameters, the Central (scanner) applies them or requests changes. The transport must handle both roles correctly.
- The Zenoh PSM (`0x1001`) is unregistered in the Bluetooth SIG PSM range. For production deployments, a registered PSM should be obtained or the PSM must be configurable.
- BLE pairing/bonding is not implemented in Phase 1. The initial implementation is unencrypted. Security is Phase 8.
- On targets with the BLE controller integrated with the application CPU (nRF52, ESP32), BLE interrupts and softdevice context switching may interfere with Zenoh protocol timing. This is a platform integration concern, not a transport architecture concern.
- BLE advertising interval for peripheral role affects discoverability latency. Default is 20ms–10.24s. The transport should expose `?adv_interval=<ms>` in the endpoint URI.