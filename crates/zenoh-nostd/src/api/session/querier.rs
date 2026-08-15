use core::time::Duration;
use zenoh_proto::{SessionError, keyexpr};

use crate::{api::session::Session, config::ZSessionConfig, session::GetBuilder};

pub struct Querier<'a, 's, 'res, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'s, 'res, Config>,
    ke: &'static keyexpr,
    parameters: Option<&'a str>,
    payload: Option<&'a [u8]>,
    timeout: Option<Duration>,
}

impl<'a, 's, 'res, Config> Querier<'a, 's, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub fn get(&self) -> GetBuilder<'a, 's, 'res, Config> {
        GetBuilder {
            session: self.session,
            ke: self.ke,
            parameters: self.parameters,
            payload: self.payload,
            timeout: self.timeout,
            callback: None,
            receiver: None,
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

pub struct QuerierBuilder<'a, 's, 'res, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'s, 'res, Config>,
    ke: &'static keyexpr,
    parameters: Option<&'a str>,
    payload: Option<&'a [u8]>,
    timeout: Option<Duration>,
}

impl<'a, 's, 'res, Config> QuerierBuilder<'a, 's, 'res, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(session: &'a Session<'s, 'res, Config>, ke: &'static keyexpr) -> Self {
        Self {
            session,
            ke,
            parameters: None,
            payload: None,
            timeout: None,
        }
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

    pub async fn finish(self) -> core::result::Result<Querier<'a, 's, 'res, Config>, SessionError> {
        // TODO: send interest msg
        Ok(Querier {
            session: self.session,
            ke: self.ke,
            parameters: self.parameters,
            payload: self.payload,
            timeout: self.timeout,
        })
    }
}

impl<'s, 'res, Config> Session<'s, 'res, Config>
where
    Config: ZSessionConfig,
    'res: 's,
{
    pub fn declare_querier(&self, ke: &'static keyexpr) -> QuerierBuilder<'_, 's, 'res, Config> {
        QuerierBuilder::new(self, ke)
    }
}
