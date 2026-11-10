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

/// Fixed-capacity queryable registry selected through [`ZSessionConfig`].
///
/// `CAPACITY` counts registrations; `MAX_KEYEXPR` limits each copied key in UTF-8
/// bytes (128 by default). `Callback` and `Future` independently choose object
/// storage. A full table or overlong key rejects registration. Selecting `alloc`
/// does not remove these limits; use `AllocQueryableCallbacks` for heap-backed tables.
pub type FixedCapacityQueryableCallbacks<
    'transport,
    'resources,
    Config,
    const CAPACITY: usize,
    Callback = RawOrBox<16>,
    Future = RawOrBox<128>,
    const MAX_KEYEXPR: usize = 128,
> = FixedCapacityCallbacks<
    'transport,
    QueryableQueryRef<'transport, 'resources, Config>,
    CAPACITY,
    Callback,
    Future,
    MAX_KEYEXPR,
>;

#[cfg(feature = "alloc")]
/// Heap-backed queryable registry, available with `alloc`.
///
/// Entry count and copied key length grow with the allocator. Callback/future
/// storage still follows the independently selected storage parameters.
pub type AllocQueryableCallbacks<
    'transport,
    'resources,
    Config,
    Callback = RawOrBox<16>,
    Future = RawOrBox<128>,
> = AllocCallbacks<'transport, QueryableQueryRef<'transport, 'resources, Config>, Callback, Future>;

/// An active queryable declaration with optional queued incoming queries.
///
/// Dropping this handle does not undeclare the callback or finalize accepted
/// requests. Finalize each accepted handler's work separately, then call
/// [`Self::undeclare`] after successful operation completion. Undeclaration removes
/// the callback but leaves incoming-request counters for outstanding work intact.
pub struct Queryable<
    'a,
    'transport,
    'resources: 'transport,
    Config,
    OwnedQuery = (),
    const CHANNEL: bool = false,
> where
    Config: ZSessionConfig + 'resources,
    OwnedQuery: 'transport,
{
    session: &'a Session<'transport, 'resources, Config>,
    id: u32,
    receiver: Option<DynamicReceiver<'transport, OwnedQuery>>,
}

impl<'a, 'transport, 'resources: 'transport, Config, OwnedQuery, const CHANNEL: bool>
    Queryable<'a, 'transport, 'resources, Config, OwnedQuery, CHANNEL>
where
    Config: ZSessionConfig,
{
    /// Remove the callback and send a queryable undeclaration.
    ///
    /// Finalize already accepted requests separately. This consumes the handle but
    /// does not clear their counters. On write error or cancellation, discard the
    /// connection. This method acquires session state: do not await inside a callback.
    pub async fn undeclare(self) -> core::result::Result<(), SessionError> {
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

        Ok(())
    }
}

impl<'a, 'transport, 'resources: 'transport, Config, OwnedQuery>
    Queryable<'a, 'transport, 'resources, Config, OwnedQuery, true>
where
    Config: ZSessionConfig,
{
    pub fn try_recv(&self) -> Option<OwnedQuery> {
        self.receiver.as_ref().unwrap().try_receive().ok()
    }

    /// Wait for a query from the configured channel while a separate task polls the driver.
    ///
    /// No automatic channel closure is signalled on undeclaration/disconnection.
    /// The consumer is responsible for replying and finalizing each accepted request.
    pub async fn recv(&self) -> Option<OwnedQuery> {
        Some(self.receiver.as_ref().unwrap().receive().await)
    }
}

type CallbackStorage<'transport, 'resources, Config> =
    <<Config as ZSessionConfig>::QueryableCallbacks<'transport, 'resources> as ZCallbacks<
        'transport,
        QueryableQueryRef<'transport, 'resources, Config>,
    >>::Callback;

type FutureStorage<'transport, 'resources, Config> =
    <<Config as ZSessionConfig>::QueryableCallbacks<'transport, 'resources> as ZCallbacks<
        'transport,
        QueryableQueryRef<'transport, 'resources, Config>,
    >>::Future;

/// Configure a queryable callback/channel before declaring it with `finish`.
///
/// Callbacks execute under the session-state lock. Queue owned request data and
/// its request ID for application-side replies/finalization; never await
/// [`Session::finalize`] in the callback. Reject work that cannot be queued with
/// [`QueryableQuery::reject`]. Callback-based delivery supports scoped sessions;
/// the built-in owned-query channel currently requires a static session.
pub struct QueryableBuilder<
    'a,
    'transport,
    'resources: 'transport,
    Config,
    OwnedQuery = (),
    const READY: bool = false,
    const CHANNEL: bool = false,
> where
    Config: ZSessionConfig + 'resources,
    OwnedQuery: 'transport,
{
    session: &'a Session<'transport, 'resources, Config>,
    ke: &'a keyexpr,

    callback: Option<
        DynCallback<
            'transport,
            CallbackStorage<'transport, 'resources, Config>,
            FutureStorage<'transport, 'resources, Config>,
            QueryableQueryRef<'transport, 'resources, Config>,
        >,
    >,
    receiver: Option<DynamicReceiver<'transport, OwnedQuery>>,
}

