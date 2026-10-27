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
pub mod token;

pub(crate) struct SessionState<'res, Config>
where
    Config: ZSessionConfig + 'res,
{
    next: u32,
    sub_callbacks: Config::SubCallbacks<'res>,
    get_callbacks: Config::GetCallbacks<'res>,
    queryable_callbacks: Config::QueryableCallbacks<'res>,
}

impl<'res, Config> SessionState<'res, Config>
where
    Config: ZSessionConfig,
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

pub struct Session<'res, Config>
where
    Config: ZSessionConfig,
{
    driver: Driver<'res, <Config::LinkManager as ZLinkManager>::Link<'res>, Config::Buff>,
    state: Mutex<NoopRawMutex, SessionState<'res, Config>>,
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
            state: Mutex::new(SessionState::new()),
        }
    }

    pub(crate) async fn state(&self) -> MutexGuard<'_, NoopRawMutex, SessionState<'res, Config>> {
        self.state.lock().await
    }
}

/// Prepares a session using caller-provided resources and configuration.
///
/// Options are applied after the transport handshake and before the session's
/// driver is constructed. Each builder is consumed by `connect` or `listen`.
#[must_use = "a session builder must be connected or listened on"]
pub struct SessionBuilder<'res, Config: ZSessionConfig> {
    #[doc(hidden)]
    pub resources: &'res mut Resources<'res, Config>,
    #[doc(hidden)]
    pub config: &'res Config,
    #[doc(hidden)]
    pub ignore_invalid_sn: bool,
}

impl<'res, Config: ZSessionConfig> SessionBuilder<'res, Config> {
    pub fn new(resources: &'res mut Resources<'res, Config>, config: &'res Config) -> Self {
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
    ) -> core::result::Result<Session<'res, Config>, TransportLinkError> {
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
    ) -> core::result::Result<Session<'res, Config>, TransportLinkError> {
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
            <Config::LinkManager as ZLinkManager>::Link<'res>,
            Config::Buff,
        >,
    ) -> Session<'res, Config> {
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
    session: &'static StaticCell<Session<'static, Config>>,
}

impl<Config: ZSessionConfig + 'static> StaticSessionBuilder<Config> {
    /// Prepares a session using static resources, configuration, and an empty session cell.
    pub fn new(
        resources: &'static mut Resources<'static, Config>,
        config: &'static Config,
        session: &'static StaticCell<Session<'static, Config>>,
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
    ) -> core::result::Result<&'static Session<'static, Config>, TransportLinkError> {
        let session = self.builder.connect(endpoint).await?;
        Ok(self.session.init(session))
    }

    /// Accepts an incoming connection and stores the session for the program's lifetime.
    pub async fn listen(
        self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<&'static Session<'static, Config>, TransportLinkError> {
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
        static SESSION: $crate::session::__StaticCell<$crate::session::Session<'static, $CONFIG>> =
            $crate::session::__StaticCell::new();

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
