# Change: Raw Ethernet Transport

## Why

zenoh-pico supports `Z_FEATURE_RAWETH_TRANSPORT` for raw Ethernet II frames (Linux AF_PACKET socket, Zephyr Ethernet driver). This is critical for real-time and industrial networks where TCP/UDP/IP overhead is unacceptable — deterministic control loops, AVB/TSN networks, and tightly-coupled embedded systems sharing a Layer 2 segment.

zenoh-nostd has no raw Ethernet transport. The current link architecture centers on stream-oriented transports (TCP, serial) and datagram transports (UDP). Raw Ethernet is neither — it operates at Layer 2 with its own addressing (MAC), framing (Ethernet II / 802.3), and MTU constraints.

Without raw Ethernet, nostd cannot participate in hard real-time networks where the IP stack adds jitter, cannot run on platforms without a TCP/IP stack, and cannot interoperate with zenoh-pico nodes on shared EtherType-based Layer 2 segments.

## What Changes

### 1. zenoh-proto — Endpoint URI scheme for raw Ethernet

Add `eth` URI scheme parsing in `zenoh-proto`'s `Endpoint` type:

- `eth://interface?ethertype=0xZENO&src=AA:BB:CC:DD:EE:FF`
- `eth://eth0?ethertype=0x88B5&promisc=true`
- `eth://eth1?ethertype=0xZENO&vlan=100`

Add `LinkType::Ethernet` variant. Add `EthernetConfig` struct:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct EthernetConfig {
    pub interface: HeaplessString<64>,
    pub ethertype: u16,              // configurable EtherType for Zenoh frames
    pub src_mac: Option<MacAddress>,
    pub promiscuous: bool,
    pub vlan_id: Option<u16>,        // 802.1Q VLAN tagging
}
```

Add `MacAddress` type:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MacAddress([u8; 6]);
```

Add golden vectors for raw Ethernet endpoint URI parsing, including MAC address and EtherType variants.

### 2. zenoh-sansio — Feature flag `transport-raweth`

Add to `crates/zenoh-sansio/Cargo.toml`:

```toml
[features]
transport-raweth = []
```

No transport codec changes — raw Ethernet uses the same Zenoh session handshake (`InitSyn`/`InitAck`/`OpenSyn`/`OpenAck`) once a link is established. The sansio layer only needs the feature flag for build gating. All link-level logic lives in the transport crate.

### 3. zenoh-nostd — Raw Ethernet link type

Create `crates/zenoh-nostd/src/io/link/ethernet.rs`:

```rust
pub struct RawEthernetLink<Driver: RawEthernetDriver> {
    driver: Driver,
    config: EthernetConfig,
    mtu: usize,
    // Framing buffer for reassembly
    rx_buf: heapless::Vec<u8, { MAX_ETH_FRAME }>,
}
```

Raw Ethernet is a datagram-oriented transport (`is_streamed = false`). The link must handle:

- **Ethernet II framing**: Each send constructs a complete Ethernet frame with destination MAC (broadcast or unicast), source MAC, EtherType, payload, and FCS (handled by NIC hardware).
- **Frame-level MTU**: Standard Ethernet MTU is 1500 bytes, jumbo frames up to 9000 bytes. The link MUST reject sends larger than the negotiated MTU.
- **Fragmentation**: For Zenoh messages exceeding MTU, implement transport-level fragmentation (the Zenoh protocol already supports `TransportMessage` fragmentation at the session layer — wire into the existing `write_fragmented` path).
- **EtherType filtering**: Receive path filters by configured EtherType to avoid processing non-Zenoh traffic.
- **Promiscuous mode**: When enabled, the link receives all frames on the interface, not just those addressed to the local MAC.

Implement `ZLink`, `ZLinkTx`, `ZLinkRx` for `RawEthernetLink<Driver>`.

### 4. zenoh-nostd — Raw Ethernet driver trait

```rust
pub trait RawEthernetDriver {
    type Error: core::fmt::Debug;

    /// Transmit a raw Ethernet frame.
    /// The frame includes Ethernet II header + payload (no FCS — handled by hardware).
    fn send(&mut self, frame: &[u8]) -> Result<(), Self::Error>;

    /// Receive a raw Ethernet frame. Blocks until a frame is available or timeout.
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    /// Get the interface MAC address.
    fn mac_address(&self) -> MacAddress;

    /// Get the interface MTU (maximum frame payload size excluding Ethernet header).
    fn mtu(&self) -> usize;

    /// Enable or disable promiscuous mode.
    fn set_promiscuous(&mut self, enabled: bool) -> Result<(), Self::Error>;
}
```

