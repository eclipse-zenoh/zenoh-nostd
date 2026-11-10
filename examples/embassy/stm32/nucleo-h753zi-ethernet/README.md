# NUCLEO-H753ZI Ethernet

> [!WARNING]
> This firmware has **not been tested on a physical NUCLEO-H753ZI board**.
> Compilation and release linking have been checked; hardware operation remains unverified.

Standalone firmware for the [ST NUCLEO-H753ZI](https://www.st.com/en/evaluation-tools/nucleo-h753zi.html)
(MB1364, STM32H753ZIT6, onboard LAN8742A Ethernet PHY). It uses stable Rust and
`thumbv7em-none-eabihf`, independently of the ESP HAL and toolchain.

Enter `nix develop` from the repository root. The environment supplies the ARM
target and `probe-rs`. Connect the ST-LINK USB port to the host and the RJ45 port
to a LAN with DHCP and a reachable Zenoh router:

```sh
export ENDPOINT=tcp/192.168.1.10:7447
just build-stm32
just stm32
```

The second command flashes via the onboard ST-LINK and displays RTT/defmt logs.
Alternatively, from **this directory**:

```sh
cargo build --locked --release --bin zenoh-app
cargo run --locked --release --bin zenoh-app
cargo run --locked --release --bin zenoh-app --features alloc
```

The default text console uses USART3, TX PD8 and RX PD9, at 115200 baud, 8N1, without
flow control. On this board it is connected to the ST-LINK virtual COM port.
Open that serial port in a text terminal separately from the probe-rs RTT logs.
At the prompt, use `z_open <endpoint>`, then `z_get <key>` or other commands.
Ctrl-C returns to the prompt; `help` lists the available operations.

`just stm32 console-usb` selects the shared `embassy-usb` CDC-ACM console on
user connector CN13 (PA12 D+, PA11 D-). Add `alloc,console-usb` for allocated
storage. ST-LINK CN1 remains the programming/debug connector; power the board
through it or another supported supply before connecting CN13. USB builds use
HSI48 with USB SOF synchronization. See the [Embassy guide](../../README.md).

The application owns the HAL, RMII pin mapping, PHY address, MAC derivation,
DHCP, executor, time driver, hardware RNG backend, panic handler, and linker
configuration. It runs from the internal 64 MHz HSI and uses HSI48 for RNG,
without relying on the ST-LINK clock output or external oscillator jumpers.
The locally administered unicast MAC is derived from the chip's unique ID.

| RMII signal | Pin |
| --- | --- |
| REF_CLK | PA1 |
| MDIO | PA2 |
| MDC | PC1 |
| CRS_DV | PA7 |
| RXD0 / RXD1 | PC4 / PC5 |
| TXD0 / TXD1 | PG13 / PB13 |
| TX_EN | PG11 |

The PHY address is 0. This mapping is for the stated board, not a generic H7
board. `memory.x` places application RAM, including Ethernet buffers, in D2
SRAM (288 KiB at `0x30000000`), accessible by DMA. It deliberately excludes DTCM.
The M7 data cache stays disabled; enabling it requires DMA cache coherency work.

No allocator is installed in the default configuration. `--features alloc`
installs a 64 KiB heap and enables heap-backed Zenoh storage and the broker.
`ENDPOINT` is an optional connection default; `z_open` accepts an address at
runtime. The broker uses that default or its upstream command argument.
See the [command guide](../../../../docs/applications.md#cli),
[shared configuration](../../../README.md) and the
[hardware smoke test](../../../../docs/applications.md#hardware-smoke-test).

HAL API and driver implementation:
[embassy-stm32 0.4.0](https://github.com/embassy-rs/embassy/tree/3e8d8fec15286eb25b8bba7d103c8fc279544551/embassy-stm32).

Board wiring reference:
[ST UM2407, STM32H7 Nucleo-144 boards](https://www.st.com/resource/en/user_manual/um2407-stm32h7-nucleo144-board-stmicroelectronics.pdf).

## Startup scenarios without a CLI

The `cli` feature remains enabled by default. To build without a command console,
run `COUNT=3 just build-stm32-headless pub` from the repository root.
Use `just stm32-headless pub` to build and flash; scenario settings are
compiled into the firmware. Add `alloc` after the scenario to select allocated
storage. See the [headless guide](../../../../docs/applications.md#headless).
