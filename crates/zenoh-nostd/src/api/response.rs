use super::sample::*;
use zenoh_proto::{CollectionError, keyexpr};

#[derive(Debug)]
/// A borrowed successful or error reply to an outgoing get.
///
/// Copy reply data before the callback returns. `FixedCapacityGetResponse` and,
/// with `alloc`, `AllocGetResponse` provide owned forms for channel delivery.
/// An error reply is distinct from a transport/session error; final completion is
/// tracked by [`GetResponses`](crate::session::GetResponses).
pub enum GetResponse<'a> {
    /// A successful data reply, which does not by itself indicate request completion.
    Ok(Sample<'a>),
    /// An application error reply, distinct from transport failure or request completion.
    Err(Sample<'a>),
}

impl<'a> GetResponse<'a> {
    /// Construct a successful reply view borrowing the key and payload.
    pub fn ok(ke: &'a keyexpr, payload: &'a [u8]) -> Self {
        Self::Ok(Sample::new(ke, payload))
    }

    /// Construct an application error reply view borrowing the key and error payload.
    pub fn err(ke: &'a keyexpr, payload: &'a [u8]) -> Self {
        Self::Err(Sample::new(ke, payload))
    }
}

#[derive(Debug)]
/// Owned get reply with independent key and payload byte capacities.
///
/// `TryFrom<&GetResponse>` preserves success/error status and copies both fields,
/// returning `CollectionTooSmall` if either capacity is exceeded.
///
/// ```
/// use zenoh_nostd::session::{FixedCapacityGetResponse, GetResponse};
/// use zenoh_nostd::session::zenoh::keyexpr;
/// let reply = GetResponse::err(keyexpr::new("demo/temp")?, b"offline");
/// let owned = FixedCapacityGetResponse::<32, 16>::try_from(&reply)?;
/// match owned.as_ref() {
///     GetResponse::Err(sample) => assert_eq!(sample.payload(), b"offline"),
///     GetResponse::Ok(_) => panic!("error replies must preserve their status"),
/// }
/// # Ok::<(), zenoh_nostd::session::Error>(())
/// ```
pub enum FixedCapacityGetResponse<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize> {
    /// A successful data reply, which does not by itself indicate request completion.
    Ok(FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD>),
    /// An application error reply, distinct from transport failure or request completion.
    Err(FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD>),
}

impl<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize>
    FixedCapacityGetResponse<MAX_KEYEXPR, MAX_PAYLOAD>
{
    /// Borrow a reply view without copying data, preserving its success/error variant.
    pub fn as_ref(&self) -> GetResponse<'_> {
        match self {
            Self::Ok(sample) => GetResponse::Ok(sample.as_ref()),
            Self::Err(sample) => GetResponse::Err(sample.as_ref()),
        }
    }
}

impl<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize> TryFrom<&GetResponse<'_>>
    for FixedCapacityGetResponse<MAX_KEYEXPR, MAX_PAYLOAD>
{
    type Error = CollectionError;

    fn try_from(value: &GetResponse<'_>) -> Result<Self, Self::Error> {
        match value {
            GetResponse::Ok(sample) => Ok(Self::Ok(sample.try_into()?)),
            GetResponse::Err(sample) => Ok(Self::Err(sample.try_into()?)),
        }
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug)]
/// Heap-owned get reply, preserving success/error status when copied from [`GetResponse`].
///
/// Available with `alloc`; allocation follows the application allocator.
pub enum AllocGetResponse {
    /// A successful data reply, which does not by itself indicate request completion.
    Ok(AllocSample),
    /// An application error reply, distinct from transport failure or request completion.
    Err(AllocSample),
}

#[cfg(feature = "alloc")]
impl AllocGetResponse {
    /// Borrow a reply view without copying data, preserving its success/error variant.
    pub fn as_ref(&self) -> GetResponse<'_> {
        match self {
            Self::Ok(sample) => GetResponse::Ok(sample.as_ref()),
            Self::Err(sample) => GetResponse::Err(sample.as_ref()),
        }
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<&GetResponse<'_>> for AllocGetResponse {
    type Error = CollectionError;

    fn try_from(value: &GetResponse<'_>) -> Result<Self, Self::Error> {
        match value {
            GetResponse::Ok(sample) => Ok(Self::Ok(sample.try_into()?)),
            GetResponse::Err(sample) => Ok(Self::Err(sample.try_into()?)),
        }
    }
}
