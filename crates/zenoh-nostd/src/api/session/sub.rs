use dyn_utils::{DynObject, storage::RawOrBox};
use zenoh_proto::{exts::QoS, fields::*, msgs::*, *};

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

pub type FixedCapacitySubCallbacks<
    'a,
    const CAPACITY: usize,
    Callback = RawOrBox<16>,
> = FixedCapacityCallbacks<'a, SampleRef, CAPACITY, Callback>;

#[cfg(feature = "alloc")]
pub type AllocSubCallbacks<'a, Callback = RawOrBox<16>> =
    AllocCallbacks<'a, SampleRef, Callback>;

pub struct Subscriber<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    id: u32,
    ke: &'static keyexpr,
    session: &'a Session<'res, Config>,
}

impl<'a, 'res, Config> Subscriber<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn undeclare(self) -> core::result::Result<(), SessionError> {
        let msg = Declare {
            body: DeclareBody::UndeclareSubscriber(UndeclareSubscriber {
                id: self.id,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.session.state().sub_callbacks.remove(self.id)?;

        self.session
            .driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: exts::QoS::default(),
                body: NetworkBody::Declare(msg),
            }))?;

        Ok(())
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }
}

type CallbackStorage<'res, Config> =
    <<Config as ZSessionConfig>::SubCallbacks<'res> as ZCallbacks<'res, SampleRef>>::Callback;

pub struct SubscriberBuilder<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'res, Config>,
    ke: &'static keyexpr,
    callback: Option<
        DynCallback<'res, CallbackStorage<'res, Config>, SampleRef>,
    >,
    allowed_origin: Locality,
}

impl<'a, 'res, Config> SubscriberBuilder<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(session: &'a Session<'res, Config>, ke: &'static keyexpr) -> Self {
        Self {
            session,
            ke,
            callback: None,
            allowed_origin: Locality::default(),
        }
    }

    pub fn allowed_origin(mut self, locality: Locality) -> Self {
        self.allowed_origin = locality;
        self
    }

    pub fn callback(
        self,
        callback: impl FnMut(&Sample<'_>) + 'res,
    ) -> Self {
        Self {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
            allowed_origin: self.allowed_origin,
        }
    }

    pub fn callback_sync(
        self,
        callback: impl FnMut(&Sample<'_>) + 'res,
    ) -> Self {
        Self {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
            allowed_origin: self.allowed_origin,
        }
    }

    pub fn finish(
        self,
    ) -> core::result::Result<Subscriber<'a, 'res, Config>, SessionError> {
        let state = self.session.state();
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
            .driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Declare(msg),
            }))?;

        Ok(Subscriber {
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
    pub fn declare_subscriber(&self, ke: &'static keyexpr) -> SubscriberBuilder<'_, 'res, Config> {
        SubscriberBuilder::new(self, ke)
    }
}