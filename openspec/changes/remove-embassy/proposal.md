# Change: Remove All Async — Fully Synchronous, Fully `no_std`

## Why

zenoh-nostd currently has an async IO layer built on Embassy (`embassy-futures`, `embassy-sync`, `embassy-time`) in its core crate, plus tokio in `platforms/zenoh-std/`. This async layer is a thin shell over fundamentally synchronous state machines:

- `zenoh-sansio` (transport state machine) — **already 100% sync**, zero async
- `zenoh-proto` (codec) — **already 100% sync**, zero async
- `zenoh-nostd/src/io/driver.rs` — async `select3` loop multiplexing TX/RX/timer
- `zenoh-nostd/src/api/session.rs` — async session operations
- `platforms/zenoh-std/src/*.rs` — async link implementations (tokio + async-net)

zenoh-pico is entirely synchronous — it uses pthreads for std concurrency and interrupt-driven models for no_std. nostd should match this.

Removing async eliminates:
- Two async runtimes (embassy + tokio) and their version compatibility
- All async plumbing (Futures, Pin, Waker) from the API surface
- executor/spawner abstractions
- Binary size overhead from async machinery
- Compile time from 500+ async crate deps

## What Changes

### 1. Make `ZLink` traits synchronous

Current (async):
```rust
pub trait ZLinkTx: ZLinkInfo {
    fn write_all(&mut self, buffer: &[u8])
        -> impl Future<Output = Result<(), LinkError>>;
}
```

New (sync):
```rust
pub trait ZLinkTx: ZLinkInfo {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError>;
}

pub trait ZLinkRx: ZLinkInfo {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError>;
    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError>;
}
```

### 2. Rewrite IO driver as synchronous poll

Replace `embassy_futures::select3` loop with a synchronous `poll()` that:
- Checks if a frame is pending in TX queue → sends it
- Checks if data is available on the link → reads and dispatches
- Checks if a timer has expired → fires the callback
- Blocks on link read with a configurable timeout via platform-specific IO multiplexing

For std: `epoll`/`kqueue`/`poll()` on link's file descriptor
For no_std: platform-specific polling (interrupt flags, HAL tick)

### 3. Rewrite session API as synchronous

Every `async fn` in `Session` becomes a regular `fn`:
- `put()`, `get()`, `reply()`, `subscribe()`, `declare_queryable()` — all sync
- Internal operations queue to the IO driver via synchronous channels (e.g., `heapless::MPSC`)

### 4. Remove all async crates from core

Remove from `crates/zenoh-nostd/Cargo.toml`:
- `embassy-futures`, `embassy-sync`, `embassy-time`, `embassy-executor`
- All async runtime abstractions

### 5. Rewrite `platforms/zenoh-std/` link implementations as synchronous

- `tcp.rs`: Replace `async_net::TcpStream` with `std::net::TcpStream`, use `set_read_timeout` for non-blocking read with timeout
- `udp.rs`: Replace `async_net::UdpSocket` with `std::net::UdpSocket`
- `mcast.rs`: Same, `std::net::UdpSocket` with multicast options
- `serial.rs`: Use `std::io::serial` or termios directly (already sync)
- `tls.rs`: Replace `async_native_tls::TlsConnector` with `native_tls::TlsConnector` (sync API)
- `bt.rs`: Use sync BlueZ D-Bus API or rfcomm
- `ws.rs`: Replace async WebSocket with sync tungstenite or similar

### 6. Replace async-native-tls with native-tls

`async_native_tls` → `native_tls` (synchronous blocking API)

### 7. Remove tokio dependency entirely

`tokio` is only used in `platforms/zenoh-std/src/tls.rs` (for `tokio::sync::Mutex` wrapper). With sync links, no mutex wrapping is needed.

### 8. Remove `platforms/zenoh-embassy/` crate entirely

Already covered — delete the directory.

### 9. Single-threaded poll model (primary design)

The IO driver is driven by the application calling `Session::poll()` in its main loop:

```rust
loop {
    session.poll();  // drive IO, fire callbacks, handle timeouts
    session.put(&key, b"hello").ok();
    // ... other app logic
}
```

`poll()` does:
1. Check TX queue → send pending frames on the link
2. Check link readability (platform-specific: `poll()` on FD, interrupt flag, HAL tick) → read and dispatch
3. Check timer expiry → fire lease/keepalive/liveliness callbacks
4. Return immediately (non-blocking)

For **std platforms**, a convenience `session.run_background()` spawns a thread that calls `poll()` in a loop — but the core API is always `poll()`.

No executor, no async runtime, no background threads in the core design.

### 10. Update `no_std` constraint documentation

