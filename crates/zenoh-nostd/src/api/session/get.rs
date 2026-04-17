use core::time::Duration;

use dyn_utils::{DynObject, storage::RawOrBox};
use embassy_futures::select::{Either, select};
use embassy_sync::channel::{DynamicReceiver, DynamicSender};
use embassy_time::{Instant, Timer};
use zenoh_proto::{
    SessionError,
    exts::{QoS, QueryTarget, Value},
    fields::{ConsolidationMode, Reliability, WireExpr},
    keyexpr,
    msgs::{NetworkBody, NetworkMessage, Query, Request, RequestBody},
};

use crate::api::{callbacks::ZDynCallback, query::QueryableQuery};

#[cfg(feature = "alloc")]
use crate::api::callbacks::AllocCallbacks;
use crate::{
    api::{
        arg::GetResponseRef,
        callbacks::{AsyncCallback, DynCallback, FixedCapacityCallbacks, SyncCallback, ZCallbacks},
        session::Session,
    },
    config::ZSessionConfig,
    io::transport::ZTransportLinkTx,
    session::GetResponse,
};

pub type FixedCapacityGetCallbacks<
    'a,
    const CAPACITY: usize,
    Callback = RawOrBox<16>,
    Future = RawOrBox<128>,
> = FixedCapacityCallbacks<'a, GetResponseRef, CAPACITY, Callback, Future>;

#[cfg(feature = "alloc")]
pub type AllocGetCallbacks<'a, Callback = RawOrBox<16>, Future = RawOrBox<128>> =
    AllocCallbacks<'a, GetResponseRef, Callback, Future>;

pub struct GetResponses<'res, OwnedResponse = (), const CHANNEL: bool = false> {
    ke: &'static keyexpr,
    timedout: Instant,
    receiver: Option<DynamicReceiver<'res, OwnedResponse>>,
}

impl<'res, OwnedResponse, const CHANNEL: bool> GetResponses<'res, OwnedResponse, CHANNEL> {
    pub fn cancel(self) {
        todo!()
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }
}

impl<'res, OwnedResponse> GetResponses<'res, OwnedResponse, true> {
    pub fn try_recv(&self) -> Option<OwnedResponse> {
        self.receiver.as_ref().unwrap().try_receive().ok()
    }

    pub async fn recv(&self) -> Option<OwnedResponse> {
        match select(
            Timer::at(self.timedout),
            self.receiver.as_ref().unwrap().receive(),
        )
        .await
        {
            Either::First(_) => None,
            Either::Second(v) => Some(v),
        }
    }
}

type CallbackStorage<'res, Config> =
    <<Config as ZSessionConfig>::GetCallbacks<'res> as ZCallbacks<'res, GetResponseRef>>::Callback;

type FutureStorage<'res, Config> =
    <<Config as ZSessionConfig>::GetCallbacks<'res> as ZCallbacks<'res, GetResponseRef>>::Future;

pub struct GetBuilder<
    'a,
    'res,
    Config,
    OwnedResponse = (),
    const READY: bool = false,
    const CHANNEL: bool = false,
> where
    Config: ZSessionConfig,
{
    pub(crate) session: &'a Session<'res, Config>,
    pub(crate) ke: &'static keyexpr,
    pub(crate) parameters: Option<&'a str>,
    pub(crate) payload: Option<&'a [u8]>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) target: QueryTarget,
    pub(crate) consolidation: ConsolidationMode,
    pub(crate) callback: Option<
        DynCallback<
            'res,
            CallbackStorage<'res, Config>,
            FutureStorage<'res, Config>,
            GetResponseRef,
        >,
    >,
    pub(crate) receiver: Option<DynamicReceiver<'res, OwnedResponse>>,
}

