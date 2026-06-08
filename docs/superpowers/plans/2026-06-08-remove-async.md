# Remove All Async — Fully Synchronous, Fully `no_std` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate all async from zenoh-nostd — replace embassy-based IO driver with synchronous `poll()`, make all link traits sync, remove tokio/embassy dependencies, and deliver a single-threaded poll-driven API matching zenoh-pico's architecture.

**Architecture:** The core insight is that `zenoh-sansio` already has sync `connect()`/`listen()` methods taking `FnMut` closures. The async is only in:
1. `ZLink`/`ZLinkTx`/`ZLinkRx`/`ZLinkManager` traits (return `impl Future`)
2. `TransportLinkManager` (wraps sansio's async handshake with `with_timeout`)
3. `Driver` (embassy `select3` loop)
4. `Session` methods (all `async fn`)
5. `platforms/zenoh-std/` link implementations (tokio + async-net)

The plan: make traits sync → rewrite driver as sync poll → make session sync → rewrite platform links → remove deps.

**Tech Stack:** `std::net`, `std::thread`, `native_tls`, `embedded-hal` (sync), `heapless`

---

### Task 1: Make `ZLink` traits synchronous

**Files:**
- Modify: `crates/zenoh-nostd/src/io/link.rs:8-53`
- Modify: `crates/zenoh-nostd/src/io/link.rs:120-186` (EmbeddedIOLink impls)

- [ ] **Step 1: Rewrite `ZLinkTx`, `ZLinkRx`, `ZLinkManager` traits to remove `impl Future`**

```rust
// crates/zenoh-nostd/src/io/link.rs

pub trait ZLinkTx: ZLinkInfo {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), zenoh_proto::LinkError>;
}

pub trait ZLinkRx: ZLinkInfo {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, zenoh_proto::LinkError>;
    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), zenoh_proto::LinkError>;
}

pub trait ZLinkManager {
    type Link<'a>: ZLink
    where
        Self: 'a;

    fn connect(
        &self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<Self::Link<'_>, LinkError>;

    fn listen(
        &self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<Self::Link<'_>, LinkError>;
}
```

- [ ] **Step 2: Rewrite `EmbeddedIOLink` impls to be synchronous**

Replace `embedded_io_async::Write`/`embedded_io_async::Read` with `embedded_io::Write`/`embedded_io::Read`:

```rust
// crates/zenoh-nostd/src/io/link.rs

use embedded_io::{Read, Write};

pub struct EmbeddedIOLink<Tx: Write, Rx: Read> {
    tx: Tx,
    rx: Rx,
    mtu: u16,
    is_streamed: bool,
}

impl<Tx: Write, Rx: Read> ZLinkTx for EmbeddedIOLink<Tx, Rx> {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), zenoh_proto::LinkError> {
        self.tx.write_all(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl<Tx: Write, Rx: Read> ZLinkRx for EmbeddedIOLink<Tx, Rx> {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, zenoh_proto::LinkError> {
        self.rx.read(buffer).map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), zenoh_proto::LinkError> {
        self.rx.read_exact(buffer).map_err(|_| LinkError::LinkRxFailed)
    }
}
```

Do the same for `EmbeddedIOLinkTx` and `EmbeddedIOLinkRx`.

- [ ] **Step 3: Run test to verify it compiles**

Run: `cargo check -p zenoh-nostd --no-default-features`
Expected: compilation errors in `TransportLinkManager` and `Driver` (they still use async) — that's expected, we fix those next.

- [ ] **Step 4: Commit**

```bash
git add crates/zenoh-nostd/src/io/link.rs
git commit -m "refactor: make ZLink/ZLinkTx/ZLinkRx/ZLinkManager traits synchronous"
```

---

### Task 2: Rewrite `TransportLinkManager` to use sync sansio API

**Files:**
- Modify: `crates/zenoh-nostd/src/io/transport.rs:126-287`

- [ ] **Step 1: Rewrite `bridge_connect`, `bridge_listen`, `connect`, `listen` as sync methods**

The sansio crate already has `Transport::builder(...).connect(...)` (sync, takes `FnMut`) and `Transport::builder(...).listen(...)` (sync). Use those instead of `connect_async`/`listen_async`.

Replace `with_timeout` with a simple loop + `Instant::now()` check (or use `std::time::Instant` behind `#[cfg(feature = "std")]` and a no-op for no_std).

```rust
// crates/zenoh-nostd/src/io/transport.rs

use core::time::Duration;
use zenoh_proto::{Endpoint, TransportLinkError, fields::{Resolution, WhatAmI, ZenohIdProto}};
use zenoh_sansio::{Transport, ZTransportRx, ZTransportTx};
use crate::io::link::EmbeddedIOLink;
use super::link::{ZLink, ZLinkInfo, ZLinkManager, ZLinkRx, ZLinkTx};

impl<LinkManager> TransportLinkManager<LinkManager> {
    pub fn bridge_connect<Tx: Write, Rx: Read, Buff>(
        &self,
        mut link: EmbeddedIOLink<Tx, Rx>,
        buff: Buff,
    ) -> core::result::Result<TransportLink<EmbeddedIOLink<Tx, Rx>, Buff>, TransportLinkError>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
    {
        let streamed = link.is_streamed();
        let transport = Transport::builder(buff)
            .with_zid(self.zid)
            .with_whatami(self.whatami)
            .with_lease(self.lease)
            .with_resolution(self.resolution)
            .connect(
                &mut link,
                |link, bytes| {
                    if link.is_streamed() {
                        link.read_exact(bytes).map(|_| bytes.len())
                    } else {
                        link.read(bytes)
                    }
                },
                |link, bytes| link.write_all(bytes),
            )
            .with_prefixed(streamed)
            .finish(self.open_timeout)
            .map_err(|e| e.flatten_map::<TransportLinkError>())?;

        Ok(TransportLink::new(link, transport))
    }

    pub fn bridge_listen<Tx: Write, Rx: Read, Buff>(
        &self,
        mut link: EmbeddedIOLink<Tx, Rx>,
        buff: Buff,
    ) -> core::result::Result<TransportLink<EmbeddedIOLink<Tx, Rx>, Buff>, TransportLinkError>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
    {
        let streamed = link.is_streamed();
        let transport = Transport::builder(buff)
            .with_zid(self.zid)
            .with_whatami(self.whatami)
            .with_lease(self.lease)
            .with_resolution(self.resolution)
            .listen(
                &mut link,
                |link, bytes| {
                    if link.is_streamed() {
                        link.read_exact(bytes).map(|_| bytes.len())
                    } else {
                        link.read(bytes)
                    }
                },
                |link, bytes| link.write_all(bytes),
            )
            .with_prefixed(streamed)
            .finish(self.open_timeout)
            .map_err(|e| e.flatten_map::<TransportLinkError>())?;

        Ok(TransportLink::new(link, transport))
    }

    pub fn connect<Buff>(
        &self,
        endpoint: Endpoint<'_>,
        buff: Buff,
    ) -> core::result::Result<TransportLink<LinkManager::Link<'_>, Buff>, TransportLinkError>
    where
        LinkManager: ZLinkManager,
        Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
    {
        let mut link = self.link_manager.connect(endpoint)?;
        let streamed = link.is_streamed();
        let transport = Transport::builder(buff)
            .with_zid(self.zid)
            .with_whatami(self.whatami)
            .with_lease(self.lease)
            .with_resolution(self.resolution)
            .connect(
                &mut link,
                |link, bytes| {
                    if link.is_streamed() {
                        link.read_exact(bytes).map(|_| bytes.len())
                    } else {
                        link.read(bytes)
                    }
                },
                |link, bytes| link.write_all(bytes),
            )
            .with_prefixed(streamed)
            .finish(self.open_timeout)
            .map_err(|e| e.flatten_map::<TransportLinkError>())?;

        Ok(TransportLink::new(link, transport))
    }

    pub fn listen<Buff>(
        &self,
        endpoint: Endpoint<'_>,
        buff: Buff,
    ) -> core::result::Result<TransportLink<LinkManager::Link<'_>, Buff>, TransportLinkError>
    where
        LinkManager: ZLinkManager,
        Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
    {
        let mut link = self.link_manager.listen(endpoint)?;
        let streamed = link.is_streamed();
        let transport = Transport::builder(buff)
            .with_zid(self.zid)
            .with_whatami(self.whatami)
            .with_lease(self.lease)
            .with_resolution(self.resolution)
            .listen(
                &mut link,
                |link, bytes| {
                    if link.is_streamed() {
                        link.read_exact(bytes).map(|_| bytes.len())
                    } else {
                        link.read(bytes)
                    }
                },
                |link, bytes| link.write_all(bytes),
            )
            .with_prefixed(streamed)
            .finish(self.open_timeout)
            .map_err(|e| e.flatten_map::<TransportLinkError>())?;

        Ok(TransportLink::new(link, transport))
    }
}
```

- [ ] **Step 2: Remove `embassy_time::with_timeout` import and `async` keywords**

Remove the `use embassy_time::with_timeout;` import. Remove `async` from all method signatures.

- [ ] **Step 3: Run test to verify it compiles**

Run: `cargo check -p zenoh-nostd --no-default-features`
Expected: compilation errors in `Driver` (still async) — expected.

- [ ] **Step 4: Commit**

```bash
git add crates/zenoh-nostd/src/io/transport.rs
git commit -m "refactor: make TransportLinkManager methods synchronous, use sansio sync API"
```

---

### Task 3: Rewrite IO driver as synchronous `poll()`

**Files:**
- Modify: `crates/zenoh-nostd/src/io/driver.rs`

- [ ] **Step 1: Rewrite `Driver` struct and methods as synchronous**

Remove all embassy types. Replace `select3` loop with a synchronous `poll()` method.

```rust
// crates/zenoh-nostd/src/io/driver.rs

use core::time::Duration;
use zenoh_proto::{EitherError, TransportLinkError, fields::ZenohIdProto, msgs::NetworkMessage};

use crate::{
    api::connectivity::{ConnectivityState, TransportEvent},
    io::transport::{TransportLink, TransportLinkRx, TransportLinkTx, ZTransportLinkRx, ZTransportLinkTx},
    platform::ZLink,
};

pub struct Driver<'res, Link, Buff>
where
    Link: ZLink + 'res,
    Buff: Clone,
{
    zid: ZenohIdProto,
    tx: TransportLinkTx<'res, Link::Tx<'res>, Buff>,
    rx: TransportLinkRx<'res, Link::Rx<'res>, Buff>,
}

impl<'res, Link, Buff> Driver<'res, Link, Buff>
where
    Link: ZLink,
    Buff: Clone,
{
    pub fn new(transport: &'res mut TransportLink<Link, Buff>) -> Self {
        let zid = transport.transport().other_zid;
        let (tx, rx) = transport.split();
        Self { zid, tx, rx }
    }

    pub fn zid(&self) -> ZenohIdProto {
        self.zid
    }

    pub fn tx_mut(&mut self) -> &mut TransportLinkTx<'res, Link::Tx<'res>, Buff> {
        &mut self.tx
    }

    /// Drive IO: send pending frames, receive and dispatch messages, handle timeouts.
    /// Returns `true` if any work was done, `false` if idle (no data to send/receive).
    /// Call this in your main loop. Non-blocking.
    pub fn poll<State, E>(
        &mut self,
        state: &mut State,
        connectivity: &ConnectivityState<'_>,
        mut update: impl FnMut(ZenohIdProto, &mut State, NetworkMessage<'_>, &[u8]) -> core::result::Result<(), E>,
    ) -> core::result::Result<bool, EitherError<TransportLinkError, E>>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]>,
    {
        let mut did_work = false;

        // 1. Sync transport state machines (lease, keepalive timers)
        self.rx.transport_mut().sync(None, Duration::ZERO);
        self.tx.transport_mut().sync(Some(self.rx.transport()), Duration::ZERO);

        // 2. Check if transport is closed
        if self.rx.transport().closed() {
            connectivity.emit(TransportEvent::Disconnected);
            return Err(EitherError::A(TransportLinkError::TransportClosed));
        }

        // 3. Try to send pending frames
        if self.tx.transport().should_send_keepalive(Duration::ZERO) {
            self.tx.send(
                core::iter::once(NetworkMessage {
                    reliability: zenoh_proto::fields::Reliability::default(),
                    qos: zenoh_proto::exts::QoS::default(),
                    body: zenoh_proto::msgs::NetworkBody::Unit(zenoh_proto::msgs::Unit),
                })
            )?;
            did_work = true;
        }

        // 4. Try to receive frames (non-blocking)
        match self.rx.recv() {
            Ok(Some(messages)) => {
                for msg in messages {
                    update(self.zid, state, msg.0, msg.1).map_err(EitherError::B)?;
                }
                did_work = true;
            }
            Ok(None) => {} // no data available
            Err(e) => {
                connectivity.emit(TransportEvent::Disconnected);
                return Err(EitherError::A(e));
            }
        }

        // 5. Check close condition
        if self.rx.transport().should_close(Duration::ZERO) {
            self.tx.close()?;
            connectivity.emit(TransportEvent::Disconnected);
            return Err(EitherError::A(TransportLinkError::TransportClosed));
        }

        Ok(did_work)
    }
}
```

- [ ] **Step 2: Remove all embassy imports**

Remove: `embassy_futures::select`, `embassy_sync::blocking_mutex`, `embassy_sync::mutex`, `embassy_time`.

- [ ] **Step 3: Run test to verify it compiles**

Run: `cargo check -p zenoh-nostd --no-default-features`
Expected: compilation errors in `Session` (still uses async `state()` and `driver.tx().await`) — expected.

- [ ] **Step 4: Commit**

```bash
git add crates/zenoh-nostd/src/io/driver.rs
git commit -m "refactor: rewrite IO driver as synchronous poll() — no embassy, no async"
```

---

### Task 4: Make `Session` API synchronous

**Files:**
- Modify: `crates/zenoh-nostd/src/api/session.rs`
- Modify: `crates/zenoh-nostd/src/api/session/put.rs`
- Modify: `crates/zenoh-nostd/src/api/session/sub.rs`
- Modify: `crates/zenoh-nostd/src/api/session/get.rs`
- Modify: `crates/zenoh-nostd/src/api/session/queryable.rs`
- Modify: `crates/zenoh-nostd/src/api/session/liveliness.rs`
- Modify: `crates/zenoh-nostd/src/api/session/delete.rs`
- Modify: `crates/zenoh-nostd/src/api/mesh.rs`
- Modify: `crates/zenoh-nostd/src/api/callbacks.rs`
- Modify: `crates/zenoh-nostd/src/api/broker.rs`
- Modify: `crates/zenoh-nostd/src/api/auto_reconnect.rs`
- Modify: `crates/zenoh-nostd/src/api/connectivity.rs`

- [ ] **Step 1: Rewrite `Session` struct to remove `Mutex` and make `state` direct**

Remove `embassy_sync::mutex::Mutex`. Store `state` directly (no lock needed — single-threaded).

```rust
// crates/zenoh-nostd/src/api/session.rs

pub struct Session<'res, Config>
where
    Config: ZSessionConfig,
{
    driver: Driver<'res, <Config::LinkManager as ZLinkManager>::Link<'res>, Config::Buff>,
    state: SessionState<'res, Config>,
    connectivity: ConnectivityState<'static>,
}

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn new(
        transport: &'res mut TransportLink<
            <Config::LinkManager as ZLinkManager>::Link<'res>,
            Config::Buff,
        >,
    ) -> Self {
        Self {
            driver: Driver::new(transport),
            state: SessionState::new(),
            connectivity: ConnectivityState::new(),
        }
    }

    pub fn poll<State, E>(
        &mut self,
        state: &mut State,
        update: impl FnMut(ZenohIdProto, &mut State, NetworkMessage<'_>, &[u8]) -> core::result::Result<(), E>,
    ) -> core::result::Result<bool, EitherError<TransportLinkError, E>>
    where
        Config::Buff: AsMut<[u8]> + AsRef<[u8]>,
    {
        self.driver.poll(state, &self.connectivity, update)
    }

    pub(crate) fn state(&mut self) -> &mut SessionState<'res, Config> {
        &mut self.state
    }

    pub fn batch_start(&mut self) -> core::result::Result<(), TransportLinkError> {
        let (_, transport) = self.driver.tx_mut().tx();
        transport.batch_start();
        Ok(())
    }

    pub fn batch_flush(&mut self) -> core::result::Result<(), TransportLinkError> {
        ZTransportLinkTx::batch_flush(self.driver.tx_mut())
    }

    pub fn batch_stop(&mut self) -> core::result::Result<(), TransportLinkError> {
        {
            let (_, transport) = self.driver.tx_mut().tx();
            transport.batch_stop();
        }
        ZTransportLinkTx::batch_flush(self.driver.tx_mut())
    }

    pub fn zid(&self) -> ZenohIdProto {
        self.driver.zid()
    }

    pub(crate) fn _undeclare_queryable(
        &mut self,
        id: u32,
    ) -> core::result::Result<(), SessionError> {
        self.state.queryable_callbacks.remove(id)?;

        let msg = Declare {
            body: DeclareBody::UndeclareQueryable(UndeclareQueryable {
                id,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.driver
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))?;

        Ok(())
    }

    pub fn on_connect(&mut self, sender: DynamicSender<'static, TransportEvent>) {
        let _ = self.connectivity.transport_senders.push(sender);
    }

    pub fn on_disconnect(&mut self, sender: DynamicSender<'static, TransportEvent>) {
        let _ = self.connectivity.transport_senders.push(sender);
    }

    #[cfg(feature = "acl")]
    pub fn check_acl_permission(
        &self,
        _operation: AclOperation,
        _ke: &keyexpr,
    ) -> core::result::Result<(), SessionError> {
        Ok(())
    }
}
```

- [ ] **Step 2: Rewrite `session_connect`, `session_listen` as sync functions**

```rust
// crates/zenoh-nostd/src/api/session.rs

pub fn session_connect<'res, Config>(
    resources: &'res mut Resources<'res, Config>,
    config: &'res Config,
    endpoint: Endpoint<'_>,
) -> core::result::Result<Session<'res, Config>, TransportLinkError>
where
    Config: ZSessionConfig,
{
    Ok(Session::new(resources.init(
        config.transports().connect(endpoint, config.buff())?,
    )))
}

pub fn session_listen<'res, Config>(
    resources: &'res mut Resources<'res, Config>,
    config: &'res Config,
    endpoint: Endpoint<'_>,
) -> core::result::Result<Session<'res, Config>, TransportLinkError>
where
    Config: ZSessionConfig,
{
    Ok(Session::new(resources.init(
        config.transports().listen(endpoint, config.buff())?,
    )))
}
```

- [ ] **Step 3: Rewrite `session_connect_ignore_invalid_sn` and `session_listen_ignore_invalid_sn` as sync**

Same pattern — remove `.await`, use `?` directly.

- [ ] **Step 4: Update macros `__session_connect!` and `__session_listen!`**

Remove `.await` from macro bodies.

- [ ] **Step 5: Rewrite all sub-module methods as sync**

For each of `put.rs`, `sub.rs`, `get.rs`, `queryable.rs`, `liveliness.rs`, `delete.rs`:
- Remove `async` from all `fn` signatures
- Replace `self.driver.tx().await` with `self.driver.tx_mut()`
- Replace `self.state().await` with `self.state()`
- Replace `.await` on send/recv calls with `?`

For `mesh.rs`, `callbacks.rs`, `broker.rs`, `auto_reconnect.rs`, `connectivity.rs`:
- Same pattern — remove `async`, remove `.await`

- [ ] **Step 6: Remove `embassy_sync` imports from session.rs**

Remove `use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::DynamicSender, mutex::{Mutex, MutexGuard}};`

- [ ] **Step 7: Run test to verify it compiles**

Run: `cargo check -p zenoh-nostd --no-default-features`
Expected: clean compilation (no async errors). May have errors in platform link implementations — those are next.

- [ ] **Step 8: Commit**

```bash
git add crates/zenoh-nostd/src/api/session.rs crates/zenoh-nostd/src/api/session/put.rs crates/zenoh-nostd/src/api/session/sub.rs crates/zenoh-nostd/src/api/session/get.rs crates/zenoh-nostd/src/api/session/queryable.rs crates/zenoh-nostd/src/api/session/liveliness.rs crates/zenoh-nostd/src/api/session/delete.rs crates/zenoh-nostd/src/api/mesh.rs crates/zenoh-nostd/src/api/callbacks.rs crates/zenoh-nostd/src/api/broker.rs crates/zenoh-nostd/src/api/auto_reconnect.rs crates/zenoh-nostd/src/api/connectivity.rs
git commit -m "refactor: make Session API synchronous — remove all async fn, remove embassy mutex"
```

---

### Task 5: Rewrite `platforms/zenoh-std/` link implementations as synchronous

**Files:**
- Modify: `platforms/zenoh-std/src/tcp.rs`
- Modify: `platforms/zenoh-std/src/udp.rs`
- Modify: `platforms/zenoh-std/src/mcast.rs`
- Modify: `platforms/zenoh-std/src/serial.rs`
- Modify: `platforms/zenoh-std/src/tls.rs`
- Modify: `platforms/zenoh-std/src/bt.rs`
- Modify: `platforms/zenoh-std/src/ws.rs`
- Modify: `platforms/zenoh-std/src/lib.rs`

- [ ] **Step 1: Rewrite `tcp.rs` — replace `async_net::TcpStream` with `std::net::TcpStream`**

```rust
// platforms/zenoh-std/src/tcp.rs

use std::net::{TcpStream, SocketAddr};
use std::io::{Read, Write};
use zenoh_nostd::platform::*;

pub struct StdTcpLink {
    stream: TcpStream,
    mtu: u16,
}

impl StdTcpLink {
    pub fn connect(addr: SocketAddr) -> Result<Self, LinkError> {
        let stream = TcpStream::connect(addr).map_err(|_| LinkError::CouldNotConnect)?;
        stream.set_nodelay(true).map_err(|_| LinkError::CouldNotConnect)?;
        stream.set_read_timeout(Some(std::time::Duration::from_millis(10)))
            .map_err(|_| LinkError::CouldNotConnect)?;
        Ok(Self { stream, mtu: u16::MAX })
    }

    pub fn listen(addr: SocketAddr) -> Result<(Self, SocketAddr), LinkError> {
        let listener = std::net::TcpListener::bind(addr).map_err(|_| LinkError::CouldNotConnect)?;
        let (stream, peer) = listener.accept().map_err(|_| LinkError::CouldNotConnect)?;
        stream.set_nodelay(true).map_err(|_| LinkError::CouldNotConnect)?;
        stream.set_read_timeout(Some(std::time::Duration::from_millis(10)))
            .map_err(|_| LinkError::CouldNotConnect)?;
        Ok((Self { stream, mtu: u16::MAX }, peer))
    }
}

impl ZLinkInfo for StdTcpLink {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for StdTcpLink {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        self.stream.write_all(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for StdTcpLink {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        self.stream.read(buffer).map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        self.stream.read_exact(buffer).map_err(|_| LinkError::LinkRxFailed)
    }
}

impl ZLink for StdTcpLink {
    type Tx<'link> = StdTcpLinkTx where Self: 'link;
    type Rx<'link> = StdTcpLinkRx where Self: 'link;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        let mtu = self.mtu;
        (
            StdTcpLinkTx { stream: &mut self.stream, mtu },
            StdTcpLinkRx { stream: &mut self.stream, mtu },
        )
    }
}

pub struct StdTcpLinkTx<'a> { stream: &'a mut TcpStream, mtu: u16 }
pub struct StdTcpLinkRx<'a> { stream: &'a mut TcpStream, mtu: u16 }

impl ZLinkInfo for StdTcpLinkTx<'_> {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for StdTcpLinkTx<'_> {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        self.stream.write_all(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkInfo for StdTcpLinkRx<'_> {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkRx for StdTcpLinkRx<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        self.stream.read(buffer).map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        self.stream.read_exact(buffer).map_err(|_| LinkError::LinkRxFailed)
    }
}
```

- [ ] **Step 2: Rewrite `udp.rs` — replace `async_net::UdpSocket` with `std::net::UdpSocket`**

Same pattern as TCP but with `UdpSocket`. Key difference: UDP is datagram-based, so `read` returns one datagram, `is_streamed()` returns `false`.

- [ ] **Step 3: Rewrite `mcast.rs` — replace `async_net::UdpSocket` with `std::net::UdpSocket`**

Same as UDP but with multicast join options (`IP_MULTICAST_LOOP`, `IP_ADD_MEMBERSHIP`).

- [ ] **Step 4: Rewrite `serial.rs` — already mostly sync, just remove any async wrappers**

The serial implementation likely already uses `std::io::serial` or termios. Just ensure `ZLinkTx`/`ZLinkRx` impls don't use `async fn`.

- [ ] **Step 5: Rewrite `tls.rs` — replace `async_native_tls` with `native_tls`, remove tokio**

```rust
// platforms/zenoh-std/src/tls.rs

use std::net::SocketAddr;
use std::sync::Arc;
use native_tls::{TlsConnector, TlsStream};
use std::net::TcpStream;
use std::io::{Read, Write};
use zenoh_nostd::platform::*;

pub struct StdTlsLink {
    stream: TlsStream<TcpStream>,
    mtu: u16,
}

impl StdTlsLink {
    pub fn connect(addr: SocketAddr, domain: &str) -> Result<Self, LinkError> {
        let tcp = TcpStream::connect(addr).map_err(|_| LinkError::CouldNotConnect)?;
        tcp.set_nodelay(true).map_err(|_| LinkError::CouldNotConnect)?;
        let connector = TlsConnector::default().build().map_err(|_| LinkError::CouldNotConnect)?;
        let stream = connector.connect(domain, tcp).map_err(|_| LinkError::CouldNotConnect)?;
        Ok(Self { stream, mtu: u16::MAX - 60 })
    }
}

impl ZLinkInfo for StdTlsLink {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for StdTlsLink {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        self.stream.write_all(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for StdTlsLink {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        self.stream.read(buffer).map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        self.stream.read_exact(buffer).map_err(|_| LinkError::LinkRxFailed)
    }
}

impl ZLink for StdTlsLink {
    type Tx<'link> = StdTlsLinkTx where Self: 'link;
    type Rx<'link> = StdTlsLinkRx where Self: 'link;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        let mtu = self.mtu;
        (StdTlsLinkTx { stream: &mut self.stream, mtu }, StdTlsLinkRx { stream: &mut self.stream, mtu })
    }
}

pub struct StdTlsLinkTx<'a> { stream: &'a mut TlsStream<TcpStream>, mtu: u16 }
pub struct StdTlsLinkRx<'a> { stream: &'a mut TlsStream<TcpStream>, mtu: u16 }

impl ZLinkInfo for StdTlsLinkTx<'_> {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for StdTlsLinkTx<'_> {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        self.stream.write_all(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkInfo for StdTlsLinkRx<'_> {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkRx for StdTlsLinkRx<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        self.stream.read(buffer).map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        self.stream.read_exact(buffer).map_err(|_| LinkError::LinkRxFailed)
    }
}
```

- [ ] **Step 6: Rewrite `bt.rs` — sync BlueZ D-Bus or rfcomm calls**

Replace async BlueZ calls with sync equivalents. If using rfcomm, use `std::net::TcpStream`-like API from `bluer` (sync) or raw `/dev/rfcomm*` file I/O.

- [ ] **Step 7: Rewrite `ws.rs` — replace async WebSocket with sync tungstenite**

```rust
// platforms/zenoh-std/src/ws.rs

use std::net::TcpStream;
use std::io::{Read, Write};
use tungstenite::{WebSocket, Message, stream::MaybeTlsStream};
use url::Url;
use zenoh_nostd::platform::*;
```

- [ ] **Step 8: Update `platforms/zenoh-std/src/lib.rs`**

Remove any async wrappers. Ensure `StdLinkManager` methods are sync.

- [ ] **Step 9: Run test to verify it compiles**

Run: `cargo check -p zenoh-nostd --features std`
Expected: clean compilation.

- [ ] **Step 10: Commit**

```bash
git add platforms/zenoh-std/
git commit -m "refactor(platform-std): rewrite all link implementations as synchronous (tcp, udp, mcast, serial, tls, bt, ws)"
```

---

### Task 6: Remove async dependencies from Cargo.toml

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `crates/zenoh-nostd/Cargo.toml`
- Modify: `platforms/zenoh-std/Cargo.toml`

- [ ] **Step 1: Remove embassy deps from workspace `Cargo.toml`**

Remove:
```toml
embassy-futures = "0.1.2"
embassy-time = "0.5.0"
embassy-sync = "0.7.2"
embassy-executor = "0.9.1"
```

- [ ] **Step 2: Remove embassy deps from `crates/zenoh-nostd/Cargo.toml`**

Remove:
```toml
embassy-futures = { workspace = true }
embassy-executor = { workspace = true }
embassy-time = { workspace = true }
embassy-sync = { workspace = true }
```

Remove `platform-embassy` feature flag if present.

- [ ] **Step 3: Remove tokio, async-net, async-native-tls, futures-lite from `platforms/zenoh-std/Cargo.toml`**

Remove:
```toml
tokio = { workspace = true }
async-net = { workspace = true }
async-native-tls = { workspace = true }
futures-lite = { workspace = true }
```

Add:
```toml
native-tls = { workspace = true }
tungstenite = { workspace = true }
```

- [ ] **Step 4: Remove `zenoh-embassy` from workspace members/exclude**

Update `Cargo.toml`:
```toml
exclude = ["platforms/zenoh-std", "platforms/zenoh-wasm", "platforms/zenoh-stm32"]
```

- [ ] **Step 5: Run test to verify it compiles**

Run: `cargo check -p zenoh-nostd --features std`
Expected: clean compilation.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/zenoh-nostd/Cargo.toml platforms/zenoh-std/Cargo.toml
git commit -m "chore: remove embassy, tokio, async-net, async-native-tls deps; add native-tls, tungstenite"
```

---

### Task 7: Delete `platforms/zenoh-embassy/` crate

**Files:**
- Delete: `platforms/zenoh-embassy/` (entire directory)

- [ ] **Step 1: Delete the directory**

Run: `rm -rf platforms/zenoh-embassy/`

- [ ] **Step 2: Verify no remaining references**

Run: `grep -rn "zenoh-embassy\|platform-embassy" crates/ platforms/ Cargo.toml`
Expected: no matches.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "chore: remove zenoh-embassy platform crate"
```

---

### Task 8: Update tests and examples

**Files:**
- Modify: All test files using `.await`
- Modify: All example files using `.await`

- [ ] **Step 1: Find all `.await` usages in tests**

Run: `grep -rn "\.await" crates/zenoh-nostd/tests/ platforms/zenoh-std/tests/ examples/ --include="*.rs"`

- [ ] **Step 2: Remove `.await` from all test files**

Replace `foo().await` with `foo()`. Replace `async fn test_name` with `fn test_name`. Remove `#[tokio::test]` / `#[embassy_executor::test]` with `#[test]`.

- [ ] **Step 3: Remove `.await` from all example files**

Same pattern.

- [ ] **Step 4: Run tests**

Run: `cargo test -p zenoh-nostd --features std`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "test: remove .await from all tests and examples, make test harnesses synchronous"
```

---

### Task 9: Update documentation

**Files:**
- Modify: `docs/feature_support_matrix.md`
- Modify: `docs/no_std_constraints.md`
- Modify: `docs/protocol_compatibility.md`
- Modify: `README.md`

- [ ] **Step 1: Update feature support matrix**

Remove `platform-embassy` row. Add note: "All async removed — fully synchronous poll-driven IO."

- [ ] **Step 2: Update no_std constraints**

Document: "zenoh-nostd is fully synchronous. No async runtime required. IO is driven by `Session::poll()` in the application's main loop."

- [ ] **Step 3: Update protocol compatibility**

Remove references to Embassy-specific limitations.

- [ ] **Step 4: Update README**

Remove Embassy from supported platforms. Add `Session::poll()` to the quickstart example.

- [ ] **Step 5: Commit**

```bash
git add docs/ README.md
git commit -m "docs: update for synchronous poll-driven architecture, remove embassy references"
```

---

### Task 10: Final verification

- [ ] **Step 1: Full feature check**

Run: `cargo check -p zenoh-nostd --no-default-features`
Expected: clean.

Run: `cargo check -p zenoh-nostd --features std`
Expected: clean.

Run: `cargo check -p zenoh-nostd --features std,acl,matching,connectivity,ring_channels,scouting,publisher_detection,background_subscriber,bytes,serialization,cancellation_token,encoding_ext,publisher_id,query_reply_ext,advanced_subscriber`
Expected: clean.

- [ ] **Step 2: Clippy**

Run: `cargo clippy -p zenoh-nostd --features std -- -D warnings`
Expected: clean.

- [ ] **Step 3: Tests**

Run: `cargo test -p zenoh-nostd --features std`
Expected: all pass.

- [ ] **Step 4: Verify no remaining async in core crate**

Run: `grep -rn "\.await\|async fn\|impl Future\|embassy_\|tokio" crates/zenoh-nostd/src/ --include="*.rs"`
Expected: no matches (or only in comments).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: final verification — all async removed, all features compile, all tests pass"
```