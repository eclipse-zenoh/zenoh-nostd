use super::sample::*;
use zenoh_proto::CollectionError;
use zenoh_proto::fields::Encoding;

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

    pub fn err_payload(&self) -> Option<&[u8]> {
        match self {
            GetResponse::Ok(_) => None,
            GetResponse::Err(s) => Some(s.payload()),
        }
    }

    pub fn err_encoding(&self) -> Option<&Encoding<'_>> {
        match self {
            GetResponse::Ok(_) => None,
            GetResponse::Err(s) => s.encoding(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sample::SampleKind;
    use zenoh_proto::exts::QoS;
    use zenoh_proto::fields::{Encoding, Reliability};
    use zenoh_proto::keyexpr;

    fn make_err_sample<'a>(ke: &'a keyexpr, payload: &'a [u8], encoding: Option<Encoding<'a>>) -> Sample<'a> {
        Sample::new(
            ke,
            payload,
            SampleKind::Put,
            encoding,
            None,
            None,
            QoS::default(),
            Reliability::default(),
        )
    }

    #[test]
    fn test_get_response_err_payload_and_encoding() {
        let ke = keyexpr::new("test/err").unwrap();
        let payload = b"error payload";
        let enc = Encoding::text_plain();
        let sample = make_err_sample(ke, payload, Some(enc));
        let resp = GetResponse::Err(sample);
        assert_eq!(resp.err_payload(), Some(&b"error payload"[..]));
        assert!(resp.err_encoding().is_some());
        assert_eq!(*resp.err_encoding().unwrap(), Encoding::text_plain());
    }

    #[test]
    fn test_get_response_err_payload_none_on_ok() {
        let ke = keyexpr::new("test/ok").unwrap();
        let sample = make_err_sample(ke, b"ok", None);
        let resp = GetResponse::Ok(sample);
        assert!(resp.err_payload().is_none());
        assert!(resp.err_encoding().is_none());
    }
}
