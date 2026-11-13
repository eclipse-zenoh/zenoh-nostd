use dyn_utils::{DynObject, storage::RawOrBox};
use embassy_sync::channel::{DynamicReceiver, DynamicSender};
use zenoh_proto::{exts::QoS, fields::*, msgs::*, *};

#[cfg(feature = "alloc")]
use crate::api::callbacks::AllocCallbacks;

use crate::{
    api::{
        arg::SampleRef,
        callbacks::{AsyncCallback, DynCallback, FixedCapacityCallbacks, SyncCallback, ZCallbacks},
        sample::Sample,
        session::Session,
    },
    config::ZSessionConfig,
    io::transport::ZTransportLinkTx,
};

/// Fixed-capacity subscription registry selected through [`ZSessionConfig`].
///
/// `CAPACITY` counts registrations; `MAX_KEYEXPR` limits each copied key in UTF-8
/// bytes (128 by default). `Callback` and `Future` independently choose object
/// storage. A full table or overlong key rejects registration. Selecting `alloc`
/// does not remove these limits; use `AllocSubCallbacks` for heap-backed tables.
pub type FixedCapacitySubCallbacks<
    'a,
    const CAPACITY: usize,
    Callback = RawOrBox<16>,
    Future = RawOrBox<128>,
    const MAX_KEYEXPR: usize = 128,
> = FixedCapacityCallbacks<'a, SampleRef, CAPACITY, Callback, Future, MAX_KEYEXPR>;

#[cfg(feature = "alloc")]
/// Heap-backed subscription registry, available with `alloc`.
///
/// Entry count and copied key length grow with the allocator. Callback/future
/// storage still follows the independently selected storage parameters.
pub type AllocSubCallbacks<'a, Callback = RawOrBox<16>, Future = RawOrBox<128>> =
    AllocCallbacks<'a, SampleRef, Callback, Future>;

/// An active subscription with a borrowed key and optional sample channel.
///
/// The registry owns a copy of the key, but this handle still borrows the caller's
/// key text. Dropping the handle does not unregister its callback or notify peers.
/// Use [`Self::undeclare`] after successful operation completion. After an interrupted
/// operation or transport write failure, discard the whole connection instead.
///
/// # Example
///
/// The caller must poll `session.run()` concurrently. After successful work,
/// explicitly undeclare before reusing the connection.
///
/// ```no_run
/// use zenoh_nostd::session::{Session, SessionError, ZSessionConfig};
/// use zenoh_nostd::session::zenoh::keyexpr;
///
/// async fn subscribe_briefly<C: ZSessionConfig>(
///     session: &Session<'_, '_, C>,
/// ) -> Result<(), SessionError> {
///     let key = keyexpr::new("demo/**")?;
///     let subscriber = session.declare_subscriber(key)
///         .callback_sync(|sample| { let _ = sample.payload(); })
///         .finish().await?;
///     embassy_time::Timer::after_millis(100).await;
///     subscriber.undeclare().await
/// }
/// ```
pub struct Subscriber<
    'a,
    'transport,
    'resources: 'transport,
    Config,
    OwnedSample = (),
    const CHANNEL: bool = false,
> where
    Config: ZSessionConfig,
{
    id: u32,
    ke: &'a keyexpr,
    session: &'a Session<'transport, 'resources, Config>,
    receiver: Option<DynamicReceiver<'transport, OwnedSample>>,
}

impl<'a, 'transport, 'resources: 'transport, Config, OwnedSample, const CHANNEL: bool>
    Subscriber<'a, 'transport, 'resources, Config, OwnedSample, CHANNEL>
where
    Config: ZSessionConfig,
{
    /// Remove the local callback and send a subscriber undeclaration.
    ///
    /// Consumes the handle. Success means the message was written to the link, not
    /// that a router acknowledged it. If the write fails or this future is cancelled,
    /// discard the session and transport. Do not await this from a receive callback.
    pub async fn undeclare(self) -> core::result::Result<(), SessionError> {
        let msg = Declare {
            body: DeclareBody::UndeclareSubscriber(UndeclareSubscriber {
                id: self.id,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.session.state().await.sub_callbacks.remove(self.id)?;

        self.session
            .driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: exts::QoS::default(),
                body: NetworkBody::Declare(msg),
            }))
            .await?;

        Ok(())
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }
}

impl<'a, 'transport, 'resources: 'transport, Config, OwnedSample>
    Subscriber<'a, 'transport, 'resources, Config, OwnedSample, true>
where
    Config: ZSessionConfig,
{
    pub fn try_recv(&self) -> Option<OwnedSample> {
        self.receiver.as_ref().unwrap().try_receive().ok()
    }

    /// Wait for an owned sample from the configured channel.
    ///
    /// The session driver must be polled separately. The channel has no automatic
    /// end-of-subscription notification; this method does not detect disconnection.
    pub async fn recv(&self) -> Option<OwnedSample> {
        Some(self.receiver.as_ref().unwrap().receive().await)
    }
}

