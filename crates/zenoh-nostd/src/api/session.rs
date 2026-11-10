use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    mutex::{Mutex, MutexGuard},
};
use static_cell::StaticCell;
use zenoh_proto::{Endpoint, TransportLinkError};

use crate::{
    api::callbacks::ZCallbacks,
    config::ZSessionConfig,
    io::{driver::Driver, transport::TransportLink},
    platform::ZLinkManager,
    resources::Resources,
};

mod run;

pub mod get;
pub mod r#pub;
pub mod put;
pub mod querier;
pub mod queryable;
pub mod sub;

pub(crate) struct SessionState<'transport, 'resources: 'transport, Config>
where
    Config: ZSessionConfig + 'resources,
{
    next: u32,
    sub_callbacks: Config::SubCallbacks<'transport>,
    get_callbacks: Config::GetCallbacks<'transport>,
    queryable_callbacks: Config::QueryableCallbacks<'transport, 'resources>,
}

impl<'transport, 'resources: 'transport, Config> SessionState<'transport, 'resources, Config>
where
    Config: ZSessionConfig + 'resources,
{
    pub fn new() -> Self {
        Self {
            next: 0,
            sub_callbacks: Config::SubCallbacks::empty(),
            get_callbacks: Config::GetCallbacks::empty(),
            queryable_callbacks: Config::QueryableCallbacks::empty(),
        }
    }

    pub(crate) fn next(&mut self) -> u32 {
        let next = self.next;
        self.next += 1;
        next
    }
}

/// A session borrowing its transport and link resources.
///
/// Cancelling an operation or encountering a transport write error requires
/// stopping the driver and dropping the session and its transport before any
/// further network I/O. [`Self::run`] must be polled concurrently with operations;
/// constructing a session starts no executor task. Session access is confined to
/// one executor/thread context because it uses `NoopRawMutex`.
///
/// Reconnect with a new transport to continue. Use
/// [`Self::flush`] for normal completion, not to recover an interrupted write.
pub struct Session<'transport, 'resources: 'transport, Config>
where
    Config: ZSessionConfig + 'resources,
{
    driver:
        Driver<'transport, <Config::LinkManager as ZLinkManager>::Link<'resources>, Config::Buff>,
    state: Mutex<NoopRawMutex, SessionState<'transport, 'resources, Config>>,
}

impl<'transport, 'resources: 'transport, Config> Session<'transport, 'resources, Config>
where
    Config: ZSessionConfig + 'resources,
{
    /// Borrow an already-handshaken transport and split it into driver RX/TX halves.
    ///
    /// `'transport` is the duration of this mutable borrow; `'resources` covers
    /// resources borrowed by the underlying link and must outlive it. No background
    /// task is started. Poll [`Self::run`] concurrently with application operations,
    /// and keep all session access in one executor/thread context.
    ///
    /// Drop operations and the session before dropping or replacing the transport.
    /// After cancellation or a transport write error, discard the entire connection;
    /// this constructor does not reset an existing transport.
    pub fn new(
        transport: &'transport mut TransportLink<
            <Config::LinkManager as ZLinkManager>::Link<'resources>,
            Config::Buff,
        >,
    ) -> Self {
        Self {
            driver: Driver::new(transport),
            state: Mutex::new(SessionState::new()),
        }
    }

    pub(crate) async fn state(
        &self,
    ) -> MutexGuard<'_, NoopRawMutex, SessionState<'transport, 'resources, Config>> {
        self.state.lock().await
    }
}

/// Prepares a session using caller-provided resources and configuration.
///
/// Options are applied after the transport handshake and before the session's
/// driver is constructed. Each builder is consumed by `connect` or `listen`.
#[must_use = "a session builder must be connected or listened on"]
pub struct SessionBuilder<'resources, Config: ZSessionConfig> {
    #[doc(hidden)]
    pub resources: &'resources mut Resources<'resources, Config>,
    #[doc(hidden)]
    pub config: &'resources Config,
    #[doc(hidden)]
    pub ignore_invalid_sn: bool,
}

