# Running zenoh-app

Every target builds one `zenoh-app` binary. The default `cli` feature lets you
choose operations at runtime. Disable it to run a configured scenario at startup.
`alloc` independently enables allocated Zenoh storage and broker support.

Run the commands below from the repository root inside `nix develop`.

## Native

Start a local router in one terminal:

```sh
zenohd --listen tcp/127.0.0.1:7447
```

In a second terminal, subscribe; in a third, publish:

```sh
just std z_sub 'demo/**'
```

```sh
just std z_put demo/example "hello world"
```

`just std` opens an interactive console. `ENDPOINT` is read at process startup
and defaults to `tcp/127.0.0.1:7447`. For a query, first start
`just std z_queryable 'demo/**' 'reply'`, then use
`just std z_get demo/example` in another terminal. Quote shell wildcards.

## CLI

At an interactive prompt:

```text
z_open tcp/192.168.1.10:7447
z_put demo/example "hello from the device"
z_sub demo/**
```

Use a router address reachable from the device. Ctrl-C stops a foreground
operation, closes its connection and returns to the prompt; `z_close` releases
an idle session. Close
before opening a different endpoint. A disconnected operation can open the
configured default automatically. There is no automatic reconnection.

| Commands | Purpose |
| --- | --- |
| `help`, `help z_get`, `status` | Discover syntax, limits, session/network state, and dropped output |
| `z_open [endpoint] [--listen]`, `z_close` | Manage a connection |
| `z_put`, `z_pub`, `z_sub` | One-shot publication, repeated publication, subscription |
| `z_get`, `z_querier`, `z_queryable` | One-shot query, repeated queries, replies |
| `z_ping`, `z_pong`, `z_pub_thr`, `z_sub_thr` | Latency and throughput scenarios |
| `z_broker` | Broker with upstream forwarding; requires `alloc` and listening support |
| `quit`, `exit` | End the application console |

Use generated `help <command>` for flags and defaults. One operation runs at a
time; help/status remain available. Broker mode requires a closed session.
`just std-alloc z_broker` enables it on the host; default listeners use ports
7444 and 7445. Browser listening/brokering is unsupported.

The frontend uses `embedded-cli`, with completion, cursor editing, and history.
Example limits are 512-byte input lines, 128-byte keys, and 256-byte payloads;
`alloc` does not remove those limits. Slow output can fill bounded queues;
check `status` for drops. Subscription readiness is local registration, not an
acknowledgment that the router has propagated it.

## Microcontrollers

| Board guide | Build | Build and flash | Toolchain |
| --- | --- | --- | --- |
| [ESP32-C6-Pico Wi-Fi](../examples/embassy/esp/esp32c6-pico-wifi/README.md) | `just build-esp32c6` | `just esp32c6` | Stable RISC-V |
| [ESP32-S3 Wi-Fi](../examples/embassy/esp/esp32s3-wifi/README.md) | `just build-esp32s3` | `just esp32s3` | Xtensa `+esp` |
| [NUCLEO-H753ZI Ethernet](../examples/embassy/stm32/nucleo-h753zi-ethernet/README.md) | `just build-stm32` | `just stm32` | Stable ARM |

ESP builds require local `WIFI_PASSWORD`; `WIFI_SSID` defaults to `Zenoh`.
Set these in your shell or an approved local environment file; keep credentials
out of Git. `ENDPOINT` supplies an optional compiled connection default for CLI
firmware. Headless MCU firmware requires it. Use a reachable LAN address, not
`127.0.0.1`. Network setup uses DHCP.

UART is the default text console: 115200 baud, 8N1, no flow control. See each
board guide for pins, USB connectors, and flashing recovery. Optional USB:

```sh
just esp32c6 console-usb
just terminal /dev/cu.usbmodemXXXX
# Allocated storage and USB console:
just build-stm32 alloc,console-usb
```

Replace the port with your device (`/dev/ttyACM0`, for example, on Linux).
Ctrl-C goes to the device; Ctrl-] exits the terminal. Close it before flashing.
ESP32-S3 and STM32 use shared `embassy-usb` CDC-ACM; C6 uses hardware USB
Serial/JTAG. Read the [Embassy console guide](../examples/embassy/README.md)
for S3 BOOT/RESET recovery and STM32's separate user USB connector/power setup.

