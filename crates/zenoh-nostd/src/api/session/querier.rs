use core::time::Duration;
use zenoh_proto::{SessionError, exts::QueryTarget, fields::{Locality, Reliability}, keyexpr};

use crate::{api::session::Session, config::ZSessionConfig, session::GetBuilder};

pub struct Querier<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'res, Config>,
    ke: &'static keyexpr,
    parameters: Option<&'a str>,
    payload: Option<&'a [u8]>,
    timeout: Option<Duration>,
    reliability: Reliability,
    query_target: QueryTarget,
}

impl<'a, 'res, Config> Querier<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn get(&self) -> GetBuilder<'a, 'res, Config> {
        GetBuilder {
            session: self.session,
            ke: self.ke,
            parameters: self.parameters,
            payload: self.payload,
            timeout: self.timeout,
            callback: None,
            receiver: None,
            reliability: self.reliability,
            query_target: self.query_target,
        }
    }

    #[allow(dead_code)]
    async fn undeclare(self) -> core::result::Result<(), SessionError> {
        todo!("send undeclare interest")
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }
}

pub struct QuerierBuilder<'a, 'res, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'res, Config>,
    ke: &'static keyexpr,
    parameters: Option<&'a str>,
    payload: Option<&'a [u8]>,
    timeout: Option<Duration>,
    allowed_destination: Locality,
    reliability: Reliability,
    query_target: QueryTarget,
}

impl<'a, 'res, Config> QuerierBuilder<'a, 'res, Config>
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
            allowed_destination: Locality::default(),
            reliability: Reliability::default(),
            query_target: QueryTarget::default(),
        }
    }

    pub fn allowed_destination(mut self, locality: Locality) -> Self {
        self.allowed_destination = locality;
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

    pub async fn finish(self) -> core::result::Result<Querier<'a, 'res, Config>, SessionError> {
        // TODO: send interest msg
        Ok(Querier {
            session: self.session,
            ke: self.ke,
            parameters: self.parameters,
            payload: self.payload,
            timeout: self.timeout,
            reliability: self.reliability,
            query_target: self.query_target,
        })
    }
}

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn declare_querier(&self, ke: &'static keyexpr) -> QuerierBuilder<'_, 'res, Config> {
        QuerierBuilder::new(self, ke)
    }
}
