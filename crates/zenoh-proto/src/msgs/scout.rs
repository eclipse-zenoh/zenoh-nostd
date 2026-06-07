use crate::{fields::*, *};

/// Scout message ID in the scouting sub-protocol header (5-bit).
const SCOUT_ID: u8 = 0x01;
/// Hello message ID in the scouting sub-protocol header (5-bit).
const HELLO_ID: u8 = 0x02;

/// Maximum number of locators in a Hello message.
pub const MAX_LOCATORS: usize = 8;
/// Maximum length of a locator protocol string.
pub const MAX_LOCATOR_PROTOCOL: usize = 16;
/// Maximum length of a locator address string.
pub const MAX_LOCATOR_ADDRESS: usize = 64;

/// A locator string in the form `protocol/address`, e.g. `tcp/192.168.1.1:7447`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZLocator {
    /// Protocol part (e.g. "tcp", "udp", "tls").
    pub protocol: heapless::String<MAX_LOCATOR_PROTOCOL>,
    /// Address part (e.g. "192.168.1.1:7447", "[::1]:7447").
    pub address: heapless::String<MAX_LOCATOR_ADDRESS>,
}

impl ZLocator {
    pub fn as_str(&self) -> heapless::String<{ MAX_LOCATOR_PROTOCOL + 1 + MAX_LOCATOR_ADDRESS }> {
        let mut s = heapless::String::new();
        s.push_str(&self.protocol).ok();
        s.push('/').ok();
        s.push_str(&self.address).ok();
        s
    }
}

impl ZBodyLen for ZLocator {
    fn z_body_len(&self) -> usize {
        // Length encoding: protocol_len (1 byte) + protocol + '/' + address
        let loc_str = self.as_str();
        // We encode as a z8-prefixed string: 1 byte for length + str bytes
        1 + loc_str.len()
    }
}

impl ZBodyEncode for ZLocator {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        let loc_str = self.as_str();
        let len = loc_str.len();
        if len > u8::MAX as usize {
            return Err(crate::CodecError::CouldNotWrite);
        }
        w.write_u8(len as u8)?;
        w.write_exact(loc_str.as_bytes())?;
        Ok(())
    }
}

impl<'a> ZBodyDecode<'a> for ZLocator {
    type Ctx = ();

    fn z_body_decode(
        r: &mut impl crate::ZReadable<'a>,
        _: (),
    ) -> core::result::Result<Self, crate::CodecError> {
        let len = r.read_u8()? as usize;
        let bytes = r.read_slice(len)?;
        let s = core::str::from_utf8(bytes).map_err(|_| crate::CodecError::CouldNotParseField)?;
        let slash_pos = s.find('/').ok_or(crate::CodecError::CouldNotParseField)?;
        let protocol_str = &s[..slash_pos];
        let address_str = &s[slash_pos + 1..];
        if protocol_str.is_empty() || address_str.is_empty() {
            return Err(crate::CodecError::CouldNotParseField);
        }
        let mut protocol = heapless::String::new();
        protocol
            .push_str(protocol_str)
            .map_err(|_| crate::CodecError::CouldNotParseField)?;
        let mut address = heapless::String::new();
        address
            .push_str(address_str)
            .map_err(|_| crate::CodecError::CouldNotParseField)?;
        Ok(ZLocator { protocol, address })
    }
}

crate::derive_zstruct_with_body!(ZLocator);

/// Scout message — sent to discover Zenoh routers and peers.
///
/// Wire format (upstream Zenoh protocol spec):
/// ```text
///  7 6 5 4 3 2 1 0
/// +-+-+-+-+-+-+-+-+
/// |Z|X|X|  SCOUT  |   ID = 0x01
/// +-+-+-+-+-------+
/// |    version    |   Protocol version (u8); current = 0x09
/// +---------------+
/// |zid_len|I|WHAT |   Packed byte
/// +-+-+-+-+-+-+-+-+
/// ~      ZID      ~   if I==1: ZenohID (1 + zid_len bytes)
/// +---------------+
/// ~  [ScoutExts]  ~   if Z==1: extension chain
/// +---------------+
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scout {
    /// Version byte (VERSION = 0x09).
    pub version: u8,
    /// WhatAmI matcher bitmap.
    pub what: WhatAmIMatcher,
    /// Optional Zenoh ID (for self-suppression).
    pub zid: Option<ZenohIdProto>,
}

