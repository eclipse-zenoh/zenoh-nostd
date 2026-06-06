use core::str::FromStr;

use zenoh_proto::{
    CollectionError,
    exts::QoS,
    fields::{Encoding, Reliability, Timestamp},
    keyexpr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleKind {
    Put,
    Delete,
}

#[derive(Debug)]
pub struct Sample<'a> {
    ke: &'a keyexpr,
    payload: &'a [u8],
    kind: SampleKind,
    encoding: Option<Encoding<'a>>,
    timestamp: Option<Timestamp>,
    attachment: Option<&'a [u8]>,
    qos: QoS,
    reliability: Reliability,
}

impl<'a> Sample<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ke: &'a keyexpr,
        payload: &'a [u8],
        kind: SampleKind,
        encoding: Option<Encoding<'a>>,
        timestamp: Option<Timestamp>,
        attachment: Option<&'a [u8]>,
        qos: QoS,
        reliability: Reliability,
    ) -> Self {
        Self {
            ke,
            payload,
            kind,
            encoding,
            timestamp,
            attachment,
            qos,
            reliability,
        }
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }

    pub fn payload(&self) -> &[u8] {
        self.payload
    }

    pub fn kind(&self) -> SampleKind {
        self.kind
    }

    pub fn encoding(&self) -> Option<&Encoding<'_>> {
        self.encoding.as_ref()
    }

    pub fn timestamp(&self) -> Option<&Timestamp> {
        self.timestamp.as_ref()
    }

    pub fn attachment(&self) -> Option<&[u8]> {
        self.attachment
    }

    pub fn qos(&self) -> QoS {
        self.qos
    }

    pub fn reliability(&self) -> Reliability {
        self.reliability
    }
}

#[derive(Debug)]
pub struct FixedCapacitySample<
    const MAX_KEYEXPR: usize,
    const MAX_PAYLOAD: usize,
    const MAX_ATTACHMENT: usize,
> {
    ke: heapless::String<MAX_KEYEXPR>,
    payload: heapless::Vec<u8, MAX_PAYLOAD>,
    kind: SampleKind,
    encoding: Option<EncodingOwned<MAX_KEYEXPR>>,
    timestamp: Option<Timestamp>,
    attachment: Option<heapless::Vec<u8, MAX_ATTACHMENT>>,
    qos: QoS,
    reliability: Reliability,
}

#[derive(Debug)]
struct EncodingOwned<const MAX: usize> {
    id: u16,
    schema: Option<heapless::String<MAX>>,
}

impl<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize, const MAX_ATTACHMENT: usize>
    FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD, MAX_ATTACHMENT>
{
    pub fn keyexpr(&self) -> &keyexpr {
        keyexpr::from_str_unchecked(self.ke.as_str())
    }

    pub fn payload(&self) -> &[u8] {
        self.payload.as_slice()
    }

    pub fn kind(&self) -> SampleKind {
        self.kind
    }

    pub fn encoding(&self) -> Option<Encoding<'_>> {
        self.encoding.as_ref().map(|e| Encoding {
            id: e.id,
            schema: e.schema.as_ref().map(|s| s.as_bytes()),
        })
    }

    pub fn timestamp(&self) -> Option<&Timestamp> {
        self.timestamp.as_ref()
    }

    pub fn attachment(&self) -> Option<&[u8]> {
        self.attachment.as_deref()
    }

    pub fn qos(&self) -> QoS {
        self.qos
    }

    pub fn reliability(&self) -> Reliability {
        self.reliability
    }

    pub fn as_ref(&self) -> Sample<'_> {
        Sample {
            ke: self.keyexpr(),
            payload: self.payload(),
            kind: self.kind,
            encoding: self.encoding(),
            timestamp: self.timestamp,
            attachment: self.attachment(),
            qos: self.qos,
            reliability: self.reliability,
        }
    }
}

impl<const MAX_KEYEXPR: usize, const MAX_PAYLOAD: usize, const MAX_ATTACHMENT: usize>
    TryFrom<&Sample<'_>>
    for FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD, MAX_ATTACHMENT>
{
    type Error = CollectionError;

    fn try_from(value: &Sample<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            ke: heapless::String::from_str(value.keyexpr().as_str())
                .map_err(|_| CollectionError::CollectionTooSmall)?,
            payload: heapless::Vec::from_slice(value.payload())
                .map_err(|_| CollectionError::CollectionTooSmall)?,
            kind: value.kind,
            encoding: value.encoding.as_ref().map(|enc| {
                let schema = enc.schema.map(|s| {
                    core::str::from_utf8(s)
                        .ok()
                        .and_then(|s| heapless::String::from_str(s).ok())
                });
                EncodingOwned {
                    id: enc.id,
                    schema: schema.flatten(),
                }
            }),
            timestamp: value.timestamp,
            attachment: value.attachment.and_then(|a| {
                let mut v = heapless::Vec::new();
                v.extend_from_slice(a).ok()?;
                Some(v)
            }),
            qos: value.qos,
            reliability: value.reliability,
        })
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug)]
pub struct AllocSample {
    ke: alloc::string::String,
    payload: alloc::vec::Vec<u8>,
    kind: SampleKind,
    encoding: Option<OwnedEncoding>,
    timestamp: Option<Timestamp>,
    attachment: Option<alloc::vec::Vec<u8>>,
    qos: QoS,
    reliability: Reliability,
}

#[cfg(feature = "alloc")]
#[derive(Debug)]
struct OwnedEncoding {
    id: u16,
    schema: Option<alloc::string::String>,
}

#[cfg(feature = "alloc")]
impl AllocSample {
    pub fn keyexpr(&self) -> &keyexpr {
        keyexpr::from_str_unchecked(self.ke.as_str())
    }

    pub fn payload(&self) -> &[u8] {
        self.payload.as_slice()
    }

    pub fn kind(&self) -> SampleKind {
        self.kind
    }

    pub fn encoding(&self) -> Option<Encoding<'_>> {
        self.encoding.as_ref().map(|e| Encoding {
            id: e.id,
            schema: e.schema.as_ref().map(|s| s.as_bytes()),
        })
    }

    pub fn timestamp(&self) -> Option<&Timestamp> {
        self.timestamp.as_ref()
    }

    pub fn attachment(&self) -> Option<&[u8]> {
        self.attachment.as_deref()
    }

    pub fn qos(&self) -> QoS {
        self.qos
    }

    pub fn reliability(&self) -> Reliability {
        self.reliability
    }

    pub fn as_ref(&self) -> Sample<'_> {
        Sample {
            ke: self.keyexpr(),
            payload: self.payload(),
            kind: self.kind,
            encoding: self.encoding(),
            timestamp: self.timestamp,
            attachment: self.attachment(),
            qos: self.qos,
            reliability: self.reliability,
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
            kind: value.kind,
            encoding: value.encoding.as_ref().map(|enc| OwnedEncoding {
                id: enc.id,
                schema: enc.schema.and_then(|s| {
                    core::str::from_utf8(s)
                        .ok()
                        .map(alloc::string::String::from)
                }),
            }),
            timestamp: value.timestamp,
            attachment: value
                .attachment
                .map(alloc::vec::Vec::from),
            qos: value.qos,
            reliability: value.reliability,
        })
    }
}
