use uhlc::NTP64;
pub use uhlc::Timestamp;

use crate::*;

#[repr(u8)]
#[derive(ZRU8, Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Reliability {
    BestEffort = 0,
    #[default]
    Reliable = 1,
}

#[repr(u8)]
#[derive(Debug, Default, PartialEq)]
pub enum Priority {
    #[default]
    Data = 5,
}

#[derive(Debug, Default, PartialEq)]
#[repr(u8)]
pub enum CongestionControl {
    #[default]
    Drop = 0,
    Block = 1,
}

#[repr(u8)]
#[derive(ZRU8, Debug, Default, Clone, PartialEq, Copy)]
pub enum ConsolidationMode {
    #[default]
    Auto = 0,
    None = 1,
    Monotonic = 2,
    Latest = 3,
}

#[derive(Debug, PartialEq)]
pub struct BatchSize(pub u16);

impl Default for BatchSize {
    fn default() -> Self {
        BatchSize(u16::MAX)
    }
}

impl ZBodyLen for BatchSize {
    fn z_body_len(&self) -> usize {
        self.0.to_le_bytes().len()
    }
}

impl ZBodyEncode for BatchSize {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        w.write(&self.0.to_le_bytes())?;
        Ok(())
    }
}

impl<'a> ZBodyDecode<'a> for BatchSize {
    type Ctx = ();

    fn z_body_decode(
        r: &mut impl crate::ZReadable<'a>,
        _: (),
    ) -> core::result::Result<Self, crate::CodecError> {
        let mut bytes = u16::MAX.to_le_bytes();
        r.read_exact(&mut bytes)?;
        Ok(BatchSize(u16::from_le_bytes(bytes)))
    }
}

crate::derive_zstruct_with_body!(BatchSize);

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ZenohIdProto(pub(crate) uhlc::ID);

impl ZenohIdProto {
    #[inline]
    pub fn size(&self) -> usize {
        self.0.size()
    }

    #[inline]
    pub fn as_le_bytes(&self) -> [u8; uhlc::ID::MAX_SIZE] {
        self.0.to_le_bytes()
    }
}
impl core::fmt::Debug for ZenohIdProto {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for ZenohIdProto {
    fn default() -> Self {
        Self(uhlc::ID::rand())
    }
}

impl TryFrom<&[u8]> for ZenohIdProto {
    type Error = crate::CodecError;

    fn try_from(val: &[u8]) -> core::result::Result<Self, crate::CodecError> {
        match val.try_into() {
            Ok(ok) => Ok(Self(ok)),
            Err(_) => crate::zbail!(crate::CodecError::CouldNotParseField),
        }
    }
}

impl ZBodyLen for ZenohIdProto {
    fn z_body_len(&self) -> usize {
        self.size()
    }
}

impl ZBodyEncode for ZenohIdProto {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        let bytes = &self.as_le_bytes()[..self.size()];
        <&[u8] as ZEncode>::z_encode(&bytes, w)
    }
}

impl<'a> ZBodyDecode<'a> for ZenohIdProto {
    type Ctx = ();

    fn z_body_decode(
        r: &mut impl crate::ZReadable<'a>,
        _: (),
    ) -> core::result::Result<Self, crate::CodecError> {
        let bytes = <&[u8] as ZDecode>::z_decode(r)?;
        ZenohIdProto::try_from(bytes)
    }
}

crate::derive_zstruct_with_body!(ZenohIdProto);

impl ZBodyLen for Timestamp {
    fn z_body_len(&self) -> usize {
        let bytes = &self.get_id().to_le_bytes()[..self.get_id().size()];
        let time = self.get_time().as_u64();

        <u64 as ZLen>::z_len(&time)
            + <usize as ZLen>::z_len(&bytes.len())
            + <&[u8] as ZLen>::z_len(&bytes)
    }
}

