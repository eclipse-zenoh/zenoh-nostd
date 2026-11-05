use core::time::Duration;
use zenoh_proto::{SessionError, keyexpr};

use crate::{api::session::Session, config::ZSessionConfig, session::GetBuilder};

pub struct Querier<'a, 'transport, 'resources: 'transport, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'transport, 'resources, Config>,
    ke: &'a keyexpr,
    parameters: Option<&'a str>,
    payload: Option<&'a [u8]>,
    timeout: Option<Duration>,
}

impl<'a, 'transport, 'resources: 'transport, Config> Querier<'a, 'transport, 'resources, Config>
where
    Config: ZSessionConfig,
{
    pub fn get(&self) -> GetBuilder<'a, 'transport, 'resources, Config> {
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

pub struct QuerierBuilder<'a, 'transport, 'resources: 'transport, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'transport, 'resources, Config>,
    ke: &'a keyexpr,
    parameters: Option<&'a str>,
    payload: Option<&'a [u8]>,
    timeout: Option<Duration>,
}

impl<'a, 'transport, 'resources: 'transport, Config>
    QuerierBuilder<'a, 'transport, 'resources, Config>
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

    pub async fn finish(
        self,
    ) -> core::result::Result<Querier<'a, 'transport, 'resources, Config>, SessionError> {
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

impl<'transport, 'resources: 'transport, Config> Session<'transport, 'resources, Config>
where
    Config: ZSessionConfig,
{
    pub fn declare_querier<'a>(
        &'a self,
        ke: &'a keyexpr,
    ) -> QuerierBuilder<'a, 'transport, 'resources, Config> {
        QuerierBuilder::new(self, ke)
    }
}
