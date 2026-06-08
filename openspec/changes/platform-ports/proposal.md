# Change: Platform Ports for Zephyr, Arduino, MbedOS, FreeRTOS, RPi Pico, OpenCR, Flipper Zero

## Why

zenoh-pico supports all of these platforms natively via C HALs (`Z_FEATURE_LINK_TCP`, `Z_FEATURE_LINK_UDP`, `Z_FEATURE_LINK_SERIAL`, and platform-specific WiFi/BLE/LoRa link implementations). zenoh-nostd currently supports only four platforms: `std` (Linux/Mac/Windows via blocking TCP/UDP), `esp-idf` (ESP32 via FreeRTOS+LWIP event loop), `stm32` (Cortex-M via HAL), and `wasm` (WebAssembly via JS bindings).

The gap means nostd is unavailable on:

- **Zephyr RTOS**: Dominant in industrial IoT, running on nRF52, STM32, NXP, and Infineon MCUs. Zephyr provides POSIX-like sockets, making TCP/UDP transport straightforward.
- **Arduino**: Largest embedded developer ecosystem. Arduino Uno R4, Nano 33 BLE, GIGA R1 WiFi, Portenta H7 — all with WiFi/Ethernet shields or native connectivity.
- **MbedOS**: ARM's IoT OS, widely deployed on NXP, STM32, and Renesas MCUs. Deprecated but still in field use across existing IoT fleets.
- **FreeRTOS**: De facto standard RTOS for ESP32 (via ESP-IDF, already supported), STM32, NXP, Microchip. The gap is FreeRTOS without ESP-IDF — e.g., STM32 + FreeRTOS + LWIP.
- **Raspberry Pi Pico (RP2040/RP2350)**: Extremely popular low-cost MCU. Pico W has WiFi (CYW43439). RP2350 is the new dual-core RISC-V/Cortex-M33 successor. No nostd support exists.
- **OpenCR**: ROS ecosystem board used with TurtleBot3 and ROS 2. Built on STM32F7, has WiFi, Ethernet, and UART. Critical for robotics deployments.
- **Flipper Zero**: Popular multi-tool device with STM32WB55, WiFi (ESP32), sub-1GHz radio, infrared, and BLE. Nostalgic/hobbyist but also deployed in security research and pen-testing.

Each platform follows the same pattern: implement `Transport` and `Timer` traits from `zenoh-sansio`, provide platform-specific IO via FFI or HAL bindings, configure the build target for cross-compilation.

## What Changes

### 1. Create platform crate structure

For each platform, create a crate under `platforms/`:

```
platforms/zenoh-zephyr/
platforms/zenoh-arduino/
platforms/zenoh-mbedos/
platforms/zenoh-freertos/
platforms/zenoh-pico-sdk/     # RP2040/RP2350 via Pico SDK
platforms/zenoh-opencr/
platforms/zenoh-flipper/
```

Each crate follows the same structure:

```
Cargo.toml
src/
  lib.rs           # Re-export link types, feature-gated
  link/
    mod.rs         # Link type definitions
    tcp.rs         # TCP link via platform sockets
    udp.rs         # UDP link via platform sockets
    serial.rs      # Serial link via platform UART
  timer.rs         # Timer trait impl (platform time)
  hal.rs           # Platform HAL bindings for FFI/SDK
```

Each crate depends on:

```toml
[dependencies]
zenoh-sansio = { path = "../../crates/zenoh-sansio" }
zenoh-proto = { path = "../../crates/zenoh-proto" }
```

Plus platform-specific dependencies (see per-platform breakdown below).

### 2. Feature flags

Add to root `Cargo.toml` workspace and `crates/zenoh-nostd/Cargo.toml`:

```toml
[features]
platform-zephyr = ["dep:zenoh-zephyr"]
platform-arduino = ["dep:zenoh-arduino"]
platform-mbedos = ["dep:zenoh-mbedos"]
platform-freertos = ["dep:zenoh-freertos"]
platform-rp2040 = ["dep:zenoh-pico-sdk"]
platform-opencr = ["dep:zenoh-opencr"]
platform-flipper = ["dep:zenoh-flipper"]
```

Each platform flag enables the corresponding crate and activates platform-specific `ZLinkManager` and `Timer` implementations. Multiple platform flags can be active simultaneously for cross-platform builds (e.g., `platform-freertos` + `platform-zephyr` for dual-OS firmware), but only one `Timer` implementation will be selected at runtime based on the active platform.