### 5. zenoh-nostd — Linux AF_PACKET implementation (std)

Create `crates/zenoh-nostd/src/io/link/ethernet_afpacket.rs`, gated on `cfg(all(feature = "transport-raweth", feature = "std"))`:

- Open `AF_PACKET` socket with `SOCK_RAW` protocol bound to the configured EtherType
- `send()`: construct Ethernet II frame header + payload, write to `AF_PACKET` socket with `sendto()` to `sll_addr` (broadcast MAC or unicast)
- `recv()`: read from `AF_PACKET` socket, filter by EtherType, handle truncated frames
- `mac_address()`: via `SIOCGIFHWADDR` ioctl on the interface
- `mtu()`: via `SIOCGIFMTU` ioctl
- `set_promiscuous()`: via `SIOCGIFFLAGS`/`SIOCSIFFLAGS` with `IFF_PROMISC`

Requires:
- `libc` crate for socket/ioctl bindings (or direct `nix` crate usage)
- Privileged capabilities (`CAP_NET_RAW` or root) for `AF_PACKET` sockets

### 6. zenoh-nostd — embedded-smoltcp implementation

Create `crates/zenoh-nostd/src/io/link/ethernet_smoltcp.rs`, gated on `cfg(all(feature = "transport-raweth", feature = "smoltcp"))`:

`smoltcp` provides a `phy::Device` trait with `send()`/`recv()` methods that operate on raw Ethernet frames. Integration:

- Wrap `smoltcp::phy::Device` in the `RawEthernetDriver` trait
- bypass the TCP/IP layers in smoltcp — send/receive directly on the raw device
- Use smoltcp's `Interface` for MAC address and MTU queries

### 7. zenoh-nostd — embedded-embassy-net implementation

Create `crates/zenoh-nostd/src/io/link/ethernet_embassy.rs`, gated on `cfg(all(feature = "transport-raweth", feature = "embassy-net"))`:

`embassy-net` provides `embassy_net::driver::Driver` trait with `transmit()`/`receive()` that operate on raw Ethernet frames. Integration:

- Hook into `embassy_net::driver::Driver` for raw frame send/receive
- Bypass the TCP/UDP/ICMP layers in `embassy-net`
- Embassy-net already handles MAC/PHY initialization — reuse that, add Zenoh EtherType filtering

### 8. zenoh-nostd — ARP handling (std)

Raw Ethernet on Linux requires ARP resolution to send frames to unicast MAC addresses. For broadcast Zenoh discovery, frames go to `FF:FF:FF:FF:FF:FF` (no ARP needed). For unicast to known peers:

- Option 1 (simple): Use broadcast MAC for all traffic. Works for small segments, wasteful on larger networks.
- Option 2 (full): Implement ARP via `AF_PACKET` + `ETH_P_ARP` socket. Cache ARP entries with TTL. On send to unknown IP, issue ARP request and wait for reply (with timeout).

Propose Option 1 for initial implementation (broadcast-only), with Option 2 as a follow-up feature-gated behind `arp-support`.

### 9. zenoh-nostd — Transport link manager raw Ethernet integration

Add to `TransportLinkManager`:

```rust
impl<Driver: RawEthernetDriver> TransportLinkManager<Driver> {
    pub async fn eth_connect<Buff>(
        &self,
        endpoint: Endpoint<'_>,
        config: &EthernetConfig,
        buff: Buff,
    ) -> Result<TransportLink<RawEthernetLink<Driver>, Buff>, TransportLinkError>
    { ... }
}
```

The `eth_connect()` path:
1. Parse endpoint + config
2. Open the raw Ethernet interface via `Driver::open()`
3. Wrap in `RawEthernetLink`
4. Run Zenoh handshake via `Transport::builder(...).connect_async(...)`
5. Return `TransportLink<RawEthernetLink<Driver>, Buff>`

`listen()` for raw Ethernet: open the interface and listen for incoming `InitSyn` frames on the configured EtherType. Since Ethernet is a shared medium, a single interface can both initiate and accept connections — `listen()` enters a receive loop filtering for `InitSyn` with the Zenoh EtherType.

### 10. zenoh-nostd — Feature flags

Add to `crates/zenoh-nostd/Cargo.toml`:

```toml
[features]
transport-raweth = ["zenoh-sansio/transport-raweth"]
transport-raweth-std = ["transport-raweth", "std", "dep:libc"]
transport-raweth-smoltcp = ["transport-raweth", "dep:smoltcp"]
transport-raweth-embassy = ["transport-raweth", "dep:embassy-net"]
arp-support = ["transport-raweth"]
```

