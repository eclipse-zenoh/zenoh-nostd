use dyn_utils::{DynObject, storage::RawOrBox};
use zenoh_proto::{
    SessionError,
    exts::QoS,
    fields::{Locality, Reliability, WireExpr},
    keyexpr,
    msgs::*,
};

#[cfg(feature = "alloc")]
use crate::api::callbacks::AllocCallbacks;

use crate::{
    api::{
        arg::QueryableQueryRef,
        callbacks::{DynCallback, FixedCapacityCallbacks, SyncCallback, ZCallbacks},
        query::QueryableQuery,
        session::Session,
    },
    config::ZSessionConfig,
    io::transport::ZTransportLinkTx,
};

pub type FixedCapacityQueryableCallbacks<
    'a,
    Config,
    const CAPACITY: usize,
    Callback = RawOrBox<16>,
> = FixedCapacityCallbacks<'a, QueryableQueryRef<'a, Config>, CAPACITY, Callback>;

#[cfg(feature = "alloc")]
pub type AllocQueryableCallbacks<'a, Config, Callback = RawOrBox<16>> =
    AllocCallbacks<'a, QueryableQueryRef<'a, Config>, Callback>;

pub struct Queryable<Config>
where
    Config: ZSessionConfig + 'static,
{
    session: &'static Session<'static, Config>,
    id: u32,
}

impl<Config> Queryable<Config>
where
    Config: ZSessionConfig,
{
    pub(crate) fn id(&self) -> u32 {
        self.id
    }

    pub fn undeclare(self) -> core::result::Result<(), SessionError> {
        let msg = Declare {
            body: DeclareBody::UndeclareQueryable(UndeclareQueryable {
                id: self.id,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.session
            .state()
            .queryable_callbacks
            .remove(self.id)?;

        self.session
            .driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))?;

        Ok(())
    }
}

type CallbackStorage<Config> =
    <<Config as ZSessionConfig>::QueryableCallbacks<'static> as ZCallbacks<
        'static,
        QueryableQueryRef<'static, Config>,
    >>::Callback;

pub struct QueryableBuilder<Config>
where
    Config: ZSessionConfig + 'static,
{
    session: &'static Session<'static, Config>,
    ke: &'static keyexpr,

    callback: Option<
        DynCallback<
            'static,
            CallbackStorage<Config>,
            QueryableQueryRef<'static, Config>,
        >,
    >,
    allowed_origin: Locality,
}

impl<Config> QueryableBuilder<Config>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(session: &'static Session<'static, Config>, ke: &'static keyexpr) -> Self {
        Self {
            session,
            ke,
            callback: None,
            allowed_origin: Locality::default(),
        }
    }

    pub fn allowed_origin(mut self, locality: Locality) -> Self {
        self.allowed_origin = locality;
        self
    }

    pub fn callback(
        self,
        callback: impl FnMut(&QueryableQuery<'_, 'static, Config>) + 'static,
    ) -> Self {
        Self {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
            allowed_origin: self.allowed_origin,
        }
    }

    pub fn callback_sync(
        self,
        callback: impl FnMut(&QueryableQuery<'_, 'static, Config>) + 'static,
    ) -> Self {
        Self {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
            allowed_origin: self.allowed_origin,
        }
    }

    pub fn finish(self) -> core::result::Result<Queryable<Config>, SessionError> {
        let state = self.session.state();
        let id = state.next();

        if let Some(callback) = self.callback {
            state.queryable_callbacks.drop_timedout();
            state
                .queryable_callbacks
                .insert(id, self.ke, None, callback)?;
        }

        let msg = Declare {
            body: DeclareBody::DeclareQueryable(DeclareQueryable {
                id,
                wire_expr: WireExpr::from(self.ke),
                ..Default::default()
            }),
            ..Default::default()
        };

        self.session
            .driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))?;

        Ok(Queryable {
            id,
            session: self.session,
        })
    }
}

impl<Config> Session<'static, Config>
where
    Config: ZSessionConfig,
{
    pub fn declare_queryable(&'static self, ke: &'static keyexpr) -> QueryableBuilder<Config> {
        QueryableBuilder::new(self, ke)
    }
}