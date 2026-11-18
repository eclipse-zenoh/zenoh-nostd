use core::time::Duration;

use crate::{fields::*, *};

#[derive(ZExt, Debug, PartialEq, Default)]
#[zenoh(header = "ID:4|_:4")]
/// Globally identifies an entity using a Zenoh node ID plus local entity ID.
pub struct EntityGlobalId {
    #[zenoh(size = header(ID))]
    /// Zenoh node identifier.
    pub zid: ZenohIdProto,

    /// Entity ID local to the indicated node.
    pub eid: u32,
}

#[derive(ZExt, Debug, PartialEq, Default)]
/// Source entity and sequence number attached to an operation.
pub struct SourceInfo {
    /// Globally identified source entity.
    pub id: EntityGlobalId,
    /// Sequence number associated with the source, frame or opening message.
    pub sn: u32,
}

#[derive(ZExt, Debug, PartialEq, Default)]
/// Encoded value borrowing its schema and payload bytes.
pub struct Value<'a> {
    /// Payload encoding metadata, not a payload conversion.
    pub encoding: Encoding<'a>,

    #[zenoh(size = remain)]
    /// Borrowed payload or nested operation; the codec does not interpret application bytes.
    pub payload: &'a [u8],
}

#[derive(ZExt, Debug, PartialEq, Default, Clone)]
/// Opaque borrowed application metadata, with no interpretation by the codec.
pub struct Attachment<'a> {
    #[zenoh(size = remain)]
    /// Borrowed opaque attachment bytes.
    pub buffer: &'a [u8],
}

#[derive(ZExt, Clone, Copy, Debug, PartialEq)]
/// Packed wire quality-of-service byte. `Default` selects data priority, dropping congestion policy and non-express delivery.
pub struct QoS {
    /// Packed priority, congestion and express-delivery bits.
    pub inner: u8,
}

impl Default for QoS {
    fn default() -> Self {
        Self::new(Priority::Data, CongestionControl::Drop, false)
    }
}

impl QoS {
    const D_FLAG: u8 = 0b00001000;
    const E_FLAG: u8 = 0b00010000;

    /// QoS settings for declarations: data priority, blocking congestion policy, non-express.
    pub const fn declare() -> Self {
        Self::new(Priority::Data, CongestionControl::Block, false)
    }

    /// Pack priority, congestion policy and express-delivery flags without enforcing their behavior locally.
    pub const fn new(
        priority: Priority,
        congestion_control: CongestionControl,
        is_express: bool,
    ) -> Self {
        let mut inner = priority as u8;
        if matches!(congestion_control, CongestionControl::Block) {
            inner |= Self::D_FLAG;
        }
        if is_express {
            inner |= Self::E_FLAG;
        }
        Self { inner }
    }
}

#[derive(ZExt, Debug, PartialEq, Default)]
/// Routing node identifier extension.
pub struct NodeId {
    /// Routing node identifier.
    pub node_id: u16,
}
#[repr(u8)]
#[derive(ZRU8, Debug, Default, Clone, Copy, PartialEq)]
/// Peer-side query routing selection; does not itself discover or execute queryables.
pub enum QueryTarget {
    #[default]
    /// Route to the best matching queryables (default).
    BestMatching = 0,
    /// Route to all matching queryables.
    All = 1,
    /// Route to all matching complete queryables.
    AllComplete = 2,
}

#[derive(Debug, PartialEq, Default)]
/// Completeness and routing distance advertised by a queryable.
pub struct QueryableInfo {
    /// Whether this queryable advertises complete coverage of its expression.
    pub complete: bool,
    /// Advertised routing distance to the queryable.
    pub distance: u16,
}

impl ZBodyLen for QueryableInfo {
    fn z_body_len(&self) -> usize {
        <u64 as ZLen>::z_len(&self.as_u64())
    }
}

impl ZBodyEncode for QueryableInfo {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        <u64 as ZEncode>::z_encode(&self.as_u64(), w)
    }
}

impl<'a> ZBodyDecode<'a> for QueryableInfo {
    type Ctx = ();

    fn z_body_decode(
        r: &mut impl crate::ZReadable<'a>,
        _: (),
    ) -> core::result::Result<Self, crate::CodecError> {
        let value = <u64 as ZDecode>::z_decode(r)?;
        let complete = (value & 0b0000_0001) != 0;
        let distance = ((value >> 8) & 0xFFFF) as u16;
        Ok(QueryableInfo { complete, distance })
    }
}

crate::derive_zstruct_with_body!(QueryableInfo);

impl<'a> ZExt<'a> for QueryableInfo {
    const KIND: ZExtKind = ZExtKind::U64;
}

