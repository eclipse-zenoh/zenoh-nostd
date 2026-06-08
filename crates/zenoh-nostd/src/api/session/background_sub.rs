use dyn_utils::DynObject;

use zenoh_proto::{
    SessionError,
    exts::QoS,
    fields::{Reliability, WireExpr},
    keyexpr,
    msgs::*,
};

#[cfg(feature = "acl")]
use crate::api::acl::AclOperation;

#[cfg(feature = "alloc")]
use crate::api::callbacks::AllocCallbacks;

use crate::{
    api::{
        arg::SampleRef,
        callbacks::{DynCallback, SyncCallback, ZCallbacks},
        sample::Sample,
        session::Session,
    },
    config::ZSessionConfig,
    io::transport::ZTransportLinkTx,
};

type SubCallbackStorage<'res, Config> =
    <<Config as ZSessionConfig>::SubCallbacks<'res> as ZCallbacks<'res, SampleRef>>::Callback;

type LivelinessCallbackStorage<'res, Config> = <<Config as ZSessionConfig>::LivelinessCallbacks<
    'res,
> as ZCallbacks<'res, SampleRef>>::Callback;

/// A fire-and-forget subscriber that processes samples via callback.
///
/// Unlike the regular `Subscriber`, this type does not provide a receiver
/// channel. Samples are processed by a registered callback. For the builder
/// pattern, a `BackgroundSubscriber` handle is returned, but it provides
/// no `undeclare` method — the subscription lives until the session is dropped.
pub struct BackgroundSubscriber {
    ke: &'static keyexpr,
}

impl BackgroundSubscriber {
    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }
}

pub struct BackgroundSubscriberBuilder<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'res, Config>,
    ke: &'static keyexpr,
    callback: Option<
        DynCallback<
            'res,
            SubCallbackStorage<'res, Config>,
            SampleRef,
        >,
    >,
}

impl<'a, 'res, Config> BackgroundSubscriberBuilder<'a, 'res, Config>
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

    pub fn callback_sync(
        self,
        callback: impl FnMut(&Sample<'_>) + 'res,
    ) -> Self {
        Self {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
        }
    }

    pub fn callback(
        self,
        callback: impl FnMut(&Sample<'_>) + 'res,
    ) -> Self {
        Self {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
        }
    }

    pub fn finish(self) -> core::result::Result<BackgroundSubscriber, SessionError> {
        #[cfg(feature = "acl")]
        self.session
            .check_acl_permission(AclOperation::Subscribe, self.ke)?;

        let mut state = self.session.state();
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

        Ok(BackgroundSubscriber { ke: self.ke })
    }
}

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn declare_background_subscriber(
        &self,
        ke: &'static keyexpr,
    ) -> BackgroundSubscriberBuilder<'_, 'res, Config> {
        BackgroundSubscriberBuilder::new(self, ke)
    }

    /// Declare a fire-and-forget subscriber that processes samples via a
    /// synchronous callback. No handle is returned — the subscription lives
    /// until the session is dropped.
    pub fn declare_background_subscriber_sync(
        &self,
        ke: &'static keyexpr,
        callback: impl FnMut(&Sample<'_>) + 'res,
    ) -> core::result::Result<(), SessionError> {
        self.declare_background_subscriber(ke)
            .callback_sync(callback)
            .finish()?;
        Ok(())
    }

    /// Declare a fire-and-forget liveliness subscriber that processes samples
    /// via a synchronous callback. No handle is returned — the subscription
    /// lives until the session is dropped.
    pub fn declare_liveliness_background_subscriber_sync(
        &self,
        ke: &'static keyexpr,
        callback: impl FnMut(&Sample<'_>) + 'res,
    ) -> core::result::Result<(), SessionError> {
        #[cfg(feature = "acl")]
        self.check_acl_permission(AclOperation::Subscribe, ke)?;

        let mut state = self.state();
        let id = state.next();

        let callback: DynCallback<
            'res,
            LivelinessCallbackStorage<'res, Config>,
            SampleRef,
        > = DynObject::new(SyncCallback::new(callback));

        state.liveliness_callbacks.insert(id, ke, None, callback)?;

        let msg = Interest {
            id,
            mode: InterestMode::CurrentFuture,
            inner: InterestInner {
                options: InterestOptions::TOKENS.options,
                wire_expr: Some(WireExpr::from(ke)),
            },
            ..Default::default()
        };

        self.driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Interest(msg),
            }))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::callbacks::{SyncCallback, ZCallback};
    use crate::api::sample::{Sample, SampleKind};

    #[test]
    fn test_background_subscriber_type_exists() {
        let ke = keyexpr::from_str_unchecked("test/bg");
        let _sub = BackgroundSubscriber { ke };
    }

    #[test]
    fn test_background_subscriber_keyexpr() {
        let ke = keyexpr::from_str_unchecked("test/bg");
        let sub = BackgroundSubscriber { ke };
        assert_eq!(sub.keyexpr().as_str(), "test/bg");
    }

    #[test]
    fn test_sync_callback_wraps_fn_pointer() {
        fn my_callback(_s: &Sample<'_>) {}

        let _cb = SyncCallback::<SampleRef, _>::new(my_callback as fn(&Sample<'_>));
    }

    #[test]
    fn test_sync_callback_wraps_closure() {
        let called = core::cell::Cell::new(false);

        let _cb = SyncCallback::<SampleRef, _>::new(|_s: &Sample<'_>| {
            called.set(true);
        });
    }

    #[test]
    fn test_sync_callback_invocation() {
        let ke = keyexpr::from_str_unchecked("test/foo");
        let sample = Sample::new(
            ke,
            b"hello",
            SampleKind::Put,
            None,
            None,
            None,
            QoS::default(),
            Reliability::default(),
        );

        let called = core::cell::Cell::new(false);
        let mut cb = SyncCallback::<SampleRef, _>::new(|s: &Sample<'_>| {
            called.set(true);
            assert_eq!(s.payload(), b"hello");
        });

        ZCallback::call(&mut cb, &sample);
        assert!(called.get());
    }
}