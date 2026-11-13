# ESP32-S3 command console

One Xtensa firmware application uses ESP HAL/RTOS, Wi-Fi DHCP, and the common
runtime command interface. Shared ESP support lives in `examples/embassy/esp/`; this
project owns its chip features, heap placement, toolchain, linker, and runner.

```sh
# From the repository root:
nix develop
export WIFI_SSID=your-access-point
export WIFI_PASSWORD=your-password
export ENDPOINT=tcp/192.168.1.10:7447  # Optional connection default
just build-esp32s3
just esp32s3
```

UART1 is the default console: TX GPIO17, RX GPIO18, 115200 baud, 8N1, no flow
control. Verify these pins are exposed on your S3 board, then use a 3.3 V adapter
with crossed TX/RX and shared GND. `just esp32s3 console-usb` selects USB
OTG CDC-ACM through the shared `embassy-usb` adapter (GPIO20 D+, GPIO19 D-).
It suppresses defmt records on that connection. Add
`alloc,console-usb` for allocated storage and broker capability. Select the flash
port with `ESPFLASH_PORT`; use `just terminal <port>` to open a text monitor.

USB OTG takes over the internal PHY from USB Serial/JTAG. To reflash a running
CDC application, use BOOT/RESET to enter ROM download mode, then select the flash
port. See the [Embassy guide](../../README.md) for USB configuration and limitations.

From this directory, use
`cargo +esp build --locked --release --bin zenoh-app` or
`cargo +esp run --locked --release --bin zenoh-app --features console-usb`.
Nix selects stable Rust for general work, so the S3 commands explicitly use
`+esp`. The flake does not provide this compiler on Intel macOS.

Use release builds for linked firmware: the pinned compiler's debug build has
an existing `xtensa-lx-rt` assembly issue involving `LBEG`. Release linking can
emit the existing RWX segment warning. Default and allocated UART/USB builds
are checked separately.

The radio uses a 73,744-byte heap in reclaimed internal RAM. Zenoh allocation
is disabled by default; `alloc` adds allocated storage and the broker inside
the same application. Use `help` at the prompt for operations and Ctrl-C to stop
one. See the [command guide](../../../../docs/applications.md#cli) and
[hardware procedure](../../../../docs/applications.md#hardware-smoke-test).

## Startup scenarios without a CLI

The `cli` feature remains enabled by default. To build without a command console,
run `COUNT=3 just build-esp32s3-headless pub` from the repository root.
Use `just esp32s3-headless pub` to build and flash; scenario settings are
compiled into the firmware. Add `alloc` after the scenario to select allocated
storage. See the [headless guide](../../../../docs/applications.md#headless).