impl ZBodyEncode for Timestamp {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        <u64 as ZEncode>::z_encode(&self.get_time().as_u64(), w)?;
        let bytes = &self.get_id().to_le_bytes()[..self.get_id().size()];
        <usize as ZEncode>::z_encode(&bytes.len(), w)?;
        <&[u8] as ZEncode>::z_encode(&bytes, w)
    }
}

impl<'a> ZBodyDecode<'a> for Timestamp {
    type Ctx = ();

    fn z_body_decode(
        r: &mut impl crate::ZReadable<'a>,
        _: (),
    ) -> core::result::Result<Self, crate::CodecError> {
        let time = NTP64(<u64 as ZDecode>::z_decode(r)?);
        let id_len = <usize as ZDecode>::z_decode(r)?;
        let id_bytes = <&[u8] as ZDecode>::z_decode(&mut r.read_slice(id_len)?)?;
        let id = uhlc::ID::try_from(id_bytes).map_err(|_| crate::CodecError::CouldNotParseField)?;
        Ok(Timestamp::new(time, id))
    }
}

crate::derive_zstruct_with_body!(Timestamp);

impl<'a> ZExt<'a> for Timestamp {
    const KIND: ZExtKind = ZExtKind::ZStruct;
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct Encoding<'a> {
    pub id: u16,

    pub schema: Option<&'a [u8]>,
}

