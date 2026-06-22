use dyn_utils::{DynObject, storage::RawOrBox};
use embassy_sync::channel::{DynamicReceiver, DynamicSender};
use zenoh_proto::{
    SessionError,
    exts::QoS,
    fields::{ConsolidationMode, Reliability, WireExpr},
    keyexpr,
    msgs::*,
};

#[cfg(feature = "alloc")]
use crate::api::callbacks::AllocCallbacks;

use crate::{
    api::{
        arg::QueryableQueryRef,
        callbacks::{
            AsyncCallback, DynCallback, FixedCapacityCallbacks, SyncCallback, ZCallbacks,
            ZDynCallback,
        },
        query::QueryableQuery,
        response::GetResponse,
        sample::Sample,
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
    Future = RawOrBox<128>,
> = FixedCapacityCallbacks<'a, QueryableQueryRef<'a, Config>, CAPACITY, Callback, Future>;

#[cfg(feature = "alloc")]
pub type AllocQueryableCallbacks<'a, Config, Callback = RawOrBox<16>, Future = RawOrBox<128>> =
    AllocCallbacks<'a, QueryableQueryRef<'a, Config>, Callback, Future>;

pub struct Queryable<Config, OwnedQuery = (), const CHANNEL: bool = false>
where
    Config: ZSessionConfig + 'static,
    OwnedQuery: 'static,
{
    session: &'static Session<'static, Config>,
    id: u32,
    receiver: Option<DynamicReceiver<'static, OwnedQuery>>,
}

impl<Config, OwnedQuery, const CHANNEL: bool> Queryable<Config, OwnedQuery, CHANNEL>
where
    Config: ZSessionConfig,
{
    pub fn id(&self) -> u32 {
        self.id
    }

    #[allow(dead_code)]
    async fn undeclare(self) -> core::result::Result<(), SessionError> {
        let msg = Declare {
            body: DeclareBody::UndeclareQueryable(UndeclareQueryable {
                id: self.id,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.session
            .state()
            .await
            .queryable_callbacks
            .remove(self.id)?;

        self.session
            .driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))
            .await?;

        todo!("Also stop the channel if any")
    }
}

impl<Config, OwnedQuery> Queryable<Config, OwnedQuery, true>
where
    Config: ZSessionConfig,
{
    pub fn try_recv(&self) -> Option<OwnedQuery> {
        self.receiver.as_ref().unwrap().try_receive().ok()
    }

    pub async fn recv(&self) -> Option<OwnedQuery> {
        Some(self.receiver.as_ref().unwrap().receive().await)
    }
}

type CallbackStorage<Config> =
    <<Config as ZSessionConfig>::QueryableCallbacks<'static> as ZCallbacks<
        'static,
        QueryableQueryRef<'static, Config>,
    >>::Callback;

type FutureStorage<Config> =
    <<Config as ZSessionConfig>::QueryableCallbacks<'static> as ZCallbacks<
        'static,
        QueryableQueryRef<'static, Config>,
    >>::Future;

pub struct QueryableBuilder<
    Config,
    OwnedQuery = (),
    const READY: bool = false,
    const CHANNEL: bool = false,
> where
    Config: ZSessionConfig + 'static,
    OwnedQuery: 'static,
{
    session: &'static Session<'static, Config>,
    ke: &'static keyexpr,

    callback: Option<
        DynCallback<
            'static,
            CallbackStorage<Config>,
            FutureStorage<Config>,
            QueryableQueryRef<'static, Config>,
        >,
    >,
    receiver: Option<DynamicReceiver<'static, OwnedQuery>>,
}

impl<Config> QueryableBuilder<Config, (), false, false>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(session: &'static Session<'static, Config>, ke: &'static keyexpr) -> Self {
        Self {
            session,
            ke,
            callback: None,
            receiver: None,
        }
    }

    pub fn callback(
        self,
        callback: impl AsyncFnMut(&QueryableQuery<'_, 'static, Config>) + 'static,
    ) -> QueryableBuilder<Config, (), true, false> {
        QueryableBuilder {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(AsyncCallback::new(callback))),
            receiver: None,
        }
    }

    pub fn callback_sync(
        self,
        callback: impl FnMut(&QueryableQuery<'_, 'static, Config>) + 'static,
    ) -> QueryableBuilder<Config, (), true, false> {
        QueryableBuilder {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
            receiver: None,
        }
    }
}

impl<Config> QueryableBuilder<Config, (), false, false>
where
    Config: ZSessionConfig,
{
    pub fn channel<OwnedQuery, E>(
        self,
        sender: DynamicSender<'static, OwnedQuery>,
        receiver: DynamicReceiver<'static, OwnedQuery>,
    ) -> QueryableBuilder<Config, OwnedQuery, true, true>
    where
        OwnedQuery: for<'any> TryFrom<
                (
                    &'any QueryableQuery<'any, 'static, Config>,
                    &'static Session<'static, Config>,
                ),
                Error = E,
            >,
    {
        QueryableBuilder {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(AsyncCallback::new(
                async move |resp: &'_ QueryableQuery<'_, 'static, Config>| {
                    if let Ok(resp) = OwnedQuery::try_from((resp, self.session)) {
                        sender.send(resp).await;
                    } else {
                        zenoh_proto::error!(
                            "{}: Couldn't convert to a transferable query",
                            zenoh_proto::zctx!()
                        )
                    }
                },
            ))),
            receiver: Some(receiver),
        }
    }
}

