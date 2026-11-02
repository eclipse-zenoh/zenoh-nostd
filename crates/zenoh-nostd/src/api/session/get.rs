use core::{cell::Cell, time::Duration};

use dyn_utils::{DynObject, storage::RawOrBox};
use embassy_futures::select::{Either, select};
use embassy_sync::channel::{DynamicReceiver, DynamicSender};
use embassy_time::{Instant, Timer};
use zenoh_proto::{
    SessionError,
    exts::{QoS, Value},
    fields::{ConsolidationMode, Reliability, WireExpr},
    keyexpr,
    msgs::{NetworkBody, NetworkBodyRef, NetworkMessage, Query, Request, RequestBody},
};

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
    session::{GetResponse, ZOwnedGetResponse},
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
    /// Set once the end-of-get marker has been received.
    finished: Cell<bool>,
}

impl<'res, OwnedResponse, const CHANNEL: bool> GetResponses<'res, OwnedResponse, CHANNEL> {
    pub fn cancel(self) {
        todo!()
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }
}

impl<'res, OwnedResponse> GetResponses<'res, OwnedResponse, true>
where
    OwnedResponse: ZOwnedGetResponse,
{
    /// Returns the next reply if one is ready.
    ///
    /// Returns `None` if no reply is ready, or once the get has completed.
    pub fn try_recv(&self) -> Option<OwnedResponse> {
        if self.finished.get() {
            return None;
        }

        let response = self.receiver.as_ref().unwrap().try_receive().ok()?;
        self.unless_final(response)
    }

    /// Waits for the next reply.
    ///
    /// Returns `None` once the get has completed (the router sent its final
    /// response) or timed out.
    pub async fn recv(&self) -> Option<OwnedResponse> {
        if self.finished.get() {
            return None;
        }

        match select(
            Timer::at(self.timedout),
            self.receiver.as_ref().unwrap().receive(),
        )
        .await
        {
            Either::First(_) => None,
            Either::Second(response) => self.unless_final(response),
        }
    }

    fn unless_final(&self, response: OwnedResponse) -> Option<OwnedResponse> {
        if response.is_final() {
            self.finished.set(true);
            None
        } else {
            Some(response)
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

        let mut state = self.session.state().await;
        let rid = state.next();

        let msg = Request {
            id: rid,
            wire_expr: WireExpr::from(self.ke),
            payload: RequestBody::Query(Query {
                consolidation: ConsolidationMode::None,
                parameters: self.parameters.unwrap_or_default(),
                body: self.payload.map(|p| Value {
                    payload: p,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        // A router never routes the request back to this session, so the
        // session hands it to its own queryables.
        let session = self.session;
        let wants_local = session.local_queryables.get() > 0 && session.loopback.enabled();
        let local = wants_local && session.loopback.push(NetworkBodyRef::Request(&msg));
        if wants_local && !local {
            zenoh_proto::warn!(
                "{}: {}, not delivering get on {} to local queryables",
                zenoh_proto::zctx!(),
                SessionError::LoopbackFull,
                self.ke.as_str()
            );
        }

        if let Some(callback) = self.callback {
            state.get_callbacks.drop_timedout();
            state
                .get_callbacks
                .insert(rid, self.ke, Some(timedout), callback)?;
            // The get completes once every destination has sent its
            // ResponseFinal: the network, and the session if it got the
            // request too. The state lock is held, so none can arrive first.
            state
                .get_callbacks
                .set_counter(rid, if local { 2 } else { 1 })?;
        }

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

        Ok(GetResponses {
            ke: self.ke,
            timedout,
            receiver: self.receiver,
            finished: Cell::new(false),
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
