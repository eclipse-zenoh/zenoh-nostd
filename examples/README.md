# Example applications

Each project builds one zenoh-app. The optional cli feature is enabled by default;
use the headless recipes to omit it. Run from the repository root in nix develop:

```sh
just std
just std z_put demo/example "hello world"
just std z_get 'demo/example/**'
just std-alloc z_broker
just build-esp32c6
just build-esp32s3
just build-stm32
just wasm
SCENARIO=pub COUNT=3 just std-headless
```

MCU consoles use UART at 115200 baud. C6: TX GPIO4/RX GPIO5; S3: TX GPIO17/RX GPIO18;
NUCLEO-H753ZI: USART3 PD8 TX/PD9 RX via ST-LINK. Use a 3.3 V adapter and crossed
TX/RX for ESP UART. Open the port with `just terminal <port>`; Ctrl-C goes to the
board and Ctrl-] exits. Close the terminal before flashing. ESP credentials are
local build-time settings; use a router ENDPOINT reachable from the device.

Use help/status and z_open at the prompt. Ctrl-C stops the operation and closes
its connection; z_close releases an idle connection. A later operation may open
the configured default endpoint again. Browser networking uses WebSocket endpoints and cannot listen or
run the broker. Native ENDPOINT is read at startup; MCU/browser settings are
compiled in. Run just test-cli and just test-headless for isolated router tests.