### 3. Zephyr port (`platforms/zenoh-zephyr`)

**SDK**: Zephyr RTOS with `CONFIG_NETWORKING`, `CONFIG_NET_TCP`, `CONFIG_NET_UDP`.

**Build**: Use `CONFIG_ARM`/`CONFIG_RISCV`/`CONFIG_X86` with Zephyr's CMake-based build. Cross-compile via `west build -b <board>`.

```toml
[dependencies]
zephyr-sys = { version = "0.3", features = ["networking", "kernel"] }
```

**Link implementation**:

- TCP/UDP via Zephyr's BSD sockets API (`zephyr_sys::net::socket`, `connect`, `send`, `recv`)
- Zephyr sockets are POSIX-like and support `select()` for timeout-controlled receive
- Serial via Zephyr UART API (`device_get_binding`, `uart_irq_rx_enable`, `uart_fifo_read`)
- Timer via Zephyr kernel timing (`k_uptime_get`, `k_sleep`, `K_USEC`, `K_MSEC`)

**Targets**:

| Board | SoC | Connectivity |
|---|---|---|
| nRF52840DK / nRF52840 Dongle | nRF52840 | BLE, USB, UART |
| nRF5340DK | nRF5340 | BLE, Ethernet, USB |
| STM32F746G-Discovery | STM32F7 | Ethernet |
| STM32H743I-EVAL | STM32H7 | Ethernet, UART |
| NXP i.MX RT1050 EVKB | i.MX RT1050 | Ethernet |
| ESP32-DevKitC (Zephyr) | ESP32 | WiFi, UART |

### 4. Arduino port (`platforms/zenoh-arduino`)

**SDK**: Arduino cores (AVR, SAMD, SAM, RP2040, ESP32, nRF52).

**Build**: Use `arduino-cli` or platformio for compilation. Rust cross-compilation via `cargo build --target <triple>` with Arduino SDK as C dependency.

```toml
[dependencies]
arduino-hal = { version = "0.1" }  # Rust Arduino HAL if available, else FFI to Arduino SDK
```

**Link implementation**:

- **WiFi**: Arduino WiFi library (`WiFi.begin`, `WiFiClient`) via `esp32-hal`, `wifi101`, `mkr1000`, `nina-w102` modules
- **Ethernet**: Arduino Ethernet library (`Ethernet.begin`, `EthernetClient`) via W5100/W5500
- **Serial**: Arduino `Serial` class via `embedded-hal` UART traits or direct register access
- **TLS**: ArduinoBearSSL or ArduinoMbedTLS for secure connections (future phase)

**Targets**:

| Board | SoC / Module | Connectivity |
|---|---|---|
| Arduino Uno R4 WiFi | RA4M1 + ESP32-S3 | WiFi, UART |
| Arduino Nano 33 BLE | nRF52840 | BLE, UART |
| Arduino GIGA R1 WiFi | STM32H7 + WiFi | WiFi, Ethernet, UART |
| Arduino Portenta H7 | STM32H7 | WiFi, Ethernet, UART |
| Arduino MKR WiFi 1010 | SAMD21 + WiFi | WiFi, UART |
| Arduino RP2040 Connect | RP2040 + WiFi | WiFi, UART |

### 5. MbedOS port (`platforms/zenoh-mbedos`)

**SDK**: MbedOS 6.x (legacy, but still in field deployment).

**Build**: Use `mbed compile` for Mbed projects; Rust cross-compilation with MbedOS as C dependency.

```toml
[dependencies]
mbedos-sys = { version = "0.2" }  # FFI bindings to MbedOS C API
```

**Link implementation**:

- TCP/UDP via MbedOS `TCPSocket` / `UDPSocket` (wraps LWIP)
- Serial via MbedOS `Serial` / `UnbufferedSerial`
- Timer via MbedOS `Timer` / `Ticker` classes or `us_ticker_read()`

**Targets**:

| Board | SoC | Connectivity |
|---|---|---|
| NXP FRDM-K64F | K64F (Cortex-M4) | Ethernet |
| NXP FRDM-K66F | K66F (Cortex-M4) | Ethernet |
| STM32F429I-Discovery | STM32F4 | Ethernet |
| STM32 NUCLEO-F767ZI | STM32F7 | Ethernet, UART |
| MAX32625PICO | MAX32625 | UART (USB) |