impl Encoding<'_> {
    const FLAG_S: u8 = 0b0000_0001;

    pub const fn bytes() -> Self {
        Self {
            id: 0,
            schema: None,
        }
    }

    pub const fn string() -> Self {
        Self {
            id: 1,
            schema: None,
        }
    }

    pub const fn zenoh_bytes() -> Self {
        Self {
            id: 0,
            schema: None,
        }
    }

    pub const fn zenoh_string() -> Self {
        Self {
            id: 1,
            schema: None,
        }
    }

    pub const fn zenoh_serialized() -> Self {
        Self {
            id: 2,
            schema: None,
        }
    }

    pub const fn application_octet_stream() -> Self {
        Self {
            id: 3,
            schema: None,
        }
    }

    pub const fn text_plain() -> Self {
        Self {
            id: 4,
            schema: None,
        }
    }

    pub const fn application_json() -> Self {
        Self {
            id: 5,
            schema: None,
        }
    }

    pub const fn text_json() -> Self {
        Self {
            id: 6,
            schema: None,
        }
    }

    pub const fn application_cdr() -> Self {
        Self {
            id: 7,
            schema: None,
        }
    }

    pub const fn application_cbor() -> Self {
        Self {
            id: 8,
            schema: None,
        }
    }

    pub const fn application_yaml() -> Self {
        Self {
            id: 9,
            schema: None,
        }
    }

    pub const fn text_yaml() -> Self {
        Self {
            id: 10,
            schema: None,
        }
    }

    pub const fn text_json5() -> Self {
        Self {
            id: 11,
            schema: None,
        }
    }

    pub const fn application_python_serialized_object() -> Self {
        Self {
            id: 12,
            schema: None,
        }
    }

    pub const fn application_protobuf() -> Self {
        Self {
            id: 13,
            schema: None,
        }
    }

    pub const fn application_java_serialized_object() -> Self {
        Self {
            id: 14,
            schema: None,
        }
    }

    pub const fn application_openmetrics_text() -> Self {
        Self {
            id: 15,
            schema: None,
        }
    }

    pub const fn image_png() -> Self {
        Self {
            id: 16,
            schema: None,
        }
    }

    pub const fn image_jpeg() -> Self {
        Self {
            id: 17,
            schema: None,
        }
    }

    pub const fn image_gif() -> Self {
        Self {
            id: 18,
            schema: None,
        }
    }

    pub const fn image_bmp() -> Self {
        Self {
            id: 19,
            schema: None,
        }
    }

    pub const fn image_webp() -> Self {
        Self {
            id: 20,
            schema: None,
        }
    }

    pub const fn application_xml() -> Self {
        Self {
            id: 21,
            schema: None,
        }
    }

    pub const fn application_x_www_form_urlencoded() -> Self {
        Self {
            id: 22,
            schema: None,
        }
    }

    pub const fn text_html() -> Self {
        Self {
            id: 23,
            schema: None,
        }
    }

    pub const fn text_xml() -> Self {
        Self {
            id: 24,
            schema: None,
        }
    }

    pub const fn text_css() -> Self {
        Self {
            id: 25,
            schema: None,
        }
    }

    pub const fn text_javascript() -> Self {
        Self {
            id: 26,
            schema: None,
        }
    }

    pub const fn text_markdown() -> Self {
        Self {
            id: 27,
            schema: None,
        }
    }

    pub const fn text_csv() -> Self {
        Self {
            id: 28,
            schema: None,
        }
    }

    pub const fn application_sql() -> Self {
        Self {
            id: 29,
            schema: None,
        }
    }

    pub const fn application_coap_payload() -> Self {
        Self {
            id: 30,
            schema: None,
        }
    }

    pub const fn application_json_patch_json() -> Self {
        Self {
            id: 31,
            schema: None,
        }
    }

    pub const fn application_json_seq() -> Self {
        Self {
            id: 32,
            schema: None,
        }
    }

    pub const fn application_jsonpath() -> Self {
        Self {
            id: 33,
            schema: None,
        }
    }

    pub const fn application_jwt() -> Self {
        Self {
            id: 34,
            schema: None,
        }
    }

    pub const fn application_mp4() -> Self {
        Self {
            id: 35,
            schema: None,
        }
    }

    pub const fn application_soap_xml() -> Self {
        Self {
            id: 36,
            schema: None,
        }
    }

    pub const fn application_yang() -> Self {
        Self {
            id: 37,
            schema: None,
        }
    }

    pub const fn audio_aac() -> Self {
        Self {
            id: 38,
            schema: None,
        }
    }

    pub const fn audio_flac() -> Self {
        Self {
            id: 39,
            schema: None,
        }
    }

    pub const fn audio_mp4() -> Self {
        Self {
            id: 40,
            schema: None,
        }
    }

    pub const fn audio_ogg() -> Self {
        Self {
            id: 41,
            schema: None,
        }
    }

    pub const fn audio_vorbis() -> Self {
        Self {
            id: 42,
            schema: None,
        }
    }

    pub const fn video_h261() -> Self {
        Self {
            id: 43,
            schema: None,
        }
    }

    pub const fn video_h263() -> Self {
        Self {
            id: 44,
            schema: None,
        }
    }

    pub const fn video_h264() -> Self {
        Self {
            id: 45,
            schema: None,
        }
    }

    pub const fn video_h265() -> Self {
        Self {
            id: 46,
            schema: None,
        }
    }

    pub const fn video_h266() -> Self {
        Self {
            id: 47,
            schema: None,
        }
    }

    pub const fn video_mp4() -> Self {
        Self {
            id: 48,
            schema: None,
        }
    }

    pub const fn video_ogg() -> Self {
        Self {
            id: 49,
            schema: None,
        }
    }

    pub const fn video_raw() -> Self {
        Self {
            id: 50,
            schema: None,
        }
    }

    pub const fn video_vp8() -> Self {
        Self {
            id: 51,
            schema: None,
        }
    }

    pub const fn video_vp9() -> Self {
        Self {
            id: 52,
            schema: None,
        }
    }

    pub fn from_str(s: &str) -> Encoding<'_> {
        for (i, name) in ENCODING_NAMES.iter().enumerate() {
            if *name == s {
                return Encoding {
                    id: i as u16,
                    schema: None,
                };
            }
        }
        Encoding {
            id: 0,
            schema: Some(s.as_bytes()),
        }
    }

    pub fn to_string(&self) -> &str {
        match self.schema {
            Some(s) => core::str::from_utf8(s).unwrap_or("zenoh/bytes"),
            None => ENCODING_NAMES
                .get(self.id as usize)
                .copied()
                .unwrap_or("zenoh/bytes"),
        }
    }
}

