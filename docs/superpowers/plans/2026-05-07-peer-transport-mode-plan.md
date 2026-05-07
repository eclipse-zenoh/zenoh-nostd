# Peer Transport Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `WhatAmI::Peer` transport mode with simultaneous-open handling so zenoh-nostd nodes can connect directly without a router.

**Architecture:** Thread `WhatAmI` as a configuration value from `TransportLinkManager` → `TransportBuilder` → `InitIdentifier`. Add simultaneous-open ZID-comparison logic to the establishment state machine. No new traits — the existing `ZLinkManager`/`ZSessionConfig` trait stack is unchanged.

**Tech Stack:** Rust (edition 2024), zenoh-sansio crate, zenoh-nostd crate, embassy-executor (examples)

---

## File Map

| File | Role |
|------|------|
| `crates/zenoh-sansio/src/transport.rs` | Add `whatami` field + `with_whatami()` to `TransportBuilder`. Pass `whatami` into `InitIdentifier` and `State` variants. |
| `crates/zenoh-sansio/src/transport/establishment.rs` | Add `mine_whatami` to state variants. Fix `InitAck` default. Add simultaneous-open logic. |
| `crates/zenoh-sansio/src/tests/transport.rs` | Update existing test. Add peer handshake test + simultaneous-open tests. |
| `crates/zenoh-nostd/src/io/transport.rs` | Add `whatami` field + `with_whatami()` to `TransportLinkManager`. Wire through all connect/listen methods. |
| `examples/z_peer.rs` | Example: two peers talking directly, no router. |

---

### Task 1: Add `whatami` to `TransportBuilder`

**Files:**
- Modify: `crates/zenoh-sansio/src/transport.rs`

- [ ] **Step 1: Add `whatami` field and `with_whatami()` method to `TransportBuilder`**

Replace the `TransportBuilder` struct definition (line 21-28):

```rust
pub struct TransportBuilder<Buff> {
    zid: ZenohIdProto,
    whatami: WhatAmI,
    batch_size: u16,
    lease: Duration,
    resolution: Resolution,

    buff: Buff,
}
```

In `TransportBuilder::new()`, add default initialization after `zid` (insert at line 37):

```rust
fn new(buff: Buff) -> Self
where
    Buff: AsRef<[u8]>,
{
    TransportBuilder {
        zid: ZenohIdProto::default(),
        whatami: WhatAmI::default(),
        batch_size: buff.as_ref().len() as u16,
        lease: Duration::from_secs(10),
        resolution: Resolution::default(),
        buff,
    }
}
```

Add the builder method after `with_zid` (after line 46):

```rust
pub fn with_whatami(mut self, whatami: WhatAmI) -> Self {
    self.whatami = whatami;
    self
}
```

- [ ] **Step 2: Pass `whatami` into `InitIdentifier` in `connect()`**

In `connect()` (line 226-229), replace:

```rust
init: InitSyn {
    identifier: InitIdentifier {
        zid: self.zid,
        ..Default::default()
    },
```

with:

```rust
init: InitSyn {
    identifier: InitIdentifier {
        zid: self.zid,
        whatami: self.whatami,
        ..Default::default()
    },
```

Same change in `connect_async()` (line 283-286):

```rust
init: InitSyn {
    identifier: InitIdentifier {
        zid: self.zid,
        whatami: self.whatami,
        ..Default::default()
    },
```

- [ ] **Step 3: Pass `mine_whatami` into `State` variants in all four connect/listen methods**

In `connect()` (line 201-206), insert `mine_whatami: self.whatami,`:

```rust
let state = State::WaitingInitAck {
    mine_zid: self.zid,
    mine_whatami: self.whatami,
    mine_batch_size: self.batch_size,
    mine_resolution: self.resolution,
    mine_lease: self.lease,
};
```

Same in `connect_async()` (line 258-263):

```rust
let state = State::WaitingInitAck {
    mine_zid: self.zid,
    mine_whatami: self.whatami,
    mine_batch_size: self.batch_size,
    mine_resolution: self.resolution,
    mine_lease: self.lease,
};
```

In `listen()` (line 109-114), insert `mine_whatami: self.whatami,`:

