use core::str::FromStr;

use zenoh_proto::{CollectionError, keyexpr};

#[derive(Debug)]
/// A borrowed sample view; copy it before returning from a receive callback.
///
/// Use [`FixedCapacitySample`] for bounded owned storage, or `AllocSample` with
/// `alloc`. The owned forms implement `TryFrom<&Sample>` for channel delivery.
pub struct Sample<'a> {
    ke: &'a keyexpr,
    payload: &'a [u8],
}

impl<'a> Sample<'a> {
    pub fn new(ke: &'a keyexpr, payload: &'a [u8]) -> Self {
        Self { ke, payload }
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }

    pub fn payload(&self) -> &[u8] {
        self.payload
    }
}

#[derive(Debug)]
/// Owned sample with key/payload capacities measured in bytes.
///
/// Conversion from a borrowed [`Sample`] copies both fields and returns
/// `CollectionTooSmall` if either does not fit.
pub struct FixedCapacitySample<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize> {
    ke: heapless::String<MAX_KEYEXPR>,
    payload: heapless::Vec<u8, MAX_PAYLOAD>,
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

    pub fn as_ref(&self) -> Sample<'_> {
        Sample {
            ke: self.keyexpr(),
            payload: self.payload(),
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
        })
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug)]
/// Heap-owned sample copied from a borrowed [`Sample`], available with `alloc`.
pub struct AllocSample {
    ke: alloc::string::String,
    payload: alloc::vec::Vec<u8>,
}

#[cfg(feature = "alloc")]
impl AllocSample {
    pub fn keyexpr(&self) -> &keyexpr {
        keyexpr::from_str_unchecked(self.ke.as_str())
    }

    pub fn payload(&self) -> &[u8] {
        self.payload.as_slice()
    }

    pub fn as_ref(&self) -> Sample<'_> {
        Sample {
            ke: self.keyexpr(),
            payload: self.payload(),
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
        })
    }
}
