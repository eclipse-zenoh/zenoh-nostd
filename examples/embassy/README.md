# Embassy MCU applications

The files directly in this directory form the `zenoh-examples-embassy` support
library. Board applications remain independent Cargo projects, each with its
own manifest, lockfile, target, linker setup and flash runner.

```text
embassy/
├── Cargo.toml
├── Cargo.lock
├── lib.rs
├── headless.rs                     # Shared startup scenario runner
├── usb_console.rs                  # Shared embassy-usb CDC-ACM console
├── esp/
│   ├── network.rs                  # ESP Wi-Fi setup and network tasks
│   ├── runtime.rs                  # ESP startup, entropy and logger policy
│   ├── usb_serial_jtag.rs          # C6 fixed-function USB adapter
│   ├── esp32s3-wifi/
│   └── esp32c6-pico-wifi/
└── stm32/
    └── nucleo-h753zi-ethernet/
```

`headless.rs` handles startup settings, bounded callback context and scenario
execution. Each board supplies its initialized configuration, endpoint and
reporter. Board entry points retain their executor attribute and HAL setup.
The generic scenario engine and optional `embedded-cli` frontend remain in
[common](../common/).

`usb_console.rs` owns CDC descriptors and stream adapters, parameterized by an
`embassy-usb` driver. It preserves partial reads of full USB packets, splits
output into 64-byte packets, sends terminating zero-length packets when needed,
and waits for endpoint re-enablement after disconnects. Boards run the USB
device future alongside the CLI, so enumeration proceeds while DHCP is pending.
These are console capabilities, independent of the network transport adapter
in [platforms/zenoh-embassy](../../platforms/zenoh-embassy/).

## Console selection

UART remains the default; `console-usb` enables the CLI and selects the board's
USB backend. `alloc` selects allocated Zenoh storage independently.

| Board | Default console | `console-usb` |
| --- | --- | --- |
| ESP32-S3 | UART1 GPIO17 TX / GPIO18 RX | `embassy-usb` CDC-ACM, USB OTG GPIO20 D+ / GPIO19 D- |
| ESP32-C6-Pico | UART1 GPIO4 TX / GPIO5 RX | Fixed-function USB Serial/JTAG, GPIO13 D+ / GPIO12 D- |
| NUCLEO-H753ZI | USART3 PD8 TX / PD9 RX through ST-LINK | `embassy-usb` CDC-ACM on user connector CN13, PA12 D+ / PA11 D- |

C6's USB Serial/JTAG controller is implemented in hardware and cannot run a
software USB device stack. Its existing adapter stays in `esp/`; S3 and STM32
share the Embassy CDC implementation. See
[Espressif's controller description](https://docs.espressif.com/projects/esp-idf/en/v5.1/esp32c6/api-guides/usb-serial-jtag-console.html).

S3 USB OTG uses the internal PHY shared with USB Serial/JTAG. Once the firmware
starts, its CDC port replaces the ROM/JTAG interface. To flash again, enter ROM
download mode using BOOT/RESET and select the resulting flash port with
`ESPFLASH_PORT`. ESP binary logging is suppressed in USB-console builds.

On Nucleo, ST-LINK CN1 remains the debug/programming connection and default UART
bridge. The Embassy CDC console appears on the separate user USB connector
CN13. Power the board through ST-LINK or another supported supply before
connecting CN13, following [ST UM2407](https://www.st.com/resource/en/user_manual/um2407-stm32h7-nucleo144-boards-mb1364-stmicroelectronics.pdf).
The USB build enables the HSI48 USB clock with USB SOF synchronization. RTT
logging remains separate. The example device descriptors use VID/PID
`0xc0de:0xcafe`; identity and power configuration live in each board entry point.

```sh
# From the repository root, inside nix develop:
just build-esp32s3 console-usb
just build-stm32 alloc,console-usb
just build-esp32c6 console-usb
just terminal /dev/cu.usbmodemXXXX
COUNT=3 just build-esp32c6-headless pub
```

Root command names are unchanged after the directory move. Headless and UART
builds exclude the `embassy-usb` device stack. USB support is optional in this
library and enabled by S3/STM32's `console-usb` feature.

## Validation

`just check` includes shared-library tests and all board CLI/headless checks.
CDC tests use a mock USB driver to cover packet boundaries, short CLI reads,
zero-length packets and endpoint disconnect recovery. Firmware CI links fixed
and allocated UART/USB builds plus fixed and allocated headless builds for all
three boards (18 combinations). These checks do not establish physical USB
enumeration or cable-reconnect behavior; those require the boards and their
console connectors. Follow the
[hardware smoke test](../../docs/applications.md#hardware-smoke-test) for physical validation.
