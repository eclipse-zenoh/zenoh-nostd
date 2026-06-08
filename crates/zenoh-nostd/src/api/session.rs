use core::cell::UnsafeCell;
use zenoh_proto::{
    Endpoint, SessionError, TransportLinkError, keyexpr,
    exts::QoS,
    fields::{ConsolidationMode, Reliability, WireExpr, ZenohIdProto},
    msgs::{
        Declare, DeclareBody, Del, Err, NetworkBody, NetworkMessage, PushBody, Put, Reply,
        Response, ResponseBody, ResponseFinal, UndeclareQueryable,
    },
};
use zenoh_sansio::ZTransportTx;

use crate::{
    api::callbacks::ZCallbacks,
    api::connectivity::{ConnectivityState, TransportEvent},
    config::ZSessionConfig,
    io::{driver::Driver, transport::{TransportLink, ZTransportLinkTx}},
    platform::ZLinkManager,
    resources::Resources,
};

#[cfg(feature = "acl")]
use crate::api::acl::AclOperation;

pub(crate) mod run;

pub mod admin_space;
#[cfg(feature = "background_subscriber")]
pub mod background_sub;
pub mod delete;
pub mod get;
pub mod liveliness;
pub mod put;
pub mod querier;
pub mod queryable;
pub mod r#pub;
#[cfg(feature = "scouting")]
pub mod scout;
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

    #[allow(dead_code)]
    pub fn ke_mappings(&self) -> &[KeMapping] {
        &self.ke_mappings
    }

    #[allow(dead_code)]
    pub fn for_each_subscription(&self, f: impl FnMut(u32, &'static keyexpr)) {
        self.sub_callbacks.for_each_keyexpr(f);
    }

    #[allow(dead_code)]
    pub fn for_each_queryable(&self, f: impl FnMut(u32, &'static keyexpr)) {
        self.queryable_callbacks.for_each_keyexpr(f);
    }

    #[allow(dead_code)]
    pub fn for_each_liveliness(&self, f: impl FnMut(u32, &'static keyexpr)) {
        self.liveliness_callbacks.for_each_keyexpr(f);
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
    driver: UnsafeCell<Driver<'res, <Config::LinkManager as ZLinkManager>::Link<'res>, Config::Buff>>,
    state: UnsafeCell<SessionState<'res, Config>>,
    connectivity: ConnectivityState<'static>,
}

// SAFETY: Session is single-threaded. UnsafeCell fields are accessed with
// exclusive access guarantees from the caller (no concurrent access).
unsafe impl<'res, Config> Send for Session<'res, Config> where Config: ZSessionConfig {}
unsafe impl<'res, Config> Sync for Session<'res, Config> where Config: ZSessionConfig {}

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
            driver: UnsafeCell::new(Driver::new(transport)),
            state: UnsafeCell::new(SessionState::new()),
            connectivity: ConnectivityState::new(),
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub(crate) fn driver(&self) -> &mut Driver<'res, <Config::LinkManager as ZLinkManager>::Link<'res>, Config::Buff> {
        // SAFETY: Single-threaded execution. Callers guarantee exclusive access.
        unsafe { &mut *self.driver.get() }
    }

    #[allow(clippy::mut_from_ref)]
    pub(crate) fn state(&self) -> &mut SessionState<'res, Config> {
        // SAFETY: Single-threaded execution. Callers guarantee exclusive access.
        unsafe { &mut *self.state.get() }
    }

    pub fn batch_start(&self) -> core::result::Result<(), TransportLinkError> {
        let (_, transport) = self.driver().tx_mut().tx();
        transport.batch_start();
        Ok(())
    }

    pub fn batch_flush(&self) -> core::result::Result<(), TransportLinkError> {
        self.driver().tx_mut().batch_flush()
    }

    pub fn batch_stop(&self) -> core::result::Result<(), TransportLinkError> {
        {
            let (_, transport) = self.driver().tx_mut().tx();
            transport.batch_stop();
        }
        self.driver().tx_mut().batch_flush()
    }

    pub fn zid(&self) -> ZenohIdProto {
        // SAFETY: zid() only reads immutable data, safe via shared ref.
        unsafe { &*self.driver.get() }.zid()
    }

    pub(crate) fn _undeclare_queryable(
        &self,
        id: u32,
    ) -> core::result::Result<(), SessionError> {
        self.state().queryable_callbacks.remove(id)?;

        let msg = Declare {
            body: DeclareBody::UndeclareQueryable(UndeclareQueryable {
                id,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))?;

        Ok(())
    }

    pub fn on_connect(&mut self, f: &'static dyn Fn(TransportEvent)) {
        let _ = self.connectivity.on_event(f);
    }

    pub fn on_disconnect(&mut self, f: &'static dyn Fn(TransportEvent)) {
        let _ = self.connectivity.on_event(f);
    }

    pub(crate) fn reply(
        &self,
        rid: u32,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        Ok(self
            .driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Response(Response {
                    rid,
                    wire_expr: WireExpr::from(ke),
                    payload: ResponseBody::Reply(Reply {
                        consolidation: ConsolidationMode::None,
                        payload: PushBody::Put(Put {
                            payload,
                            ..Default::default()
                        }),
                    }),
                    ..Default::default()
                }),
            }))?)
    }

    pub(crate) fn err(
        &self,
        rid: u32,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        Ok(self
            .driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Response(Response {
                    rid,
                    wire_expr: WireExpr::from(ke),
                    payload: ResponseBody::Err(Err {
                        payload,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
            }))?)
    }

    pub(crate) fn reply_del(
        &self,
        rid: u32,
        ke: &keyexpr,
    ) -> core::result::Result<(), SessionError> {
        Ok(self
            .driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Response(Response {
                    rid,
                    wire_expr: WireExpr::from(ke),
                    payload: ResponseBody::Reply(Reply {
                        consolidation: ConsolidationMode::None,
                        payload: PushBody::Del(Del::default()),
                    }),
                    ..Default::default()
                }),
            }))?)
    }

    pub(crate) fn finalize(&self, rid: u32) -> core::result::Result<(), SessionError> {
        if self.state().queryable_callbacks.decrease(rid) {
            self.driver()
                .tx_mut()
                .send(core::iter::once(NetworkMessage {
                    reliability: Reliability::default(),
                    qos: QoS::default(),
                    body: NetworkBody::ResponseFinal(ResponseFinal {
                        rid,
                        ..Default::default()
                    }),
                }))?;
        }

        Ok(())
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
                    .connect($endpoint, config.buff())?,
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
                .init(config.transports().listen($endpoint, config.buff())?),
        )) as &$crate::session::Session<'static, $CONFIG>
    }};
}

pub fn session_connect_ignore_invalid_sn<'res, Config>(
    resources: &'res mut Resources<'res, Config>,
    config: &'res Config,
    endpoint: Endpoint<'_>,
) -> core::result::Result<Session<'res, Config>, TransportLinkError>
where
    Config: ZSessionConfig,
{
    let mut transport = config.transports().connect(endpoint, config.buff())?;
    transport.transport_mut().rx.ignore_invalid_sn();

    Ok(Session::new(resources.init(transport)))
}

pub fn session_listen_ignore_invalid_sn<'res, Config>(
    resources: &'res mut Resources<'res, Config>,
    config: &'res Config,
    endpoint: Endpoint<'_>,
) -> core::result::Result<Session<'res, Config>, TransportLinkError>
where
    Config: ZSessionConfig,
{
    let mut transport = config.transports().listen(endpoint, config.buff())?;
    transport.transport_mut().rx.ignore_invalid_sn();
    Ok(Session::new(resources.init(transport)))
}