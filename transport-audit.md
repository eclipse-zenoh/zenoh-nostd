# Transport Audit

## TCP
**File:** `platforms/zenoh-std/src/tcp.rs`
**Status:** ⚠️ Functional with bugs

### ZLink trait correctness
- ✅ `ZLinkInfo`: mtu() and is_streamed() return correct values (streamed = true)
- ✅ `ZLinkTx`: write_all delegates to TcpStream::write_all
- ✅ `ZLinkRx`: read and read_exact delegate to TcpStream
- ✅ `ZLink`: split() via try_clone into StdTcpLinkTx/Rx
- ✅ connect() via lib.rs with set_nodelay
- ✅ listen() via TcpListener::accept in lib.rs

### Issues
- ❌ **`expect("try_clone failed")` in `split()` (line 80-81):** Panic path if try_clone fails. Should propagate as `LinkError` instead of panicking.
- ❌ **Wrong error variant for read errors (lines 56, 60, 66, 70):** Uses `LinkError::LinkTxFailed` for read failures. Should be `LinkRxFailed`. This is a systematic bug across several transports.
- ❌ **No read timeout set:** `TcpStream::set_read_timeout` is never called in lib.rs, making blocking reads potentially indefinite.

### Matches zenoh-pico?
zenoh-pico TCP feature: `Z_FEATURE_LINK_TCP` — connect, listen, nodelay, timeout. Nearly complete, but the `expect` panic and missing read timeout diverge.

---

## UDP unicast
**File:** `platforms/zenoh-std/src/udp.rs`
**Status:** ❌ Has panic paths

### ZLink trait correctness
- ✅ `ZLinkInfo`: mtu(), is_streamed = false (correct for UDP)
- ✅ `ZLinkTx`: write_all via socket.send() (correct for connected UDP)
- ⚠️ `ZLinkRx`: read via socket.recv_from() — OK
- ✅ connect() via lib.rs: binds INADDR_ANY, connects to peer
- ✅ listen() via lib.rs: binds addr, peeks to discover peer, connects

### Issues
- ❌ **`unimplemented!()` in `read_exact` (lines 58-59, 68-69):** Panic path. UDP is datagram-based so read_exact is semantically wrong, but should return `Err(LinkError::LinkRxFailed)` instead of panicking.
- ❌ **`expect("try_clone failed")` in `split()` (lines 79-80):** Panic path.
- ❌ **Wrong error variant for read (lines 55, 65):** Uses `LinkTxFailed` instead of `LinkRxFailed`.
- ❌ **`write_all` drops byte count silently:** `socket.send()` can send fewer bytes than buffer length (though rare for UDP < MTU). Should check return value.

### Matches zenoh-pico?
zenoh-pico UDP unicast feature: `Z_FEATURE_LINK_UDP_UNICAST`. Core behavior matches, but the `unimplemented!()` panic means any code path calling read_exact on UDP will crash the process.

---

## UDP multicast
**File:** `platforms/zenoh-std/src/mcast.rs`
**Status:** ❌ Dead code — not compiled

### ZLink trait correctness
- ⚠️ `ZLinkInfo`: mtu() = 8192, is_streamed = false (correct)
- ⚠️ `ZLinkTx`: write_all via async send_to
- ⚠️ `ZLinkRx`: read via async recv_from, read_exact returns Err (correct for datagram)
- ⚠️ `ZLink`: split() via Arc clone

### Issues
- ❌ **Not included in lib.rs:** No `mod mcast;` — the entire file is dead code never compiled or used by StdLinkManager.
- ❌ **Async methods with sync trait:** Uses `async fn write_all`, `async fn read`, `async fn read_exact`, but `ZLinkTx`/`ZLinkRx` traits are **synchronous**. If `mod mcast` were added to lib.rs, it would fail to compile.
- ❌ **IPv4 only (lines 28-31):** Rejects IPv6 with `Err(LinkError::CouldNotConnect)` — should provide a clearer error.
- ❌ **No TTL/loop configuration:** zenoh-pico allows configuring multicast TTL and loop. These are hardcoded to OS defaults.

