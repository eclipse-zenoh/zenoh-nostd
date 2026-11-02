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
        callbacks::{AsyncCallback, DynCallback, FixedCapacityCallbacks, SyncCallback, ZCallbacks},
        query::QueryableQuery,
        session::{Origin, Session},
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
                async move |query: &'_ QueryableQuery<'_, 'static, Config>| {
                    if let Ok(owned) = OwnedQuery::try_from((query, self.session)) {
                        // The receiver now owns the query and must finalize it.
                        query.hand_off();
                        sender.send(owned).await;
                    } else {
                        // Not handed off, so the session finalizes it.
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
            self.session
                .local_queryables
                .set(self.session.local_queryables.get() + 1);
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

/// Request ids from the network and from the session's own gets are
/// separate id spaces, but in-flight queries of both kinds are tracked in one
/// table. Local ones are keyed with the top bit set. A remote id only reaches
/// that range after 2^31 requests on one link.
const LOCAL_QUERY: u32 = 1 << 31;

/// The key under which an in-flight query is tracked.
pub(crate) fn query_key(rid: u32, origin: Origin) -> u32 {
    match origin {
        Origin::Remote => rid,
        Origin::Local => rid | LOCAL_QUERY,
    }
}

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    /// Sends `response` back to where its query came from: over the network
    /// for a remote query, through the loopback for a local one.
    async fn respond(
        &self,
        origin: Origin,
        response: Response<'_>,
    ) -> core::result::Result<(), SessionError> {
        match origin {
            Origin::Remote => Ok(self
                .driver
                .tx()
                .await
                .send(core::iter::once(NetworkMessage {
                    reliability: Reliability::default(),
                    qos: QoS::default(),
                    body: NetworkBody::Response(response),
                }))
                .await?),
            Origin::Local => {
                if self.loopback.push(NetworkBodyRef::Response(&response)) {
                    Ok(())
                } else {
                    Err(SessionError::LoopbackFull)
                }
            }
        }
    }

    pub(crate) async fn reply(
        &self,
        rid: u32,
        origin: Origin,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.respond(
            origin,
            Response {
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
            },
        )
        .await
    }

    pub(crate) async fn err(
        &self,
        rid: u32,
        origin: Origin,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.respond(
            origin,
            Response {
                rid,
                wire_expr: WireExpr::from(ke),
                payload: ResponseBody::Err(Err {
                    payload,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
    }

    /// Finalizes one owner of query `rid`, sending `ResponseFinal` once
    /// every owner has finalized.
    pub(crate) async fn finalize(
        &self,
        rid: u32,
        origin: Origin,
    ) -> core::result::Result<(), SessionError> {
        let done = self
            .state()
            .await
            .queryable_callbacks
            .decrease(query_key(rid, origin));
        if done {
            self.response_final(rid, origin).await?;
        }

        Ok(())
    }

    /// Tells the querier that query `rid` gets no more replies.
    pub(crate) async fn response_final(
        &self,
        rid: u32,
        origin: Origin,
    ) -> core::result::Result<(), SessionError> {
        let msg = ResponseFinal {
            rid,
            ..Default::default()
        };

        match origin {
            Origin::Remote => Ok(self
                .driver
                .tx()
                .await
                .send(core::iter::once(NetworkMessage {
                    reliability: Reliability::default(),
                    qos: QoS::default(),
                    body: NetworkBody::ResponseFinal(msg),
                }))
                .await?),
            Origin::Local => {
                if self.loopback.push(NetworkBodyRef::ResponseFinal(&msg)) {
                    Ok(())
                } else {
                    Err(SessionError::LoopbackFull)
                }
            }
        }
    }
}