```rust
let state = State::WaitingInitSyn {
    mine_zid: self.zid,
    mine_whatami: self.whatami,
    mine_batch_size: self.batch_size,
    mine_resolution: self.resolution,
    mine_lease: self.lease,
};
```

Same in `listen_async()` (line 155-160):

```rust
let state = State::WaitingInitSyn {
    mine_zid: self.zid,
    mine_whatami: self.whatami,
    mine_batch_size: self.batch_size,
    mine_resolution: self.resolution,
    mine_lease: self.lease,
};
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check -p zenoh-sansio
```

Expected: build errors about missing `mine_whatami` in `establishment.rs` (will fix next task).

- [ ] **Step 5: Commit**

```bash
git add crates/zenoh-sansio/src/transport.rs
git commit -m "feat(sansio): add whatami to TransportBuilder, pass to InitIdentifier and State"
```

---

### Task 2: Update establishment State variants + fix InitAck default

**Files:**
- Modify: `crates/zenoh-sansio/src/transport/establishment.rs`

- [ ] **Step 1: Add `mine_whatami` to `WaitingInitSyn`, `WaitingOpenSyn`, `WaitingInitAck`**

Replace the three state variants (lines 29-72):

```rust
pub(crate) enum State {
    WaitingInitSyn {
        /// Mine zid
        mine_zid: ZenohIdProto,
        /// Mine whatami
        mine_whatami: WhatAmI,
        /// Mine startup batch_size
        mine_batch_size: u16,
        /// Mine startup resolution
        mine_resolution: Resolution,
        /// Mine lease,
        mine_lease: Duration,
    },
    WaitingOpenSyn {
        /// Mine zid
        mine_zid: ZenohIdProto,
        /// Mine whatami
        mine_whatami: WhatAmI,
        /// Mine startup batch_size
        mine_batch_size: u16,
        /// Mine startup resolution
        mine_resolution: Resolution,
        /// Mine lease,
        mine_lease: Duration,
    },
    WaitingInitAck {
        /// Mine zid
        mine_zid: ZenohIdProto,
        /// Mine whatami
        mine_whatami: WhatAmI,
        /// Mine startup batch_size
        mine_batch_size: u16,
        /// Mine startup resolution
        mine_resolution: Resolution,
        /// Mine lease,
        mine_lease: Duration,
    },
    WaitingOpenAck {
        // unchanged
        ...
    },
    Opened(Description),
}
```

- [ ] **Step 2: Fix `InitAck` identifier in `WaitingInitSyn` handler to use `mine_whatami`**

In the `WaitingInitSyn` handler (lines 90-125), replace the `InitAck` construction:

Current lines 110-114:
```rust
Some(TransportMessage::InitAck(InitAck {
    identifier: InitIdentifier {
        zid: mine_zid,
        ..Default::default()
    },
```

Replace with:
```rust
Some(TransportMessage::InitAck(InitAck {
    identifier: InitIdentifier {
        zid: mine_zid,
        whatami: mine_whatami,
        ..Default::default()
    },
```

- [ ] **Step 3: Update the `WaitingInitSyn` match arm destructuring to include `mine_whatami`**

Lines 91-96, replace:
```rust
Self::WaitingInitSyn {
    mine_zid,
    mine_batch_size,
    mine_resolution,
    mine_lease,
} => {
```

with:
```rust
Self::WaitingInitSyn {
    mine_zid,
    mine_whatami,
    mine_batch_size,
    mine_resolution,
    mine_lease,
} => {
```

And update the `*self = Self::WaitingOpenSyn { ... }` (line 103-108) to include `mine_whatami`:

```rust
*self = Self::WaitingOpenSyn {
    mine_zid,
    mine_whatami,
    mine_batch_size,
    mine_resolution,
    mine_lease,
};
```

- [ ] **Step 4: Update the `InitAck` handler destructuring to include `mine_whatami`**

Lines 130-135, replace:
```rust
Self::WaitingInitAck {
    mine_zid,
    mine_batch_size,
    mine_resolution,
    mine_lease,
} => {
```

