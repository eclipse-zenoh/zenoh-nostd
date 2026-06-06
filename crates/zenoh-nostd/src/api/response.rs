use super::sample::*;
use zenoh_proto::CollectionError;

#[derive(Debug)]
pub enum GetResponse<'a> {
    Ok(Sample<'a>),
    Err(Sample<'a>),
}

impl<'a> GetResponse<'a> {
    pub fn is_ok(&self) -> bool {
        matches!(self, GetResponse::Ok(_))
    }

    pub fn is_err(&self) -> bool {
        matches!(self, GetResponse::Err(_))
    }

    pub fn ok(&self) -> Option<&Sample<'_>> {
        match self {
            GetResponse::Ok(s) => Some(s),
            GetResponse::Err(_) => None,
        }
    }

    pub fn err(&self) -> Option<&Sample<'_>> {
        match self {
            GetResponse::Ok(_) => None,
            GetResponse::Err(s) => Some(s),
        }
    }
}

#[derive(Debug)]
pub enum FixedCapacityGetResponse<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize, const MAX_ATTACHMENT: usize> {
    Ok(FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD, MAX_ATTACHMENT>),
    Err(FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD, MAX_ATTACHMENT>),
}

impl<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize, const MAX_ATTACHMENT: usize>
    FixedCapacityGetResponse<MAX_KEYEXPR, MAX_PAYLOAD, MAX_ATTACHMENT>
{
    pub fn as_ref(&self) -> GetResponse<'_> {
        match self {
            Self::Ok(sample) => GetResponse::Ok(sample.as_ref()),
            Self::Err(sample) => GetResponse::Err(sample.as_ref()),
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    pub fn is_err(&self) -> bool {
        matches!(self, Self::Err(_))
    }

    pub fn ok(&self) -> Option<&FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD, MAX_ATTACHMENT>> {
        match self {
            Self::Ok(s) => Some(s),
            Self::Err(_) => None,
        }
    }

    pub fn err(&self) -> Option<&FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD, MAX_ATTACHMENT>> {
        match self {
            Self::Ok(_) => None,
            Self::Err(s) => Some(s),
        }
    }
}

impl<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize, const MAX_ATTACHMENT: usize> TryFrom<&GetResponse<'_>>
    for FixedCapacityGetResponse<MAX_KEYEXPR, MAX_PAYLOAD, MAX_ATTACHMENT>
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
pub enum AllocGetResponse {
    Ok(AllocSample),
    Err(AllocSample),
}

#[cfg(feature = "alloc")]
impl AllocGetResponse {
    pub fn as_ref(&self) -> GetResponse<'_> {
        match self {
            Self::Ok(sample) => GetResponse::Ok(sample.as_ref()),
            Self::Err(sample) => GetResponse::Err(sample.as_ref()),
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    pub fn is_err(&self) -> bool {
        matches!(self, Self::Err(_))
    }

    pub fn ok(&self) -> Option<&AllocSample> {
        match self {
            Self::Ok(s) => Some(s),
            Self::Err(_) => None,
        }
    }

    pub fn err(&self) -> Option<&AllocSample> {
        match self {
            Self::Ok(_) => None,
            Self::Err(s) => Some(s),
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