impl Scout {
    pub fn packed_byte(&self) -> u8 {
        // Bits 7:4 = zid_len (0-15), Bit 3 = I flag, Bits 2:0 = WHAT
        let mut pb: u8 = self.what.bits() & 0x07;
        if let Some(zid) = &self.zid {
            let zid_len = zid.size();
            let zid_field = if zid_len == 0 { 0 } else { (zid_len - 1) as u8 };
            pb |= 0x08; // I flag
            pb |= (zid_field & 0x0f) << 4;
        }
        pb
    }
}

impl Default for Scout {
    fn default() -> Self {
        Self {
            version: crate::VERSION,
            what: WhatAmIMatcher::from_bits(0x07),
            zid: None,
        }
    }
}

impl ZBodyLen for Scout {
    fn z_body_len(&self) -> usize {
        // header byte + version byte + packed byte + optional ZID
        let mut len = 1 + 1 + 1;
        if let Some(zid) = &self.zid {
            len += zid.size();
        }
        len
    }
}

impl ZLen for Scout {
    fn z_len(&self) -> usize {
        self.z_body_len()
    }
}

impl ZBodyEncode for Scout {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        // Header byte (no extensions, no reserved flags)
        let header = SCOUT_ID;
        w.write_u8(header)?;
        // Version
        w.write_u8(self.version)?;
        // Packed byte: zid_len|I|WHAT
        w.write_u8(self.packed_byte())?;
        // Optional ZID
        if let Some(zid) = &self.zid {
            let bytes = &zid.as_le_bytes()[..zid.size()];
            w.write_exact(bytes)?;
        }
        Ok(())
    }
}

impl ZEncode for Scout {
    fn z_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        self.z_body_encode(w)
    }
}

impl<'a> ZBodyDecode<'a> for Scout {
    type Ctx = ();

    fn z_body_decode(
        r: &mut impl crate::ZReadable<'a>,
        _: (),
    ) -> core::result::Result<Self, crate::CodecError> {
        // Read header byte and verify ID
        let header = r.read_u8()?;
        if header & 0x1f != SCOUT_ID {
            return Err(crate::CodecError::CouldNotParseHeader);
        }
        // Version
        let version = r.read_u8()?;
        if version != crate::VERSION {
            return Err(crate::CodecError::CouldNotParseHeader);
        }
        // Packed byte
        let pb = r.read_u8()?;
        let what_bits = pb & 0x07;
        let what = WhatAmIMatcher::from_bits(what_bits);
        let i_flag = (pb & 0x08) != 0;
        let zid_len_field = (pb >> 4) & 0x0f;

        let zid = if i_flag {
            let actual_len = 1 + zid_len_field as usize;
            let zid_bytes = r.read_slice(actual_len)?;
            Some(ZenohIdProto::try_from(zid_bytes)?)
        } else {
            None
        };

        Ok(Scout { version, what, zid })
    }
}

impl<'a> ZDecode<'a> for Scout {
    fn z_decode(
        r: &mut impl crate::ZReadable<'a>,
    ) -> core::result::Result<Self, crate::CodecError> {
        <Self as ZBodyDecode>::z_body_decode(r, ())
    }
}

/// Hello message — response to a Scout.
///
/// Wire format (upstream Zenoh protocol spec):
/// ```text
///  7 6 5 4 3 2 1 0
/// +-+-+-+-+-+-+-+-+
/// |Z|X|L|  HELLO  |   ID = 0x02
/// +-+-+-+-+-------+
/// |    version    |   Protocol version (u8); current = 0x09
/// +---------------+
/// |zid_len|X|X|WAI|   Packed byte
/// +-+-+-+-+-+-+-+-+
/// ~      ZID      ~   ZenohID (1 + zid_len bytes)
/// +---------------+
/// % loc_count: z8 %   if L==1: number of locator strings
/// ~   locators    ~   if L==1: loc_count × <utf8;z8> locator strings
/// +---------------+
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hello {
    /// Protocol version.
    pub version: u8,
    /// WhatAmI of the responding node.
    pub whatami: WhatAmI,
    /// Zenoh ID (UUID).
    pub zid: ZenohIdProto,
    /// Locators where the node can be reached.
    pub locators: heapless::Vec<ZLocator, MAX_LOCATORS>,
}

impl Hello {
    fn header(&self) -> u8 {
        let mut h = HELLO_ID;
        if !self.locators.is_empty() {
            h |= 0b0010_0000;
        }
        h
    }

    pub fn packed_byte(&self) -> u8 {
        let zid_len = self.zid.size();
        let zid_field = if zid_len == 0 { 0 } else { (zid_len - 1) as u8 };
        (zid_field & 0x0f) << 4 | (self.whatami as u8 & 0x03)
    }
}