### Matches zenoh-pico?
zenoh-pico multicast: `Z_FEATURE_LINK_UDP_MULTICAST`. The concept is correct but the module is dead code. Multicast is handled ad-hoc in `discovery.rs` which directly uses `async_net::UdpSocket` with `join_multicast_v4`.

---

## Serial (std)
**File:** `platforms/zenoh-std/src/serial.rs`
**Status:** ⚠️ Functional but has bugs

### Issues
- ❌ **`is_streamed()` returns `false` (line 38):** UART is a streamed protocol. Should return `true`. All embedded platforms (Flipper, Zephyr, RP2040, Arduino, etc.) correctly return `true`.
- ❌ **MTU = `u16::MAX` (line 22):** Completely unrealistic. Serial ports typically handle 256-1500 bytes. Should be a configurable or sensible default (~256). This can cause buffer over-allocation upstream.
- ✅ Arc<Mutex<>> split works correctly.
- ✅ Timeout set to 10ms.
- ✅ No `expect`/`unwrap`/`panic` in production paths.

### Matches zenoh-pico?
zenoh-pico serial: `Z_FEATURE_LINK_SERIAL`. Core functionality matches, but the wrong `is_streamed` value and absurd MTU could cause protocol framing issues.

---

## Serial (embedded platforms)

### Flipper (zenoh-flipper)
**File:** `platforms/zenoh-flipper/src/serial.rs`
**Status:** ✅ Complete
- Uses FuriHal UART FFI (`furi_hal_uart_init/tx/rx/deinit`)
- is_streamed = true ✅
- MTU = 256 ✅
- Proper Drop (deinit)
- read_exact loops with 1000ms timeout
- ❌ Unsafe FFI without safety comments

### Zephyr
**File:** `platforms/zenoh-zephyr/src/serial.rs`
**Status:** ✅ Complete
- Uses Zephyr UART FFI (`device_get_binding`, `uart_poll_out/in`)
- is_streamed = true ✅
- MTU = 256 ✅
- Busy-loop polling read
- read_exact with retry on -1, error on < -1
- ❌ `unsafe impl Send/Sync` without safety justification

### RP2040
**File:** `platforms/zenoh-rp2040/src/serial.rs`
**Status:** ✅ Complete
- Uses RP2040 SDK UART FFI
- is_streamed = true ✅
- MTU = 256 ✅
- UART init with configurable baud, 8N1 format
- Read uses `uart_is_readable()` before `uart_getc()`
- ❌ write_all silently ignores errors (no return value check on `uart_putc_raw`)

### Arduino
**File:** `platforms/zenoh-arduino/src/serial.rs`
**Status:** ✅ Complete
- Uses Arduino Serial FFI
- is_streamed = true ✅
- MTU = 256 ✅
- Proper `available()` check before read
- read_exact loops with retry on 0

### FreeRTOS
**File:** `platforms/zenoh-freertos/src/serial.rs`
**Status:** ✅ Complete
- Uses FreeRTOS UART FFI
- is_streamed = true ✅
- MTU = 256 ✅
- Byte-by-byte read/write with timeout
- Proper Drop (deinit via FFI)

### Mbed OS
**File:** `platforms/zenoh-mbedos/src/serial.rs`
**Status:** ✅ Complete
- Uses Mbed serial FFI (`mbed_serial_open/close/write/read`)
- is_streamed = true ✅
- MTU = 256 ✅
- Proper Drop (close handle)
- read_exact loops with retry on 0

### STM32
**File:** `platforms/zenoh-stm32/src/serial.rs`
**Status:** ❌ Stub
- All operations return errors
- No actual STM32 HAL integration
- Documented as "requires STM32 HAL UART FFI"