### 6. FreeRTOS port (`platforms/zenoh-freertos`)

**SDK**: FreeRTOS kernel + LWIP (or other TCP/IP stack).

**Build**: Standard `cargo build` with FreeRTOS header/source paths set via build script.

```toml
[dependencies]
freertos-rust = { version = "0.2", features = ["lwip"] }  # Rust FreeRTOS bindings
```

**Link implementation**:

- TCP/UDP via LWIP BSD socket API (`lwip_socket`, `lwip_connect`, etc.) or direct `tcp_new`/`tcp_connect` callback-based API
- Serial via FreeRTOS+UART or platform UART driver
- Timer via FreeRTOS `vTaskDelay`, `xTaskGetTickCount`, `vTaskSetTimeOutState`
- Task management via `xTaskCreate` for background IO handling

**Targets**: Any MCU/board running FreeRTOS + LWIP:

| Board | SoC | Connectivity |
|---|---|---|
| STM32F4-Discovery (FreeRTOS) | STM32F4 | Ethernet (LWIP) |
| STM32 NUCLEO-H743ZI | STM32H7 | Ethernet (LWIP) |
| NXP LPC54628 | LPC54628 | Ethernet |
| Microchip SAMA5D2 | ATSAMA5D2 | Ethernet |
| TI Tiva C Series TM4C129 | TM4C129 | Ethernet |

### 7. RPi Pico port (`platforms/zenoh-pico-sdk`)

**SDK**: Raspberry Pi Pico SDK (C SDK for RP2040/RP2350).

**Build**: Use `PICO_SDK_PATH` environment variable. Build script (`build.rs`) calls CMake to compile Pico SDK C files and links them.

```toml
[dependencies]
pico-sys = { version = "0.1" }  # FFI bindings to Pico SDK
```

**Link implementation**:

- **Pico W (RP2040 + CYW43439)**: WiFi via `cyw43_arch` — `cyw43_arch_wifi_connect`, `cyw43_tcpip_link_status`, LWIP sockets for TCP/UDP
- **Pico (RP2040) / Pico 2 (RP2350)**: No native network. Serial via Pico SDK UART (`uart_init`, `uart_putc_raw`, `uart_is_readable`). USB CDC via `tusb` for virtual COM port.
- **Ethernet (W5500 via Pico)**: External W5500 SPI Ethernet module, TCP via `pico-sdk` LWIP integration with `cyw43` or `wiznet` driver
- **Timer**: Pico SDK `time_us_64()`, `sleep_us`, `alarm_pool` for timer callbacks
- **USB CDC**: TinyUSB (`tusb`) for CDC ACM virtual serial — enables plug-and-play USB connectivity

**Targets**:

| Board | SoC | Connectivity |
|---|---|---|
| Raspberry Pi Pico | RP2040 | Serial (UART/USB CDC) |
| Raspberry Pi Pico W | RP2040 + CYW43439 | WiFi (2.4GHz 802.11n), Serial |
| Raspberry Pi Pico 2 | RP2350 (RISC-V/Cortex-M33) | Serial (UART/USB CDC) |
| Raspberry Pi Pico 2 W | RP2350 + CYW43439 | WiFi, Serial |
| Pico + W5500 module | RP2040/RP2350 | Ethernet (SPI) |

### 8. OpenCR port (`platforms/zenoh-opencr`)

**SDK**: OpenCR board support (STM32F7 + Ethernet + WiFi).

OpenCR is built around STM32F746IGH6 (Cortex-M7, 216MHz) with:
- Ethernet controller (STM32 internal MAC + external PHY)
- WiFi via ESP8266 (serial AT command interface)
- UART via STM32 USART for serial transport
- USB (device and host) via STM32 USB OTG

**Build**: Use `embassy-stm32` HAL (as existing `platform-stm32`) or direct STM32 HAL via FFI.

```toml
[dependencies]
opencr-hal = { version = "0.1", features = ["stm32f7", "eth", "wifi"] }
```

**Link implementation**:

- **Ethernet**: STM32 FMC + ETH MAC + LAN8742A PHY (or similar). Use `embassy-stm32` Ethernet peripheral or raw register access. TCP/UDP via LWIP or lightweight TCP stack.
- **WiFi (ESP8266)**: Serial AT command interface (`AT+CIPSTART`, `AT+CIPSEND`, `AT+CIPRECV`). TCP/UDP tunnel via UART. This is a serial-to-network bridge — reuse `transport-serial` for the UART link, then forward TCP/UDP data.
- **Serial**: STM32 UART (`USART1`, `USART2`, `USART3` exposed on OpenCR headers) — reuse `transport-serial`
- **Timer**: STM32 timers via HAL or bare-metal register access