static ENCODING_NAMES: [&'static str; 53] = [
    "zenoh/bytes",
    "zenoh/string",
    "zenoh/serialized",
    "application/octet-stream",
    "text/plain",
    "application/json",
    "text/json",
    "application/cdr",
    "application/cbor",
    "application/yaml",
    "text/yaml",
    "text/json5",
    "application/python-serialized-object",
    "application/protobuf",
    "application/java-serialized-object",
    "application/openmetrics-text",
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/bmp",
    "image/webp",
    "application/xml",
    "application/x-www-form-urlencoded",
    "text/html",
    "text/xml",
    "text/css",
    "text/javascript",
    "text/markdown",
    "text/csv",
    "application/sql",
    "application/coap-payload",
    "application/json-patch+json",
    "application/json-seq",
    "application/jsonpath",
    "application/jwt",
    "application/mp4",
    "application/soap+xml",
    "application/yang",
    "audio/aac",
    "audio/flac",
    "audio/mp4",
    "audio/ogg",
    "audio/vorbis",
    "video/h261",
    "video/h263",
    "video/h264",
    "video/h265",
    "video/h266",
    "video/mp4",
    "video/ogg",
    "video/raw",
    "video/vp8",
    "video/vp9",
];
impl ZBodyLen for Encoding<'_> {
    fn z_body_len(&self) -> usize {
        <u32 as ZLen>::z_len(&((self.id as u32) << 1))
            + if let Some(schema) = self.schema.as_ref() {
                let len: usize = <&[u8] as ZLen>::z_len(schema);

                <usize as ZLen>::z_len(&len) + len
            } else {
                0
            }
    }
}

impl ZBodyEncode for Encoding<'_> {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        let mut id = (self.id as u32) << 1;

        if self.schema.is_some() {
            id |= Self::FLAG_S as u32;
        }

        <u32 as ZEncode>::z_encode(&id, w)?;

        if let Some(schema) = &self.schema {
            <usize as ZEncode>::z_encode(&schema.len(), w)?;
            <&[u8] as ZEncode>::z_encode(schema, w)?;
        }

        Ok(())
    }
}

impl<'a> ZBodyDecode<'a> for Encoding<'a> {
    type Ctx = ();

    fn z_body_decode(
        r: &mut impl crate::ZReadable<'a>,
        _: (),
    ) -> core::result::Result<Self, crate::CodecError> {
        let id = <u32 as ZDecode>::z_decode(r)?;

        let has_schema = (id as u8) & Self::FLAG_S != 0;
        let id = (id >> 1) as u16;

        let schema = if has_schema {
            let len = <usize as ZDecode>::z_decode(r)?;
            let schema: &[u8] = <&[u8] as ZDecode>::z_decode(&mut r.read_slice(len)?)?;
            Some(schema)
        } else {
            None
        };

        Ok(Encoding { id, schema })
    }
}