type CallbackStorage<'transport, Config> =
    <<Config as ZSessionConfig>::SubCallbacks<'transport> as ZCallbacks<'transport, SampleRef>>::Callback;

type FutureStorage<'transport, Config> =
    <<Config as ZSessionConfig>::SubCallbacks<'transport> as ZCallbacks<'transport, SampleRef>>::Future;

/// Configure sample delivery before declaring a subscription with `finish`.
///
/// Select a callback or channel first. Callback captures live for `'transport`,
/// independently of the borrowed key's lifetime. The session driver awaits callbacks
/// while holding session state; defer further session operations to application code.
pub struct SubscriberBuilder<
    'a,
    'transport,
    'resources: 'transport,
    Config,
    OwnedSample = (),
    const READY: bool = false,
    const CHANNEL: bool = false,
> where
    Config: ZSessionConfig,
{
    session: &'a Session<'transport, 'resources, Config>,
    ke: &'a keyexpr,
    callback: Option<
        DynCallback<
            'transport,
            CallbackStorage<'transport, Config>,
            FutureStorage<'transport, Config>,
            SampleRef,
        >,
    >,
    receiver: Option<DynamicReceiver<'transport, OwnedSample>>,
}

impl<'a, 'transport, 'resources: 'transport, Config>
    SubscriberBuilder<'a, 'transport, 'resources, Config, (), false, false>
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
        callback: impl AsyncFnMut(&Sample<'_>) + 'transport,
    ) -> SubscriberBuilder<'a, 'transport, 'resources, Config, (), true, false> {
        SubscriberBuilder {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(AsyncCallback::new(callback))),
            receiver: None,
        }
    }

    pub fn callback_sync(
        self,
        callback: impl FnMut(&Sample<'_>) + 'transport,
    ) -> SubscriberBuilder<'a, 'transport, 'resources, Config, (), true, false> {
        SubscriberBuilder {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
            receiver: None,
        }
    }

    /// Copy received samples into the supplied application-owned channel.
    ///
    /// Conversion failures are logged and the sample is dropped. A full channel makes
    /// the driver wait for space, delaying all receive/lease processing; drain it
    /// concurrently. For a nonblocking overflow policy, use a callback with `try_send`.
    pub fn channel<OwnedSample, E>(
        self,
        sender: DynamicSender<'transport, OwnedSample>,
        receiver: DynamicReceiver<'transport, OwnedSample>,
    ) -> SubscriberBuilder<'a, 'transport, 'resources, Config, OwnedSample, true, true>
    where
        OwnedSample: for<'any> TryFrom<&'any Sample<'any>, Error = E>,
    {
        SubscriberBuilder {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(AsyncCallback::new(
                async move |resp: &'_ Sample<'_>| {
                    if let Ok(resp) = OwnedSample::try_from(resp) {
                        sender.send(resp).await;
                    } else {
                        zenoh_proto::error!(
                            "{}: Couldn't convert to a transferable sample",
                            zenoh_proto::zctx!()
                        )
                    }
                },
            ))),
            receiver: Some(receiver),
        }
    }
}

impl<'a, 'transport, 'resources: 'transport, Config, OwnedSample, const CHANNEL: bool>
    SubscriberBuilder<'a, 'transport, 'resources, Config, OwnedSample, true, CHANNEL>
where
    Config: ZSessionConfig,
{
    /// Register the callback and send the subscriber declaration.
    ///
    /// Full registry/key limits return an error. A successful send does not acknowledge
    /// router propagation; allow for propagation before expecting remote samples.
    /// On transport write failure or cancellation, discard the connection.
    pub async fn finish(
        self,
    ) -> core::result::Result<
        Subscriber<'a, 'transport, 'resources, Config, OwnedSample, CHANNEL>,
        SessionError,
    > {
        let mut state = self.session.state().await;
        let id = state.next();

        if let Some(callback) = self.callback {
            state.sub_callbacks.insert(id, self.ke, None, callback)?;
        }

        let msg = Declare {
            body: DeclareBody::DeclareSubscriber(DeclareSubscriber {
                id,
                wire_expr: WireExpr::from(self.ke),
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
            state.sub_callbacks.remove(id)?;
            return Err(error.into());
        }

        Ok(Subscriber {
            ke: self.ke,
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
    /// Configure a subscription on `ke`; select delivery and call `finish` to declare it.
    ///
    /// The returned handle borrows `ke`; dropping it does not undeclare the subscription.
    pub fn declare_subscriber<'a>(
        &'a self,
        ke: &'a keyexpr,
    ) -> SubscriberBuilder<'a, 'transport, 'resources, Config> {
        SubscriberBuilder::new(self, ke)
    }
}
