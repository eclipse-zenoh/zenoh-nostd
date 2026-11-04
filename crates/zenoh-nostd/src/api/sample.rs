use core::str::FromStr;

use zenoh_proto::{CollectionError, keyexpr};

#[derive(Debug)]
pub struct Sample<'a> {
    ke: &'a keyexpr,
    payload: &'a [u8],
    attachment: Option<&'a [u8]>,
}

/// Longest attachment kept by the owned sample types; longer ones make the conversion fail.
pub const MAX_ATTACHMENT: usize = 48;

impl<'a> Sample<'a> {
    pub fn new(ke: &'a keyexpr, payload: &'a [u8]) -> Self {
        Self {
            ke,
            payload,
            attachment: None,
        }
    }

    pub fn with_attachment(mut self, attachment: Option<&'a [u8]>) -> Self {
        self.attachment = attachment;
        self
    }

    pub fn attachment(&self) -> Option<&[u8]> {
        self.attachment
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }

    pub fn payload(&self) -> &[u8] {
        self.payload
    }
}

#[derive(Debug)]
pub struct FixedCapacitySample<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize> {
    ke: heapless::String<MAX_KEYEXPR>,
    payload: heapless::Vec<u8, MAX_PAYLOAD>,
    attachment: Option<heapless::Vec<u8, MAX_ATTACHMENT>>,
}

impl<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize>
    FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD>
{
    pub fn keyexpr(&self) -> &keyexpr {
        keyexpr::from_str_unchecked(self.ke.as_str())
    }

    pub fn payload(&self) -> &[u8] {
        self.payload.as_slice()
    }

    pub fn attachment(&self) -> Option<&[u8]> {
        self.attachment.as_deref()
    }

    pub fn as_ref(&self) -> Sample<'_> {
        Sample {
            ke: self.keyexpr(),
            payload: self.payload(),
            attachment: self.attachment(),
        }
    }
}

impl<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize> TryFrom<&Sample<'_>>
    for FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD>
{
    type Error = CollectionError;

    fn try_from(value: &Sample<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            ke: heapless::String::from_str(value.keyexpr().as_str())
                .map_err(|_| CollectionError::CollectionTooSmall)?,
            payload: heapless::Vec::from_slice(value.payload())
                .map_err(|_| CollectionError::CollectionTooSmall)?,
            attachment: value
                .attachment()
                .map(heapless::Vec::from_slice)
                .transpose()
                .map_err(|_| CollectionError::CollectionTooSmall)?,
        })
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug)]
pub struct AllocSample {
    ke: alloc::string::String,
    payload: alloc::vec::Vec<u8>,
    attachment: Option<alloc::vec::Vec<u8>>,
}

#[cfg(feature = "alloc")]
impl AllocSample {
    pub fn keyexpr(&self) -> &keyexpr {
        keyexpr::from_str_unchecked(self.ke.as_str())
    }

    pub fn payload(&self) -> &[u8] {
        self.payload.as_slice()
    }

    pub fn attachment(&self) -> Option<&[u8]> {
        self.attachment.as_deref()
    }

    pub fn as_ref(&self) -> Sample<'_> {
        Sample {
            ke: self.keyexpr(),
            payload: self.payload(),
            attachment: self.attachment(),
        }
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<&Sample<'_>> for AllocSample {
    type Error = CollectionError;

    fn try_from(value: &Sample<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            ke: alloc::string::String::from(value.keyexpr().as_str()),
            payload: alloc::vec::Vec::from(value.payload()),
            attachment: value.attachment().map(alloc::vec::Vec::from),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_sample_keeps_the_attachment() {
        let ke = keyexpr::new("a/b").unwrap();
        let att = [1u8, 2, 3];
        let sample = Sample::new(ke, b"hi").with_attachment(Some(&att));
        let owned = FixedCapacitySample::<16, 16>::try_from(&sample).unwrap();
        assert_eq!(owned.attachment(), Some(&att[..]));
        assert_eq!(owned.as_ref().attachment(), Some(&att[..]));

        let none = FixedCapacitySample::<16, 16>::try_from(&Sample::new(ke, b"hi")).unwrap();
        assert_eq!(none.attachment(), None);

        let too_long = [0u8; MAX_ATTACHMENT + 1];
        let sample = Sample::new(ke, b"hi").with_attachment(Some(&too_long));
        assert!(FixedCapacitySample::<16, 16>::try_from(&sample).is_err());
    }
}