with:
```rust
Self::WaitingInitAck {
    mine_zid,
    mine_whatami: _,
    mine_batch_size,
    mine_resolution,
    mine_lease,
} => {
```

(`mine_whatami` is unused in this arm — prefix with `_` prefix convention won't work with destructuring, use `_:` or just `mine_whatami` which gets a warning. Use `_mine_whatami` or approach differently. Actually in Rust, you can just name it and not use it, but we get a warning. Let's use `let _ = mine_whatami;` or better, just prefix with underscore in the pattern.)

Actually wait, looking at this more carefully, we CAN use `_` prefixes in struct destructuring:

```rust
Self::WaitingInitAck {
    mine_zid,
    mine_whatami: _,
    mine_batch_size,
    mine_resolution,
    mine_lease,
}
```

This should work — `_:` syntax suppresses the unused warning.

- [ ] **Step 5: Update the `OpenSyn` handler destructuring to include `mine_whatami`**

Lines 198-203, replace:
```rust
Self::WaitingOpenSyn {
    mine_zid,
    mine_batch_size,
    mine_resolution,
    mine_lease,
} => {
```

with:
```rust
Self::WaitingOpenSyn {
    mine_zid,
    mine_whatami: _,
    mine_batch_size,
    mine_resolution,
    mine_lease,
} => {
```

- [ ] **Step 6: Verify compilation**

```bash
cargo check -p zenoh-sansio
```

Expected: compilation succeeds (no more errors about missing `mine_whatami`).

- [ ] **Step 7: Commit**

```bash
git add crates/zenoh-sansio/src/transport/establishment.rs
git commit -m "feat(sansio): add mine_whatami to establishment states, fix InitAck whatami"
```

---

### Task 3: Add simultaneous-open logic to `State::poll()`

**Files:**
- Modify: `crates/zenoh-sansio/src/transport/establishment.rs`

- [ ] **Step 1: Add `InitSyn` handling in the `WaitingInitAck` state**

In `State::poll()`, the `TransportMessage::InitSyn(syn)` match arm (line 90). Currently it only handles `WaitingInitSyn`. Add handling for `WaitingInitAck` BEFORE the `_ =>` catch-all:

Replace the entire `TransportMessage::InitSyn(syn)` match arm (lines 90-127):

```rust
TransportMessage::InitSyn(syn) => match *self {
    Self::WaitingInitSyn {
        mine_zid,
        mine_whatami,
        mine_batch_size,
        mine_resolution,
        mine_lease,
    } => {
        zenoh_proto::debug!(
            "Received InitSyn on transport {:?} -> NEW!({:?})",
            mine_zid,
            syn.identifier.zid
        );

        *self = Self::WaitingOpenSyn {
            mine_zid,
            mine_whatami,
            mine_batch_size,
            mine_resolution,
            mine_lease,
        };

        (
            Some(TransportMessage::InitAck(InitAck {
                identifier: InitIdentifier {
                    zid: mine_zid,
                    whatami: mine_whatami,
                    ..Default::default()
                },
                resolution: InitResolution {
                    resolution: mine_resolution,
                    batch_size: BatchSize(mine_batch_size),
                },
                cookie: buff, // TODO: cypher ChaCha20
                ..Default::default()
            })),
            None,
        )
    }
    Self::WaitingInitAck {
        mine_zid,
        mine_whatami,
        mine_batch_size,
        mine_resolution,
        mine_lease,
    } => {
        if mine_zid > syn.identifier.zid {
            zenoh_proto::debug!(
                "Simultaneous open: {:?} yields to {:?} (higher ZID)",
                mine_zid,
                syn.identifier.zid
            );

            *self = Self::WaitingOpenSyn {
                mine_zid,
                mine_whatami,
                mine_batch_size,
                mine_resolution,
                mine_lease,
            };

            (
                Some(TransportMessage::InitAck(InitAck {
                    identifier: InitIdentifier {
                        zid: mine_zid,
                        whatami: mine_whatami,
                        ..Default::default()
                    },
                    resolution: InitResolution {
                        resolution: mine_resolution,
                        batch_size: BatchSize(mine_batch_size),
                    },
                    cookie: buff, // TODO: cypher ChaCha20
                    ..Default::default()
                })),
                None,
            )
        } else if mine_zid < syn.identifier.zid {
            let _ = mine_whatami;
            zenoh_proto::debug!(
                "Simultaneous open: {:?} wins over {:?} (lower ZID)",
                mine_zid,
                syn.identifier.zid
            );

            (None, None)
        } else {
            zenoh_proto::zbail!(@ret (None, None), TransportError::InvalidAttribute)
        }
    }
    _ => zenoh_proto::zbail!(@ret (None, None), TransportError::InvalidState),
},
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check -p zenoh-sansio
```

