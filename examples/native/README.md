# native

Run from the repository root inside `nix develop`:

```sh
just std z_put
just std z_get
```

For direct Cargo commands, enter this project directory so Cargo loads the local
target and runner configuration. Use `cargo +esp` for ESP32-S3.