### OpenCR
**File:** `platforms/zenoh-opencr/src/serial.rs`
**Status:** ✅ Complete
- Uses STM32 HAL UART FFI (`huart_lookup`, `HAL_UART_Transmit/Receive`)
- is_streamed = true ✅
- MTU = 256 ✅
- read_exact chunks in 64-byte blocks
- read() capped at 256 bytes
- ❌ read() returns `len` (the requested size) not actual received count — lies about how many bytes were read

---

## BLE (Bluetooth)
**File:** `platforms/zenoh-std/src/bt.rs`
**Status:** ❌ Stub, 37 lines

### Issues
- ❌ **Complete stub:** All operations return `Err(LinkError::LinkTxFailed)` or `Err(LinkError::LinkRxFailed)`.
- ❌ **`unimplemented!()` in `split()` (line 36):** Panic path. This is unreachable if all operations error, but still a code smell.
- ❌ **No actual Bluetooth implementation:** No BlueZ, no btleplug, no RFCOMM support.
- ❌ **Not included in listen() path in lib.rs:** Only connect is theoretically supported; listen has no "bt" arm.

### Matches zenoh-pico?
zenoh-pico BLE: `Z_FEATURE_LINK_BLUETOOTH`. zenoh-pico has a real BLE implementation for Arduino. This is entirely absent.

---

## TLS
**File:** `platforms/zenoh-std/src/tls.rs`
**Status:** ⚠️ Partial implementation

### ZLink trait correctness
- ✅ connect() with `TlsConnector`
- ✅ connect_with_cert() with PKCS#12 identity
- ✅ set_nodelay on underlying TCP
- ✅ MTU = `u16::MAX - 60`
- ✅ `is_streamed = true`
- ✅ No `expect`/`unwrap`/`panic` in production paths

### Issues
- ❌ **No listen() path in lib.rs (line 179-252):** The `listen()` match has no "tls" arm. Azenoh-pico supports TLS on both client and server sides.
- ❌ **No explicit SNI configuration (line 23):** The `connector.connect(domain, tcp)` passes the domain name which native_tls uses for SNI, so this is actually correct implicitly.
- ❌ **No certificate verification customization:** zenoh-pico allows disabling server verification for testing. No equivalent.

### Matches zenoh-pico?
zenoh-pico TLS: `Z_FEATURE_LINK_TLS`. zenoh-pico supports both client and server TLS. This implementation is client-only.

---

## WebSocket
**File:** `platforms/zenoh-std/src/ws.rs`
**Status:** ⚠️ Has implementation bugs

### ZLink trait correctness
- ✅ connect() in lib.rs via tungstenite client
- ✅ Wraps TcpStream with WebSocket framing
- ✅ `ZLink`: split() via Arc clone

### Issues
- ❌ **No listen() path in lib.rs:** The `listen()` match has no "ws" arm. zenoh-pico supports WebSocket on both client and server (via Emscripten).
- ❌ **`is_streamed()` returns `false` (lines 29, 34, 39):** WebSocket is a framed streaming protocol over TCP. While individual messages are discrete, the underlying transport is streamed. This should likely be `true`.
- ❌ **`read_exact` incorrectly delegates to `read` (lines 69-71, 87-89):** Just calls `self.read(buffer).map(|_| ())` which doesn't ensure the buffer is completely filled. A short read returns success with partial data.
- ❌ **Hidden allocation on every write (lines 45, 52):** `Message::Binary(buffer.to_vec())` allocates a Vec for every outgoing message. No_std incompatible. Should use `Message::Binary(buffer.into())` if possible or use a write buffer.
- ❌ **Non-binary frames cause `LinkRxFailed` (lines 64-65):** Ping/Pong/Close frames from the peer cause data loss. Should at minimum handle WebSocket control frames.

### Matches zenoh-pico?
zenoh-pico WS: `Z_FEATURE_LINK_WS`. zenoh-pico supports WebSocket in Emscripten builds. This implementation has proper connect, but the listen path is missing and read_exact is buggy.

---

## Raw Ethernet
**File:** `platforms/zenoh-std/src/raw_eth.rs`
**Status:** ✅ Complete (Linux-only)