impl<Config, OwnedQuery, const CHANNEL: bool> QueryableBuilder<Config, OwnedQuery, true, CHANNEL>
where
    Config: ZSessionConfig,
{
    pub async fn finish(
        self,
    ) -> core::result::Result<Queryable<Config, OwnedQuery, CHANNEL>, SessionError> {
        let mut state = self.session.state().await;
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
            .driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))
            .await?;

        Ok(Queryable {
            id,
            session: self.session,
            receiver: self.receiver,
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

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    pub async fn undeclare_queryable(&self, id: u32) -> core::result::Result<(), SessionError> {
        let msg = Declare {
            body: DeclareBody::UndeclareQueryable(UndeclareQueryable {
                id,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.state().await.queryable_callbacks.remove(id)?;

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

    pub async fn reply(
        &self,
        rid: u32,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        // Local loopback: if `rid` belongs to a get issued on this very session,
        // deliver the reply directly to the waiting callback instead of routing
        // through the network (the router never sends a Response back to the
        // session that originated the corresponding Request).
        {
            let mut state = self.state().await;
            if let Some(cb) = state.get_callbacks.get(rid) {
                let sample = Sample::new(ke, payload);
                let response = GetResponse::Ok(sample);
                cb.call(&response).await;
                return Ok(());
            }
        }

        // Remote query: send Response over the wire.
        Ok(self
            .driver
            .tx()
            .await
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
            }))
            .await?)
    }

    pub async fn err(
        &self,
        rid: u32,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        // Local loopback for error replies: same logic as reply().
        {
            let mut state = self.state().await;
            if let Some(cb) = state.get_callbacks.get(rid) {
                let sample = Sample::new(ke, payload);
                let response = GetResponse::Err(sample);
                cb.call(&response).await;
                return Ok(());
            }
        }

        Ok(self
            .driver
            .tx()
            .await
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
            }))
            .await?)
    }

    pub async fn finalize(&self, rid: u32) -> core::result::Result<(), SessionError> {
        let counter_reached_zero = {
            let mut state = self.state().await;
            state.queryable_callbacks.decrease(rid)
        };

        if counter_reached_zero {
            // Local loopback: if this rid belongs to a local get, handle cleanup
            // locally instead of sending ResponseFinal over the wire.
            //
            // Additionally, finish() may have set a counter of 1 on get_callbacks
            // to signal that a remote ResponseFinal is expected.  Consume that flag
            // here: if it was set, leave Done-delivery and cleanup to the incoming
            // ResponseFinal so that remote replies are not dropped.  If it was not
            // set, clean up immediately (no remote pending).
            let (is_local, pending_remote) = {
                let mut state = self.state().await;
                let is_local = state.get_callbacks.get(rid).is_some();
                // decrease() returns true only when a counter entry existed and
                // reached zero — i.e. the "pending remote" flag was consumed.
                let pending_remote = if is_local {
                    state.get_callbacks.decrease(rid)
                } else {
                    false
                };
                (is_local, pending_remote)
            };

            if is_local {
                if !pending_remote {
                    // No remote outstanding: deliver Done and remove the callback now.
                    let mut state = self.state().await;
                    let done = GetResponse::Done;
                    if let Some(cb) = state.get_callbacks.get(rid) {
                        cb.call(&done).await;
                    }
                    state.get_callbacks.remove(rid)?;
                }
                // else: pending_remote == true — the incoming ResponseFinal will
                // deliver Done and remove the callback; nothing to do here.
            } else {
                self.driver
                    .tx()
                    .await
                    .send(core::iter::once(NetworkMessage {
                        reliability: Reliability::default(),
                        qos: QoS::default(),
                        body: NetworkBody::ResponseFinal(ResponseFinal {
                            rid,
                            ..Default::default()
                        }),
                    }))
                    .await?;
            }
        }

        Ok(())
    }
}