**Targets**: OpenCR board only (no other boards use this port).

| Board | SoC | Connectivity |
|---|---|---|
| ROBOTIS OpenCR | STM32F746IGH6 | Ethernet, WiFi (ESP8266), UART, USB, BLE (optional module) |

### 9. Flipper Zero port (`platforms/zenoh-flipper`)

**SDK**: Flipper Zero firmware SDK (STM32WB55 + ESP32).

Flipper Zero hardware:
- **MCU**: STM32WB55RGV6 (Cortex-M4 64MHz + Cortex-M0+ BLE coprocessor)
- **WiFi**: ESP32-S2 (via external module, connected over UART+SPI)
- **Storage**: microSD, internal flash, EEPROM
- **IO**: UART (3.3V TX/RX on GPIO), USB (device mode), IR, sub-1GHz, NFC, iButton, BLE

**Build**: Use Flipper Zero build system (`fbt`) with Flipper Application Package (FAP) as the target. Rust compiled to staticlib, linked into FAP.

```toml
[dependencies]
flipper-sys = { version = "0.1" }  # FFI bindings to Flipper SDK
```

**Link implementation**:

- **WiFi (ESP32-S2)**: Flipper talks to ESP32-S2 over UART+SPI at 8+ Mbps. The ESP32 runs a TCP/IP stack and provides a serial tunnel. TCP/UDP transport via the ESP32 WiFi -> UART bridge. The link sends raw IP packets over the ESP32 UART bridge.
- **Serial**: STM32WB55 LPUART1 on GPIO pins (TX=PA14, RX=PA13). Reuse `transport-serial`.
- **USB**: STM32 USB device mode — CDC ACM virtual COM port. Reuse `transport-serial` with USB CDC.
- **BLE**: STM32WB55 built-in BLE (Cortex-M0+ coprocessor). BLE transport via Flipper SDK BLE API readiness. Note: BLE transport is a separate concern (`transport-ble` change).
- **Timer**: STM32 timer via Flipper SDK `furi_time` or STM32 HAL

**Targets**: Flipper Zero only.

| Board | SoC | Connectivity |
|---|---|---|
| Flipper Zero | STM32WB55 + ESP32-S2 | WiFi, BLE, UART, USB, sub-1GHz |

### 10. CI cross-compilation matrix

Add to CI for each platform where toolchain is available:

| Platform | Target Triple | CI Status |
|---|---|---|
| Zephyr (nRF52) | `thumbv7em-none-eabihf` (with Zephyr SDK) | Manual (needs Zephyr SDK + board config) |
| Arduino (SAMD21) | `thumbv6m-none-eabi` | Manual |
| Arduino (SAMD51) | `thumbv7em-none-eabihf` | Manual |
| Arduino (RP2040) | `thumbv6m-none-eabi` | Manual |
| MbedOS (K64F) | `thumbv7em-none-eabihf` | Manual (needs Mbed CLI) |
| FreeRTOS (STM32F4) | `thumbv7em-none-eabihf` | Manual |
| RPi Pico (RP2040) | `thumbv6m-none-eabi` | Build via Pico SDK + `cargo` |
| RPi Pico 2 (RP2350) | `riscv32imc-unknown-none-elf` or `thumbv8m.main-none-eabihf` | Build via Pico 2 SDK + `cargo` |
| OpenCR (STM32F7) | `thumbv7em-none-eabihf` | Manual |
| Flipper Zero (STM32WB) | `thumbv7em-none-eabihf` | Manual (needs Flipper SDK) |

CI for each platform should:

1. `cargo check --target <triple> --features platform-<name>`
2. `cargo build --target <triple> --features platform-<name>` (verify compilation, no linking)
3. Cross-compilation via `cross` or platform SDK Docker image where available

### 11. Documentation

Create platform-specific documentation:

```
docs/platforms/zephyr.md          # Setup guide, board support, example
docs/platforms/arduino.md         # Setup guide, board support, library installation
docs/platforms/mbedos.md          # Setup guide, board support
docs/platforms/freertos.md        # Setup guide, integration with FreeRTOS projects
docs/platforms/rpi-pico.md        # Setup guide, Pico W WiFi, USB CDC
docs/platforms/opencr.md          # Setup guide, ROS/TurtleBot integration
docs/platforms/flipper-zero.md    # Setup guide, FAP building, limitations
```

