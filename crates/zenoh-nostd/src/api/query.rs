use core::str::FromStr;

use zenoh_proto::{CollectionError, SessionError, keyexpr};

use crate::{api::session::Session, config::ZSessionConfig};

/// Borrowed incoming query delivered to a queryable callback.
///
/// Key, parameters and payload refer to receive data and cannot outlive the callback.
/// For deferred handling, copy what is needed into bounded application storage along
/// with [`Self::request_id`]. After the callback returns, use [`Session::reply`] or
/// [`Session::err`], then [`Session::finalize`] exactly once for this accepted handler.
/// Reject immediately with [`Self::reject`] when work cannot be accepted.
///
/// Dropping a query does not finalize it. Do not wait for finalization from inside
/// the receive callback: finalization acquires the state lock held by the driver.
pub struct QueryableQuery<'a, 'transport, 'resources: 'transport, Config>
where
    Config: ZSessionConfig,
{
    session: &'a Session<'transport, 'resources, Config>,
    rid: u32,
    ke: &'a keyexpr,
    parameters: Option<&'a str>,
    payload: Option<&'a [u8]>,
    finalized: bool,
    pub(crate) rejected: core::cell::Cell<bool>,
}

impl<'a, 'transport, 'resources: 'transport, Config>
    QueryableQuery<'a, 'transport, 'resources, Config>
where
    Config: ZSessionConfig,
{
    pub(crate) fn new(
        session: &'a Session<'transport, 'resources, Config>,
        rid: u32,
        ke: &'a keyexpr,
        parameters: Option<&'a str>,
        payload: Option<&'a [u8]>,
    ) -> Self {
        Self {
            session,
            rid,
            ke,
            parameters,
            payload,
            finalized: false,
            rejected: core::cell::Cell::new(false),
        }
    }

    /// Decline a query when an application queue is full. The session sends a
    /// final response once all matching handlers have either finalized or declined.
    pub fn reject(&self) {
        self.rejected.set(true);
    }

    /// Identifier for replying from an application-owned queue.
    pub fn request_id(&self) -> u32 {
        self.rid
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }

    pub fn parameters(&self) -> Option<&str> {
        self.parameters
    }

    pub fn payload(&self) -> Option<&[u8]> {
        self.payload
    }

    /// Send a reply without finalizing the request. See [`Session::reply`].
    pub async fn reply(
        &self,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.session.reply(self.rid, ke, payload).await
    }

    /// Send an error reply without finalizing the request. See [`Session::err`].
    pub async fn err(
        &self,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.session.err(self.rid, ke, payload).await
    }

    /// Finish this handler's accepted request once; repeated calls on this handle
    /// are no-ops after success. Dropping the handle does not finalize it.
    ///
    /// Call outside the receive callback because finalization locks session state.
    /// For a borrowed callback query, defer its request ID and use [`Session::finalize`]
    /// after returning. On write failure or cancellation, discard the connection.
    pub async fn finalize(&mut self) -> core::result::Result<(), SessionError> {
        if !self.finalized {
            self.session.finalize(self.rid).await?;
            self.finalized = true
        }

        Ok(())
    }
}

/// Owned, bounded query data for the built-in static-session channel.
///
/// Key/parameter/payload capacities count bytes; conversion fails with
/// `CollectionTooSmall` when an input exceeds its limit. The stored session
/// reference requires static session storage. For a scoped session, queue copied
/// data and a request ID in application-owned storage instead.
///
/// The consumer must explicitly finalize accepted work; dropping this value does
/// not finalize the request or decrement its counter.
pub struct FixedCapacityQueryableQuery<
    Config,
    const MAX_KEYEXPR: usize,
    const MAX_PARAMETERS: usize,
    const MAX_PAYLOAD: usize,
> where
    Config: ZSessionConfig + 'static,
{
    session: &'static Session<'static, 'static, Config>,
    rid: u32,
    ke: heapless::String<MAX_KEYEXPR>,
    parameters: Option<heapless::String<MAX_PARAMETERS>>,
    payload: Option<heapless::Vec<u8, MAX_PAYLOAD>>,
    finalized: bool,
}

impl<Config, const MAX_KEYEXPR: usize, const MAX_PARAMETERS: usize, const MAX_PAYLOAD: usize>
    FixedCapacityQueryableQuery<Config, MAX_KEYEXPR, MAX_PARAMETERS, MAX_PAYLOAD>