impl<'a, 'transport, 'resources: 'transport, Config>
    QueryableBuilder<'a, 'transport, 'resources, Config, (), false, false>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(
        session: &'a Session<'transport, 'resources, Config>,
        ke: &'a keyexpr,
    ) -> Self {
        Self {
            session,
            ke,
            callback: None,
            receiver: None,
        }
    }

    pub fn callback(
        self,
        callback: impl AsyncFnMut(&QueryableQuery<'_, 'transport, 'resources, Config>) + 'transport,
    ) -> QueryableBuilder<'a, 'transport, 'resources, Config, (), true, false> {
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
        callback: impl FnMut(&QueryableQuery<'_, 'transport, 'resources, Config>) + 'transport,
    ) -> QueryableBuilder<'a, 'transport, 'resources, Config, (), true, false> {
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

impl<Config> QueryableBuilder<'static, 'static, 'static, Config, (), false, false>
where
    Config: ZSessionConfig,
{
    /// Queue owned queries using a static session and application-owned channel.
    ///
    /// The built-in conversion carries a static session reference. A full channel
    /// waits for space inside the receive callback and stalls the driver. Conversion
    /// failures are logged; they are not automatically rejected or finalized. Prefer
    /// a custom callback using `try_send` and [`QueryableQuery::reject`] when either
    /// conversion or queue capacity may fail. Consumers must finalize accepted work.
    pub fn channel<OwnedQuery, E>(
        self,
        sender: DynamicSender<'static, OwnedQuery>,
        receiver: DynamicReceiver<'static, OwnedQuery>,
    ) -> QueryableBuilder<'static, 'static, 'static, Config, OwnedQuery, true, true>
    where
        OwnedQuery: for<'any> TryFrom<
                (
                    &'any QueryableQuery<'any, 'static, 'static, Config>,
                    &'static Session<'static, 'static, Config>,
                ),
                Error = E,
            >,
    {
        QueryableBuilder {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(AsyncCallback::new(
                async move |resp: &'_ QueryableQuery<'_, 'static, 'static, Config>| {
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

impl<'a, 'transport, 'resources: 'transport, Config, OwnedQuery, const CHANNEL: bool>
    QueryableBuilder<'a, 'transport, 'resources, Config, OwnedQuery, true, CHANNEL>
where
    Config: ZSessionConfig,
{
    /// Register the callback and send a queryable declaration.
    ///
    /// Full registry/key limits return an error. Transport write errors or cancellation
    /// require connection teardown; a successful send is not a router acknowledgment.
    pub async fn finish(
        self,
    ) -> core::result::Result<
        Queryable<'a, 'transport, 'resources, Config, OwnedQuery, CHANNEL>,
        SessionError,
    > {
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

        let result = self
            .session
            .driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))
            .await;
        if let Err(error) = result {
            state.queryable_callbacks.remove(id)?;
            return Err(error.into());
        }

        Ok(Queryable {
            id,
            session: self.session,
            receiver: self.receiver,
        })
    }
}

impl<'transport, 'resources: 'transport, Config> Session<'transport, 'resources, Config>
where
    Config: ZSessionConfig,
{
    /// Configure query handling for `ke`; select delivery and call `finish` to declare it.
    pub fn declare_queryable<'a>(
        &'a self,
        ke: &'a keyexpr,
    ) -> QueryableBuilder<'a, 'transport, 'resources, Config> {
        QueryableBuilder::new(self, ke)
    }
}

impl<'transport, 'resources: 'transport, Config> Session<'transport, 'resources, Config>
where
    Config: ZSessionConfig,
{
    /// Send a successful reply for an accepted incoming request ID.
    ///
    /// Does not finalize the request. After all replies for this handler are sent,
    /// call [`Self::finalize`] once outside the receive callback. Write errors or
    /// cancellation require dropping the session and transport.
    pub async fn reply(
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

    /// Send an error reply for an accepted request; it still needs [`Self::finalize`].
    ///
    /// As with successful replies, an interrupted or failed write ends the connection.
    pub async fn err(
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

    /// Finish one accepted handler's work for `rid`.
    ///
    /// Decrement the shared incoming-request counter and send a final response when
    /// all matching handlers have finalized or rejected. Call exactly once for each
    /// accepted handler: unlike a query handle's `finalize`, this ID-based method does
    /// not track per-handler completion and repeated calls can finish another handler.
    ///
    /// Acquires session state. Call from application code after the receive callback
    /// returns; awaiting it inside a callback deadlocks on the held state lock. After
    /// write error or cancellation, discard the connection rather than retrying.
    pub async fn finalize(&self, rid: u32) -> core::result::Result<(), SessionError> {
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
struct QueryableAsyncCallback<'transport, 'resources: 'transport, Config, F>(
    F,
    core::marker::PhantomData<(&'transport (), &'resources Config)>,
);
impl<'transport, 'resources: 'transport, Config: ZSessionConfig, F> ZCallback
    for QueryableAsyncCallback<'transport, 'resources, Config, F>
where
    F: AsyncFnMut(&QueryableQuery<'_, 'transport, 'resources, Config>),
{
    type Arg = QueryableQueryRef<'transport, 'resources, Config>;
    async fn call(&mut self, arg: <Self::Arg as crate::api::arg::ZArg>::Of<'_>) {
        (self.0)(arg).await
    }
}
struct QueryableSyncCallback<'transport, 'resources: 'transport, Config, F>(
    F,
    core::marker::PhantomData<(&'transport (), &'resources Config)>,
);
impl<'transport, 'resources: 'transport, Config: ZSessionConfig, F> ZCallback
    for QueryableSyncCallback<'transport, 'resources, Config, F>
where
    F: FnMut(&QueryableQuery<'_, 'transport, 'resources, Config>),
{
    type Arg = QueryableQueryRef<'transport, 'resources, Config>;
    #[dyn_utils::sync]
    async fn call(&mut self, arg: <Self::Arg as crate::api::arg::ZArg>::Of<'_>) {
        (self.0)(arg)
    }
}
