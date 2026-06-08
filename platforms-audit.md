# Platform Crates Audit

**Date:** 2026-06-08
**Scope:** All 10 platform crates in `platforms/`

---

## Summary

| Crate | Compiles | Link Variants | Unsafe (no SAFETY) | unwrap/expect/panic | unimplemented!() | Stubs | Tests |
|-------|----------|---------------|--------------------|-----|------------------|-------|-------|
| zenoh-std | ✅ | 8 (Tcp,Udp,Serial,Ws,Tls,Bt,RawEth,UsbCdc) | ⚠️ (raw_eth) | ⚠️ (3 expect) | ⚠️ (2) | Bt | ⚠️ (proptests empty) |
| zenoh-wasm | ✅ | 1 (Ws) | ⚠️ (all blocks) | ✅ none | ✅ none | none | ❌ none |
| zenoh-stm32 | ✅ (2 warnings) | 3 (Tcp,Udp,Serial) | ⚠️ (all blocks) | ✅ none | ✅ none | Serial | ❌ none |
| zenoh-zephyr | ✅ (2 warnings) | 3 (Tcp,Udp,Serial) | ⚠️ (all blocks) | ✅ none | ✅ none | none | ❌ none |
| zenoh-mbedos | ✅ (2 warnings) | 3 (Tcp,Udp,Serial) | ⚠️ (all blocks) | ✅ none | ✅ none | none | ❌ none |
| zenoh-freertos | ✅ (2 warnings) | 3 (Tcp,Udp,Serial) | ⚠️ (all blocks) | ✅ none | ✅ none | none | ❌ none |
| zenoh-opencr | ✅ (2 warnings) | 3 (Tcp,Udp,Serial) | ⚠️ (all blocks) | ✅ none | ✅ none | none | ❌ none |
| zenoh-arduino | ✅ (2 warnings) | 3 (Tcp,Udp,Serial) | ⚠️ (all blocks) | ✅ none | ✅ none | none | ✅ (sockets tests) |
| zenoh-rp2040 | ✅ (2 warnings) | 3 (Tcp,Udp,Serial) | ⚠️ (all blocks) | ✅ none | ✅ none | none | ✅ (sockets tests) |
| zenoh-flipper | ✅ (2 warnings) | 4 (Ble,Tcp,Udp,Serial) | ⚠️ (all blocks) | ✅ none | ✅ none | BLE | ✅ (sockets tests) |

---

## Per-Crate Findings

### 1. zenoh-std (`platforms/zenoh-std`)

**Status:** Compiles cleanly

**Link Enum:** ✅ Complete — 8 variants (`Bt`, `RawEth`, `Serial`, `Tcp`, `Tls`, `Udp`, `UsbCdc`, `Ws`)

**ZLinkManager::connect():**
- ✅ All 8 protocols dispatched with proper error mapping
- ⚠️ TLS `connect()` uses `unwrap_or` on `rsplitn(2, ':').last()` (benign — fallback to full address string)
- ⚠️ MSS computation uses `socket2::SockRef::from(&socket).tcp_mss().unwrap_or(mtu as u32)` — `unwrap_or` is safe (has default)

**ZLinkManager::listen():**
- ⚠️ TCP listen blocks on `accept()` — expected for std, but no timeout documented
- ✅ UDP listen uses `peek_from` to discover remote before `connect()`
- ✅ Serial/raw-eth/usb-cdc handle listen identically to connect

**PROBLEMS (production paths):**

| File | Line | Issue |
|------|------|-------|
| `tcp.rs:80` | `try_clone().expect("try_clone failed")` | Panic in `split()` |
| `tcp.rs:81` | `try_clone().expect("try_clone failed")` | Panic in `split()` |
| `udp.rs:59` | `unimplemented!()` | `read_exact` on UDP |
| `udp.rs:69` | `unimplemented!()` | `read_exact` on UDP Rx |
| `udp.rs:79` | `try_clone().expect("try_clone failed")` | Panic in `split()` |
| `udp.rs:80` | `try_clone().expect("try_clone failed")` | Panic in `split()` |
| `bt.rs:36` | `unimplemented!()` | `split()` on Bt stream stub |

