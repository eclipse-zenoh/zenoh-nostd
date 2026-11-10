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

/// Fixed-capacity pending-get registry selected through [`ZSessionConfig`].
///
/// `CAPACITY` counts registrations; `MAX_KEYEXPR` limits each copied key in UTF-8
/// bytes (128 by default). `Callback` and `Future` independently choose object
/// storage. A full table or overlong key rejects registration. Selecting `alloc`
/// does not remove these limits; use `AllocGetCallbacks` for heap-backed tables.
pub type FixedCapacityGetCallbacks<
    'a,
    const CAPACITY: usize,
    Callback = RawOrBox<16>,
    Future = RawOrBox<128>,
    const MAX_KEYEXPR: usize = 128,
> = FixedCapacityCallbacks<'a, GetResponseRef, CAPACITY, Callback, Future, MAX_KEYEXPR>;

#[cfg(feature = "alloc")]
/// Heap-backed pending-get registry, available with `alloc`.
///
/// Entry count and copied key length grow with the allocator. Callback/future
/// storage still follows the independently selected storage parameters.
pub type AllocGetCallbacks<'a, Callback = RawOrBox<16>, Future = RawOrBox<128>> =
    AllocCallbacks<'a, GetResponseRef, Callback, Future>;

/// Track one outgoing query and optional owned reply channel.
///
/// The handle borrows the key even though the registry owns a copy. Dropping it
/// does not cancel its callback. A final response removes the registration;
/// timeouts are cleaned up when completion is checked or another get is inserted.
/// Use [`Self::cancel`] to release a completed-send request early.
///
/// # Example
///
/// Keep the session driver polled in parallel; this function only sends the get
/// and observes its completion. Replies are borrowed during callback execution.
///
/// ```no_run
/// use core::time::Duration;
/// use zenoh_nostd::session::{Session, SessionError, ZSessionConfig};
/// use zenoh_nostd::session::zenoh::keyexpr;
///
/// async fn query_once<C: ZSessionConfig>(
///     session: &Session<'_, '_, C>,
/// ) -> Result<(), SessionError> {
///     let key = keyexpr::new("demo/example")?;
///     let replies = session.get(key)
///         .timeout(Duration::from_secs(2))
///         .callback_sync(|reply| { let _ = reply; })
///         .finish().await?;
///     while !replies.is_complete().await {
///         embassy_time::Timer::after_millis(10).await;
///     }
///     Ok(())
/// }
/// ```
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
    /// Remove the local callback so late replies for this request are ignored.
    ///
    /// Does not send a remote cancellation or undo an interrupted transport write.
    /// Use after the get's `finish` has completed; cancelling that send future
    /// requires dropping the whole connection. Already queued replies are not
    /// removed from an application-owned channel. Do not await inside a callback.
    pub async fn cancel(self) -> Result<(), SessionError> {
        Ok(self.session.state().await.get_callbacks.remove(self.id)?)
    }

    /// Whether the callback has been removed by a final response, timeout, or cleanup.
    ///
    /// Performs timeout cleanup for the get registry. Completion is independent of
    /// whether the owned reply channel still contains data. This method acquires
    /// session state and must not be awaited from a receive callback.
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

    /// Receive a queued reply, or `None` after completion/timeout when none is queued.
    ///
    /// Buffered replies are returned before checking completion. Poll the session
    /// driver concurrently; this method alone does not receive network traffic.
    /// Completion checks poll at 10 ms intervals while waiting for channel input.
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

/// Configure one outgoing query; select a callback/channel and call `finish`.
///
/// Replies may arrive multiple times before a final response. Callback captures
/// must live for `'transport`; input key, parameters and payload are borrowed.
/// Callbacks execute under the driver's session-state lock; defer state-changing
/// operations until after the callback returns.
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

    /// Convert replies to owned values and send them through the supplied channel.
    ///
    /// Conversion errors drop a reply after logging. A full channel blocks the driver
    /// until drained. Consume concurrently, or choose a callback with a nonblocking
    /// queue policy when driver progress must not depend on channel capacity.
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

    /// Set the request timeout (30 seconds by default), measured from `finish`.
    ///
    /// The deadline is cleaned up lazily when completion is checked or another get
    /// is registered; it is not a dedicated timer that cancels remote processing.
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
    /// Register reply delivery and write the query request to the link.
    ///
    /// Returns a handle after sending, without waiting for replies. Full registry or
    /// key limits fail registration. Transport write errors or cancelling this future
    /// require discarding the session and transport, even if local state was rolled back.
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
    /// Configure a query on `ke`; select reply delivery and call `finish` to send it.
    pub fn get<'a>(&'a self, ke: &'a keyexpr) -> GetBuilder<'a, 'transport, 'resources, Config> {
        GetBuilder::new(self, ke)
    }
}