### 11. Tests

| Test | Scope | Method |
|---|---|---|
| Ethernet endpoint URI parsing | `zenoh-proto` | Unit test + golden vectors |
| EthernetConfig roundtrip | `zenoh-proto` | Encode/decode `EthernetConfig` |
| RawEthernetLink send/recv | `zenoh-nostd` | Loopback with in-memory Ethernet frame queue (fake `RawEthernetDriver`) |
| RawEthernetLink handshake | `zenoh-nostd` | Full `TransportLinkManager` connect + listen over in-memory Ethernet pair |
| AF_PACKET socket creation | Host | Requires `CAP_NET_RAW` — test with `veth` pair |
| MTU enforcement | `zenoh-nostd` | Send oversized frame, verify error |
| EtherType filtering | `zenoh-nostd` | Send non-Zenoh EtherType, verify frame dropped |
| Promiscuous mode | `zenoh-nostd` | Toggle promiscuous, verify behavior |
| MAC address parsing | `zenoh-proto` | Valid and invalid MAC formats |
| VLAN tagging | `zenoh-proto` | Parse `vlan=100`, verify `EthernetConfig` |

### 12. Fuzzing

Add fuzz target for raw Ethernet endpoint parsing:
```
fuzz_targets/ethernet_endpoint.rs
```

### 13. Property tests

```rust
proptest! {
    #[test]
    fn mac_address_roundtrip(mac: MacAddress) {
        let s = mac.to_string();
        let parsed = MacAddress::try_from(&s).unwrap();
        assert_eq!(mac, parsed);
    }

    #[test]
    fn ethernet_config_roundtrip(config: EthernetConfig) {
        let encoded = config.encode();
        let decoded = EthernetConfig::decode(&encoded);
        assert_eq!(config, decoded);
    }
}
```

### 14. Kani harnesses

Add bounded verification for:
- `RawEthernetLink` framing buffer (no overrun on maximum MTU receive)
- MAC address parsing bounds (6-byte fixed array)
- MTU validation (no integer overflow in length checks)
- EtherType filtering (no panic on zero-length frame with filtered EtherType)

## Capabilities

### New

- `raw-ethernet-transport` — Raw Ethernet II frame transport with configurable EtherType, MAC addressing, promiscuous mode, and VLAN tagging
- Platform-specific driver backends: `af-packet` (std Linux), `smoltcp` (embedded), `embassy-net` (embedded)

### Modified

- `zenoh-proto`: `Endpoint` gains `LinkType::Ethernet` and `EthernetConfig` with `MacAddress`
- `zenoh-sansio`: New feature flag `transport-raweth`
- `zenoh-nostd`: New link type, driver trait, platform implementations, feature flags

### Unchanged

- Transport codec (`TransportTx`, `TransportRx`, `Transport`) — raw Ethernet uses existing handshake and session protocol
- Session state machine — raw Ethernet yields the same `TransportLink<_, Buff>`
- Driver, keepalive, lease management — identical to existing transports

## Impact

| Crate | Change | Risk |
|---|---|---|
| `zenoh-proto` | `Endpoint` URI parsing for `eth://`; `EthernetConfig`, `MacAddress` types; golden vectors | Low — additive, no existing code modified |
| `zenoh-sansio` | New feature flag `transport-raweth` | None |
| `zenoh-nostd` | `RawEthernetLink`, `RawEthernetDriver` trait, AF_PACKET (std), smoltcp and embassy-net backends, feature flags | Medium — platform-specific socket/driver integration, privileges |
| `openspec/protocol-features` | Add raw Ethernet to transport matrix | None |

## Known Gaps

- ARP resolution for unicast MAC addressing is deferred behind `arp-support` feature flag; initial implementation uses broadcast-only
- Ethernet frame checksum (FCS) is handled by NIC hardware on all supported platforms — no software FCS generation or verification
- Jumbo frame support (>1500 MTU) depends on NIC/driver capability; the link will negotiate MTU at session establishment
- VLAN tagging (802.1Q) requires kernel `CONFIG_VLAN_8021Q` on Linux; on embedded, smoltcp/embassy-net may not support VLAN in raw mode
- `AF_PACKET` requires `CAP_NET_RAW` capability on Linux — not available in containers without `--cap-add=NET_RAW`
- Platform-specific: macOS does not support `AF_PACKET` sockets (BPF needed instead); Windows requires WinPcap/Npcap — these are deferred