**Unsafe:**
- `raw_eth.rs` — Linux raw socket FFI has NO `// SAFETY:` comments on any of 11 unsafe blocks (lines 22-32, 35, 37-39, 43-54, 56-58, 66, 75, 86-88)

**Split Types:** ✅ All implemented correctly with Arc<Mutex<>> sharing where applicable

**Discovery file:** `discovery.rs` uses `async_net::UdpSocket` and `zenoh_proto` types — compiled successfully via transitive deps

**Formal verification files:**
- `proptests.rs` — empty (only comments)
- `kani_proofs.rs` — empty (only comments)  
- `verus_proofs.rs` — trivial test only
- `creusot_contracts.rs` — trivial test only

---

### 2. zenoh-wasm (`platforms/zenoh-wasm`)

**Status:** Compiles cleanly

**Link Enum:** ✅ 1 variant (`Ws`) — appropriate for WASM target

**ZLinkManager:**
- `connect()`: Only `ws` protocol supported
- `listen()`: Always returns `CouldNotListen` — correct for WASM

**PROBLEMS:**

| File | Line | Issue |
|------|------|-------|
| `ws.rs:75` | `unsafe { &mut *self.socket }` | No `// SAFETY:` comment |
| `ws.rs:108` | `unsafe { &mut *self.socket }` | No `// SAFETY:` comment |
| `ws.rs:142-155` | Raw pointer split | No `// SAFETY:` comment |

**Unsafe:** ⚠️ All unsafe blocks lack SAFETY comments. The split pattern uses raw pointers with `PhantomData<&'a ()>` for lifetime tracking. This is a correct alias pattern but undocumented.

**Allocation:** `buffer.to_vec()` in `write_all` performs heap allocation — expected for wasm target but worth noting for memory-constrained wasm environments.

---

### 3–9. POSIX-Socket Embedded Platforms

Platforms: **zenoh-stm32**, **zenoh-zephyr**, **zenoh-mbedos**, **zenoh-freertos**, **zenoh-opencr**, **zenoh-arduino**, **zenoh-rp2040**

**Status:** All compile (each with 2 warnings about unused `SO_SNDTIMEO` and `dup`)

**Common Pattern:** Nearly identical code copied across all 7 platforms:

| Component | Lines | Notes |
|-----------|-------|-------|
| `lib.rs` | ~95 | Struct definitions + manager impl — CRITICAL: type name is the only differentiator |
| `sockets.rs` | ~90 | FFI definitions — same structs, same constants |
| `tcp.rs` | ~217 | Nearly byte-for-byte identical across platforms |
| `udp.rs` | ~134 | Nearly byte-for-byte identical across platforms |
| `serial.rs` | ~70–185 | Different per-platform (different HAL FFI) |

**Sockets.rs Issues (all 7 platforms):**

| Issue | Severity |
|-------|----------|
| `to_sockaddr` — IPv4: `v4.to_bits().to_be()` duplicates byte swap | ⚠️ Correct but confusing |
| `to_sockaddr` — returns `unsafe { core::mem::zeroed() }` for unused sockaddr variant | ⚠️ Returns uninitialized memory but unused variant is never read |
| `SO_SNDTIMEO` constant declared but never used | 🟡 Dead code |
| `dup()` FFI declared but never used | 🟡 Dead code |
| No `// SAFETY:` on any unsafe block across ANY of the 7 platforms | 🔴 100% missing |

**Note:** `zenoh-arduino` and `zenoh-rp2040` and `zenoh-flipper` have IPv6 bracket-stripping in `to_sockaddr()` that's missing from stm32, zephyr, mbedos, opencr, freertos:

```rust
// Present in arduino/rp2040/flipper:
let ip_str = ip_str.trim_start_matches('[').trim_end_matches(']');

// MISSING in stm32/zephyr/mbedos/opencr/freertos:
// (no bracket stripping)
```

This means IPv6 addresses in `[::1]:7447` format would fail to parse on 5 of 7 POSIX-embedded platforms.

