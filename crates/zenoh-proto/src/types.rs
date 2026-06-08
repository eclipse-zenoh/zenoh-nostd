use crate::{
    exts, fields, {ZBodyDecode, ZBodyEncode, ZBodyLen, ZDecode, ZEncode, ZExt, ZExtKind, ZLen},
};

/// NTP64 timestamp, matching zenoh-pico's `z_timestamp_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp {
    /// NTP64 time in 2^64ths of a second
    pub ntp64_time: u64,
    /// UUID/ID of the generating entity
    pub id: EntityGlobalId,
}

impl Timestamp {
    pub fn new(ntp64_time: u64, id: EntityGlobalId) -> Self {
        Self { ntp64_time, id }
    }

    pub fn ntp64_time(&self) -> u64 {
        self.ntp64_time
    }

    pub fn id(&self) -> &EntityGlobalId {
        &self.id
    }
}

// ─── Entity Global ID ─────────────────────────────────────────────────────────

/// UUID identifying an entity, matching zenoh-pico's `z_entity_global_id_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityGlobalId {
    /// Zenoh ID (16-byte UUID)
    pub znid: [u8; 16],
    /// Optional entity sub-identifier
    pub entity_id: Option<EntityId>,
}

impl EntityGlobalId {
    pub fn new(znid: [u8; 16]) -> Self {
        Self {
            znid,
            entity_id: None,
        }
    }

    pub fn with_entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }

    pub fn znid(&self) -> &[u8; 16] {
        &self.znid
    }

    pub fn entity_id(&self) -> Option<&EntityId> {
        self.entity_id.as_ref()
    }
}

/// Sub-identifier within an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId {
    pub key_id: u32,
}

impl EntityId {
    pub const fn new(key_id: u32) -> Self {
        Self { key_id }
    }
}

// ─── Source Info ──────────────────────────────────────────────────────────────

/// Source information, matching zenoh-pico's `z_source_info_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceInfo {
    /// Sequence number
    pub sn: u32,
    /// Entity ID (optional)
    pub id: Option<EntityGlobalId>,
    /// Source kind
    pub kind: SourceInfoKind,
}

impl SourceInfo {
    pub fn new(sn: u32, kind: SourceInfoKind) -> Self {
        Self { sn, id: None, kind }
    }

    pub fn with_id(mut self, id: EntityGlobalId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn sn(&self) -> u32 {
        self.sn
    }

    pub fn id(&self) -> Option<&EntityGlobalId> {
        self.id.as_ref()
    }

    pub fn kind(&self) -> SourceInfoKind {
        self.kind
    }
}

/// Kind of source entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SourceInfoKind {
    Auto = 0,
    Publisher = 1,
    Subscriber = 2,
    Queryable = 3,
    Router = 4,
}

// ─── Conversions to/from wire types ──────────────────────────────────────────

impl From<fields::Timestamp> for Timestamp {
    fn from(ts: fields::Timestamp) -> Self {
        let time = ts.get_time().as_u64();
        let wire_id = ts.get_id();
        let znid = {
            let mut buf = [0u8; 16];
            let bytes = wire_id.to_le_bytes();
            let len = bytes.len().min(16);
            buf[..len].copy_from_slice(&bytes[..len]);
            buf
        };
        Self {
            ntp64_time: time,
            id: EntityGlobalId::new(znid),
        }
    }
}

impl From<Timestamp> for (fields::Timestamp, bool) {
    fn from(ts: Timestamp) -> Self {
        let time = uhlc::NTP64(ts.ntp64_time);
        let id = {
            // Build uhlc::ID from the raw bytes
            let mut buf = [0u8; uhlc::ID::MAX_SIZE];
            let len = ts.id.znid.len().min(uhlc::ID::MAX_SIZE);
            buf[..len].copy_from_slice(&ts.id.znid[..len]);
            // Use try_from with &[u8], falling back to random on failure
            match uhlc::ID::try_from(&buf[..len]) {
                Ok(id) => id,
                Err(_) => {
                    // This should not happen for valid 16-byte input, but handle gracefully
                    uhlc::ID::rand()
                }
            }
        };
        (fields::Timestamp::new(time, id), false)
    }
}

