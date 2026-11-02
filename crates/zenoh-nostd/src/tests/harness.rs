//! Test harness: an in-memory link and a scripted fake router.
//!
//! [`run`] connects a [`Session`] to a [`Router`] over a pair of in-memory
//! datagram pipes, then drives the test body, the session run loop and the
//! router run loop concurrently. The router behaves like `zenohd` in one
//! important respect: it never echoes a message back to the session that sent
//! it. Everything else the router does is up to the test.

use core::{cell::Cell, future::Future};
use std::{boxed::Box, format, string::String, vec::Vec};

use embassy_futures::select::{Either3, select3};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel, mutex::Mutex};
use embassy_time::{Duration, Instant, Timer, with_timeout};
use zenoh_proto::{
    Endpoint, LinkError, SessionError,
    exts::QoS,
    fields::{Reliability, WireExpr},
    msgs::*,
};

use crate::{
    api::{
        callbacks::storage::RawOrBox,
        session::{Session, SessionBuilder},
    },
    config::ZSessionConfig,
    io::{
        driver::Driver,
        transport::{TransportLinkManager, ZTransportLinkTx},
    },
    platform::{ZLink, ZLinkInfo, ZLinkManager, ZLinkRx, ZLinkTx},
    resources::Resources,
    session::{
        FixedCapacityGetCallbacks, FixedCapacityQueryableCallbacks, FixedCapacitySubCallbacks,
    },
};

/// Size of the transport buffers on both sides.
pub const BUFF: usize = 4096;
/// Upper bound for a whole test, so a hang fails instead of blocking CI.
pub const TEST_TIMEOUT: Duration = Duration::from_secs(5);
/// How long [`Router::wait_for`] waits for a matching message.
pub const WAIT: Duration = Duration::from_secs(1);

// ── In-memory datagram link ─────────────────────────────────────────────────

type Pipe = Channel<NoopRawMutex, Vec<u8>, 64>;

#[derive(Clone, Copy)]
pub struct MemLink {
    tx: &'static Pipe,
    rx: &'static Pipe,
}

pub struct MemLinkTx {
    tx: &'static Pipe,
}

pub struct MemLinkRx {
    rx: &'static Pipe,
}

macro_rules! impl_info {
    ($($t:ty),*) => {$(
        impl ZLinkInfo for $t {
            fn mtu(&self) -> u16 {
                BUFF as u16
            }

            fn is_streamed(&self) -> bool {
                false
            }
        }
    )*};
}

impl_info!(MemLink, MemLinkTx, MemLinkRx);

async fn pipe_write(pipe: &Pipe, buffer: &[u8]) -> Result<(), LinkError> {
    pipe.send(buffer.to_vec()).await;
    Ok(())
}

async fn pipe_read(pipe: &Pipe, buffer: &mut [u8]) -> Result<usize, LinkError> {
    let msg = pipe.receive().await;
    if msg.len() > buffer.len() {
        return Err(LinkError::LinkRxFailed);
    }
    buffer[..msg.len()].copy_from_slice(&msg);
    Ok(msg.len())
}

impl ZLinkTx for MemLink {
    async fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        pipe_write(self.tx, buffer).await
    }
}

impl ZLinkTx for MemLinkTx {
    async fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        pipe_write(self.tx, buffer).await
    }
}

