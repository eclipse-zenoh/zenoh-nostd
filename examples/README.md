# Standalone examples

Enter `nix develop` at the repository root. Use `just std z_put`,
`just std z_sub`, or `just wasm z_put` to run an existing scenario.
Add `alloc` after the scenario for allocated Zenoh storage.

- [native](native/README.md)
- [wasm](wasm/README.md)
- [embassy/esp/esp32s3-wifi](embassy/esp/esp32s3-wifi/README.md)
- [embassy/esp/esp32c6-pico-wifi](embassy/esp/esp32c6-pico-wifi/README.md)

Build-only recipes never flash. ESP32-S3 uses `cargo +esp`; other projects use stable Rust.
