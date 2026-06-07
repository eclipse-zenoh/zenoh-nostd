use zenoh_proto::{ZBodyLen, ZEncode};

use heapless::Vec;

use zenoh_proto::fields::{WhatAmI, WhatAmIMatcher, ZenohIdProto};
use zenoh_proto::msgs::{Hello, MAX_LOCATORS, Scout, ZLocator};

/// Result of a scouting operation: a discovered Zenoh node.
#[derive(Debug, Clone)]
pub struct ScoutResult {
    /// Zenoh ID of the discovered node.
    pub zid: [u8; 16],
    /// Role of the discovered node.
    pub whatami: WhatAmI,
    /// Locators where the node can be reached.
    pub locators: Vec<ZLocator, MAX_LOCATORS>,
}

/// Error type for scouting operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScoutError {
    /// Encoding the scout message failed.
    EncodeFailed,
    /// No transport available for sending scout.
    NoTransport,
    /// Timeout waiting for replies.
    Timeout,
    /// Buffer too small.
    BufferTooSmall,
}

/// A builder for configuring and performing a scouting operation.
///
/// # Example (std-enabled)
/// ```ignore
/// use zenoh_proto::fields::WhatAmIMatcher;
/// let results = ScoutBuilder::new()
///     .what(WhatAmIMatcher::ROUTER | WhatAmIMatcher::PEER)
///     .timeout_ms(1000)
///     .call()
///     .unwrap();
/// ```
pub struct ScoutBuilder {
    what: WhatAmIMatcher,
    timeout_ms: u32,
    zid: Option<[u8; 16]>,
}

impl ScoutBuilder {
    pub fn new() -> Self {
        Self {
            what: WhatAmIMatcher::from_bits(0x07),
            timeout_ms: 1000,
            zid: None,
        }
    }

    /// Set the WhatAmI matcher for scouting.
    pub fn what(mut self, what: WhatAmIMatcher) -> Self {
        self.what = what;
        self
    }

    /// Set the scouting timeout in milliseconds.
    pub fn timeout_ms(mut self, timeout: u32) -> Self {
        self.timeout_ms = timeout;
        self
    }

    /// Set the local Zenoh ID (for self-suppression).
    pub fn zid(mut self, zid: [u8; 16]) -> Self {
        self.zid = Some(zid);
        self
    }

    /// Encode the Scout message into a buffer.
    ///
    /// Returns the number of bytes written.
    pub fn encode_scout(&self, buf: &mut [u8]) -> Result<usize, ScoutError> {
        let zid_proto = self
            .zid
            .map(|z| ZenohIdProto::try_from(&z[..]).unwrap_or_default());
        let scout = Scout {
            version: zenoh_proto::VERSION,
            what: self.what,
            zid: zid_proto,
        };
        let len = scout.z_body_len();
        if buf.len() < len {
            return Err(ScoutError::BufferTooSmall);
        }
        scout
            .z_encode(&mut &mut buf[..])
            .map_err(|_| ScoutError::EncodeFailed)?;
        Ok(len)
    }

    /// Decode a Hello message from a buffer.
    pub fn decode_hello(buf: &[u8]) -> Result<ScoutResult, ScoutError> {
        let hello = <Hello as zenoh_proto::ZDecode>::z_decode(&mut &buf[..])
            .map_err(|_| ScoutError::EncodeFailed)?;
        let mut zid = [0u8; 16];
        let zid_bytes = &hello.zid.as_le_bytes()[..hello.zid.size().min(16)];
        zid[..zid_bytes.len()].copy_from_slice(zid_bytes);
        let locators = hello.locators;
        Ok(ScoutResult {
            zid,
            whatami: hello.whatami,
            locators,
        })
    }
}

impl Default for ScoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUF_SIZE: usize = 256;

    #[test]
    fn scout_builder_encode() {
        let builder = ScoutBuilder::new()
            .what(WhatAmIMatcher::ROUTER)
            .timeout_ms(500);
        let mut buf = [0u8; BUF_SIZE];
        let len = builder.encode_scout(&mut buf).unwrap();
        assert!(len > 0);
        // Decode it back to verify
        let decoded = <Scout as zenoh_proto::ZDecode>::z_decode(&mut &buf[..len]).unwrap();
        assert_eq!(decoded.what, WhatAmIMatcher::ROUTER);
        assert_eq!(decoded.version, zenoh_proto::VERSION);
    }

    #[test]
    fn scout_builder_encode_with_zid() {
        let builder = ScoutBuilder::new()
            .what(WhatAmIMatcher::PEER)
            .zid([0x01; 16]);
        let mut buf = [0u8; BUF_SIZE];
        let len = builder.encode_scout(&mut buf).unwrap();
        let decoded = <Scout as zenoh_proto::ZDecode>::z_decode(&mut &buf[..len]).unwrap();
        assert!(decoded.zid.is_some());
    }

    #[test]
    fn scout_builder_buffer_too_small() {
        let builder = ScoutBuilder::new();
        let mut buf = [0u8; 2];
        let result = builder.encode_scout(&mut buf);
        assert_eq!(result, Err(ScoutError::BufferTooSmall));
    }

    #[test]
    fn scout_builder_decode_hello() {
        let mut locators = Vec::new();
        locators
            .push(ZLocator {
                protocol: heapless::String::try_from("tcp").unwrap(),
                address: heapless::String::try_from("10.0.0.1:7447").unwrap(),
            })
            .unwrap();
        let hello = Hello {
            version: zenoh_proto::VERSION,
            whatami: WhatAmI::Router,
            zid: zenoh_proto::fields::ZenohIdProto::default(),
            locators,
        };
        let mut buf = [0u8; BUF_SIZE];
        let len = <Hello as zenoh_proto::ZBodyLen>::z_body_len(&hello);
        <Hello as zenoh_proto::ZEncode>::z_encode(&hello, &mut &mut buf[..]).unwrap();
        let result = ScoutBuilder::decode_hello(&buf[..len]).unwrap();
        assert_eq!(result.whatami, WhatAmI::Router);
        assert_eq!(result.locators.len(), 1);
    }

    #[test]
    fn scout_builder_decode_invalid_hello() {
        let buf = [0xFFu8; 16];
        let result = ScoutBuilder::decode_hello(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn scout_builder_default() {
        let builder = ScoutBuilder::new();
        assert_eq!(builder.what, WhatAmIMatcher::from_bits(0x07));
        assert_eq!(builder.timeout_ms, 1000);
    }
}