where
    Config: ZSessionConfig,
{
    pub fn keyexpr(&self) -> &keyexpr {
        keyexpr::from_str_unchecked(self.ke.as_str())
    }

    pub fn parameters(&self) -> Option<&str> {
        self.parameters.as_ref().map(|p| p.as_str())
    }

    pub fn payload(&self) -> Option<&[u8]> {
        self.payload.as_ref().map(|p| p.as_slice())
    }

    /// Send a reply without finalizing the request. See [`Session::reply`].
    pub async fn reply(
        &self,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.session.reply(self.rid, ke, payload).await
    }

    /// Send an error reply without finalizing the request. See [`Session::err`].
    pub async fn err(
        &self,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.session.err(self.rid, ke, payload).await
    }

    /// Finish this handler's accepted request once; repeated calls on this handle
    /// are no-ops after success. Dropping the handle does not finalize it.
    ///
    /// Call outside the receive callback because finalization locks session state.
    /// For a borrowed callback query, defer its request ID and use [`Session::finalize`]
    /// after returning. On write failure or cancellation, discard the connection.
    pub async fn finalize(&mut self) -> core::result::Result<(), SessionError> {
        if !self.finalized {
            self.session.finalize(self.rid).await?;
            self.finalized = true
        }

        Ok(())
    }
}

impl<'a, Config, const MAX_KEYEXPR: usize, const MAX_PARAMETERS: usize, const MAX_PAYLOAD: usize>
    TryFrom<(
        &QueryableQuery<'a, 'static, 'static, Config>,
        &'static Session<'static, 'static, Config>,
    )> for FixedCapacityQueryableQuery<Config, MAX_KEYEXPR, MAX_PARAMETERS, MAX_PAYLOAD>
where
    Config: ZSessionConfig,
{
    type Error = CollectionError;

    fn try_from(
        value: (
            &QueryableQuery<'a, 'static, 'static, Config>,
            &'static Session<'static, 'static, Config>,
        ),
    ) -> Result<Self, Self::Error> {
        let (value, session) = value;

        Ok(Self {
            session,
            rid: value.rid,
            ke: heapless::String::from_str(value.keyexpr().as_str())
                .map_err(|_| CollectionError::CollectionTooSmall)?,
            parameters: value
                .parameters
                .map(heapless::String::from_str)
                .transpose()
                .map_err(|_| CollectionError::CollectionTooSmall)?,
            payload: value
                .payload
                .map(heapless::Vec::from_slice)
                .transpose()
                .map_err(|_| CollectionError::CollectionTooSmall)?,
            finalized: value.finalized,
        })
    }
}

#[cfg(feature = "alloc")]
/// Heap-owned incoming query data retaining a static session reference.
///
/// Available with `alloc`. Conversion copies key, parameters and payload using the
/// application allocator. The consumer must finalize accepted work explicitly;
/// dropping this value leaves the incoming-request counter unchanged.
pub struct AllocQueryableQuery<Config>
where
    Config: ZSessionConfig + 'static,
{
    session: &'static Session<'static, 'static, Config>,
    rid: u32,
    ke: alloc::string::String,
    parameters: Option<alloc::string::String>,
    payload: Option<alloc::vec::Vec<u8>>,
    finalized: bool,
}

#[cfg(feature = "alloc")]
impl<Config> AllocQueryableQuery<Config>
where
    Config: ZSessionConfig,
{
    pub fn keyexpr(&self) -> &keyexpr {
        keyexpr::from_str_unchecked(self.ke.as_str())
    }

    pub fn parameters(&self) -> Option<&str> {
        self.parameters.as_deref()
    }

    pub fn payload(&self) -> Option<&[u8]> {
        self.payload.as_deref()
    }

    /// Send a reply without finalizing the request. See [`Session::reply`].
    pub async fn reply(
        &self,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.session.reply(self.rid, ke, payload).await
    }

    /// Send an error reply without finalizing the request. See [`Session::err`].
    pub async fn err(
        &self,
        ke: &keyexpr,
        payload: &[u8],
    ) -> core::result::Result<(), SessionError> {
        self.session.err(self.rid, ke, payload).await
    }

    /// Finish this handler's accepted request once; repeated calls on this handle
    /// are no-ops after success. Dropping the handle does not finalize it.
    ///
    /// Call outside the receive callback because finalization locks session state.
    /// For a borrowed callback query, defer its request ID and use [`Session::finalize`]
    /// after returning. On write failure or cancellation, discard the connection.
    pub async fn finalize(&mut self) -> core::result::Result<(), SessionError> {
        if !self.finalized {
            self.session.finalize(self.rid).await?;
            self.finalized = true
        }

        Ok(())
    }
}

#[cfg(feature = "alloc")]
impl<'a, Config>
    TryFrom<(
        &QueryableQuery<'a, 'static, 'static, Config>,
        &'static Session<'static, 'static, Config>,
    )> for AllocQueryableQuery<Config>
where
    Config: ZSessionConfig,
{
    type Error = CollectionError;

    fn try_from(
        value: (
            &QueryableQuery<'a, 'static, 'static, Config>,
            &'static Session<'static, 'static, Config>,
        ),
    ) -> Result<Self, Self::Error> {
        let (value, session) = value;

        Ok(Self {
            session,
            rid: value.rid,
            ke: alloc::string::String::from(value.keyexpr().as_str()),
            parameters: value.parameters.map(alloc::string::String::from),
            payload: value.payload.map(alloc::vec::Vec::from),
            finalized: value.finalized,
        })
    }
}