impl From<exts::EntityGlobalId> for EntityGlobalId {
    fn from(wire: exts::EntityGlobalId) -> Self {
        let mut znid = [0u8; 16];
        let bytes = wire.zid.as_le_bytes();
        let len = bytes.len().min(16);
        znid[..len].copy_from_slice(&bytes[..len]);
        let entity_id = if wire.eid != 0 {
            Some(EntityId::new(wire.eid))
        } else {
            None
        };
        Self { znid, entity_id }
    }
}

impl From<EntityGlobalId> for exts::EntityGlobalId {
    fn from(public: EntityGlobalId) -> Self {
        let zid = {
            let mut buf = [0u8; uhlc::ID::MAX_SIZE];
            let len = public.znid.len().min(uhlc::ID::MAX_SIZE);
            buf[..len].copy_from_slice(&public.znid[..len]);
            fields::ZenohIdProto::try_from(&buf[..len]).unwrap_or_default()
        };
        let eid = public.entity_id.map_or(0, |e| e.key_id);
        exts::EntityGlobalId { zid, eid }
    }
}

impl From<exts::SourceInfo> for SourceInfo {
    fn from(wire: exts::SourceInfo) -> Self {
        Self {
            sn: wire.sn,
            id: Some(EntityGlobalId::from(wire.id)),
            kind: SourceInfoKind::Auto,
        }
    }
}

impl From<SourceInfo> for exts::SourceInfo {
    fn from(public: SourceInfo) -> Self {
        let id = public
            .id
            .map(exts::EntityGlobalId::from)
            .unwrap_or_else(|| exts::EntityGlobalId {
                zid: fields::ZenohIdProto::default(),
                eid: 0,
            });
        exts::SourceInfo { id, sn: public.sn }
    }
}

// ─── Codec impls (delegate to wire types) ────────────────────────────────────

impl ZBodyLen for Timestamp {
    fn z_body_len(&self) -> usize {
        let wire_ts: (fields::Timestamp, bool) = (*self).into();
        wire_ts.0.z_body_len()
    }
}

impl ZBodyEncode for Timestamp {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        let wire_ts: (fields::Timestamp, bool) = (*self).into();
        wire_ts.0.z_body_encode(w)
    }
}

impl<'a> ZBodyDecode<'a> for Timestamp {
    type Ctx = ();

    fn z_body_decode(
        r: &mut impl crate::ZReadable<'a>,
        _: (),
    ) -> core::result::Result<Self, crate::CodecError> {
        let wire_ts = <fields::Timestamp as ZBodyDecode>::z_body_decode(r, ())?;
        Ok(Timestamp::from(wire_ts))
    }
}

crate::derive_zstruct_with_body!(Timestamp);

impl<'a> ZExt<'a> for Timestamp {
    const KIND: ZExtKind = ZExtKind::ZStruct;
}

impl ZBodyLen for EntityGlobalId {
    fn z_body_len(&self) -> usize {
        let wire: exts::EntityGlobalId = (*self).into();
        wire.z_len()
    }
}

impl ZBodyEncode for EntityGlobalId {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        let wire: exts::EntityGlobalId = (*self).into();
        wire.z_encode(w)
    }
}

impl<'a> ZBodyDecode<'a> for EntityGlobalId {
    type Ctx = ();

    fn z_body_decode(
        r: &mut impl crate::ZReadable<'a>,
        _: (),
    ) -> core::result::Result<Self, crate::CodecError> {
        let wire = <exts::EntityGlobalId as ZDecode>::z_decode(r)?;
        Ok(EntityGlobalId::from(wire))
    }
}

crate::derive_zstruct_with_body!(EntityGlobalId);

impl<'a> ZExt<'a> for EntityGlobalId {
    const KIND: ZExtKind = ZExtKind::ZStruct;
}

impl ZBodyLen for SourceInfo {
    fn z_body_len(&self) -> usize {
        let wire: exts::SourceInfo = (*self).into();
        wire.z_body_len()
    }
}

impl ZBodyEncode for SourceInfo {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        let wire: exts::SourceInfo = (*self).into();
        wire.z_body_encode(w)
    }
}

impl<'a> ZBodyDecode<'a> for SourceInfo {
    type Ctx = ();