impl<'a, 'res, Config> GetBuilder<'a, 'res, Config, (), false, false>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(session: &'a Session<'res, Config>, ke: &'static keyexpr) -> Self {
        Self {
            session,
            ke,
            parameters: None,
            payload: None,
            timeout: None,
            target: QueryTarget::default(),
            consolidation: ConsolidationMode::default(),
            callback: None,
            receiver: None,
        }
    }

    pub fn callback(
        self,
        callback: impl AsyncFnMut(&GetResponse<'_>) + 'res,
    ) -> GetBuilder<'a, 'res, Config, (), true> {
        GetBuilder {
            session: self.session,
            ke: self.ke,
            parameters: self.parameters,
            payload: self.payload,
            timeout: self.timeout,
            target: self.target,
            consolidation: self.consolidation,
            callback: Some(DynObject::new(AsyncCallback::new(callback))),
            receiver: None,
        }
    }

    pub fn callback_sync(
        self,
        callback: impl FnMut(&GetResponse<'_>) + 'res,
    ) -> GetBuilder<'a, 'res, Config, (), true> {
        GetBuilder {
            session: self.session,
            ke: self.ke,
            parameters: self.parameters,
            payload: self.payload,
            timeout: self.timeout,
            target: self.target,
            consolidation: self.consolidation,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
            receiver: None,
        }
    }

    pub fn channel<OwnedResponse, E>(
        self,
        sender: DynamicSender<'res, OwnedResponse>,
        receiver: DynamicReceiver<'res, OwnedResponse>,
    ) -> GetBuilder<'a, 'res, Config, OwnedResponse, true, true>
    where
        OwnedResponse: for<'any> TryFrom<&'any GetResponse<'any>, Error = E>,
    {
        GetBuilder {
            session: self.session,
            ke: self.ke,
            parameters: self.parameters,
            payload: self.payload,
            timeout: self.timeout,
            target: self.target,
            consolidation: self.consolidation,
            callback: Some(DynObject::new(AsyncCallback::new(
                async move |resp: &'_ GetResponse<'_>| {
                    if let Ok(resp) = OwnedResponse::try_from(resp) {
                        sender.send(resp).await;
                    } else {
                        zenoh_proto::error!(
                            "{}: Couldn't convert to a transferable response",
                            zenoh_proto::zctx!()
                        )
                    }
                },
            ))),
            receiver: Some(receiver),
        }
    }
}

impl<'a, 'res, Config, OwnedResponse, const READY: bool, const CHANNEL: bool>
    GetBuilder<'a, 'res, Config, OwnedResponse, READY, CHANNEL>
where
    Config: ZSessionConfig,
{
    pub fn keyexpr(mut self, ke: &'static keyexpr) -> Self {
        self.ke = ke;
        self
    }

    pub fn parameters(mut self, parameters: &'a str) -> Self {
        self.parameters = Some(parameters);
        self
    }

    pub fn payload(mut self, payload: &'a [u8]) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn target(mut self, target: QueryTarget) -> Self {
        self.target = target;
        self
    }

    pub fn consolidation(mut self, consolidation: ConsolidationMode) -> Self {
        self.consolidation = consolidation;
        self
    }
}

impl<'a, 'res, Config, OwnedResponse, const CHANNEL: bool>
    GetBuilder<'a, 'res, Config, OwnedResponse, true, CHANNEL>
where
    Config: ZSessionConfig,
{
    pub async fn finish(
        self,
    ) -> core::result::Result<GetResponses<'res, OwnedResponse, CHANNEL>, SessionError> {
        let timedout = Instant::now()
            + self
                .timeout
                .unwrap_or(Duration::from_secs(30))
                .try_into()
                .unwrap();

        // Scope the state guard so it is dropped before the network send and
        // the local-dispatch block — holding it across an await on the same
        // mutex causes a deadlock with NoopRawMutex's cooperative lock bit.
        let rid = {
            let mut state = self.session.state().await;
            let rid = state.next();

            if let Some(callback) = self.callback {
                state.get_callbacks.drop_timedout();
                state
                    .get_callbacks
                    .insert(rid, self.ke, Some(timedout), callback)?;
            }
            rid
        };

        let msg = Request {
            id: rid,
            wire_expr: WireExpr::from(self.ke),
            target: self.target,
            payload: RequestBody::Query(Query {
                consolidation: self.consolidation,
                parameters: self.parameters.unwrap_or_default(),
                body: self.payload.map(|p| Value {
                    payload: p,
                    ..Default::default()
                }),
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
                body: NetworkBody::Request(msg),
            }))
            .await?;

        // Local loopback: the router never routes a Request back to the session
        // that issued it.  Mirror what put.rs does for subscribers: dispatch
        // directly to any queryable on this session whose key expression
        // intersects the get key expression.
        {
            let query = QueryableQuery::new(
                self.session,
                rid,
                self.ke,
                self.parameters,
                self.payload,
            );
            let mut state = self.session.state().await;
            let local_count = state.queryable_callbacks.intersects(self.ke).count();
            if local_count > 0 {
                // Flag this get callback so that finalize() (called by the local
                // queryable) does NOT remove it early — remote replies from the
                // network can still arrive and must be delivered.  ResponseFinal
                // from the router will perform the final cleanup.
                state.get_callbacks.set_counter(rid, 1)?;
                state.queryable_callbacks.set_counter(rid, local_count)?;
                for cb in state.queryable_callbacks.intersects(self.ke) {
                    cb.call(&query).await;
                }
            }
        }

        Ok(GetResponses {
            ke: self.ke,
            timedout,
            receiver: self.receiver,
        })
    }
}

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn get(&self, ke: &'static keyexpr) -> GetBuilder<'_, 'res, Config> {
        GetBuilder::new(self, ke)
    }
}