impl QueryableInfo {
    fn as_u64(&self) -> u64 {
        let mut flags: u8 = 0;
        if self.complete {
            flags |= 0b0000_0001;
        }
        (flags as u64) | ((self.distance as u64) << 8)
    }
}

#[derive(ZExt, Debug, PartialEq, Default)]
/// Query-routing budget extension.
pub struct Budget {
    /// Remaining query-routing budget value.
    pub budget: u32,
}

impl ZHeader for Duration {
    fn z_header(&self) -> u8 {
        let header = 0u8;
        match self.as_millis() % 1_000 {
            0 => header | 0b0000_0001,
            _ => header,
        }
    }
}

impl ZBodyLen for Duration {
    fn z_body_len(&self) -> usize {
        let v = match self.as_millis() % 1_000 {
            0 => self.as_millis() / 1_000,
            _ => self.as_millis(),
        } as u64;

        <u64 as ZLen>::z_len(&v)
    }
}

impl ZLen for Duration {
    fn z_len(&self) -> usize {
        <u64 as ZLen>::z_len(&(self.as_millis() as u64))
    }
}

impl ZBodyEncode for Duration {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        let v = match self.as_millis() % 1_000 {
            0 => self.as_millis() / 1_000,
            _ => self.as_millis(),
        } as u64;

        <u64 as ZEncode>::z_encode(&v, w)
    }
}

impl ZEncode for Duration {
    fn z_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        <u64 as ZEncode>::z_encode(&(self.as_millis() as u64), w)
    }
}

impl<'a> ZBodyDecode<'a> for Duration {
    type Ctx = u8;

    fn z_body_decode(
        r: &mut impl crate::ZReadable<'a>,
        h: u8,
    ) -> core::result::Result<Self, crate::CodecError> {
        let is_seconds = (h & 0b0000_0001) != 0;
        let value = <u64 as ZDecode>::z_decode(r)?;
        if is_seconds {
            Ok(Duration::from_secs(value))
        } else {
            Ok(Duration::from_millis(value))
        }
    }
}

impl<'a> ZDecode<'a> for Duration {
    fn z_decode(
        r: &mut impl crate::ZReadable<'a>,
    ) -> core::result::Result<Self, crate::CodecError> {
        let value = <u64 as ZDecode>::z_decode(r)?;
        Ok(Duration::from_millis(value))
    }
}

impl<'a> ZExt<'a> for Duration {
    const KIND: ZExtKind = ZExtKind::U64;
}

#[derive(ZExt, Debug, PartialEq, Default)]
/// Flag extension advertising QoS support.
pub struct HasQoS {}

#[derive(ZExt, Debug, PartialEq, Default)]
/// Per-link QoS negotiation value.
pub struct QoSLink {
    /// Packed per-link QoS negotiation value.
    pub qos: u64,
}

#[derive(ZExt, Debug, PartialEq, Default)]
/// Opaque authentication extension bytes; encoding them does not implement an authentication mechanism.
pub struct Auth<'a> {
    #[zenoh(size = remain)]
    /// Borrowed payload or nested operation; the codec does not interpret application bytes.
    pub payload: &'a [u8],
}

#[derive(ZExt, Debug, PartialEq, Default)]
/// Opaque multilink initialization extension bytes.
pub struct MultiLink<'a> {
    #[zenoh(size = remain)]
    /// Borrowed payload or nested operation; the codec does not interpret application bytes.
    pub payload: &'a [u8],
}

#[derive(ZExt, Debug, PartialEq, Default)]
/// Opaque multilink open-request extension bytes.
pub struct MultiLinkSyn<'a> {
    #[zenoh(size = remain)]
    /// Borrowed payload or nested operation; the codec does not interpret application bytes.
    pub payload: &'a [u8],
}

#[derive(ZExt, Debug, PartialEq, Default)]
/// Flag acknowledging multilink negotiation.
pub struct HasMultiLinkAck {}

#[derive(ZExt, Debug, PartialEq, Default)]
/// Flag advertising low-latency transport capability.
pub struct HasLowLatency {}

#[derive(ZExt, Debug, PartialEq, Default)]
/// Flag advertising compression capability; the codec does not compress payloads.
pub struct HasCompression {}

#[derive(ZExt, Debug, PartialEq, Default)]
/// Protocol patch-level negotiation value.
pub struct Patch {
    /// Protocol patch level; zero disables patch extensions and one is the current level.
    pub int: u8,
}

impl Patch {
    /// Patch level zero, indicating no patch extension.
    pub fn none() -> Self {
        Self { int: 0 }
    }

    /// Current supported patch level (one).
    pub fn current() -> Self {
        Self { int: 1 }
    }
}
