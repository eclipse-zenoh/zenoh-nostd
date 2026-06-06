use dyn_utils::{DynObject, storage::RawOrBox};
use embassy_sync::channel::{DynamicReceiver, DynamicSender};
use zenoh_proto::{
    SessionError,
    exts::QoS,
    fields::{Reliability, WireExpr},
    keyexpr,
    msgs::*,
};

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

pub type FixedCapacityLivelinessCallbacks<
    'a,
    const CAPACITY: usize,
    Callback = RawOrBox<16>,
    Future = RawOrBox<128>,
> = FixedCapacityCallbacks<'a, SampleRef, CAPACITY, Callback, Future>;

#[cfg(feature = "alloc")]
pub type AllocLivelinessCallbacks<'a, Callback = RawOrBox<16>, Future = RawOrBox<128>> =
    AllocCallbacks<'a, SampleRef, Callback, Future>;

pub struct LivelinessToken {
    id: u32,
}

pub struct LivelinessSubscriber<'a, 'res, Config, OwnedSample = (), const CHANNEL: bool = false>
where
    Config: ZSessionConfig,
{
    id: u32,
    ke: &'static keyexpr,
    session: &'a Session<'res, Config>,
    receiver: Option<DynamicReceiver<'res, OwnedSample>>,
}

impl<'a, 'res, Config, OwnedSample, const CHANNEL: bool> LivelinessSubscriber<'a, 'res, Config, OwnedSample, CHANNEL>
where
    Config: ZSessionConfig,
{
    pub async fn undeclare(self) -> core::result::Result<(), SessionError> {
        self.session
            .state()
            .await
            .liveliness_callbacks
            .remove(self.id)?;

        self.session
            .driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Interest(Interest {
                    id: self.id,
                    mode: InterestMode::Final,
                    inner: InterestInner::default(),
                    ..Default::default()
                }),
            }))
            .await?;

        let _ = self.receiver;
        Ok(())
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }
}

type CallbackStorage<'res, Config> =
    <<Config as ZSessionConfig>::LivelinessCallbacks<'res> as ZCallbacks<'res, SampleRef>>::Callback;

type FutureStorage<'res, Config> =
    <<Config as ZSessionConfig>::LivelinessCallbacks<'res> as ZCallbacks<'res, SampleRef>>::Future;

pub struct LivelinessSubscriberBuilder<
    'a,
    'res,
    Config,
    OwnedSample = (),
    const READY: bool = false,
    const CHANNEL: bool = false,
>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'res, Config>,
    ke: &'static keyexpr,
    callback: Option<
        DynCallback<'res, CallbackStorage<'res, Config>, FutureStorage<'res, Config>, SampleRef>,
    >,
    receiver: Option<DynamicReceiver<'res, OwnedSample>>,
}

impl<'a, 'res, Config> LivelinessSubscriberBuilder<'a, 'res, Config, (), false, false>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(session: &'a Session<'res, Config>, ke: &'static keyexpr) -> Self {
        Self {
            session,
            ke,
            callback: None,
            receiver: None,
        }
    }

    pub fn callback(
        self,
        callback: impl AsyncFnMut(&Sample<'_>) + 'res,
    ) -> LivelinessSubscriberBuilder<'a, 'res, Config, (), true, false> {
        LivelinessSubscriberBuilder {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(AsyncCallback::new(callback))),
            receiver: None,
        }
    }

    pub fn callback_sync(
        self,
        callback: impl FnMut(&Sample<'_>) + 'res,
    ) -> LivelinessSubscriberBuilder<'a, 'res, Config, (), true, false> {
        LivelinessSubscriberBuilder {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
            receiver: None,
        }
    }

    pub fn channel<OwnedSample, E>(
        self,
        sender: DynamicSender<'res, OwnedSample>,
        receiver: DynamicReceiver<'res, OwnedSample>,
    ) -> LivelinessSubscriberBuilder<'a, 'res, Config, OwnedSample, true, true>
    where
        OwnedSample: for<'any> TryFrom<&'any Sample<'any>, Error = E>,
    {
        LivelinessSubscriberBuilder {
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

impl<'a, 'res, Config, OwnedSample, const CHANNEL: bool>
    LivelinessSubscriberBuilder<'a, 'res, Config, OwnedSample, true, CHANNEL>
where
    Config: ZSessionConfig,
{
    pub async fn finish(
        self,
    ) -> core::result::Result<LivelinessSubscriber<'a, 'res, Config, OwnedSample, CHANNEL>, SessionError>
    {
        let mut state = self.session.state().await;
        let id = state.next();

        if let Some(callback) = self.callback {
            state
                .liveliness_callbacks
                .insert(id, self.ke, None, callback)?;
        }

        let msg = Interest {
            id,
            mode: InterestMode::CurrentFuture,
            inner: InterestInner {
                options: InterestOptions::TOKENS.options,
                wire_expr: Some(WireExpr::from(self.ke)),
            },
            ..Default::default()
        };

        self.session
            .driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Interest(msg),
            }))
            .await?;

        Ok(LivelinessSubscriber {
            ke: self.ke,
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
    pub async fn declare_liveliness_token(
        &self,
        ke: &keyexpr,
    ) -> core::result::Result<LivelinessToken, SessionError> {
        let mut state = self.state().await;
        let id = state.next();

        let msg = Declare {
            body: DeclareBody::DeclareToken(DeclareToken {
                id,
                wire_expr: WireExpr::from(ke),
            }),
            ..Default::default()
        };

        self.driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))
            .await?;

        Ok(LivelinessToken { id })
    }

    pub async fn undeclare_liveliness_token(
        &self,
        token: LivelinessToken,
    ) -> core::result::Result<(), SessionError> {
        let msg = Declare {
            body: DeclareBody::UndeclareToken(UndeclareToken {
                id: token.id,
                ..Default::default()
            }),
            ..Default::default()
        };

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

    pub fn declare_liveliness_subscriber(
        &self,
        ke: &'static keyexpr,
    ) -> LivelinessSubscriberBuilder<'_, 'res, Config> {
        LivelinessSubscriberBuilder::new(self, ke)
    }

    pub fn liveliness_get<'a>(
        &'a self,
        ke: &'static keyexpr,
    ) -> super::get::GetBuilder<'a, 'res, Config> {
        super::get::GetBuilder::new(self, ke)
    }
}
