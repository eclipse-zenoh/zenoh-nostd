use dyn_utils::{DynObject, storage::RawOrBox};
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
        callbacks::{DynCallback, FixedCapacityCallbacks, SyncCallback, ZCallbacks},
        sample::Sample,
        session::Session,
    },
    config::ZSessionConfig,
    io::transport::ZTransportLinkTx,
};

pub type FixedCapacityLivelinessCallbacks<'a, const CAPACITY: usize, Callback = RawOrBox<16>> =
    FixedCapacityCallbacks<'a, SampleRef, CAPACITY, Callback>;

#[cfg(feature = "alloc")]
pub type AllocLivelinessCallbacks<'a, Callback = RawOrBox<16>> =
    AllocCallbacks<'a, SampleRef, Callback>;

pub struct LivelinessToken {
    id: u32,
}

pub struct LivelinessSubscriber<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    id: u32,
    ke: &'static keyexpr,
    session: &'a Session<'res, Config>,
}

impl<'a, 'res, Config> LivelinessSubscriber<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn undeclare(self) -> core::result::Result<(), SessionError> {
        self.session.state().liveliness_callbacks.remove(self.id)?;

        self.session
            .driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Interest(Interest {
                    id: self.id,
                    mode: InterestMode::Final,
                    inner: InterestInner::default(),
                    ..Default::default()
                }),
            }))?;

        Ok(())
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }
}

type CallbackStorage<'res, Config> =
    <<Config as ZSessionConfig>::LivelinessCallbacks<'res> as ZCallbacks<'res, SampleRef>>::Callback;

pub struct LivelinessSubscriberBuilder<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'res, Config>,
    ke: &'static keyexpr,
    callback: Option<DynCallback<'res, CallbackStorage<'res, Config>, SampleRef>>,
}

impl<'a, 'res, Config> LivelinessSubscriberBuilder<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(session: &'a Session<'res, Config>, ke: &'static keyexpr) -> Self {
        Self {
            session,
            ke,
            callback: None,
        }
    }

    pub fn callback(self, callback: impl FnMut(&Sample<'_>) + 'res) -> Self {
        Self {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
        }
    }

    pub fn callback_sync(self, callback: impl FnMut(&Sample<'_>) + 'res) -> Self {
        Self {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
        }
    }

    pub fn finish(
        self,
    ) -> core::result::Result<LivelinessSubscriber<'a, 'res, Config>, SessionError> {
        let state = self.session.state();
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
            .driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Interest(msg),
            }))?;

        Ok(LivelinessSubscriber {
            ke: self.ke,
            id,
            session: self.session,
        })
    }
}

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn declare_liveliness_token(
        &self,
        ke: &keyexpr,
    ) -> core::result::Result<LivelinessToken, SessionError> {
        let state = self.state();
        let id = state.next();

        let msg = Declare {
            body: DeclareBody::DeclareToken(DeclareToken {
                id,
                wire_expr: WireExpr::from(ke),
            }),
            ..Default::default()
        };

        self.driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))?;

        Ok(LivelinessToken { id })
    }

    pub fn undeclare_liveliness_token(
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

        self.driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))?;

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