Expected: compile succeeds.

- [ ] **Step 3: Commit**

```bash
git add crates/zenoh-sansio/src/transport/establishment.rs
git commit -m "feat(sansio): add simultaneous-open handling for peer mode"
```

---

### Task 4: Update existing test + add peer transport tests

**Files:**
- Modify: `crates/zenoh-sansio/src/tests/transport.rs`

- [ ] **Step 1: Update `transport_state_handshake` test to include `mine_whatami`**

The test constructs `State` variants directly (lines 6-66). Add `mine_whatami: WhatAmI::Client` to both states:

Line 8-13, replace:
```rust
let mut a = State::WaitingInitSyn {
    mine_zid: a_zid,
    mine_batch_size: 512,
    mine_resolution: Resolution::default(),
    mine_lease: Duration::from_secs(30),
};
```

with:
```rust
let mut a = State::WaitingInitSyn {
    mine_zid: a_zid,
    mine_whatami: WhatAmI::Client,
    mine_batch_size: 512,
    mine_resolution: Resolution::default(),
    mine_lease: Duration::from_secs(30),
};
```

Line 16-21, replace:
```rust
let mut b = State::WaitingInitAck {
    mine_zid: b_zid,
    mine_batch_size: 1025,
    mine_resolution: Resolution::default(),
    mine_lease: Duration::from_secs(37),
};
```

with:
```rust
let mut b = State::WaitingInitAck {
    mine_zid: b_zid,
    mine_whatami: WhatAmI::Client,
    mine_batch_size: 1025,
    mine_resolution: Resolution::default(),
    mine_lease: Duration::from_secs(37),
};
```

- [ ] **Step 2: Run the existing test to verify it still passes**

```bash
cargo test -p zenoh-sansio transport_state_handshake
```

Expected: PASS.

- [ ] **Step 3: Add `transport_peer_handshake` test**

Add after the `transport_state_handshake` test closing brace (after line 66):

```rust
#[test]
fn transport_peer_handshake() {
    let socket = ([0u8; 512], 0usize, 0usize);
    let socket_ref = RefCell::new(socket);

    let a = Transport::builder([0u8; 512]).with_whatami(WhatAmI::Peer);
    let b = Transport::builder([0u8; 512]).with_whatami(WhatAmI::Peer);

    let read = |socket: &mut &RefCell<([u8; 512], usize, usize)>,
                bytes: &mut [u8]|
     -> core::result::Result<usize, i32> {
        let mut borrow_mut = socket.borrow_mut();

        let to_read = bytes.len().min(borrow_mut.2);

        let slice = &borrow_mut.0[borrow_mut.1..(to_read + borrow_mut.1)];
        bytes[..slice.len()].copy_from_slice(slice);
        borrow_mut.1 += to_read;

        Ok(to_read)
    };

    let write = |socket: &mut &RefCell<([u8; 512], usize, usize)>,
                 bytes: &[u8]|
     -> core::result::Result<(), i32> {
        let mut borrow_mut = socket.borrow_mut();
        borrow_mut.0[..bytes.len()].copy_from_slice(bytes);
        borrow_mut.1 = 0;
        borrow_mut.2 = bytes.len();
        Ok(())
    };

    let mut ha = a.listen(&socket_ref, &read, &write);
    let mut hb = b.connect(&socket_ref, &read, &write);

    hb.poll().unwrap();

    for _ in 0..2 {
        ha.poll().unwrap();
        hb.poll().unwrap();
    }

    let ta = ha
        .poll()
        .expect("Unexpected Error")
        .expect("Transport A is not opened yet")
        .open();

    let tb = hb
        .poll()
        .expect("Unexpected Error")
        .expect("Transport B is not opened yet")
        .open();

    assert_eq!(ta.mine_zid, tb.other_zid);
    assert_eq!(ta.other_zid, tb.mine_zid);
}
```