impl Default for Hello {
    fn default() -> Self {
        Self {
            version: crate::VERSION,
            whatami: WhatAmI::default(),
            zid: ZenohIdProto::default(),
            locators: heapless::Vec::new(),
        }
    }
}

impl ZBodyLen for Hello {
    fn z_body_len(&self) -> usize {
        // header + version + packed_byte + ZID + optional locator list
        let mut len = 1 + 1 + 1 + self.zid.size();
        if !self.locators.is_empty() {
            len += 1; // loc_count
            for loc in &self.locators {
                len += loc.z_body_len();
            }
        }
        len
    }
}

impl ZLen for Hello {
    fn z_len(&self) -> usize {
        self.z_body_len()
    }
}

impl ZBodyEncode for Hello {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        // Header byte
        w.write_u8(self.header())?;
        // Version
        w.write_u8(self.version)?;
        // Packed byte: zid_len|ZID|WAI
        w.write_u8(self.packed_byte())?;
        // ZID (fixed-length 16 bytes)
        {
            let bytes = &self.zid.as_le_bytes()[..self.zid.size()];
            w.write_exact(bytes)?;
        }
        // Locator list (if L flag set)
        if !self.locators.is_empty() {
            let count = self.locators.len();
            if count > u8::MAX as usize {
                return Err(crate::CodecError::CouldNotWrite);
            }
            w.write_u8(count as u8)?;
            for loc in &self.locators {
                loc.z_body_encode(w)?;
            }
        }
        Ok(())
    }
}

impl ZEncode for Hello {
    fn z_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        self.z_body_encode(w)
    }
}

impl<'a> ZBodyDecode<'a> for Hello {
    type Ctx = ();

    fn z_body_decode(
        r: &mut impl crate::ZReadable<'a>,
        _: (),
    ) -> core::result::Result<Self, crate::CodecError> {
        // Read header byte
        let header = r.read_u8()?;
        if header & 0x1f != HELLO_ID {
            return Err(crate::CodecError::CouldNotParseHeader);
        }
        let l_flag = (header & 0b001_00000) != 0;

        // Version
        let version = r.read_u8()?;
        if version != crate::VERSION {
            return Err(crate::CodecError::CouldNotParseHeader);
        }

        // Packed byte: zid_len|XX|WAI
        let pb = r.read_u8()?;
        let wai_val = pb & 0x03;
        let whatami = match wai_val {
            0 => WhatAmI::Router,
            1 => WhatAmI::Peer,
            _ => WhatAmI::Client,
        };
        let zid_len_field = (pb >> 4) & 0x0f;
        let actual_zid_len = 1 + zid_len_field as usize;

        // ZID
        let zid_bytes = r.read_slice(actual_zid_len)?;
        let zid = ZenohIdProto::try_from(zid_bytes)?;

        // Locator list
        let mut locators: heapless::Vec<ZLocator, MAX_LOCATORS> = heapless::Vec::new();
        if l_flag {
            let count = r.read_u8()? as usize;
            if count > MAX_LOCATORS {
                return Err(crate::CodecError::CouldNotParseField);
            }
            for _ in 0..count {
                let loc = ZLocator::z_body_decode(r, ())?;
                locators
                    .push(loc)
                    .map_err(|_| crate::CodecError::CouldNotParseField)?;
            }
        }

        Ok(Hello {
            version,
            whatami,
            zid,
            locators,
        })
    }
}