impl ZLinkRx for MemLink {
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        pipe_read(self.rx, buffer).await
    }

    async fn read_exact(&mut self, _: &mut [u8]) -> Result<(), LinkError> {
        // Datagram link: the transport never calls this.
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLinkRx for MemLinkRx {
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        pipe_read(self.rx, buffer).await
    }

    async fn read_exact(&mut self, _: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for MemLink {
    type Tx<'link> = MemLinkTx;
    type Rx<'link> = MemLinkRx;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (MemLinkTx { tx: self.tx }, MemLinkRx { rx: self.rx })
    }
}

/// Hands out its single pre-wired link on the first `connect` or `listen`.
pub struct MemLinkManager {
    link: Cell<Option<MemLink>>,
}

impl MemLinkManager {
    fn new(link: MemLink) -> Self {
        Self {
            link: Cell::new(Some(link)),
        }
    }
}

impl ZLinkManager for MemLinkManager {
    type Link<'a> = MemLink;

    async fn connect(&self, _: Endpoint<'_>) -> Result<MemLink, LinkError> {
        self.link.take().ok_or(LinkError::CouldNotConnect)
    }

    async fn listen(&self, _: Endpoint<'_>) -> Result<MemLink, LinkError> {
        self.link.take().ok_or(LinkError::CouldNotConnect)
    }
}

// ── Session configuration ───────────────────────────────────────────────────

pub struct TestConfig {
    transports: TransportLinkManager<MemLinkManager>,
}

impl ZSessionConfig for TestConfig {
    type Buff = [u8; BUFF];
    type LinkManager = MemLinkManager;

    type SubCallbacks<'res> = FixedCapacitySubCallbacks<'res, 8, RawOrBox<64>, RawOrBox<600>>;
    type GetCallbacks<'res> = FixedCapacityGetCallbacks<'res, 8, RawOrBox<64>, RawOrBox<512>>;
    type QueryableCallbacks<'res> =
        FixedCapacityQueryableCallbacks<'res, Self, 8, RawOrBox<64>, RawOrBox<952>>;

    fn transports(&self) -> &TransportLinkManager<Self::LinkManager> {
        &self.transports
    }

    fn buff(&self) -> Self::Buff {
        [0u8; BUFF]
    }
}

pub type TestSession = Session<'static, TestConfig>;

// ── Recorded network messages ───────────────────────────────────────────────

/// An owned summary of a network message sent by the session.
#[derive(Debug, Clone, PartialEq)]
pub enum Seen {
    Push {
        ke: String,
        payload: Vec<u8>,
        encoding: u16,
        qos: u8,
        reliable: bool,
    },
    Request {
        id: u32,
        ke: String,
        parameters: String,
        payload: Option<Vec<u8>>,
        target: String,
        consolidation: String,
    },
    Reply {
        rid: u32,
        ke: String,
        payload: Vec<u8>,
    },
    ReplyErr {
        rid: u32,
        payload: Vec<u8>,
    },
    ResponseFinal {
        rid: u32,
    },
    Interest {
        id: u32,
        mode: String,
        ke: Option<String>,
    },
    InterestFinal {
        id: u32,
    },
    DeclareSubscriber {
        id: u32,
        ke: String,
    },
    UndeclareSubscriber {
        id: u32,
    },
    DeclareQueryable {
        id: u32,
        ke: String,
    },
    UndeclareQueryable {
        id: u32,
    },
    Other(String),
}

impl Seen {
    fn from_msg(msg: &NetworkMessage<'_>) -> Self {
        let reliable = msg.reliability == Reliability::Reliable;
        match &msg.body {
            NetworkBody::Push(Push {
                wire_expr,
                payload: PushBody::Put(put),
                ..
            }) => Seen::Push {
                ke: wire_expr.suffix.into(),
                payload: put.payload.to_vec(),
                encoding: put.encoding.id,
                qos: msg.qos.inner,
                reliable,
            },
            NetworkBody::Request(Request {
                id,
                wire_expr,
                target,
                payload: RequestBody::Query(query),
                ..
            }) => Seen::Request {
                id: *id,
                ke: wire_expr.suffix.into(),
                parameters: query.parameters.into(),
                payload: query.body.as_ref().map(|b| b.payload.to_vec()),
                target: format!("{target:?}"),
                consolidation: format!("{:?}", query.consolidation),
            },
            NetworkBody::Response(Response {
                rid,
                wire_expr,
                payload,
                ..
            }) => match payload {
                ResponseBody::Reply(Reply {
                    payload: PushBody::Put(put),
                    ..
                }) => Seen::Reply {
                    rid: *rid,
                    ke: wire_expr.suffix.into(),
                    payload: put.payload.to_vec(),
                },
                ResponseBody::Err(err) => Seen::ReplyErr {
                    rid: *rid,
                    payload: err.payload.to_vec(),
                },
            },
            NetworkBody::ResponseFinal(ResponseFinal { rid, .. }) => {
                Seen::ResponseFinal { rid: *rid }
            }
            NetworkBody::Interest(Interest {
                id, mode, inner, ..
            }) => Seen::Interest {
                id: *id,
                mode: format!("{mode:?}"),
                ke: inner.wire_expr.as_ref().map(|w| w.suffix.into()),
            },
            NetworkBody::InterestFinal(InterestFinal { id, .. }) => Seen::InterestFinal { id: *id },
            NetworkBody::Declare(Declare { body, .. }) => match body {
                DeclareBody::DeclareSubscriber(d) => Seen::DeclareSubscriber {
                    id: d.id,
                    ke: d.wire_expr.suffix.into(),
                },
                DeclareBody::UndeclareSubscriber(u) => Seen::UndeclareSubscriber { id: u.id },
                DeclareBody::DeclareQueryable(d) => Seen::DeclareQueryable {
                    id: d.id,
                    ke: d.wire_expr.suffix.into(),
                },
                DeclareBody::UndeclareQueryable(u) => Seen::UndeclareQueryable { id: u.id },
                other => Seen::Other(format!("{other:?}")),
            },
        }
    }
}

// ── Fake router ─────────────────────────────────────────────────────────────

pub struct Router {
    driver: Driver<'static, MemLink, [u8; BUFF]>,
    seen: Mutex<NoopRawMutex, Vec<Seen>>,
}

impl Router {
    async fn run(&self) -> Result<(), SessionError> {
        self.driver
            .run(&self.seen, async |_, seen, msg, _| {
                seen.push(Seen::from_msg(&msg));
                Ok::<(), SessionError>(())
            })
            .await
            .map_err(|e| e.flatten_map())
    }

    /// Sends one network message to the session.
    pub async fn send(&self, body: NetworkBody<'_>) {
        self.driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body,
            }))
            .await
            .expect("router failed to send");
    }

    /// Sends a `Put` push on `ke`, as a router forwarding a remote publication.
    pub async fn push(&self, ke: &str, payload: &[u8]) {
        self.send(NetworkBody::Push(Push {
            wire_expr: WireExpr::from(zenoh_proto::keyexpr::new(ke).unwrap()),
            payload: PushBody::Put(Put {
                payload,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .await;
    }

    /// Everything the session has sent so far.
    pub async fn seen(&self) -> Vec<Seen> {
        self.seen.lock().await.clone()
    }

    /// Waits until the session sends a message matching `pred`, and returns it.
    /// Panics after [`WAIT`].
    pub async fn wait_for(&self, what: &str, pred: impl Fn(&Seen) -> bool) -> Seen {
        self.try_wait_for(pred)
            .await
            .unwrap_or_else(|| panic!("router never saw {what}"))
    }

    /// Like [`Router::wait_for`], but returns `None` after [`WAIT`].
    pub async fn try_wait_for(&self, pred: impl Fn(&Seen) -> bool) -> Option<Seen> {
        let deadline = Instant::now() + WAIT;
        loop {
            if let Some(s) = self.seen.lock().await.iter().find(|s| pred(s)) {
                return Some(s.clone());
            }
            if Instant::now() >= deadline {
                return None;
            }
            Timer::after_millis(1).await;
        }
    }
}

// ── Driver ──────────────────────────────────────────────────────────────────

fn leak<T>(value: T) -> &'static mut T {
    Box::leak(Box::new(value))
}

/// Runs `test` against a fresh session connected to a fresh fake router.
///
/// The test fails if it takes longer than [`TEST_TIMEOUT`], or if the session
/// or router run loop exits before the test body completes.
pub fn run<F, Fut>(test: F)
where
    F: FnOnce(&'static TestSession, &'static Router) -> Fut,
    Fut: Future<Output = ()>,
{
    embassy_futures::block_on(async move {
        let to_router: &'static Pipe = leak(Channel::new());
        let to_session: &'static Pipe = leak(Channel::new());

        let config: &'static TestConfig = leak(TestConfig {
            transports: TransportLinkManager::from(MemLinkManager::new(MemLink {
                tx: to_router,
                rx: to_session,
            })),
        });
        let router_transports: &'static TransportLinkManager<MemLinkManager> =
            leak(TransportLinkManager::from(MemLinkManager::new(MemLink {
                tx: to_session,
                rx: to_router,
            })));

        let endpoint = || Endpoint::try_from("tcp/127.0.0.1:7447").unwrap();
        let (session, router) = embassy_futures::join::join(
            SessionBuilder::new(leak(Resources::default()), config).connect(endpoint()),
            router_transports.listen(endpoint(), [0u8; BUFF]),
        )
        .await;

        let session: &'static TestSession = leak(session.expect("session failed to connect"));
        let router: &'static Router = leak(Router {
            driver: Driver::new(leak(router.expect("router failed to accept"))),
            seen: Mutex::new(Vec::new()),
        });

        match select3(
            with_timeout(TEST_TIMEOUT, test(session, router)),
            session.run(),
            router.run(),
        )
        .await
        {
            Either3::First(Ok(())) => {}
            Either3::First(Err(_)) => panic!("test did not finish within {TEST_TIMEOUT:?}"),
            Either3::Second(r) => panic!("session run loop exited early: {r:?}"),
            Either3::Third(r) => panic!("router run loop exited early: {r:?}"),
        }
    });
}
