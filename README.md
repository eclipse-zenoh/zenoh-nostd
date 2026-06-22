<p align="center">
  <img src="https://zenoh.io/img/zenoh-dragon-small.png" height="121">
</p>

<h1 align="center">zenoh-nostd</h1>
<p align="center">
  <strong>Zero Network Overhead. Async. No std. No alloc. Pure Rust.</strong>
</p>
<p align="center">
  <code>async</code> • <code>bare-metal</code> • <code>no_std</code> • <code>zenoh</code>
</p>
<p align="center">
  <a href="https://github.com/eclipse-zenoh/zenoh-nostd/actions/workflows/ci.yml"><img src="https://github.com/eclipse-zenoh/zenoh-nostd/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://codecov.io/gh/eclipse-zenoh/zenoh-nostd"><img src="https://codecov.io/gh/eclipse-zenoh/zenoh-nostd/graph/badge.svg" alt="Code coverage" /></a>
  <a href="https://cla-assistant.io/ZettaScaleLabs/zenoh-nostd"><img src="https://cla-assistant.io/readme/badge/ZettaScaleLabs/zenoh-nostd" alt="CLA assistant" /></a>
</p>

---

## 📦 Overview

**zenoh-nostd** is a Rust-native, `async`, `#![no_std]`, `no_alloc` library that provides a **zero-overhead network abstraction layer** for ultra-constrained and bare-metal environments. In other terms you can run this *bare metal* on your favourite microcontroller.

> ⚡ Built on the <a href="https://github.com/eclipse-zenoh/zenoh">Zenoh protocol</a>, but stripped to the bone for minimalism and raw performance.

---

## ✨ Features

- **`#![no_std]`**: No reliance on the Rust standard library.
- **No dynamic allocation**: Fully `no_alloc`, ideal for bare-metal targets.
- **Deterministic**: No heap, no surprises.
- **Safe Rust**: Entirely memory-safe.
- **Testable**: Supports both embedded and native testing environments.
- **Embassy Integration**: Seamlessly integrates with the Embassy async runtime for embedded systems.
- **Broker**: A `#![no_std]` broker to broke multiple zenoh `clients` with an optional gateway to a `zenoh` network.

---

## 🔧 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
zenoh-nostd = { git = "https://github.com/kydos/zenoh-nostd" }
```

> For embedded systems, ensure your crate uses `#![no_std]`:

```rust
#![no_std]
```

---

## 🔌 Integration

### Minimal Example

Here’s a simple example of sending a payload with `zenoh-nostd`:

```rust
async fn entry(spawner: embassy_executor::Spawner) -> zenoh::ZResult<()> {
    let config = init_session_example(&spawner).await;
    let mut resources = Resources::default();
    let session = zenoh::connect(&mut resources, &config, Endpoint::try_from(ENDPOINT)?).await?;

    let ke = zenoh::keyexpr::new("demo/example")?;
    let payload = b"Hello, from no-std!";
    
    session.put(ke, payload).finish().await?;
    
    Ok(())
}
```

---

## 🔬 MSRV

> 🛠️ **Minimum Supported Rust Version**: Currently `1.91.0`

---

## ⚠️ Current Limitations

* No serial support yet. ([#9](https://github.com/eclipse-zenoh/zenoh-nostd/issues/9)
* `Interest` protocol not implemented yet. ([#10](https://github.com/eclipse-zenoh/zenoh-nostd/issues/10)

---

## 🧪 Building and Testing

This project uses [`just`](https://github.com/casey/just) for task management. Use `just check` to check the project and examples, `just test` to run the tests and `just bench` to run the benchmarks.

> 🔍 Pull requests that slow down the bench should be avoided.

### Testing Examples

Use the following command structure:

```bash
just <platform> <example> [optional features]
```

* **Platforms**: `std`, `wasm`, `esp32s3`
* **Examples**: `z_get`, `z_open`, `z_ping`, `z_pong`, `z_pub`, `z_pub_thr`, `z_put`, `z_querier`, `z_queryable`, `z_sub`, , `z_sub_thr`,
* **Broker**: `z_broker` (with `alloc` feature)

Set the `ENDPOINT=<endpoint>` environment variable to specify the endpoint (default is `tcp/127.0.0.1:7447`). Set `LISTEN=1` to specify the connection method.

For `esp32s3`, you must also provide:

* `WIFI_SSID` (default is `WiFiPassword`).
* `WIFI_PASSWORD` (no default, must be provided).

See the ESP32 setup documentation for toolchain and target installation.

Example of few commands:

```bash
ENDPOINT=tcp/127.0.0.1:7447 just std z_pub
```

```bash
WIFI_PASSWORD=* ENDPOINT=tcp/192.168.21.1:7447 just esp32s3 z_sub
```

### Example: Local TCP

Run a Zenoh router with:

```bash
zenohd -l tcp/127.0.0.1:7447
```

In two terminals:

```bash
# Terminal 1
just std z_pub

# Terminal 2
just std z_sub
```

### Example: WebSocket + WASM

Run a Zenoh router with:

```bash
zenohd -l tcp/127.0.0.1:7447 -l ws/127.0.0.1:7446
```

Then:

```bash
# Terminal 1 (WASM)
ENDPOINT=ws/127.0.0.1:7446 just wasm z_pub

# Terminal 2 (STD)
just std z_sub
```

> 📦 Note: For WASM, ensure you have:
>
> * `wasm32-unknown-unknown` target
> * `wasm-bindgen-cli`
> * `basic-http-server` (or similar)

---

## 📁 Project Layout

```text
zenoh-nostd/            # Git repository root
├── crates/
│   ├── zenoh-derive/   # Derive macros
│   ├── zenoh-nostd/    # Zenoh with IO, embassy
│   ├── zenoh-proto/    # Zenoh Protocol
│   └── zenoh-sansio/   # Zenoh Sans-IO objects (Transport only right now)
│
├── examples/
│   ├── web/
│   │   └── index.html  # File to test wasm example
│   │
│   ├── z_broker.rs     # Example with std/wasm/embassy io
│   ├── z_get.rs        # Example with std/wasm/embassy io
│   ├── z_open.rs       # Example with std/wasm/embassy io
│   ├── z_ping.rs       # Example with std/wasm/embassy io
│   ├── z_pong.rs       # Example with std/wasm/embassy io
│   ├── z_pub.rs        # Example with std/wasm/embassy io
│   ├── z_pub_thr.rs    # Example with std/wasm/embassy io
│   ├── z_put.rs        # Example with std/wasm/embassy io
│   ├── z_querier.rs    # Example with std/wasm/embassy io
│   ├── z_queryable.rs  # Example with std/wasm/embassy io
│   ├── z_sub.rs        # Example with std/wasm/embassy io
│   └── z_sub_thr.rs    # Example with std/wasm/embassy io
│
├── platforms/          # Platform-specific implementations
│   ├── zenoh-embassy/  # Embassy platforms (no_std)
│   ├── zenoh-std/      # Standard platforms (std)
│   └── zenoh-wasm/     # WASM32 platforms (wasm)
│   └── zenoh-ts/       # TypeScript API leveraging WASM build
│
├── Cargo.toml          # Workspace + example package
└── src/
    └── lib.rs          # Example lib.rs
```