**TCP.rs Issues (all 7 embedded platforms, nearly identical):**

| Issue | Severity |
|-------|----------|
| No `unimplemented!()` or `panic!()` — proper error propagation | ✅ |
| `expect()` not used | ✅ |
| Listen blocks on `accept()` — no timeout | ⚠️ Embedded concern |
| `Drop` for Tx is empty, Rx does the actual `close(fd)` | 🟡 If Tx dropped after Rx, fd leaks |
| `split()` sets `self.fd = -1` — after split, original link is useless (fd ownership transferred) | ✅ Intentional |

**Serial.rs:**

| Platform | Lines | Implementation Status |
|----------|-------|----------------------|
| stm32 | 70 | 🔴 **Stub** — all ops return `Err` |
| zephyr | 164 | ✅ Full — UART poll in/out via FFI |
| mbedos | 157 | ✅ Full — `mbed_serial_*` FFI |
| freertos | 151 | ✅ Full — `uart_read_byte`/`uart_write_byte` |
| opencr | 148 | ✅ Full — `HAL_UART_Transmit`/`Receive` |
| arduino | 155 | ✅ Full — `arduino_serial_*` FFI |
| rp2040 | 184 | ✅ Full — Pico SDK UART FFI |

**freertos sockets.rs:** Uses `FreeRTOS_`-prefixed function names — correct for FreeRTOS+TCP API

---

### 10. zenoh-flipper (`platforms/zenoh-flipper`)

**Status:** Compiles (2 warnings)

**Link Enum:** ✅ 4 variants (`Ble`, `Tcp`, `Udp`, `Serial`) — includes BLE

**BLE (ble.rs):**
- 🔴 Stub — all operations return `Err(LinkError::LinkTxFailed/LinkRxFailed)`
- Contains TODO comments for `ble_app_init`, `ble_app_send`, `ble_app_get_data`
- `split()` works but creates non-functional Tx/Rx

**Serial:** Uses `furi_hal_uart_*` FFI — full implementation ✅

---

## Cross-Cutting Issues

### 1. Missing SAFETY Comments (🔴 Critical)

Every platform that uses `unsafe` lacks `// SAFETY:` comments on ALL unsafe blocks. This violates the AGENTS.md requirement:

> If `unsafe` is absolutely required for embedded/HAL integration, include `// SAFETY:` comments

**Total unsafe blocks across all platforms (estimated):** 250+

### 2. `unimplemented!()` in Production Paths (🔴 High)

- `zenoh-std/udp.rs:59,69` — `read_exact` on UDP
- `zenoh-std/bt.rs:36` — `split()` on Bt stream

These can be reached through the link manager.

### 3. `expect()` in Production Paths (🔴 High)

- `zenoh-std/tcp.rs:80-81` — `try_clone().expect("try_clone failed")`
- `zenoh-std/udp.rs:79-80` — `try_clone().expect("try_clone failed")`

### 4. Massive Code Duplication (🟡 Medium)

The 7 POSIX-embedded platforms share nearly identical TCP (217 lines each × 7 = 1,519 lines) and UDP (134 lines each × 7 = 938 lines) implementations. Minor drifts exist (e.g., IPv6 bracket stripping). This is a maintenance liability.

**Recommendation:** Factor into a shared `zenoh-posix-sockets` helper crate with platform-specific sockets.rs for FFI declarations.

### 5. IPv6 Bracket Handling Gap (🟡 Medium)

`stm32`, `zephyr`, `mbedos`, `opencr`, `freertos` — all 5 lack `trim_start_matches('[').trim_end_matches(']')` in `to_sockaddr()`.

### 6. Dead Code (🟡 Low)

All 7 POSIX-embedded platforms declare `SO_SNDTIMEO` constant and `dup()` FFI function that are never used.

### 7. Stubs Lacking Clear Documentation (🟡 Medium)

| Stub | Location | Non-functional since... |
|------|----------|------------------------|
| Bt (std) | `std/src/bt.rs` | Creation |
| BLE (flipper) | `flipper/src/ble.rs` | Creation |
| Serial (stm32) | `stm32/src/serial.rs` | Creation |