- [ ] **Step 4: Run peer handshake test**

```bash
cargo test -p zenoh-sansio transport_peer_handshake
```

Expected: PASS.

- [ ] **Step 5: Add `transport_peer_simultaneous_connect_lower_wins` test**

Add after the `transport_peer_handshake` test:

```rust
#[test]
fn transport_peer_simultaneous_connect_lower_wins() {
    let socket = ([0u8; 512], 0usize, 0usize);
    let socket_ref = RefCell::new(socket);

    let a = Transport::builder([0u8; 512]).with_whatami(WhatAmI::Peer);
    let b = Transport::builder([0u8; 512]).with_whatami(WhatAmI::Peer);

    let read = |socket: &mut &RefCell<([u8; 512], usize, usize)>,
                bytes: &mut [u8]|
     -> core::result::Result<usize, i32> {
        let mut borrow_mut = socket.borrow_mut();
        let to_read = bytes.len().min(borrow_mut.2);
        let slice = &borrow_mut.0[borrow_mut.1..(to_read + borrow_mut.1)];
        bytes[..slice.len()].copy_from_slice(slice);
        borrow_mut.1 += to_read;
        Ok(to_read)
    };

    let write = |socket: &mut &RefCell<([u8; 512], usize, usize)>,
                 bytes: &[u8]|
     -> core::result::Result<(), i32> {
        let mut borrow_mut = socket.borrow_mut();
        borrow_mut.0[..bytes.len()].copy_from_slice(bytes);
        borrow_mut.1 = 0;
        borrow_mut.2 = bytes.len();
        Ok(())
    };

    let mut ha = a.connect(&socket_ref, &read, &write);
    let mut hb = b.connect(&socket_ref, &read, &write);

    ha.poll().unwrap();
    hb.poll().unwrap();

    for _ in 0..5 {
        ha.poll().unwrap();
        hb.poll().unwrap();
    }

    let ta = ha
        .poll()
        .expect("Unexpected Error")
        .expect("Transport A is not opened yet")
        .open();

    let tb = hb
        .poll()
        .expect("Unexpected Error")
        .expect("Transport B is not opened yet")
        .open();

    assert_eq!(ta.mine_zid, tb.other_zid);
    assert_eq!(ta.other_zid, tb.mine_zid);
}
```

- [ ] **Step 6: Run simultaneous-connect test**

```bash
cargo test -p zenoh-sansio transport_peer_simultaneous_connect_lower_wins
```

Expected: PASS (both sides connect simultaneously, ZID comparison resolves).

- [ ] **Step 7: Run all zenoh-sansio tests**

```bash
cargo test -p zenoh-sansio
```

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add crates/zenoh-sansio/src/tests/transport.rs
git commit -m "test(sansio): add peer handshake and simultaneous-connect tests"
```

---

### Task 5: Wire `whatami` through `TransportLinkManager`

**Files:**
- Modify: `crates/zenoh-nostd/src/io/transport.rs`

- [ ] **Step 1: Add `whatami` field and `with_whatami()` to `TransportLinkManager`**

Replace the struct definition (lines 79-86):

```rust
pub struct TransportLinkManager<LinkManager> {
    link_manager: LinkManager,

