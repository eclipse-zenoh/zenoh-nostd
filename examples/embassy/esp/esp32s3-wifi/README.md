# esp32s3-wifi

Run from the repository root inside `nix develop`:

```sh
just build-esp32s3 z_put
just esp32s3 z_put
```

Set WIFI_SSID/WIFI_PASSWORD locally before building; ENDPOINT must be a
router address reachable from the board.

For direct Cargo commands, enter this project directory so Cargo loads the local
target and runner configuration. Use `cargo +esp` for ESP32-S3.