impl<'resources, Config: ZSessionConfig> SessionBuilder<'resources, Config> {
    pub fn new(
        resources: &'resources mut Resources<'resources, Config>,
        config: &'resources Config,
    ) -> Self {
        Self {
            resources,
            config,
            ignore_invalid_sn: false,
        }
    }

    /// Disables validation of received transport frame sequence numbers.
    pub fn ignore_invalid_sn(mut self) -> Self {
        self.ignore_invalid_sn = true;
        self
    }

    /// Establishes an outgoing connection and creates the session.
    pub async fn connect(
        self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<Session<'resources, 'resources, Config>, TransportLinkError> {
        let transport = self
            .config
            .transports()
            .connect(endpoint, self.config.buff())
            .await?;
        Ok(self.finish(transport))
    }

    /// Accepts an incoming connection and creates the session.
    pub async fn listen(
        self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<Session<'resources, 'resources, Config>, TransportLinkError> {
        let transport = self
            .config
            .transports()
            .listen(endpoint, self.config.buff())
            .await?;
        Ok(self.finish(transport))
    }

    fn finish(
        self,
        mut transport: TransportLink<
            <Config::LinkManager as ZLinkManager>::Link<'resources>,
            Config::Buff,
        >,
    ) -> Session<'resources, 'resources, Config> {
        if self.ignore_invalid_sn {
            transport.transport_mut().rx.ignore_invalid_sn();
        }
        Session::new(self.resources.init(transport))
    }
}

/// Prepares a session whose configuration, resources, and session have static storage.
///
/// Usually created with `zenoh::session!`. Each builder is consumed by `connect`
/// or `listen`, which returns a shared reference to the initialized session.
/// Storage cannot be reused, including after a connection error or cancellation.
#[must_use = "a session builder must be connected or listened on"]
pub struct StaticSessionBuilder<Config: ZSessionConfig + 'static> {
    builder: SessionBuilder<'static, Config>,
    session: &'static StaticCell<Session<'static, 'static, Config>>,
}

impl<Config: ZSessionConfig + 'static> StaticSessionBuilder<Config> {
    /// Prepares a session using static resources, configuration, and an empty session cell.
    pub fn new(
        resources: &'static mut Resources<'static, Config>,
        config: &'static Config,
        session: &'static StaticCell<Session<'static, 'static, Config>>,
    ) -> Self {
        Self {
            builder: SessionBuilder::new(resources, config),
            session,
        }
    }

    /// Disables validation of received transport frame sequence numbers.
    pub fn ignore_invalid_sn(mut self) -> Self {
        self.builder = self.builder.ignore_invalid_sn();
        self
    }

    /// Establishes an outgoing connection and stores the session for the program's lifetime.
    pub async fn connect(
        self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<&'static Session<'static, 'static, Config>, TransportLinkError> {
        let session = self.builder.connect(endpoint).await?;
        Ok(self.session.init(session))
    }

    /// Accepts an incoming connection and stores the session for the program's lifetime.
    pub async fn listen(
        self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<&'static Session<'static, 'static, Config>, TransportLinkError> {
        let session = self.builder.listen(endpoint).await?;
        Ok(self.session.init(session))
    }
}

#[macro_export]
macro_rules! __static_session {
    ($CONFIG:ty: $config:expr $(,)?) => {{
        static CONFIG: $crate::session::__StaticCell<$CONFIG> =
            $crate::session::__StaticCell::new();
        static RESOURCES: $crate::session::__StaticCell<
            $crate::session::Resources<'static, $CONFIG>,
        > = $crate::session::__StaticCell::new();
        static SESSION: $crate::session::__StaticCell<
            $crate::session::Session<'static, 'static, $CONFIG>,
        > = $crate::session::__StaticCell::new();

        $crate::session::StaticSessionBuilder::new(
            RESOURCES.init_with($crate::session::Resources::default),
            CONFIG.init($config),
            &SESSION,
        )
    }};
}

/// Prepares a session builder.
///
/// The expression form uses local resources. Use the `static` form when the
/// session needs static storage. Static storage can be initialized only once per call site,
/// including when opening a session fails or is cancelled.
#[macro_export]
macro_rules! __session {
    (static $CONFIG:ty: $config:expr $(,)?) => {
        $crate::__static_session!($CONFIG: $config)
    };
    ($CONFIG:ty: $config:expr $(,)?) => {{
        $crate::session::SessionBuilder::<$CONFIG> {
            resources: &mut $crate::session::Resources::default(),
            config: &$config,
            ignore_invalid_sn: false,
        }
    }};
}

