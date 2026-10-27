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
        callbacks::{AsyncCallback, DynCallback, FixedCapacityCallbacks, ZCallback, ZCallbacks},
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
    Future = RawOrBox<128>,
    const MAX_KEYEXPR: usize = 128,
> = FixedCapacityCallbacks<
    'a,
    QueryableQueryRef<'a, Config>,
    CAPACITY,
    Callback,
    Future,
    MAX_KEYEXPR,
>;

#[cfg(feature = "alloc")]
pub type AllocQueryableCallbacks<'a, Config, Callback = RawOrBox<16>, Future = RawOrBox<128>> =
    AllocCallbacks<'a, QueryableQueryRef<'a, Config>, Callback, Future>;

pub struct Queryable<'a, 'res, Config, OwnedQuery = (), const CHANNEL: bool = false>
where
    Config: ZSessionConfig + 'res,
    OwnedQuery: 'res,
{
    session: &'a Session<'res, Config>,
    id: u32,
    receiver: Option<DynamicReceiver<'res, OwnedQuery>>,
}

impl<'a, 'res, Config, OwnedQuery, const CHANNEL: bool>
    Queryable<'a, 'res, Config, OwnedQuery, CHANNEL>
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

impl<'a, 'res, Config, OwnedQuery> Queryable<'a, 'res, Config, OwnedQuery, true>
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

type CallbackStorage<'res, Config> =
    <<Config as ZSessionConfig>::QueryableCallbacks<'res> as ZCallbacks<
        'res,
        QueryableQueryRef<'res, Config>,
    >>::Callback;

type FutureStorage<'res, Config> =
    <<Config as ZSessionConfig>::QueryableCallbacks<'res> as ZCallbacks<
        'res,
        QueryableQueryRef<'res, Config>,
    >>::Future;

pub struct QueryableBuilder<
    'a,
    'res,
    Config,
    OwnedQuery = (),
    const READY: bool = false,
    const CHANNEL: bool = false,
> where
    Config: ZSessionConfig + 'res,
    OwnedQuery: 'res,
{
    session: &'a Session<'res, Config>,
    ke: &'a keyexpr,

    callback: Option<
        DynCallback<
            'res,
            CallbackStorage<'res, Config>,
            FutureStorage<'res, Config>,
            QueryableQueryRef<'res, Config>,
        >,
    >,
    receiver: Option<DynamicReceiver<'res, OwnedQuery>>,
}

impl<'a, 'res, Config> QueryableBuilder<'a, 'res, Config, (), false, false>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(session: &'a Session<'res, Config>, ke: &'a keyexpr) -> Self {
        Self {
            session,
            ke,
            callback: None,
            receiver: None,
        }
    }

    pub fn callback(
        self,
        callback: impl AsyncFnMut(&QueryableQuery<'_, 'res, Config>) + 'res,
    ) -> QueryableBuilder<'a, 'res, Config, (), true, false> {
        QueryableBuilder {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(QueryableAsyncCallback(
                callback,
                core::marker::PhantomData,
            ))),
            receiver: None,
        }
    }

    pub fn callback_sync(
        self,
        callback: impl FnMut(&QueryableQuery<'_, 'res, Config>) + 'res,
    ) -> QueryableBuilder<'a, 'res, Config, (), true, false> {
        QueryableBuilder {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(QueryableSyncCallback(
                callback,
                core::marker::PhantomData,
            ))),
            receiver: None,
        }
    }
}

impl<Config> QueryableBuilder<'static, 'static, Config, (), false, false>
where
    Config: ZSessionConfig,
{
    pub fn channel<OwnedQuery, E>(
        self,
        sender: DynamicSender<'static, OwnedQuery>,
        receiver: DynamicReceiver<'static, OwnedQuery>,
    ) -> QueryableBuilder<'static, 'static, Config, OwnedQuery, true, true>
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

impl<'a, 'res, Config, OwnedQuery, const CHANNEL: bool>
    QueryableBuilder<'a, 'res, Config, OwnedQuery, true, CHANNEL>
where
    Config: ZSessionConfig,
{
    pub async fn finish(
        self,
    ) -> core::result::Result<Queryable<'a, 'res, Config, OwnedQuery, CHANNEL>, SessionError> {
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

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn declare_queryable<'a>(&'a self, ke: &'a keyexpr) -> QueryableBuilder<'a, 'res, Config> {
        QueryableBuilder::new(self, ke)
    }
}

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) async fn reply(
        &self,
        rid: u32,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
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

    pub(crate) async fn err(
        &self,
        rid: u32,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
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

    pub(crate) async fn finalize(&self, rid: u32) -> core::result::Result<(), SessionError> {
        if self.state().await.queryable_callbacks.decrease(rid) {
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

        Ok(())
    }
}

// Spell out the argument type here: the generic callback wrapper's GAT/HRTB
// bound currently implies a static lifetime for a query's session resources.
struct QueryableAsyncCallback<'res, Config, F>(F, core::marker::PhantomData<&'res Config>);
impl<'res, Config: ZSessionConfig, F> ZCallback for QueryableAsyncCallback<'res, Config, F>
where
    F: AsyncFnMut(&QueryableQuery<'_, 'res, Config>),
{
    type Arg = QueryableQueryRef<'res, Config>;
    async fn call(&mut self, arg: <Self::Arg as crate::api::arg::ZArg>::Of<'_>) {
        (self.0)(arg).await
    }
}
struct QueryableSyncCallback<'res, Config, F>(F, core::marker::PhantomData<&'res Config>);
impl<'res, Config: ZSessionConfig, F> ZCallback for QueryableSyncCallback<'res, Config, F>
where
    F: FnMut(&QueryableQuery<'_, 'res, Config>),
{
    type Arg = QueryableQueryRef<'res, Config>;
    #[dyn_utils::sync]
    async fn call(&mut self, arg: <Self::Arg as crate::api::arg::ZArg>::Of<'_>) {
        (self.0)(arg)
    }
}
