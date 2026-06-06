use core::ops::DerefMut;

use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    mutex::{Mutex, MutexGuard},
};
use zenoh_proto::{
    Endpoint, SessionError, TransportLinkError,
    exts::QoS,
    fields::{Reliability, WireExpr, ZenohIdProto},
    msgs::{Declare, DeclareBody, NetworkBody, NetworkMessage, UndeclareQueryable},
};
use zenoh_sansio::ZTransportTx;

use crate::{
    api::callbacks::ZCallbacks,
    config::ZSessionConfig,
    io::{driver::Driver, transport::{TransportLink, ZTransportLinkTx}},
    platform::ZLinkManager,
    resources::Resources,
};

mod run;

pub mod admin_space;
pub mod delete;
pub mod get;
pub mod liveliness;
pub mod r#pub;
pub mod put;
pub mod querier;
pub mod queryable;
pub mod sub;

pub(crate) struct SessionState<'res, Config>
where
    Config: ZSessionConfig + 'res,
{
    next: u32,
    sub_callbacks: Config::SubCallbacks<'res>,
    get_callbacks: Config::GetCallbacks<'res>,
    queryable_callbacks: Config::QueryableCallbacks<'res>,
    liveliness_callbacks: Config::LivelinessCallbacks<'res>,
    admin_space: Option<u32>,
    pub(crate) ke_mappings: heapless::Vec<KeMapping, 16>,
}

pub(crate) struct KeMapping {
    pub scope: u16,
    pub keyexpr: heapless::String<64>,
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
            liveliness_callbacks: Config::LivelinessCallbacks::empty(),
            admin_space: None,
            ke_mappings: heapless::Vec::new(),
        }
    }

pub(crate) fn resolve_keyexpr<'a>(&self, wire_expr: &WireExpr<'a>) -> Option<heapless::String<64>> {
    if !wire_expr.suffix.is_empty() {
        let mut s = heapless::String::new();
        s.push_str(wire_expr.suffix).ok()?;
        return Some(s);
    }
    // Mapped keyexpr: look up scope ID
    let scope = wire_expr.scope;
    for m in &self.ke_mappings {
        if m.scope == scope {
            let mut s = heapless::String::new();
            s.push_str(&m.keyexpr).ok()?;
            return Some(s);
        }
    }
    None
}

    pub(crate) fn next(&mut self) -> u32 {
        let next = self.next;
        self.next += 1;
        next
    }

    pub(crate) fn admin_space_id(&self) -> Option<u32> {
        self.admin_space
    }

    pub(crate) fn set_admin_space_id(&mut self, id: Option<u32>) {
        self.admin_space = id;
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

    pub async fn batch_start(&self) -> core::result::Result<(), TransportLinkError> {
        let mut tx_guard = self.driver.tx().await;
        let (_, transport) = tx_guard.tx();
        transport.batch_start();
        Ok(())
    }

    pub async fn batch_flush(&self) -> core::result::Result<(), TransportLinkError> {
        ZTransportLinkTx::batch_flush(self.driver.tx().await.deref_mut()).await
    }

    pub async fn batch_stop(&self) -> core::result::Result<(), TransportLinkError> {
        {
            let mut tx_guard = self.driver.tx().await;
            let (_, transport) = tx_guard.tx();
            transport.batch_stop();
        }
        ZTransportLinkTx::batch_flush(self.driver.tx().await.deref_mut()).await
    }

    pub fn zid(&self) -> ZenohIdProto {
        self.driver.zid()
    }

    pub(crate) async fn _undeclare_queryable(
        &self,
        id: u32,
    ) -> core::result::Result<(), SessionError> {
        let mut state = self.state().await;
        state.queryable_callbacks.remove(id)?;

        let msg = Declare {
            body: DeclareBody::UndeclareQueryable(UndeclareQueryable {
                id,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))
            .await?;

        Ok(())
    }
}

pub async fn session_connect<'res, Config>(
    resources: &'res mut Resources<'res, Config>,
    config: &'res Config,
    endpoint: Endpoint<'_>,
) -> core::result::Result<Session<'res, Config>, TransportLinkError>
where
    Config: ZSessionConfig,
{
    Ok(Session::new(resources.init(
        config.transports().connect(endpoint, config.buff()).await?,
    )))
}

pub async fn session_listen<'res, Config>(
    resources: &'res mut Resources<'res, Config>,
    config: &'res Config,
    endpoint: Endpoint<'_>,
) -> core::result::Result<Session<'res, Config>, TransportLinkError>
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

pub async fn session_connect_ignore_invalid_sn<'res, Config>(
    resources: &'res mut Resources<'res, Config>,
    config: &'res Config,
    endpoint: Endpoint<'_>,
) -> core::result::Result<Session<'res, Config>, TransportLinkError>
where
    Config: ZSessionConfig,
{
    let mut transport = config.transports().connect(endpoint, config.buff()).await?;
    transport.transport_mut().rx.ignore_invalid_sn();

    Ok(Session::new(resources.init(transport)))
}

pub async fn session_listen_ignore_invalid_sn<'res, Config>(
    resources: &'res mut Resources<'res, Config>,
    config: &'res Config,
    endpoint: Endpoint<'_>,
) -> core::result::Result<Session<'res, Config>, TransportLinkError>
where
    Config: ZSessionConfig,
{
    let mut transport = config.transports().listen(endpoint, config.buff()).await?;
    transport.transport_mut().rx.ignore_invalid_sn();
    Ok(Session::new(resources.init(transport)))
}