Each document covers:

- Toolchain installation and SDK setup
- Build commands for each supported board
- Feature flags required
- Memory and performance characteristics
- Known limitations on that platform

Update `docs/feature_support_matrix.md` with the new platform rows.

### 12. Example implementations

Create minimal examples for each platform under `examples/platforms/`:

```
examples/platforms/
  zephyr/
    src/main.rs           # Zenoh pub on nRF52840
    prj.conf              # Zephyr config
    CMakeLists.txt        # Zephyr build integration
  arduino/
    src/main.rs           # Zenoh pub on MKR WiFi 1010
    sketch.json           # arduino-cli config
  rpi-pico/
    src/main.rs           # Zenoh pub over USB CDC on Pico
    CMakeLists.txt        # Pico SDK build integration
  opencr/
    src/main.rs           # Zenoh pub over Ethernet
  flipper/
    application.fam       # Flipper Application Manifest
    src/main.rs           # Zenoh pub over WiFi (ESP32)
```

## Capabilities

### New

- `platform-zephyr` — Zephyr RTOS support with TCP/UDP via Zephyr sockets, serial via Zephyr UART, timer via Zephyr kernel timing
- `platform-arduino` — Arduino platform support with WiFi (NINA, u-blox W101, ESP32), Ethernet (W5100/W5500), serial via Arduino `Serial` class
- `platform-mbedos` — MbedOS support with TCP/UDP via MbedOS sockets, serial, timer via `us_ticker`
- `platform-freertos` — FreeRTOS (non-ESP-IDF) support with TCP/UDP via LWIP, serial via platform UART, timer via FreeRTOS tick
- `platform-rp2040` / `platform-rp2350` — Raspberry Pi Pico support via Pico SDK, with WiFi on Pico W (CYW43439), serial (UART), USB CDC (TinyUSB)
- `platform-opencr` — ROBOTIS OpenCR support with Ethernet, WiFi (ESP8266 serial bridge), serial (STM32 USART)
- `platform-flipper` — Flipper Zero support with WiFi (ESP32-S2 serial bridge), serial (LPUART), USB CDC, BLE (STM32WB coprocessor)

### Modified

- `features` section in root `Cargo.toml`: 7 new feature aliases added
- `crates/zenoh-nostd/Cargo.toml`: 7 new optional dependencies for platform crates
- `docs/feature_support_matrix.md`: 7 new rows in platform support table

### Unchanged

- All existing platform support (`platform-std`, `platform-esp-idf`, `platform-stm32`, `platform-wasm`)
- All transport features (`transport-tcp`, `transport-udp`, `transport-serial`, `transport-ble`, `transport-tls`, `transport-raw-ethernet`)
- All protocol features (pub/sub, query/reply, liveliness)
- Session state machine, codec, transport layer (`zenoh-sansio`)
- Protocol types (`zenoh-proto`)
- Sansio core — all new platforms implement existing `Transport` and `Timer` traits

## Impact

| Crate | Change | Risk |
|---|---|---|
| Root `Cargo.toml` | 7 new workspace dep entries, 7 new feature aliases | Low — additive only |
| `crates/zenoh-nostd/Cargo.toml` | 7 new optional platform crate deps | Low — additive only |
| `platforms/zenoh-zephyr/` | New crate | Medium — Zephyr socket API differs from POSIX; build integration requires CMake bridging |
| `platforms/zenoh-arduino/` | New crate | Medium — Arduino variant fragmentation (AVR vs ARM); WiFi libraries diverge |
| `platforms/zenoh-mbedos/` | New crate | Medium — MbedOS deprecated; C FFI bindings may bitrot |
| `platforms/zenoh-freertos/` | New crate | Low-medium — LWIP socket API is well-understood; FreeRTOS Rust bindings exist |
| `platforms/zenoh-pico-sdk/` | New crate | Medium — Pico SDK CMake integration; CYW43439 WiFi driver complexity |
| `platforms/zenoh-opencr/` | New crate | Low — reuses STM32 HAL extensively; ESP8266 bridge is simple serial forwarding |
| `platforms/zenoh-flipper/` | New crate | Medium — Flipper build system (FAP); ESP32-S2 bridge protocol; proprietary BLE stack |
| `CI/` | New cross-compilation jobs for each platform | Medium — hardware-specific build environments; Docker images needed |
| `docs/` | New platform docs, updated feature matrix | None |

