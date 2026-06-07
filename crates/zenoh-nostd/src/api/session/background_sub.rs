use dyn_utils::DynObject;
use zenoh_proto::{
    SessionError,
    exts::QoS,
    fields::{Reliability, WireExpr},
    keyexpr,
    msgs::*,
};

use crate::{
    api::{
        arg::SampleRef,
        callbacks::{AsyncCallback, DynCallback, SyncCallback, ZCallbacks},
        sample::Sample,
        session::Session,
    },
    config::ZSessionConfig,
    io::transport::ZTransportLinkTx,
};

#[cfg(feature = "acl")]
use crate::api::acl::AclOperation;

type SubCallbackStorage<'res, Config> =
    <<Config as ZSessionConfig>::SubCallbacks<'res> as ZCallbacks<'res, SampleRef>>::Callback;

type SubFutureStorage<'res, Config> =
    <<Config as ZSessionConfig>::SubCallbacks<'res> as ZCallbacks<'res, SampleRef>>::Future;

type LivelinessCallbackStorage<'res, Config> = <<Config as ZSessionConfig>::LivelinessCallbacks<
    'res,
> as ZCallbacks<'res, SampleRef>>::Callback;

type LivelinessFutureStorage<'res, Config> =
    <<Config as ZSessionConfig>::LivelinessCallbacks<'res> as ZCallbacks<'res, SampleRef>>::Future;

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

pub struct BackgroundSubscriberBuilder<'a, 'res, Config, const READY: bool = false>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'res, Config>,
    ke: &'static keyexpr,
    callback: Option<
        DynCallback<
            'res,
            SubCallbackStorage<'res, Config>,
            SubFutureStorage<'res, Config>,
            SampleRef,
        >,
    >,
}

impl<'a, 'res, Config> BackgroundSubscriberBuilder<'a, 'res, Config, false>
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
    ) -> BackgroundSubscriberBuilder<'a, 'res, Config, true> {
        BackgroundSubscriberBuilder {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
        }
    }

    pub fn callback(
        self,
        callback: impl AsyncFnMut(&Sample<'_>) + 'res,
    ) -> BackgroundSubscriberBuilder<'a, 'res, Config, true> {
        BackgroundSubscriberBuilder {
            session: self.session,
            ke: self.ke,
            callback: Some(DynObject::new(AsyncCallback::new(callback))),
        }
    }
}

impl<'a, 'res, Config> BackgroundSubscriberBuilder<'a, 'res, Config, true>
where
    Config: ZSessionConfig,
{
    pub async fn finish(self) -> core::result::Result<BackgroundSubscriber, SessionError> {
        #[cfg(feature = "acl")]
        self.session
            .check_acl_permission(AclOperation::Subscribe, self.ke)?;

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
    ) -> BackgroundSubscriberBuilder<'_, 'res, Config, false> {
        BackgroundSubscriberBuilder::new(self, ke)
    }

    /// Declare a fire-and-forget subscriber that processes samples via a
    /// synchronous callback. No handle is returned — the subscription lives
    /// until the session is dropped.
    pub async fn declare_background_subscriber_sync(
        &self,
        ke: &'static keyexpr,
        callback: impl FnMut(&Sample<'_>) + 'res,
    ) -> core::result::Result<(), SessionError> {
        self.declare_background_subscriber(ke)
            .callback_sync(callback)
            .finish()
            .await?;
        Ok(())
    }

    /// Declare a fire-and-forget liveliness subscriber that processes samples
    /// via a synchronous callback. No handle is returned — the subscription
    /// lives until the session is dropped.
    pub async fn declare_liveliness_background_subscriber_sync(
        &self,
        ke: &'static keyexpr,
        callback: impl FnMut(&Sample<'_>) + 'res,
    ) -> core::result::Result<(), SessionError> {
        #[cfg(feature = "acl")]
        self.check_acl_permission(AclOperation::Subscribe, ke)?;

        let mut state = self.state().await;
        let id = state.next();

        let callback: DynCallback<
            'res,
            LivelinessCallbackStorage<'res, Config>,
            LivelinessFutureStorage<'res, Config>,
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

        self.driver
            .tx()
            .await
            .send(core::iter::once(NetworkMessage {
                reliability: Reliability::default(),
                qos: QoS::default(),
                body: NetworkBody::Interest(msg),
            }))
            .await?;

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
        // Verify that a fn pointer can be wrapped in SyncCallback without
        // causing type errors. This is the core mechanism used by background
        // subscribers in no_std environments.
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
    fn test_sync_callback_invocation_via_poll() {
        use core::task::{Context, Waker};

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

        let waker: &'static Waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        let mut fut = ZCallback::call(&mut cb, &sample);
        let _ = unsafe { core::pin::Pin::new_unchecked(&mut fut) }.poll(&mut cx);

        assert!(called.get());
    }
}
