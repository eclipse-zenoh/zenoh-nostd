# nucleo-h753zi-ethernet

> [!WARNING]
> This firmware has **not been tested on a physical NUCLEO-H753ZI board**.
> Compilation and release linking have been checked; hardware operation remains unverified.

Run from the repository root inside `nix develop`:

```sh
just build-stm32 z_put
just stm32 z_put
```

Set ENDPOINT to a router address reachable from the board.

For direct Cargo commands, enter this project directory so Cargo loads the local
target and runner configuration. Use `cargo +esp` for ESP32-S3.