## Browser

Run `just wasm` and open `http://127.0.0.1:8000`. The router must expose a
WebSocket endpoint; the compiled default is `ws/127.0.0.1:7446`.
Use `z_open ws/host:port` to select another endpoint. The Stop button or Ctrl-C
in the command field cancels an operation.

`just build-wasm` packages assets without starting a server. Add `alloc` to
either recipe to use allocated storage. See the [browser project](../examples/wasm/README.md).

## Headless

These recipes disable default features, including `cli`. Native settings are
read at startup; MCU/browser settings are compiled in:

```sh
SCENARIO=pub KEY_EXPR=demo/example VALUE=hello COUNT=3 just std-headless
KEY_EXPR=demo/example COUNT=3 just build-esp32c6-headless pub
COUNT=3 just esp32c6-headless pub
COUNT=10 just build-esp32s3-headless ping alloc
KEY_EXPR='demo/**' just build-stm32-headless sub
COUNT=3 just wasm-headless pub
```

MCU headless recipes take a scenario and optional feature list. The `build-*`
versions only build; their counterparts flash. `build-wasm-headless` only
packages assets. With Cargo, use `--no-default-features --bin zenoh-app`,
optionally `--features alloc`, from the application directory (`cargo +esp` on S3).
Do not add `console-usb` for headless mode: it turns `cli` back on.

| Setting | Default / use |
| --- | --- |
| `SCENARIO` | `pub`; also `open`, `put`, `get`, `sub`, `queryable`, `querier`, `ping`, `pong`, `pub_thr`, `sub_thr`, `broker` |
| `ENDPOINT` | Native TCP `127.0.0.1:7447`, browser WS `127.0.0.1:7446`; required on MCU |
| `KEY_EXPR`, `PEER_KEY` | `demo/example`; ping/pong use `test/ping` and `test/pong`, throughput uses `test/thr` |
| `VALUE`, `PARAMETERS` | Payload (`Hello, from no-std!`) and query parameters (empty) |
| `COUNT` | Unbounded by default; ping defaults to 100; limits repeating scenarios |
| `INTERVAL_MS`, `TIMEOUT_MS` | 1000 ms interval; 2000 ms query/ping timeout |
| `PAYLOAD` | Generated ping/throughput size, 8 bytes |
| `LISTEN` | `false`; accept a connection instead of dialing when supported |
| `BROKER_LISTEN_1`, `BROKER_LISTEN_2` | `tcp/0.0.0.0:7444` and `tcp/0.0.0.0:7445` |
| `CLOSE_DELAY_MS` | 2000 ms after final flush before closing; 0 disables the grace period |

`put` and `get` run once. `open` and `broker` run until stopped or failed.
Embedded completion does not automatically reboot or reconnect. Headless mode
keeps application diagnostics but does not initialize the CLI console.
The closing grace period is not a delivery acknowledgment.

To start scenarios from Rust, use typed `Options`, `Context`, and `Reporter`
with [scenarios::run](../examples/common/src/scenarios/runner.rs). It manages the
session and driver lifetime. `scenarios::operation` accepts an already-managed
session; its caller must poll the driver and discard the connection on cancellation
or a transport write error, before any further network I/O.
Keep one run per context. Native exit codes are 0 for success, 1 for failure,
2 for invalid/unsupported configuration, and 130 for cancellation.

## Hardware smoke test

After building with real network settings and flashing:

1. Check `help`/`status`, network readiness, and a clean text console.
2. Open a session to the router. Exchange publications with a native subscriber
   and publisher in both directions; allow subscription propagation first.
3. Query a host queryable with changing keys, then test a query timeout.
4. Repeat Ctrl-C, close/open, malformed input, and a valid command afterward.
5. For USB, check enumeration, commands crossing 64-byte packet boundaries,
   and unplug/replug recovery. Repeat supported storage/console modes.
6. For headless, verify startup and finite completion with no terminal attached.
   For `alloc` broker mode, verify client-to-client and upstream forwarding.

Record the commit, board/revision, features, console, router version, and observed
results in the change description. A successful build or mock USB test does not
establish physical networking, enumeration, or reconnect behavior.