These are documented but lack `#[deprecated]` or feature-gating.

### 8. No `#![forbid(unsafe_code)]` Anywhere

Only `zenoh-std` and `zenoh-wasm` don't use `#![no_std]` — all embedded platforms do, but none forbid unsafe code.

### 9. Missing Tests

| Platform | Unit Tests |
|----------|-----------|
| zenoh-std | Empty proptests, trivial verus/creusot |
| zenoh-wasm | None |
| zenoh-stm32 | None |
| zenoh-zephyr | None |
| zenoh-mbedos | None |
| zenoh-freertos | None |
| zenoh-opencr | None |
| zenoh-arduino | ✅ `sockets.rs` has 5 unit tests |
| zenoh-rp2040 | ✅ `sockets.rs` has 5 unit tests |
| zenoh-flipper | ✅ `sockets.rs` has 5 unit tests |

### 10. Default `nodelay` Not Set on Embedded TCP Connect

`zenoh-std/tcp.rs` sets `set_nodelay(true)` but none of the 7 embedded TCP implementations call `setsockopt(TCP_NODELAY)`. This is a performance concern, not a correctness issue.

---

## Scorecard

### zenoh-std
```
[ ] malformed input rejected       — N/A (uses std::net for parsing)
[ ] truncated input rejected       — ⚠️ partial (std::io returns errors)
[ ] oversized input rejected       — ✅ (bounded by MTU)
[ ] unknown field handled safely   — ✅ (unknown protocol → CouldNotParseProtocol)
[ ] unsupported feature returns explicit error — ⚠️ Bt should be gated
[✓] no panic path                  — 🔴 FAIL: 3x expect, 2x unimplemented!
[✓] no unchecked arithmetic        — ✅
[✓] no unbounded allocation        — ⚠️ ws.rs uses buffer.to_vec()
[ ] deterministic runtime behavior — ⚠️ listen() blocks on accept()
[ ] compatibility test against upstream — ❌ none
[ ] fuzz target added              — ❌ none
[ ] property test added            — ❌ none (proptests.rs is empty)
[ ] formal check considered        — ⚠️ kani/verus/creusot files exist but empty
```

### Embedded Platforms (all 7)
```
[ ] malformed input rejected       — ⚠️ to_sockaddr returns None
[ ] truncated input rejected       — ⚠️ partial
[ ] oversized input rejected       — ✅
[ ] unknown field handled safely   — ✅
[ ] unsupported feature returns explicit error — ✅
[✓] no panic path                  — ✅ (no unwrap/expect/panic in production)
[✓] no unchecked arithmetic        — ✅
[✓] no unbounded allocation        — ✅ (no_std)
[ ] deterministic runtime behavior — ⚠️ accept() blocks
[ ] compatibility test against upstream — ❌ none
[ ] fuzz target added              — ❌ none
[ ] property test added            — ❌ none
[✓] SAFETY comments on unsafe blocks — 🔴 FAIL: 0% compliance
```

---

## Recommendations (Priority Order)

1. **Add `// SAFETY:` comments** to all unsafe blocks across all platforms (250+ locations)
2. **Fix `expect()` and `unimplemented!()`** in `zenoh-std/tcp.rs`, `zenoh-std/udp.rs`, `zenoh-std/bt.rs`
3. **Add IPv6 bracket stripping** to `to_sockaddr()` in stm32, zephyr, mbedos, opencr, freertos
4. **Implement STM32 serial** or explicitly feature-gate it
5. **Implement Bt/WS/usb-cdc for std** or feature-gate the variants
6. **Implement BLE for flipper** or feature-gate it
7. **Factor shared TCP/UDP code** into a `zenoh-posix-sockets` crate
8. **Add unit tests** to remaining 7 platforms (following arduino/rp2040/flipper pattern)
9. **Add `#![forbid(unsafe_code)]`** to all `no_std` platforms (or justify each unsafe block)
10. **Set TCP_NODELAY** on embedded platform TCP connections