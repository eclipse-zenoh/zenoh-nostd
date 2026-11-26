use core::time::Duration;
use zenoh_proto::{SessionError, keyexpr};

use crate::{api::session::Session, config::ZSessionConfig, session::GetBuilder};

/// Reusable borrowed-key settings for gets.
///
/// The current implementation creates no protocol Interest declaration or callback
/// registration for this handle. Dropping it only drops local settings; `undeclare`
/// is a consuming no-op. Each operation created from it has its own lifecycle.
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
    /// Create a get using these defaults, without sending it.
    ///
    /// Select a callback or channel, then await [`GetBuilder::finish`]. Each get has
    /// its own registry slot, timeout and cleanup lifecycle.
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

    /// Consume the handle; currently sends no Interest or undeclaration message.
    pub async fn undeclare(self) -> core::result::Result<(), SessionError> {
        // No interest is sent by the current querier implementation.
        Ok(())
    }

    /// Borrow the key expression selected for this handle.
    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }
}

/// Configure reusable get defaults without sending a declaration.
///
/// Call [`Self::finish`] to create a local [`Querier`]; select reply delivery and
/// await [`GetBuilder::finish`] for each actual request.
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

    /// Set borrowed query parameters, sent verbatim; omitted parameters default to an empty string.
    pub fn parameters(mut self, parameters: &'a str) -> Self {
        self.parameters = Some(parameters);
        self
    }

    /// Set the borrowed payload bytes; the application retains ownership through sending.
    pub fn payload(mut self, payload: &'a [u8]) -> Self {
        self.payload = Some(payload);
        self
    }

    /// Set the timeout inherited by each get, measured from its `finish` call.
    ///
    /// The default is 30 seconds. This bounds local reply tracking, not remote execution.
    /// Use a duration representable by Embassy time; conversion in `finish` can panic
    /// for an unrepresentable duration or deadline overflow.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Return reusable local get settings; currently sends no Interest or declaration.
    ///
    /// This implementation always returns `Ok`. No request is issued until a get
    /// created by the returned handle is finished.
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
    /// Configure reusable get defaults for `ke`; this does not send a query or declaration.
    pub fn declare_querier<'a>(
        &'a self,
        ke: &'a keyexpr,
    ) -> QuerierBuilder<'a, 'transport, 'resources, Config> {
        QuerierBuilder::new(self, ke)
    }
}