impl<'a> ZDecode<'a> for Hello {
    fn z_decode(
        r: &mut impl crate::ZReadable<'a>,
    ) -> core::result::Result<Self, crate::CodecError> {
        <Self as ZBodyDecode>::z_body_decode(r, ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUF_SIZE: usize = 512;

    fn zid_from_bytes(bytes: &[u8; 8]) -> ZenohIdProto {
        ZenohIdProto::try_from(&bytes[..]).unwrap()
    }

    // --- Scout tests ---

    #[test]
    fn scout_encode_decode_roundtrip_no_zid() {
        let scout = Scout {
            version: crate::VERSION,
            what: WhatAmIMatcher::from_bits(0x07), // Router | Peer | Client
            zid: None,
        };
        let len = scout.z_len();
        let mut buf = [0u8; BUF_SIZE];
        scout.z_encode(&mut &mut buf[..]).unwrap();
        let decoded = <Scout as ZDecode>::z_decode(&mut &buf[..len]).unwrap();
        assert_eq!(scout, decoded);
        let mut buf2 = [0u8; BUF_SIZE];
        decoded.z_encode(&mut &mut buf2[..]).unwrap();
        assert_eq!(&buf[..len], &buf2[..len], "re-encode");
    }

    #[test]
    fn scout_encode_decode_with_zid() {
        let scout = Scout {
            version: crate::VERSION,
            what: WhatAmIMatcher::ROUTER,
            zid: Some(zid_from_bytes(&[0, 0, 0, 0, 0, 0, 0, 1])),
        };
        let len = scout.z_len();
        let mut buf = [0u8; BUF_SIZE];
        scout.z_encode(&mut &mut buf[..]).unwrap();
        let decoded = <Scout as ZDecode>::z_decode(&mut &buf[..len]).unwrap();
        assert_eq!(scout, decoded);
    }

    #[test]
    fn scout_encode_decode_peer_only() {
        let scout = Scout {
            version: crate::VERSION,
            what: WhatAmIMatcher::PEER,
            zid: None,
        };
        let len = scout.z_len();
        let mut buf = [0u8; BUF_SIZE];
        scout.z_encode(&mut &mut buf[..]).unwrap();
        let decoded = <Scout as ZDecode>::z_decode(&mut &buf[..len]).unwrap();
        assert_eq!(scout, decoded);
    }

    #[test]
    fn scout_encode_decode_router_only() {
        let scout = Scout {
            version: crate::VERSION,
            what: WhatAmIMatcher::ROUTER,
            zid: None,
        };
        let len = scout.z_len();
        let mut buf = [0u8; BUF_SIZE];
        scout.z_encode(&mut &mut buf[..]).unwrap();
        let decoded = <Scout as ZDecode>::z_decode(&mut &buf[..len]).unwrap();
        assert_eq!(scout, decoded);
    }

    #[test]
    fn scout_rejects_wrong_version() {
        // Build valid bytes then modify version
        let scout = Scout::default();
        let mut buf = [0u8; BUF_SIZE];
        scout.z_encode(&mut &mut buf[..]).unwrap();
        // Corrupt version byte (byte 1 after header)
        buf[1] = 0x00;
        let result = <Scout as ZDecode>::z_decode(&mut &buf[..scout.z_len()]);
        assert!(result.is_err());
    }

    #[test]
    fn scout_rejects_truncated() {
        let scout = Scout {
            version: crate::VERSION,
            what: WhatAmIMatcher::from_bits(0x07),
            zid: None,
        };
        let _len = scout.z_len();
        let mut buf = [0u8; BUF_SIZE];
        scout.z_encode(&mut &mut buf[..]).unwrap();
        // Try decoding only first byte
        let result = <Scout as ZDecode>::z_decode(&mut &buf[..1]);
        assert!(result.is_err());
    }

    // --- Hello tests ---

    #[test]
    fn hello_encode_decode_roundtrip_no_locators() {
        let hello = Hello {
            version: crate::VERSION,
            whatami: WhatAmI::Router,
            zid: zid_from_bytes(&[0, 0, 0, 0, 0, 0, 0, 1]),
            locators: heapless::Vec::new(),
        };
        let len = hello.z_len();
        let mut buf = [0u8; BUF_SIZE];
        hello.z_encode(&mut &mut buf[..]).unwrap();
        let decoded = <Hello as ZDecode>::z_decode(&mut &buf[..len]).unwrap();
        assert_eq!(hello, decoded);
    }

    #[test]
    fn hello_encode_decode_with_locators() {
        let mut locators: heapless::Vec<ZLocator, MAX_LOCATORS> = heapless::Vec::new();
        locators
            .push(ZLocator {
                protocol: heapless::String::try_from("tcp").unwrap(),
                address: heapless::String::try_from("192.168.1.1:7447").unwrap(),
            })
            .unwrap();
        locators
            .push(ZLocator {
                protocol: heapless::String::try_from("udp").unwrap(),
                address: heapless::String::try_from("224.0.0.224:7447").unwrap(),
            })
            .unwrap();

        let hello = Hello {
            version: crate::VERSION,
            whatami: WhatAmI::Peer,
            zid: zid_from_bytes(&[0, 0, 0, 0, 0, 0, 0, 2]),
            locators,
        };
        let len = hello.z_len();
        let mut buf = [0u8; BUF_SIZE];
        hello.z_encode(&mut &mut buf[..]).unwrap();
        let decoded = <Hello as ZDecode>::z_decode(&mut &buf[..len]).unwrap();
        assert_eq!(hello, decoded);
    }

    #[test]
    fn hello_all_whatami_values() {
        for whatami in [WhatAmI::Router, WhatAmI::Peer, WhatAmI::Client] {
            let hello = Hello {
                version: crate::VERSION,
                whatami,
                zid: zid_from_bytes(&[0xFF; 8]),
                locators: heapless::Vec::new(),
            };
            let len = hello.z_len();
            let mut buf = [0u8; BUF_SIZE];
            hello.z_encode(&mut &mut buf[..]).unwrap();
            let decoded = <Hello as ZDecode>::z_decode(&mut &buf[..len]).unwrap();
            assert_eq!(hello, decoded);
        }
    }

    #[test]
    fn hello_rejects_truncated() {
        let hello = Hello::default();
        let mut buf = [0u8; BUF_SIZE];
        hello.z_encode(&mut &mut buf[..]).unwrap();
        // Try decoding only 2 bytes (not enough for ZID)
        let result = <Hello as ZDecode>::z_decode(&mut &buf[..2]);
        assert!(result.is_err());
    }

    #[test]
    fn hello_rejects_wrong_header() {
        let hello = Hello::default();
        let len = hello.z_len();
        let mut buf = [0u8; BUF_SIZE];
        hello.z_encode(&mut &mut buf[..]).unwrap();
        // Corrupt header ID
        buf[0] = 0xFF;
        let result = <Hello as ZDecode>::z_decode(&mut &buf[..len]);
        assert!(result.is_err());
    }

    // --- ZLocator tests ---

    #[test]
    fn zlocator_encode_decode_roundtrip() {
        let loc = ZLocator {
            protocol: heapless::String::try_from("tcp").unwrap(),
            address: heapless::String::try_from("127.0.0.1:7447").unwrap(),
        };
        let mut buf = [0u8; BUF_SIZE];
        loc.z_body_encode(&mut &mut buf[..]).unwrap();
        let decoded = ZLocator::z_body_decode(&mut &buf[..], ()).unwrap();
        assert_eq!(loc, decoded);
        assert_eq!(loc.as_str(), "tcp/127.0.0.1:7447");
    }

    #[test]
    fn zlocator_as_str() {
        let loc = ZLocator {
            protocol: heapless::String::try_from("tcp").unwrap(),
            address: heapless::String::try_from("[::1]:7447").unwrap(),
        };
        assert_eq!(loc.as_str(), "tcp/[::1]:7447");
    }

    #[test]
    fn zlocator_rejects_empty_protocol() {
        let bytes = &[0x05u8, b'/', b'a', b'b', b'c', b'd'];
        let result = ZLocator::z_body_decode(&mut &bytes[..], ());
        assert!(result.is_err());
    }

    #[test]
    fn zlocator_rejects_no_slash() {
        let bytes = &[0x05u8, b'a', b'b', b'c', b'd', b'e'];
        let result = ZLocator::z_body_decode(&mut &bytes[..], ());
        assert!(result.is_err());
    }

    // --- WhatAmIMatcher tests ---

    #[test]
    fn whatami_matcher_constants() {
        assert_eq!(WhatAmIMatcher::ROUTER.bits(), 0x01);
        assert_eq!(WhatAmIMatcher::PEER.bits(), 0x02);
        assert_eq!(WhatAmIMatcher::CLIENT.bits(), 0x04);
    }

    #[test]
    fn whatami_matcher_matches() {
        let all = WhatAmIMatcher::from_bits(0x07);
        assert!(all.matches(WhatAmI::Router));
        assert!(all.matches(WhatAmI::Peer));
        assert!(all.matches(WhatAmI::Client));

        let router = WhatAmIMatcher::ROUTER;
        assert!(router.matches(WhatAmI::Router));
        assert!(!router.matches(WhatAmI::Peer));
        assert!(!router.matches(WhatAmI::Client));

        let peer = WhatAmIMatcher::PEER;
        assert!(!peer.matches(WhatAmI::Router));
        assert!(peer.matches(WhatAmI::Peer));
        assert!(!peer.matches(WhatAmI::Client));
    }

    #[test]
    fn whatami_matcher_from_bits_masks() {
        let m = WhatAmIMatcher::from_bits(0xFF);
        assert_eq!(m.bits(), 0x07);
    }

    #[test]
    fn whatami_matcher_encode_decode() {
        let m = WhatAmIMatcher::from_bits(0x05);
        let mut buf = [0u8; 4];
        m.z_body_encode(&mut &mut buf[..]).unwrap();
        let decoded = WhatAmIMatcher::z_body_decode(&mut &buf[..], ()).unwrap();
        assert_eq!(m, decoded);
    }
}