### ZLink trait correctness
- ✅ `ZLinkInfo`: mtu = 1500, is_streamed = false (correct for raw Ethernet datagrams)
- ✅ `ZLinkTx`: write_all via libc::send on AF_PACKET socket
- ✅ `ZLinkRx`: read via libc::recv, read_exact loops with manual offset tracking
- ✅ `ZLink`: split() via Arc clone
- ✅ Proper Drop for fd cleanup
- ✅ Non-Linux stub returns errors gracefully

### Issues
- ❌ **No safety comments on unsafe blocks (lines 23-31, 35, 37-39, 48-54, 56-58, 66, 75, 87):** Multiple `unsafe` blocks without safety documentation per the project's AGENTS.md requirement.
- ❌ **Non-Linux stub doesn't implement `ZLink`:** The `#[cfg(not(target_os = "linux"))]` block only defines `RawEthSocket`, not `StdRawEthLink`, `RawEthTx`, `RawEthRx`, or their trait impls. This means `StdLinkManager::connect("raw-eth/...")` on non-Linux fails at the socket open stage, but the struct and trait definitions still compile.

### Matches zenoh-pico?
zenoh-pico raweth: `Z_FEATURE_RAWETH_TRANSPORT`. zenoh-pico uses Linux AF_PACKET with EtherType 0x6C69. This implementation matches well. Proper MTU of 1500 (standard Ethernet MTU).

---

## USB CDC
**File:** `platforms/zenoh-std/src/usb_cdc.rs`
**Status:** ⚠️ Questionable implementation

### Issues
- ❌ **Uses `serialport` crate:** USB CDC ACM is a serial-over-USB protocol that appears as `/dev/ttyACM0` on Linux. Using serialport is practical on desktop, but on embedded targets USB CDC would use a different API entirely.
- ❌ **`is_streamed()` returns `false` (lines 38, 43, 48):** USB CDC is a stream. Should return `true`.
- ❌ **MTU = `u16::MAX` (line 21):** Same issue as serial.rs — unusably large.
- ❌ **No usb-cdc in listen() path (lib.rs line 245-249):** Actually it IS in listen, but the baud rate (115200) is hardcoded in `new()` with no configurable option.
- ✅ No `expect`/`unwrap`/`panic`.
- ✅ Arc<Mutex<>> split.

### Matches zenoh-pico?
zenoh-pico USB CDC: experimental on RPi Pico. This implementation wraps the desktop serialport crate, which is a reasonable approximation on std platforms but not a true USB CDC implementation.

---

## Cross-cutting issues

### 1. Systematic wrong error variant (TCP, UDP, serial)
Multiple transports return `LinkError::LinkTxFailed` for read errors. This corrupts error reporting at higher layers.

**Affected:** `tcp.rs:56,60,66,70`, `udp.rs:55,65`, and others.

### 2. Panic paths
- `tcp.rs:80-81` — `expect("try_clone failed")`
- `udp.rs:58-59,68-69` — `unimplemented!()` — **will crash process**
- `udp.rs:79-80` — `expect("try_clone failed")`
- `bt.rs:36` — `unimplemented!()` in split

### 3. Missing listen paths in lib.rs
- `"tls"` — no listen arm
- `"ws"` — no listen arm
- `"bt"` — no listen arm

### 4. Dead code
- `mcast.rs` — entire module not included in lib.rs

### 5. Stub implementations
- `bt.rs` — returns errors for everything
- `stm32/serial.rs` — returns errors for everything

### 6. Unsafe without safety comments
All embedded FFI serial implementations use `unsafe` blocks without the required `// SAFETY:` comments mandated by AGENTS.md.

### 7. `is_streamed` confusion
The `std` serial.rs and usb_cdc.rs report `is_streamed = false` while all embedded serial platforms correctly report `is_streamed = true`. WebSocket also reports `false` which may be incorrect.

### 8. No transport tests
`proptests.rs` is a stub with only comments. There are no unit tests, integration tests, or property tests for any transport implementation in `platforms/zenoh-std/`.