## Phasing

| Phase | Scope | Deliverable |
|---|---|---|
| 1 | RPi Pico port | `platforms/zenoh-pico-sdk/` with serial (UART + USB CDC) transport and WiFi (Pico W LWIP). Most approachable — Pico SDK has CMake build, well-documented APIs, active community. |
| 2 | FreeRTOS port | `platforms/zenoh-freertos/` with LWIP TCP/UDP sockets. Second priority — FreeRTOS + LWIP is the most common embedded networking stack. |
| 3 | Zephyr port | `platforms/zenoh-zephyr/` with Zephyr socket API and serial. Third priority — Zephyr is dominant in industrial IoT but the build system (west/CMake) adds complexity. |
| 4 | Arduino port | `platforms/zenoh-arduino/` with WiFi and serial. Largest user base, but also the most fragmented platform (AVR vs ARM, multiple WiFi libraries). Start with MKR WiFi 1010 (SAMD21 + NINA-W102) and Nano 33 BLE. |
| 5 | OpenCR port | `platforms/zenoh-opencr/` with Ethernet and WiFi. Lower priority but important for ROS ecosystem. Reuses STM32 HAL and serial-to-WiFi bridge pattern. |
| 6 | Flipper Zero port | `platforms/zenoh-flipper/` with WiFi (ESP32 bridge) and serial. Hobbyist/security niche. Requires understanding Flipper FAP build system and ESP32-UART bridge protocol. |
| 7 | MbedOS port | `platforms/zenoh-mbedos/`. Lowest priority — MbedOS is deprecated, but existing fleets warrant support. C FFI bindings to legacy SDK. |
| 8 | CI + Documentation | Cross-compilation CI jobs for all 7 platforms; platform-specific setup guides; feature matrix update. |

## Known Gaps

- **Hardware testing**: Most of these platforms require physical hardware for integration tests. CI can cross-compile but cannot run end-to-end tests without hardware-in-the-loop. Emulators (QEMU for Zephyr/FreeRTOS, Wokwi for ESP32/Pico) can provide partial coverage.
- **WiFi fragmentation**: Arduino WiFi libraries vary significantly: NINA-W102 (MKR), u-blox W101 (Nano), ESP32 WiFi (Portenta H7), built-in WiFi (Uno R4). Each requires different initialization and connection management. The link abstraction unifies at the socket level but platform-specific init is unavoidable.
- **BLE on Flipper Zero**: STM32WB55 has a dual-core architecture — Cortex-M4 runs application code, Cortex-M0+ runs the BLE stack. BLE transport requires inter-core communication (IPC) between the two cores. This is a separate concern from `transport-ble` but the Flipper port must handle it.
- **TLS support**: None of these platforms have built-in TLS transport in scope. TLS requires platform-specific certificate storage, entropy sources, and crypto acceleration. Defer to a future `transport-tls` platform integration phase.
- **OpenCR ESP8266 bridge**: The ESP8266 AT command set is slow (max ~460kbps UART) and unreliable under high throughput. The OpenCR port should document throughput limitations and recommend Ethernet for production use.
- **Flipper Zero ESP32 bridge**: Similar to OpenCR — the UART+SPI bridge to ESP32-S2 has limited throughput. Flipper's FAP environment is sandboxed (no direct hardware access for some peripherals). The port may require patching the Flipper firmware to expose ESP32 bridge APIs.
- **MbedOS deprecation**: ARM announced end of MbedOS development in 2022. No new features, security patches only. The port is for existing fleet compatibility only. No MbedOS 6.x features beyond basic TCP/UDP/serial will be supported.
- **Zephyr version drift**: Zephyr API changes rapidly (e.g., socket API changes between 3.x and 4.x). The port targets Zephyr 3.7 LTS initially, with documented compatibility notes for newer versions.
- **Arduino AVR limitations**: AVR-based Arduino boards (Uno, Mega) have very limited RAM (2-8KB) and flash (32-256KB). Zenoh's minimum memory footprint may exceed AVR capacity. The port should document which boards are capable of running nostd and which are not. Uno R4 (RA4M1, 256KB SRAM, 2MB flash) is the baseline viable target.