The crate becomes truly `no_std` — no runtime dependencies at all.
- `std` feature flag only pulls in `std::net`, `std::thread`, `std::time`
- `no_std` builds use `embedded-hal` for everything

## Capabilities

### New

None — this is a removal/refactoring.

### Modified

- **`platform-std`** — Link implementations rewritten as synchronous. Thread-based concurrency replaces async tasks.
- **`platform-embassy`** — Removed entirely.
- **Built-in session operations** — All `async fn` become `fn`. Breaking change for any consumer using `.await`.

### Removed Capabilities

- Async IO runtime (embassy + tokio)
- Async link implementations
- Async session API
- `platform-embassy` platform crate

## Impact

| Area | Change | Risk |
|---|---|---|
| `zenoh-sansio` | No changes needed (already sync) | None |
| `zenoh-proto` | No changes needed (already sync) | None |
| `crates/zenoh-nostd/src/io/link.rs` | `ZLinkTx`/`ZLinkRx` traits become sync | **BREAKING** — affects all link implementations |
| `crates/zenoh-nostd/src/io/driver.rs` | Complete rewrite: `select3` → sync poll() | High — core IO loop |
| `crates/zenoh-nostd/src/io/transport.rs` | Remove async wrappers | Medium |
| `crates/zenoh-nostd/src/api/session.rs` | Remove `async fn` | **BREAKING** — API surface |
| `crates/zenoh-nostd/src/api/session/sub.rs` | Remove `async fn` | Medium |
| `crates/zenoh-nostd/src/api/session/put.rs` | Remove `async fn` | Medium |
| `crates/zenoh-nostd/src/api/session/get.rs` | Remove `async fn` | Medium |
| `crates/zenoh-nostd/src/api/session/queryable.rs` | Remove `async fn` | Medium |
| `crates/zenoh-nostd/src/api/session/liveliness.rs` | Remove `async fn` | Medium |
| `crates/zenoh-nostd/src/api/mesh.rs` | Remove `async fn` | Medium |
| `crates/zenoh-nostd/src/api/callbacks.rs` | Remove `async fn` | Medium |
| `crates/zenoh-nostd/src/api/broker.rs` | Remove `async fn` | Medium |
| `crates/zenoh-nostd/src/api/auto_reconnect.rs` | Rewrite | Medium |
| `crates/zenoh-nostd/src/api/connectivity.rs` | Remove `async fn` | Medium |
| `crates/zenoh-nostd/Cargo.toml` | Remove embassy deps | Low |
| `platforms/zenoh-std/src/tcp.rs` | Rewrite: `async_net` → `std::net` | Medium |
| `platforms/zenoh-std/src/udp.rs` | Rewrite: `async_net` → `std::net` | Medium |
| `platforms/zenoh-std/src/mcast.rs` | Rewrite: `async_net` → `std::net` | Medium |
| `platforms/zenoh-std/src/serial.rs` | Already mostly sync, minor cleanup | Low |
| `platforms/zenoh-std/src/tls.rs` | Rewrite: `async_native_tls` → `native_tls`, remove tokio | Medium |
| `platforms/zenoh-std/src/bt.rs` | Rewrite: sync BlueZ calls | Medium |
| `platforms/zenoh-std/src/ws.rs` | Rewrite: sync WebSocket | Medium |
| `platforms/zenoh-embassy/` | Delete entire crate | Low |
| All examples | Remove `.await`, use sync calls | Medium |
| Tests | Remove `async` test harnesses | Medium |

## Phasing

| Phase | Scope | Deliverable |
|---|---|---|
| 1 | `ZLink` traits sync | Rewrite `ZLinkTx`/`ZLinkRx`/`ZLink`/`ZLinkManager` to remove async; fix all impls |
| 2 | IO driver rewrite | Replace `select3` loop with synchronous `poll()` using platform IO multiplexing |
| 3 | Session API sync | Remove `async fn` from all `Session` methods and internal operations |
| 4 | `platforms/zenoh-std` sync | Rewrite all 7 link implementations (tcp, udp, mcast, serial, tls, bt, ws) to use blocking I/O |
| 5 | Concurrency model | Thread-based for std (`std::thread::spawn`), poll-loop for no_std; replace embassy task abstractions |
| 6 | Dependency cleanup | Remove `embassy-*`, `tokio`, `async-net`, `async-native-tls`, `futures-lite`; add `native-tls`, sync WS lib |
| 7 | Test & example update | Remove `.await` from all tests and examples; fix test harnesses |
| 8 | Documentation | Update feature matrix, no_std constraints, README, platform list |
| 9 | Final verification | `cargo check --all-features`, `cargo check --no-default-features`, static analysis |