impl<'transport, 'resources: 'transport, Config: ZSessionConfig>
    Session<'transport, 'resources, Config>
{
    /// Drain adapter-owned TX buffers before closing a finite application.
    /// The session driver should keep being polled while this completes.
    pub async fn flush(&self) -> Result<(), zenoh_proto::SessionError> {
        use crate::io::transport::ZTransportLinkTx;
        self.driver.tx().await.flush().await.map_err(Into::into)
    }

    /// Remove application declarations and pending gets after their owners
    /// finish successfully. After cancellation or a transport write error,
    /// discard the session and transport instead of attempting this cleanup.
    pub async fn clear_declarations(&self) -> Result<(), zenoh_proto::SessionError> {
        use crate::io::transport::ZTransportLinkTx;
        use zenoh_proto::{fields::*, msgs::*};
        let mut state = self.state().await;
        state.get_callbacks.clear();
        while let Some(id) = state.sub_callbacks.pop() {
            self.driver
                .tx()
                .await
                .send(core::iter::once(NetworkMessage {
                    reliability: Reliability::default(),
                    qos: zenoh_proto::exts::QoS::default(),
                    body: NetworkBody::Declare(Declare {
                        body: DeclareBody::UndeclareSubscriber(UndeclareSubscriber {
                            id,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                }))
                .await?;
        }
        while let Some(id) = state.queryable_callbacks.pop() {
            self.driver
                .tx()
                .await
                .send(core::iter::once(NetworkMessage {
                    reliability: Reliability::default(),
                    qos: zenoh_proto::exts::QoS::default(),
                    body: NetworkBody::Declare(Declare {
                        body: DeclareBody::UndeclareQueryable(UndeclareQueryable {
                            id,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                }))
                .await?;
        }
        state.queryable_callbacks.clear();
        Ok(())
    }
}

#[cfg(test)]
mod scoped_lifetime_tests {
    extern crate std;
    use super::*;
    use crate::{platform::*, session::*};
    use core::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Default)]
    struct Manager {
        live: AtomicUsize,
    }
    #[derive(Default)]
    struct Counts {
        halves: AtomicUsize,
        links: AtomicUsize,
    }
    struct Link<'manager> {
        owner: &'manager Manager,
        counts: Arc<Counts>,
    }
    struct Half<'borrow> {
        owner: &'borrow Manager,
        counts: Arc<Counts>,
    }
    impl Manager {
        fn link(&self, counts: Arc<Counts>) -> Link<'_> {
            self.live.fetch_add(1, Ordering::SeqCst);
            Link {
                owner: self,
                counts,
            }
        }
    }
    impl Drop for Link<'_> {
        fn drop(&mut self) {
            assert_eq!(self.counts.halves.load(Ordering::SeqCst), 2);
            self.counts.links.fetch_add(1, Ordering::SeqCst);
            self.owner.live.fetch_sub(1, Ordering::SeqCst);
        }
    }
    impl Drop for Half<'_> {
        fn drop(&mut self) {
            assert_eq!(self.counts.links.load(Ordering::SeqCst), 0);
            assert_eq!(self.owner.live.load(Ordering::SeqCst), 1);
            self.counts.halves.fetch_add(1, Ordering::SeqCst);
        }
    }
    impl ZLinkInfo for Link<'_> {
        fn mtu(&self) -> u16 {
            64
        }
        fn is_streamed(&self) -> bool {
            true
        }
    }
    impl ZLinkInfo for Half<'_> {
        fn mtu(&self) -> u16 {
            64
        }
        fn is_streamed(&self) -> bool {
            true
        }
    }
    impl ZLinkTx for Link<'_> {
        async fn write_all(&mut self, _: &[u8]) -> Result<(), LinkError> {
            Ok(())
        }
    }
    impl ZLinkTx for Half<'_> {
        async fn write_all(&mut self, _: &[u8]) -> Result<(), LinkError> {
            Ok(())
        }
    }
    impl ZLinkRx for Link<'_> {
        async fn read(&mut self, _: &mut [u8]) -> Result<usize, LinkError> {
            core::future::pending().await
        }
        async fn read_exact(&mut self, _: &mut [u8]) -> Result<(), LinkError> {
            core::future::pending().await
        }
    }
    impl ZLinkRx for Half<'_> {
        async fn read(&mut self, _: &mut [u8]) -> Result<usize, LinkError> {
            core::future::pending().await
        }
        async fn read_exact(&mut self, _: &mut [u8]) -> Result<(), LinkError> {
            core::future::pending().await
        }
    }
    impl ZLink for Link<'_> {
        type Tx<'a>
            = Half<'a>
        where
            Self: 'a;
        type Rx<'a>
            = Half<'a>
        where
            Self: 'a;
        fn split(&mut self) -> (Half<'_>, Half<'_>) {
            (
                Half {
                    owner: self.owner,
                    counts: self.counts.clone(),
                },
                Half {
                    owner: self.owner,
                    counts: self.counts.clone(),
                },
            )
        }
    }
    impl ZLinkManager for Manager {
        type Link<'a>
            = Link<'a>
        where
            Self: 'a;
        async fn connect(&self, _: Endpoint<'_>) -> Result<Link<'_>, LinkError> {
            Err(LinkError::CouldNotConnect)
        }
        async fn listen(&self, _: Endpoint<'_>) -> Result<Link<'_>, LinkError> {
            Err(LinkError::CouldNotListen)
        }
    }
    struct Config<'manager> {
        owner: &'manager Manager,
    }
    impl ZSessionConfig for Config<'_> {
        type Buff = [u8; 64];
        type LinkManager = Manager;
        type SubCallbacks<'transport> = FixedCapacitySubCallbacks<'transport, 2>;
        type GetCallbacks<'transport> = FixedCapacityGetCallbacks<'transport, 2>;
        type QueryableCallbacks<'transport, 'resources>
            = FixedCapacityQueryableCallbacks<'transport, 'resources, Self, 2>
        where
            Self: 'resources,
            'resources: 'transport;
        fn transports(&self) -> &TransportLinkManager<Manager> {
            unreachable!()
        }
        fn buff(&self) -> Self::Buff {
            [0; 64]
        }
    }
    fn use_connection<'manager>(config: &Config<'manager>, counts: Arc<Counts>) {
        let mut transport = TransportLink::new(
            config.owner.link(counts),
            zenoh_sansio::Transport::builder([0; 64]).codec(),
        );
        let session = Session::<Config<'manager>>::new(&mut transport);
        let _ = &session;
    }
    #[test]
    fn scoped_connections_reopen_without_static_storage_and_drop_driver_first() {
        let owner = Manager::default();
        let config = Config { owner: &owner };
        for _ in 0..1000 {
            let counts = Arc::new(Counts::default());
            use_connection(&config, counts.clone());
            assert_eq!(counts.halves.load(Ordering::SeqCst), 2);
            assert_eq!(counts.links.load(Ordering::SeqCst), 1);
            assert_eq!(owner.live.load(Ordering::SeqCst), 0);
        }
    }
    #[test]
    fn cancelling_a_scoped_connection_drops_driver_before_transport() {
        let owner = Manager::default();
        let counts = Arc::new(Counts::default());
        {
            let counts = counts.clone();
            let owner = &owner;
            let mut connection = core::pin::pin!(async move {
                let mut transport = TransportLink::new(
                    owner.link(counts),
                    zenoh_sansio::Transport::builder([0; 64]).codec(),
                );
                let session = Session::<Config<'_>>::new(&mut transport);
                core::future::pending::<()>().await;
                drop(session);
            });
            assert!(matches!(
                connection
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop())),
                Poll::Pending
            ));
        }
        assert_eq!(counts.halves.load(Ordering::SeqCst), 2);
        assert_eq!(counts.links.load(Ordering::SeqCst), 1);
        assert_eq!(owner.live.load(Ordering::SeqCst), 0);
    }
}
