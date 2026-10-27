use core::time::Duration;

use dyn_utils::{DynObject, storage::RawOrBox};
use embassy_futures::select::{Either, select};
use embassy_sync::channel::{DynamicReceiver, DynamicSender};
use embassy_time::{Instant, Timer};
use zenoh_proto::{
    SessionError,
    exts::{QoS, Value},
    fields::{ConsolidationMode, Reliability, WireExpr},
    keyexpr,
    msgs::{NetworkBody, NetworkMessage, Query, Request, RequestBody},
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
    session::GetResponse,
};

pub type FixedCapacityGetCallbacks<
    'a,
    const CAPACITY: usize,
    Callback = RawOrBox<16>,
    Future = RawOrBox<128>,
    const MAX_KEYEXPR: usize = 128,
> = FixedCapacityCallbacks<'a, GetResponseRef, CAPACITY, Callback, Future, MAX_KEYEXPR>;

#[cfg(feature = "alloc")]
pub type AllocGetCallbacks<'a, Callback = RawOrBox<16>, Future = RawOrBox<128>> =
    AllocCallbacks<'a, GetResponseRef, Callback, Future>;

pub struct GetResponses<
    'a,
    'transport,
    'resources: 'transport,
    Config: ZSessionConfig,
    OwnedResponse = (),
    const CHANNEL: bool = false,
> {
    session: &'a Session<'transport, 'resources, Config>,
    id: u32,
    ke: &'a keyexpr,
    timedout: Instant,
    receiver: Option<DynamicReceiver<'transport, OwnedResponse>>,
}

impl<
    'a,
    'transport,
    'resources: 'transport,
    Config: ZSessionConfig,
    OwnedResponse,
    const CHANNEL: bool,
> GetResponses<'a, 'transport, 'resources, Config, OwnedResponse, CHANNEL>
{
    /// Remove the callback. Late responses for this request are ignored.
    pub async fn cancel(self) -> Result<(), SessionError> {
        Ok(self.session.state().await.get_callbacks.remove(self.id)?)
    }

    pub async fn is_complete(&self) -> bool {
        let mut state = self.session.state().await;
        state.get_callbacks.drop_timedout();
        state.get_callbacks.get(self.id).is_none()
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }
}

impl<'a, 'transport, 'resources: 'transport, Config: ZSessionConfig, OwnedResponse>
    GetResponses<'a, 'transport, 'resources, Config, OwnedResponse, true>
{
    pub fn try_recv(&self) -> Option<OwnedResponse> {
        self.receiver.as_ref().unwrap().try_receive().ok()
    }

    pub async fn recv(&self) -> Option<OwnedResponse> {
        loop {
            if let Some(value) = self.try_recv() {
                return Some(value);
            }
            if self.is_complete().await || Instant::now() >= self.timedout {
                return None;
            }
            match select(
                Timer::after(embassy_time::Duration::from_millis(10)),
                self.receiver.as_ref().unwrap().receive(),
            )
            .await
            {
                Either::First(_) => {}
                Either::Second(value) => return Some(value),
            }
        }
    }
}

type CallbackStorage<'transport, Config> =
    <<Config as ZSessionConfig>::GetCallbacks<'transport> as ZCallbacks<
        'transport,
        GetResponseRef,
    >>::Callback;

type FutureStorage<'transport, Config> =
    <<Config as ZSessionConfig>::GetCallbacks<'transport> as ZCallbacks<
        'transport,
        GetResponseRef,
    >>::Future;

pub struct GetBuilder<
    'a,
    'transport,
    'resources: 'transport,
    Config,
    OwnedResponse = (),
    const READY: bool = false,
    const CHANNEL: bool = false,
> where
    Config: ZSessionConfig,
{
    pub(crate) session: &'a Session<'transport, 'resources, Config>,
    pub(crate) ke: &'a keyexpr,
    pub(crate) parameters: Option<&'a str>,
    pub(crate) payload: Option<&'a [u8]>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) callback: Option<
        DynCallback<
            'transport,
            CallbackStorage<'transport, Config>,
            FutureStorage<'transport, Config>,
            GetResponseRef,
        >,
    >,
    pub(crate) receiver: Option<DynamicReceiver<'transport, OwnedResponse>>,
}

impl<'a, 'transport, 'resources: 'transport, Config>
    GetBuilder<'a, 'transport, 'resources, Config, (), false, false>
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
            parameters: None,
            payload: None,
            timeout: None,
            callback: None,
            receiver: None,
        }
    }

    pub fn callback(
        self,
        callback: impl AsyncFnMut(&GetResponse<'_>) + 'transport,
    ) -> GetBuilder<'a, 'transport, 'resources, Config, (), true> {
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
        callback: impl FnMut(&GetResponse<'_>) + 'transport,
    ) -> GetBuilder<'a, 'transport, 'resources, Config, (), true> {
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
        sender: DynamicSender<'transport, OwnedResponse>,
        receiver: DynamicReceiver<'transport, OwnedResponse>,
    ) -> GetBuilder<'a, 'transport, 'resources, Config, OwnedResponse, true, true>
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

impl<
    'a,
    'transport,
    'resources: 'transport,
    Config,
    OwnedResponse,
    const READY: bool,
    const CHANNEL: bool,
> GetBuilder<'a, 'transport, 'resources, Config, OwnedResponse, READY, CHANNEL>
where
    Config: ZSessionConfig,
{
    pub fn keyexpr(mut self, ke: &'a keyexpr) -> Self {
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

impl<'a, 'transport, 'resources: 'transport, Config, OwnedResponse, const CHANNEL: bool>
    GetBuilder<'a, 'transport, 'resources, Config, OwnedResponse, true, CHANNEL>
where
    Config: ZSessionConfig,
{
    pub async fn finish(
        self,
    ) -> core::result::Result<
        GetResponses<'a, 'transport, 'resources, Config, OwnedResponse, CHANNEL>,
        SessionError,
    > {
        let timedout = Instant::now()
            + self
                .timeout
                .unwrap_or(Duration::from_secs(30))
                .try_into()
                .unwrap();

        let mut state = self.session.state().await;
        let rid = state.next();

        if let Some(callback) = self.callback {
            state.get_callbacks.drop_timedout();
            state
                .get_callbacks
                .insert(rid, self.ke, Some(timedout), callback)?;
        }

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

        let result = self
            .session
            .driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Request(msg),
            }))
            .await;
        if let Err(error) = result {
            state.get_callbacks.remove(rid)?;
            return Err(error.into());
        }

        Ok(GetResponses {
            session: self.session,
            id: rid,
            ke: self.ke,
            timedout,
            receiver: self.receiver,
        })
    }
}

impl<'transport, 'resources: 'transport, Config> Session<'transport, 'resources, Config>
where
    Config: ZSessionConfig,
{
    pub fn get<'a>(&'a self, ke: &'a keyexpr) -> GetBuilder<'a, 'transport, 'resources, Config> {
        GetBuilder::new(self, ke)
    }
}