    fn z_body_decode(
        r: &mut impl crate::ZReadable<'a>,
        _: (),
    ) -> core::result::Result<Self, crate::CodecError> {
        let wire = <exts::SourceInfo as ZBodyDecode>::z_body_decode(r, ())?;
        Ok(SourceInfo::from(wire))
    }
}

crate::derive_zstruct_with_body!(SourceInfo);

impl<'a> ZExt<'a> for SourceInfo {
    const KIND: ZExtKind = ZExtKind::ZStruct;
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_new() {
        let id = EntityGlobalId::new([0u8; 16]);
        let ts = Timestamp::new(42, id);
        assert_eq!(ts.ntp64_time(), 42);
        assert_eq!(ts.id(), &id);
    }

    #[test]
    fn test_timestamp_accessors() {
        let id = EntityGlobalId::new([0u8; 16]);
        let ts = Timestamp::new(12345, id);
        assert_eq!(ts.ntp64_time(), 12345);
        assert_eq!(*ts.id(), id);
    }

    #[test]
    fn test_timestamp_eq() {
        let id = EntityGlobalId::new([0u8; 16]);
        let a = Timestamp::new(42, id);
        let b = Timestamp::new(42, id);
        assert_eq!(a, b);
    }

    #[test]
    fn test_timestamp_ne() {
        let id = EntityGlobalId::new([0u8; 16]);
        let a = Timestamp::new(42, id);
        let b = Timestamp::new(99, id);
        assert_ne!(a, b);
    }

    #[test]
    fn test_entity_global_id_new() {
        let znid = [1u8; 16];
        let eid = EntityGlobalId::new(znid);
        assert_eq!(*eid.znid(), znid);
        assert!(eid.entity_id().is_none());
    }

    #[test]
    fn test_entity_global_id_with_entity() {
        let znid = [2u8; 16];
        let eid = EntityGlobalId::new(znid).with_entity_id(EntityId::new(7));
        assert_eq!(eid.entity_id().unwrap().key_id, 7);
    }

