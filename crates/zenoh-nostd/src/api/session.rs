use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    mutex::{Mutex, MutexGuard},
};
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

pub(crate) struct SessionState<'s, 'res, Config>
where
    Config: ZSessionConfig + 'res,
    'res: 's,
{
    next: u32,
    sub_callbacks: Config::SubCallbacks<'res>,
    get_callbacks: Config::GetCallbacks<'res>,
    queryable_callbacks: Config::QueryableCallbacks<'s, 'res>,
}

impl<'s, 'res, Config> SessionState<'s, 'res, Config>
where
    Config: ZSessionConfig,
    'res: 's,
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

/// A zenoh session.
///
/// The two lifetimes are deliberately distinct. `'a` is how long the session borrows the
/// [`Resources`] it was built from; `'res` is the lifetime of the link itself, which comes
/// from the [`ZLinkManager`] inside the config.
///
/// Conflating them — as a single `'res` did — forces `&'res mut Resources<'res, Config>`,
/// which is invariant and, because `Resources` has a destructor, makes the storage outlive
/// itself. `'static` is then the only lifetime that typechecks, so a session can only ever
/// be built once. Keeping them apart lets a session be scoped and rebuilt, which is what a
/// client needs in order to reconnect after a dropped link.
pub struct Session<'s, 'res, Config>
where
    Config: ZSessionConfig,
    'res: 's,
{
    driver: Driver<'s, <Config::LinkManager as ZLinkManager>::Link<'res>, Config::Buff>,
    state: Mutex<NoopRawMutex, SessionState<'s, 'res, Config>>,
}

impl<'s, 'res, Config> Session<'s, 'res, Config>
where
    Config: ZSessionConfig,
    'res: 's,
{
    pub fn new(
        transport: &'s mut TransportLink<
            <Config::LinkManager as ZLinkManager>::Link<'res>,
            Config::Buff,
        >,
    ) -> Self {
        Self {
            driver: Driver::new(transport),
            state: Mutex::new(SessionState::new()),
        }
    }

    pub(crate) async fn state(&self) -> MutexGuard<'_, NoopRawMutex, SessionState<'s, 'res, Config>> {
        self.state.lock().await
    }
}

pub async fn session_connect<'s, 'res, Config>(
    resources: &'s mut Resources<'res, Config>,
    config: &'res Config,
    endpoint: Endpoint<'_>,
) -> core::result::Result<Session<'s, 'res, Config>, TransportLinkError>
where
    Config: ZSessionConfig,
{
    Ok(Session::new(resources.init(
        config.transports().connect(endpoint, config.buff()).await?,
    )))
}

pub async fn session_listen<'s, 'res, Config>(
    resources: &'s mut Resources<'res, Config>,
    config: &'res Config,
    endpoint: Endpoint<'_>,
) -> core::result::Result<Session<'s, 'res, Config>, TransportLinkError>
where
    Config: ZSessionConfig,
{
    Ok(Session::new(resources.init(
        config.transports().listen(endpoint, config.buff()).await?,
    )))
}

#[macro_export]
macro_rules! __session_connect {
    (
        $CONFIG:ty: $config:expr,
        $endpoint:expr
    ) => {{
        static CONFIG: static_cell::StaticCell<$CONFIG> = static_cell::StaticCell::new();
        let config = CONFIG.init($config);

        static RESOURCES: static_cell::StaticCell<$crate::session::Resources<'static, $CONFIG>> =
            static_cell::StaticCell::new();

        static SESSION: static_cell::StaticCell<$crate::session::Session<'static, $CONFIG>> =
            static_cell::StaticCell::new();

        SESSION.init($crate::session::Session::new(
            RESOURCES.init($crate::session::Resources::default()).init(
                config
                    .transports()
                    .connect($endpoint, config.buff())
                    .await?,
            ),
        )) as &$crate::session::Session<'static, $CONFIG>
    }};
}

#[macro_export]
macro_rules! __session_listen {
    (
        $CONFIG:ty: $config:expr,
        $endpoint:expr
    ) => {{
        static CONFIG: static_cell::StaticCell<$CONFIG> = static_cell::StaticCell::new();
        let config = CONFIG.init($config);

        static RESOURCES: static_cell::StaticCell<$crate::session::Resources<'static, $CONFIG>> =
            static_cell::StaticCell::new();

        static SESSION: static_cell::StaticCell<$crate::session::Session<'static, $CONFIG>> =
            static_cell::StaticCell::new();

        SESSION.init($crate::session::Session::new(
            RESOURCES
                .init($crate::session::Resources::default())
                .init(config.transports().listen($endpoint, config.buff()).await?),
        )) as &$crate::session::Session<'static, $CONFIG>
    }};
}

pub async fn session_connect_ignore_invalid_sn<'s, 'res, Config>(
    resources: &'s mut Resources<'res, Config>,
    config: &'res Config,
    endpoint: Endpoint<'_>,
) -> core::result::Result<Session<'s, 'res, Config>, TransportLinkError>
where
    Config: ZSessionConfig,
{
    let mut transport = config.transports().connect(endpoint, config.buff()).await?;
    transport.transport_mut().rx.ignore_invalid_sn();

    Ok(Session::new(resources.init(transport)))
}

pub async fn session_listen_ignore_invalid_sn<'s, 'res, Config>(
    resources: &'s mut Resources<'res, Config>,
    config: &'res Config,
    endpoint: Endpoint<'_>,
) -> core::result::Result<Session<'s, 'res, Config>, TransportLinkError>
where
    Config: ZSessionConfig,
{
    let mut transport = config.transports().listen(endpoint, config.buff()).await?;
    transport.transport_mut().rx.ignore_invalid_sn();
    Ok(Session::new(resources.init(transport)))
}
