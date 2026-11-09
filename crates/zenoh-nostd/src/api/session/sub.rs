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

pub type FixedCapacitySubCallbacks<
    'a,
    const CAPACITY: usize,
    Callback = RawOrBox<16>,
    Future = RawOrBox<128>,
    const MAX_KEYEXPR: usize = 128,
> = FixedCapacityCallbacks<'a, SampleRef, CAPACITY, Callback, Future, MAX_KEYEXPR>;

#[cfg(feature = "alloc")]
pub type AllocSubCallbacks<'a, Callback = RawOrBox<16>, Future = RawOrBox<128>> =
    AllocCallbacks<'a, SampleRef, Callback, Future>;

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
    #[allow(dead_code)]
    async fn undeclare(self) -> core::result::Result<(), SessionError> {
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

        todo!("Also stop the channel if any")
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

    pub async fn recv(&self) -> Option<OwnedSample> {
        Some(self.receiver.as_ref().unwrap().receive().await)
    }
}

type CallbackStorage<'transport, Config> =
    <<Config as ZSessionConfig>::SubCallbacks<'transport> as ZCallbacks<'transport, SampleRef>>::Callback;

type FutureStorage<'transport, Config> =
    <<Config as ZSessionConfig>::SubCallbacks<'transport> as ZCallbacks<'transport, SampleRef>>::Future;

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
    pub fn declare_subscriber<'a>(
        &'a self,
        ke: &'a keyexpr,
    ) -> SubscriberBuilder<'a, 'transport, 'resources, Config> {
        SubscriberBuilder::new(self, ke)
    }
}
