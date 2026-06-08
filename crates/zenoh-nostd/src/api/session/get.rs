use core::time::Duration;

use dyn_utils::{DynObject, storage::RawOrBox};
use zenoh_proto::{
    SessionError,
    exts::{QoS, QueryTarget, Value},
    fields::{ConsolidationMode, Reliability, WireExpr},
    keyexpr,
    msgs::{NetworkBody, NetworkMessage, Query, Request, RequestBody},
};

#[cfg(feature = "alloc")]
use crate::api::callbacks::AllocCallbacks;
use crate::{
    api::{
        arg::GetResponseRef,
        callbacks::{DynCallback, FixedCapacityCallbacks, SyncCallback, ZCallbacks},
        session::Session,
    },
    config::ZSessionConfig,
    io::transport::ZTransportLinkTx,
    session::GetResponse,
};

pub type FixedCapacityGetCallbacks<'a, const CAPACITY: usize, Callback = RawOrBox<16>> =
    FixedCapacityCallbacks<'a, GetResponseRef, CAPACITY, Callback>;

#[cfg(feature = "alloc")]
pub type AllocGetCallbacks<'a, Callback = RawOrBox<16>> =
    AllocCallbacks<'a, GetResponseRef, Callback>;

pub struct GetResponses {
    ke: &'static keyexpr,
    cancelled: core::cell::Cell<bool>,
}

impl GetResponses {
    pub fn cancel(&self) {
        self.cancelled.set(true);
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }
}

type CallbackStorage<'res, Config> =
    <<Config as ZSessionConfig>::GetCallbacks<'res> as ZCallbacks<'res, GetResponseRef>>::Callback;

pub struct GetBuilder<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) session: &'a Session<'res, Config>,
    pub(crate) ke: &'static keyexpr,
    pub(crate) parameters: Option<&'a str>,
    pub(crate) payload: Option<&'a [u8]>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) callback: Option<DynCallback<'res, CallbackStorage<'res, Config>, GetResponseRef>>,
    pub(crate) reliability: Reliability,
    pub(crate) query_target: QueryTarget,
}

impl<'a, 'res, Config> GetBuilder<'a, 'res, Config>
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
            reliability: Reliability::default(),
            query_target: QueryTarget::default(),
        }
    }

    pub fn callback(self, callback: impl FnMut(&GetResponse<'_>) + 'res) -> Self {
        Self {
            session: self.session,
            ke: self.ke,
            parameters: self.parameters,
            payload: self.payload,
            timeout: self.timeout,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
            reliability: self.reliability,
            query_target: self.query_target,
        }
    }

    pub fn callback_sync(self, callback: impl FnMut(&GetResponse<'_>) + 'res) -> Self {
        Self {
            session: self.session,
            ke: self.ke,
            parameters: self.parameters,
            payload: self.payload,
            timeout: self.timeout,
            callback: Some(DynObject::new(SyncCallback::new(callback))),
            reliability: self.reliability,
            query_target: self.query_target,
        }
    }

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

    pub fn reliability(mut self, reliability: Reliability) -> Self {
        self.reliability = reliability;
        self
    }

    pub fn query_target(mut self, query_target: QueryTarget) -> Self {
        self.query_target = query_target;
        self
    }

    pub fn finish(self) -> core::result::Result<GetResponses, SessionError> {
        let state = self.session.state();
        let rid = state.next();

        if let Some(callback) = self.callback {
            state.get_callbacks.drop_timedout();
            state.get_callbacks.insert(rid, self.ke, None, callback)?;
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
                query_target: self.query_target,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.session
            .driver()
            .tx_mut()
            .send(core::iter::once(NetworkMessage {
                reliability: self.reliability,
                qos: QoS::default(),
                body: NetworkBody::Request(msg),
            }))?;

        Ok(GetResponses {
            ke: self.ke,
            cancelled: core::cell::Cell::new(false),
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
