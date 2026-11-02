use super::sample::*;
use zenoh_proto::{CollectionError, keyexpr};

/// A response to a get.
///
/// A get receives any number of `Ok` and `Err` replies, then exactly one
/// `Final` once the router signals that no more replies will arrive. A get
/// that times out first never receives `Final`.
#[derive(Debug)]
pub enum GetResponse<'a> {
    Ok(Sample<'a>),
    Err(Sample<'a>),
    /// No more replies will arrive for this get.
    Final,
}

/// An owned get response that can mark the end of a get.
///
/// [`GetResponses::recv`](crate::session::GetResponses::recv) uses it to
/// return `None` as soon as the get completes, instead of at its timeout.
pub trait ZOwnedGetResponse {
    /// Whether this response is the end-of-get marker.
    fn is_final(&self) -> bool;
}

impl<'a> GetResponse<'a> {
    pub fn ok(ke: &'a keyexpr, payload: &'a [u8]) -> Self {
        Self::Ok(Sample::new(ke, payload))
    }

    pub fn err(ke: &'a keyexpr, payload: &'a [u8]) -> Self {
        Self::Err(Sample::new(ke, payload))
    }
}

#[derive(Debug)]
pub enum FixedCapacityGetResponse<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize> {
    Ok(FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD>),
    Err(FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD>),
    Final,
}

impl<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize>
    FixedCapacityGetResponse<MAX_KEYEXPR, MAX_PAYLOAD>
{
    pub fn as_ref(&self) -> GetResponse<'_> {
        match self {
            Self::Ok(sample) => GetResponse::Ok(sample.as_ref()),
            Self::Err(sample) => GetResponse::Err(sample.as_ref()),
            Self::Final => GetResponse::Final,
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
            GetResponse::Final => Ok(Self::Final),
        }
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug)]
pub enum AllocGetResponse {
    Ok(AllocSample),
    Err(AllocSample),
    Final,
}

#[cfg(feature = "alloc")]
impl AllocGetResponse {
    pub fn as_ref(&self) -> GetResponse<'_> {
        match self {
            Self::Ok(sample) => GetResponse::Ok(sample.as_ref()),
            Self::Err(sample) => GetResponse::Err(sample.as_ref()),
            Self::Final => GetResponse::Final,
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
            GetResponse::Final => Ok(Self::Final),
        }
    }
}

impl<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize> ZOwnedGetResponse
    for FixedCapacityGetResponse<MAX_KEYEXPR, MAX_PAYLOAD>
{
    fn is_final(&self) -> bool {
        matches!(self, Self::Final)
    }
}

#[cfg(feature = "alloc")]
impl ZOwnedGetResponse for AllocGetResponse {
    fn is_final(&self) -> bool {
        matches!(self, Self::Final)
    }
}