crate::derive_zstruct_with_body!(lt, Encoding<'a>);

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Bits {
    U8 = 0b0000_0000,
    U16 = 0b0000_0001,
    U32 = 0b0000_0010,
    U64 = 0b0000_0011,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    FrameSN = 0,
    RequestID = 2,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Resolution(pub(crate) u8);

impl Resolution {
    pub const fn get(&self, field: Field) -> Bits {
        let value = (self.0 >> (field as u8)) & 0b11;

        match value {
            0b00 => Bits::U8,
            0b01 => Bits::U16,
            0b10 => Bits::U32,
            0b11 => Bits::U64,
            _ => unreachable!(),
        }
    }

    pub fn set(&mut self, field: Field, bits: Bits) {
        self.0 &= !(0b11 << field as u8);
        self.0 |= (bits as u8) << (field as u8);
    }
}
impl Default for Resolution {
    fn default() -> Self {
        let frame_sn = Bits::U32 as u8;
        let request_id = (Bits::U32 as u8) << 2;
        Self(frame_sn | request_id)
    }
}

impl From<u8> for Resolution {
    fn from(v: u8) -> Self {
        Self(v)
    }
}

impl ZBodyLen for Resolution {
    fn z_body_len(&self) -> usize {
        <u8 as ZBodyLen>::z_body_len(&self.0)
    }
}

impl ZBodyEncode for Resolution {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        <u8 as ZBodyEncode>::z_body_encode(&self.0, w)
    }
}

impl<'a> ZBodyDecode<'a> for Resolution {
    type Ctx = ();

    fn z_body_decode(
        r: &mut impl crate::ZReadable<'a>,
        _: (),
    ) -> core::result::Result<Self, crate::CodecError> {
        Ok(Self(<u8 as crate::ZDecode>::z_decode(r)?))
    }
}

crate::derive_zstruct_with_body!(Resolution);

#[repr(u8)]
#[derive(ZRU8, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Locality {
    #[default]
    Any = 0,
    SessionLocal = 1,
    Remote = 2,
}

#[repr(u8)]
#[derive(ZRU8, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ReplyKeyexpr {
    #[default]
    Any = 0,
    MatchingQuery = 1,
}

#[cfg(test)]
mod encoding_tests {
    use super::*;

    #[test]
    fn encoding_from_str_to_string_roundtrip() {
        let e = Encoding::from_str("text/plain");
        assert_eq!(e.to_string(), "text/plain");
    }

    #[test]
    fn encoding_from_str_to_string_roundtrip_custom() {
        let e = Encoding::from_str("custom/format");
        assert_eq!(e.to_string(), "custom/format");
    }

    #[test]
    fn encoding_id_lookup() {
        let e = Encoding::from_str("zenoh/string");
        assert_eq!(e.id, 1);
        assert!(e.schema.is_none());
    }

    #[test]
    fn encoding_to_string_unknown_id() {
        let e = Encoding {
            id: 99,
            schema: None,
        };
        assert_eq!(e.to_string(), "zenoh/bytes");
    }

    #[test]
    fn encoding_to_string_schema() {
        let e = Encoding {
            id: 0,
            schema: Some(b"my/encoding"),
        };
        assert_eq!(e.to_string(), "my/encoding");
    }
}

#[cfg(test)]
mod locality_tests {
    use super::*;

    #[test]
    fn locality_default_is_any() {
        assert_eq!(Locality::default(), Locality::Any);
    }

    #[test]
    fn locality_values() {
        assert_eq!(Locality::Any as u8, 0);
        assert_eq!(Locality::SessionLocal as u8, 1);
        assert_eq!(Locality::Remote as u8, 2);
    }
}

#[repr(u8)]
#[derive(ZRU8, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WhatAmI {
    Peer = Self::U8_P,
    Router = Self::U8_R,
    #[default]
    Client = Self::U8_C,
}

impl WhatAmI {
    const U8_R: u8 = 0b0000_0000;
    const U8_P: u8 = 0b0000_0001;
    const U8_C: u8 = 0b0000_0010;
}

#[repr(u8)]
#[derive(ZRU8, Debug, Default, PartialEq, Clone, Copy)]
pub enum Mapping {
    #[default]
    Receiver = 0,
    Sender = 1,
}

#[derive(ZExt, Debug, PartialEq, Default)]
#[zenoh(header = "_:6|M|N|")]
pub struct WireExpr<'a> {
    pub scope: u16,

    #[zenoh(header = M)]
    pub mapping: Mapping,

    #[zenoh(presence = header(N), default = "", size = prefixed)]
    pub suffix: &'a str,
}

impl<'a> From<&'a keyexpr> for WireExpr<'a> {
    fn from(ke: &'a keyexpr) -> Self {
        Self {
            scope: 0,
            mapping: Mapping::Sender,
            suffix: ke.as_str(),
        }
    }
}