    open_timeout: Duration,
    zid: ZenohIdProto,
    whatami: WhatAmI,
    lease: Duration,
    resolution: Resolution,
}
```

In the `From<LinkManager>` impl (lines 88-98), add `whatami: WhatAmI::default()`:

```rust
impl<LinkManager> From<LinkManager> for TransportLinkManager<LinkManager> {
    fn from(value: LinkManager) -> Self {
        Self::new(
            value,
            Duration::from_secs(10),
            ZenohIdProto::default(),
            WhatAmI::default(),
            Duration::from_secs(10),
            Resolution::default(),
        )
    }
}
```

Update `Self::new` signature and body (lines 100-115):

```rust
impl<LinkManager> TransportLinkManager<LinkManager> {
    pub(crate) fn new(
        link_manager: LinkManager,
        open_timeout: Duration,
        zid: ZenohIdProto,
        whatami: WhatAmI,
        lease: Duration,
        resolution: Resolution,
    ) -> Self {
        Self {
            link_manager,
            open_timeout,
            zid,
            whatami,
            lease,
            resolution,
        }
    }
```

Add `with_whatami` builder method after the `new` block:

```rust
    pub fn with_whatami(mut self, whatami: WhatAmI) -> Self {
        self.whatami = whatami;
        self
    }
```

- [ ] **Step 2: Add import for `WhatAmI`**

At the top of the file (line 5-8), update the zenoh_proto import:

```rust
use zenoh_proto::{
    Endpoint, TransportLinkError,
    fields::{Resolution, WhatAmI, ZenohIdProto},
};
```

- [ ] **Step 3: Wire `.with_whatami(self.whatami)` into `bridge_connect()`**

In `bridge_connect()` (line 128-132), insert `.with_whatami(self.whatami)` after `.with_resolution(self.resolution)`:

```rust
Transport::builder(buff)
    .with_zid(self.zid)
    .with_whatami(self.whatami)
    .with_lease(self.lease)
    .with_resolution(self.resolution)
    .connect_async(
```

- [ ] **Step 4: Wire `.with_whatami(self.whatami)` into `bridge_listen()`**

In `bridge_listen()` (line 167-171), same insertion:

```rust
Transport::builder(buff)
    .with_zid(self.zid)
    .with_whatami(self.whatami)
    .with_lease(self.lease)
    .with_resolution(self.resolution)
    .listen_async(
```

- [ ] **Step 5: Wire `.with_whatami(self.whatami)` into `connect()`**

In `connect()` (line 208-212), same insertion:

```rust
Transport::builder(buff)
    .with_zid(self.zid)
    .with_whatami(self.whatami)
    .with_lease(self.lease)
    .with_resolution(self.resolution)
    .connect_async(
```

- [ ] **Step 6: Wire `.with_whatami(self.whatami)` into `listen()`**

In `listen()` (line 247-251), same insertion:

```rust
Transport::builder(buff)
    .with_zid(self.zid)
    .with_whatami(self.whatami)
    .with_lease(self.lease)
    .with_resolution(self.resolution)
    .listen_async(
```

- [ ] **Step 7: Verify compilation**

```bash
cargo check -p zenoh-nostd
```

Expected: compile succeeds.

- [ ] **Step 8: Commit**

```bash
git add crates/zenoh-nostd/src/io/transport.rs
git commit -m "feat(nostd): wire whatami through TransportLinkManager to all connect/listen methods"
```

---

### Task 6: Write `z_peer.rs` example

**Files:**
- Create: `examples/z_peer.rs`

- [ ] **Step 1: Create `z_peer.rs`**

```rust
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

#[cfg(feature = "std")]
use zenoh_examples::*;
#[cfg(feature = "std")]
use zenoh_nostd::session::*;
#[cfg(feature = "std")]
use zenoh_proto::fields::WhatAmI;

#[cfg(feature = "std")]
struct PeerConfig {
    transports: TransportLinkManager<LinkManager>,
}

#[cfg(feature = "std")]
impl ZSessionConfig for PeerConfig {
    type LinkManager = LinkManager;
    type Buff = [u8; u16::MAX as usize];

    type SubCallbacks<'res> = FixedCapacitySubCallbacks<
        'res,
        8,
        zenoh::storage::RawOrBox<56>,
        zenoh::storage::RawOrBox<600>,
    >;
    type GetCallbacks<'res> = FixedCapacityGetCallbacks<
        'res,
        8,
        zenoh::storage::RawOrBox<1>,
        zenoh::storage::RawOrBox<32>,
    >;
    type QueryableCallbacks<'res> = FixedCapacityQueryableCallbacks<
        'res,
        Self,
        8,
        zenoh::storage::RawOrBox<32>,
        zenoh::storage::RawOrBox<952>,
    >;

    fn buff(&self) -> Self::Buff {
        [0u8; u16::MAX as usize]
    }

    fn transports(&self) -> &TransportLinkManager<Self::LinkManager> {
        &self.transports
    }
}

#[cfg(feature = "std")]
fn peer_config() -> PeerConfig {
    PeerConfig {
        transports: TransportLinkManager::from(LinkManager).with_whatami(WhatAmI::Peer),
    }
}

#[cfg(feature = "std")]
#[embassy_executor::task]
async fn session_task(session: &'static Session<'static, PeerConfig>) {
    if let Err(e) = session.run().await {
        zenoh::error!("Error in session task: {}", e);
    }
}

#[cfg(feature = "std")]
async fn entry(spawner: embassy_executor::Spawner) -> zenoh::ZResult<()> {
    env_logger::init();

    zenoh::info!("zenoh-nostd z_peer example");

    let listen_ep = match option_env!("LISTEN") {
        Some(_) => Endpoint::try_from("tcp/127.0.0.1:7444")?,
        None => Endpoint::try_from("tcp/127.0.0.1:7445")?,
    };

    let config = peer_config();

    let session = if option_env!("LISTEN").is_some() {
        zenoh::listen!(PeerConfig: config, listen_ep)
    } else {
        let connect_ep = Endpoint::try_from("tcp/127.0.0.1:7444")?;
        zenoh::connect!(PeerConfig: config, connect_ep)
    };

    spawner.spawn(session_task(session)).unwrap();

    let ke = zenoh::keyexpr::new("demo/example")?;

    if option_env!("LISTEN").is_some() {
        let subscriber = session
            .declare_subscriber(ke)
            .callback_sync(|sample| {
                zenoh::info!(
                    "[Peer listener] Received: {:?}",
                    core::str::from_utf8(sample.payload()).unwrap()
                );
            })
            .finish()
            .await?;

        loop {
            embassy_time::Timer::after(embassy_time::Duration::from_secs(1)).await;
        }
    } else {
        let publisher = session
            .declare_publisher(ke)
            .finish()
            .await?;

        let payload = b"Hello from peer!";
        loop {
            publisher.put(payload).finish().await?;
            zenoh::info!(
                "[Peer connector] Sent PUT ('{}': '{}')",
                publisher.keyexpr().as_str(),
                core::str::from_utf8(payload).unwrap()
            );
            embassy_time::Timer::after(embassy_time::Duration::from_secs(1)).await;
        }
    }
}

#[cfg(feature = "std")]
#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) {
    if let Err(e) = entry(spawner).await {
        zenoh::error!("Error in main: {}", e);
    }
    zenoh::info!("Exiting main");
}

#[cfg(not(feature = "std"))]
fn main() {
    panic!("z_peer only supports std feature");
}
```

- [ ] **Step 2: Verify compilation of example**

```bash
cargo check --example z_peer --features std
```

Expected: compile succeeds.

- [ ] **Step 3: Commit**

```bash
git add examples/z_peer.rs
git commit -m "example: add z_peer demonstrating WhatAmI::Peer mode"
```

---

### Task 7: Full verification

**Files:**
- None (verification only)

- [ ] **Step 1: Run all zenoh-sansio tests**

```bash
cargo test -p zenoh-sansio
```

Expected: all pass.

- [ ] **Step 2: Run all zenoh-proto tests**

```bash
cargo test -p zenoh-proto
```

Expected: all pass.

- [ ] **Step 3: Run clippy on zenoh-sansio**

```bash
cargo clippy -p zenoh-sansio
```

Expected: no warnings.

- [ ] **Step 4: Run clippy on zenoh-nostd**

```bash
cargo clippy -p zenoh-nostd
```

Expected: no warnings.

- [ ] **Step 5: Run end-to-end peer test manually**

Terminal 1:
```bash
cargo run --example z_peer --features std
```

Terminal 2:
```bash
LISTEN=1 cargo run --example z_peer --features std
```

Expected: peer connector publishes, peer listener receives.

- [ ] **Step 6: Commit if any fixes needed**

```bash
git add -A
git commit -m "fix: clippy and test fixes for peer mode"
```
