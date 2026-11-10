# Example projects

Each target is an independent Cargo project building one `zenoh-app` binary,
with CLI enabled by default and optional headless startup. Start with the
[application guide](../docs/applications.md) for commands, features, settings,
and hardware testing.

| Project | Network | Console |
| --- | --- | --- |
| [Native](native/README.md) | TCP/UDP/WebSocket | stdin or process arguments |
| [Browser](wasm/README.md) | WebSocket | Browser text console |
| [ESP32-S3](embassy/esp/esp32s3-wifi/README.md) | Wi-Fi | UART; optional Embassy USB CDC |
| [ESP32-C6-Pico](embassy/esp/esp32c6-pico-wifi/README.md) | Wi-Fi | UART; optional USB Serial/JTAG |
| [NUCLEO-H753ZI](embassy/stm32/nucleo-h753zi-ethernet/README.md) | Ethernet | ST-LINK UART; optional Embassy USB CDC |

[common/](common/) contains configuration, typed scenarios, and the optional
`embedded-cli` frontend. [embassy/](embassy/README.md) provides shared MCU
headless and USB helpers; board projects own their HAL and target configuration.
See [architecture and porting](../docs/architecture.md) to add a target.

Recipes live in [examples/justfile](justfile), imported by the root justfile.
All example commands work from the repository root or `examples/`. In
`examples/`, `just --list` lists only example tasks. Cargo runs from each target
project, with build artifacts shared under the root `target/` directory.

Inside `nix develop`, use `just check-examples` for checks and
`just fmt-examples` for formatting. `just test-cli` and `just test-headless`
run native integration suites against isolated routers. Build-only verification
and physical testing are described in the [contributor guide](../docs/README.md).