    #[test]
    fn test_entity_global_id_copy() {
        let a = EntityGlobalId::new([3u8; 16]).with_entity_id(EntityId::new(42));
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn test_entity_id_new() {
        let id = EntityId::new(42);
        assert_eq!(id.key_id, 42);
    }

    #[test]
    fn test_source_info_new() {
        let si = SourceInfo::new(100, SourceInfoKind::Publisher);
        assert_eq!(si.sn(), 100);
        assert_eq!(si.kind(), SourceInfoKind::Publisher);
        assert!(si.id().is_none());
    }

    #[test]
    fn test_source_info_with_id() {
        let eid = EntityGlobalId::new([4u8; 16]).with_entity_id(EntityId::new(3));
        let si = SourceInfo::new(200, SourceInfoKind::Subscriber).with_id(eid);
        assert_eq!(si.sn(), 200);
        assert_eq!(si.kind(), SourceInfoKind::Subscriber);
        assert_eq!(si.id().unwrap().entity_id().unwrap().key_id, 3);
    }

    #[test]
    fn test_source_info_kind_default() {
        assert_eq!(SourceInfoKind::Auto as u8, 0);
        assert_eq!(SourceInfoKind::Publisher as u8, 1);
        assert_eq!(SourceInfoKind::Subscriber as u8, 2);
        assert_eq!(SourceInfoKind::Queryable as u8, 3);
        assert_eq!(SourceInfoKind::Router as u8, 4);
    }

    #[test]
    fn test_source_info_copy() {
        let si = SourceInfo::new(10, SourceInfoKind::Queryable);
        let si2 = si;
        assert_eq!(si, si2);
    }

    #[test]
    fn test_entity_global_id_wire_roundtrip() {
        let original = EntityGlobalId::new([5u8; 16]).with_entity_id(EntityId::new(42));
        let wire: exts::EntityGlobalId = original.into();
        let back: EntityGlobalId = wire.into();
        assert_eq!(original.znid, back.znid);
        assert_eq!(
            original.entity_id.map(|e| e.key_id),
            back.entity_id.map(|e| e.key_id)
        );
    }

    #[test]
    fn test_source_info_wire_roundtrip() {
        let eid = EntityGlobalId::new([6u8; 16]).with_entity_id(EntityId::new(7));
        let original = SourceInfo::new(300, SourceInfoKind::Publisher).with_id(eid);
        let wire: exts::SourceInfo = original.into();
        let back: SourceInfo = wire.into();
        assert_eq!(original.sn, back.sn);
        assert_eq!(original.kind, SourceInfoKind::Auto);
    }

    #[test]
    fn test_timestamp_wire_roundtrip() {
        let zmk = EntityGlobalId::new([7u8; 16]);
        let original = Timestamp::new(999999, zmk);
        let (wire, _): (fields::Timestamp, bool) = original.into();
        let back = Timestamp::from(wire);
        assert_eq!(original.ntp64_time, back.ntp64_time);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let eid = EntityGlobalId::new([8u8; 16]).with_entity_id(EntityId::new(9));
        let original = Timestamp::new(55555, eid);

        let mut buf = [0u8; 128];
        let mut writer = &mut buf[..];
        assert!(original.z_encode(&mut writer).is_ok());
        let written = 128 - writer.len();

        let decoded = <Timestamp as ZDecode>::z_decode(&mut &buf[..written]).unwrap();
        assert_eq!(original.ntp64_time, decoded.ntp64_time);
        assert_eq!(original.id.znid, decoded.id.znid);
    }

    #[test]
    fn test_entity_global_id_encode_decode() {
        let original = EntityGlobalId::new([9u8; 16]).with_entity_id(EntityId::new(10));

        let mut buf = [0u8; 128];
        let mut writer = &mut buf[..];
        assert!(original.z_encode(&mut writer).is_ok());
        let written = 128 - writer.len();

        let decoded = <EntityGlobalId as ZDecode>::z_decode(&mut &buf[..written]).unwrap();
        assert_eq!(original.znid, decoded.znid);
    }

    #[test]
    fn test_source_info_encode_decode() {
        let eid = EntityGlobalId::new([10u8; 16]).with_entity_id(EntityId::new(11));
        let original = SourceInfo::new(777, SourceInfoKind::Router).with_id(eid);

        let mut buf = [0u8; 128];
        let mut writer = &mut buf[..];
        assert!(original.z_encode(&mut writer).is_ok());
        let written = 128 - writer.len();

        let decoded = <SourceInfo as ZDecode>::z_decode(&mut &buf[..written]).unwrap();
        assert_eq!(original.sn, decoded.sn);
    }

    #[test]
    fn test_timestamp_debug() {
        let id = EntityGlobalId::new([0xABu8; 16]);
        let ts = Timestamp::new(12345, id);
        let debug = format!("{:?}", ts);
        assert!(debug.contains("Timestamp"));
        assert!(debug.contains("12345"));
    }

    #[test]
    fn test_entity_global_id_debug() {
        let eid = EntityGlobalId::new([0xBBu8; 16]).with_entity_id(EntityId::new(88));
        let debug = format!("{:?}", eid);
        assert!(debug.contains("EntityGlobalId"));
    }

    #[test]
    fn test_source_info_debug() {
        let si = SourceInfo::new(999, SourceInfoKind::Publisher);
        let debug = format!("{:?}", si);
        assert!(debug.contains("SourceInfo"));
        assert!(debug.contains("999"));
    }

    #[test]
    fn test_source_info_kind_eq() {
        assert_eq!(SourceInfoKind::Auto, SourceInfoKind::Auto);
        assert_ne!(SourceInfoKind::Auto, SourceInfoKind::Publisher);
        assert_ne!(SourceInfoKind::Publisher, SourceInfoKind::Subscriber);
        assert_ne!(SourceInfoKind::Subscriber, SourceInfoKind::Queryable);
        assert_ne!(SourceInfoKind::Queryable, SourceInfoKind::Router);
    }

    #[test]
    fn test_entity_id_hash_eq() {
        let a = EntityId::new(1);
        let b = EntityId::new(1);
        assert_eq!(a, b);
    }

    #[test]
    fn test_entity_id_hash_ne() {
        let a = EntityId::new(1);
        let b = EntityId::new(2);
        assert_ne!(a, b);
    }
}
