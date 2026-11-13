# ESP command applications

```text
esp/
├── network.rs              # Shared Wi-Fi initialization and network tasks
├── runtime.rs              # Panic handler, entropy, bootloader metadata, logger selection
├── usb_serial_jtag.rs       # C6 fixed-function USB receiver
├── esp32s3-wifi/           # One Xtensa CLI application
└── esp32c6-pico-wifi/       # One stable RISC-V CLI application
```

Each MCU project owns its manifest, lockfile, target/toolchain, linker setup,
flash runner, UART pins, and heap placement. The family directory shares source
modules and has no Cargo manifest. The parser, command runtime, and operations
live in [common](../../common/).

Both applications default to a UART text console. `console-usb` selects the
shared Embassy CDC-ACM console on S3 and fixed-function USB Serial/JTAG on C6.
Binary defmt output is suppressed in either USB mode. `alloc` adds broker
support. Shared headless and CDC helpers live directly in [embassy/](../README.md).

```sh
# From the repository root, inside nix develop:
just build-esp32c6
just esp32c6 console-usb
just terminal /dev/cu.usbmodemXXXX
just build-esp32s3 alloc
```

See the [C6 guide](esp32c6-pico-wifi/README.md),
[S3 guide](esp32s3-wifi/README.md), and
[command guide](../../../docs/applications.md#cli).
