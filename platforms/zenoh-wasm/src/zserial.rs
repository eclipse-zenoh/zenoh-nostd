//
// Copyright (c) 2022 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//   Patrick José Pereira, <patrickelectric@gmail.com>
//

//! z-serial 0.3.1 wire format, the one zenohd's serial link speaks.
//! No allocation, so it can move to no_std as is.
//!
//! Port of <https://github.com/ZettaScaleLabs/z-serial/blob/9c79c8f634d79654dadcd397f77c99532a71f30a/src/lib.rs>.

// ZSerial Frame Format
//
// Using COBS
//
// +-+-+----+------------+--------+-+
// |O|H|XXXX|ZZZZ....ZZZZ|CCCCCCCC|0|
// +-+-+----+------------+--------+-+
// |O| |Len |   Data     |  CRC32 |C|
// +-+-+-2--+----N-------+---4----+-+
//
// Header: 1 byte
// +---------------+
// |7|6|5|4|3|2|1|0|
// +---------------+
// |x|x|x|x|x|R|A|I|
// +---------------+
//
// Flags:
// I - Init
// A - Ack
// R - Reset
//
// Max Frame Size: 1510
// Max MTU: 1500
// Max On-the-wire length: 1517 (cobs::max_encoding_length(MFS) + End of packet (EOP))

/// Largest payload in one frame.
pub const MTU: usize = 1500;
/// Largest encoded frame, sentinel included.
pub const MAX_FRAME: usize = 1517;

// Initialization message exchange from cold start
//
// 1) Connect side sends a message with empty payload and I flag set.
// 2) Listen side sends a message with empty payload and I+A flags set.
// 3) Data can be exchanged from this point on.
//
//  ┌─────────┐                ┌──────────┐
//  │ Listen  │                │ Connect  │
//  └────┬────┘                └────┬─────┘
//       │                          │
//       │       Init               │
//       │◄─────────────────────────┤
//       │                          │
//       │        Init + Ack        │
//       ├─────────────────────────►│
//       │       Data               │
//       │◄─────────────────────────┤
//       ├─────────────────────────►│
//       │                          │
//
// If the connect side restarts:
//
// 1) Connect side sends a message with empty payload and I flag set.
// 2) Listen side sends a message with empty payload and R flag set.
// 3) Connect side waits and goes back to cold start case.
//
//   ┌─────────┐                ┌──────────┐
//   │ Listen  │                │ Connect  │
//   └────┬────┘                └────┬─────┘
//        │        Init              │
//        │◄─────────────────────────┤
//        │       Reset              │
//        ├─────────────────────────►│
//        │                          │
//        │                          │
//        │                          │
//        │        Init              │
//        │◄─────────────────────────┤
//        │       Init + Ack         │
//        ├─────────────────────────►│
//        │         Data             │
//        │◄─────────────────────────┤
//        ├─────────────────────────►│
//        │                          │
//        │                          │

pub const INIT: u8 = 0x01;
pub const ACK: u8 = 0x02;
pub const RESET: u8 = 0x04;

const OVERHEAD: usize = 7;

/// Reflected CRC-32 loop fed with the non reflected 0x04C11DB7 polynomial, as z-serial
/// does. It is not the usual CRC-32, so crc32fast and friends won't match it.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0x04C1_1DB7 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

/// Encodes one frame into `out`, returns its length with the sentinel, or `None` if
/// `payload` is bigger than [`MTU`].
pub fn encode(header: u8, payload: &[u8], out: &mut [u8; MAX_FRAME]) -> Option<usize> {
    if payload.len() > MTU {
        return None;
    }
    let len = payload.len();
    let mut raw = [0u8; MTU + OVERHEAD];
    // Copy into serialization buffer
    raw[0] = header;
    raw[1..3].copy_from_slice(&(len as u16).to_le_bytes());
    raw[3..3 + len].copy_from_slice(payload);
    raw[3 + len..OVERHEAD + len].copy_from_slice(&crc32(payload).to_le_bytes());

    // COBS encode
    let written = cobs::encode_with_sentinel(&raw[..OVERHEAD + len], out, 0);

    // Add sentinel byte, marks the end of a message
    out[written] = 0;
    Some(written + 1)
}

/// Decodes one frame in place, with or without its sentinel. Returns the header and the
/// payload, or `None` if the frame is broken.
pub fn decode(frame: &mut [u8]) -> Option<(u8, &[u8])> {
    let len = cobs::decode_in_place_with_sentinel(frame, 0).ok()?;
    let raw = &frame[..len];

    // Check if message has the minimum size
    if len < OVERHEAD {
        return None;
    }

    // Decoding message kind
    let header = raw[0];
    // Decoding message size
    let size = u16::from_le_bytes([raw[1], raw[2]]) as usize;

    // Check if the frame size is correct
    if size + OVERHEAD != len {
        return None;
    }

    // Getting the data
    let (payload, crc) = raw[3..].split_at(size);

    // Check CRC
    (crc32(payload).to_le_bytes() == crc).then_some((header, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ser() {
        let mut out = [0u8; MAX_FRAME];

        let data = [0x00, 0x11, 0x00];

        // COBS encoded 0x00 | 0x03 0x00 | 0x00 0x11 0x00 | 0x73 0xEC 0x75 0xF9 |
        //              Hdr |   Len     |   Data         |      CRC32          |
        let serialized = [
            0x01, 0x02, 0x03, 0x01, 0x02, 0x11, 0x05, 0x73, 0xEC, 0x75, 0xF9, 0x00,
        ];

        // Checks serialization
        let written = encode(0, &data, &mut out).unwrap();
        assert_eq!(&out[..written], &serialized);

        // 2nd Check

        let data = [0x11, 0x22, 0x00, 0x33];

        // COBS encoded 0x00 | 0x04 0x00 | 0x11 0x22 0x00 0x33 | 0x8D 0x03 0x6D 0xFB |
        //              Hdr |   Len     |   Data              |      CRC32          |
        let serialized = [
            0x01, 0x02, 0x04, 0x03, 0x11, 0x22, 0x06, 0x33, 0x8D, 0x03, 0x6D, 0xFB, 0x00,
        ];

        let written = encode(0, &data, &mut out).unwrap();
        assert_eq!(&out[..written], &serialized);
    }

    #[test]
    fn test_de() {
        let data = [0x00, 0x11, 0x00];
        // COBS encoded 0x00 | 0x03 0x00 | 0x00 0x11 0x00 | 0x73 0xEC 0x75 0xF9 |
        //              Hdr |   Len     |   Data         |      CRC32          |
        let mut serialized = [
            0x01, 0x02, 0x03, 0x01, 0x02, 0x11, 0x05, 0x73, 0xEC, 0x75, 0xF9, 0x00,
        ];

        let (header, payload) = decode(&mut serialized).unwrap();
        assert_eq!(header & INIT, 0);
        assert_eq!(payload, data);

        // 2nd Check

        let data = [0x11, 0x22, 0x00, 0x33];

        // COBS encoded 0x00 | 0x04 0x00 | 0x11 0x22 0x00 0x33 | 0x8D 0x03 0x6D 0xFB |
        //              Hdr |   Len     |   Data              |      CRC32          |
        let mut serialized = [
            0x01, 0x02, 0x04, 0x03, 0x11, 0x22, 0x06, 0x33, 0x8D, 0x03, 0x6D, 0xFB, 0x00,
        ];

        let (header, payload) = decode(&mut serialized).unwrap();
        assert_eq!(payload, data);
        assert_eq!(header & INIT, 0);
    }

    #[test]
    fn test_serde_one_byte() {
        let mut out = [0u8; MAX_FRAME];

        let data = [0x00];
        let written = encode(0, &data, &mut out).unwrap();

        let (header, payload) = decode(&mut out[..written]).unwrap();
        assert_eq!(header & INIT, 0);
        assert_eq!(payload, data);
    }

    #[test]
    fn test_serde_empty() {
        let mut out = [0u8; MAX_FRAME];

        let written = encode(0, &[], &mut out).unwrap();

        let (header, payload) = decode(&mut out[..written]).unwrap();
        assert_eq!(header & INIT, 0);
        assert_eq!(payload, []);
    }

    #[test]
    fn round_trip() {
        let mut out = [0u8; MAX_FRAME];
        for len in [0, 1, 253, 254, 255, 508, MTU] {
            let payload: Vec<u8> = (0..len).map(|i| (i % 7) as u8).collect();
            let written = encode(INIT, &payload, &mut out).unwrap();
            assert_eq!(out[written - 1], 0);
            assert!(!out[..written - 1].contains(&0));
            assert_eq!(decode(&mut out[..written - 1]), Some((INIT, &payload[..])));
        }
    }

    #[test]
    fn rejects_bad_crc() {
        let mut out = [0u8; MAX_FRAME];
        let written = encode(0, b"zenoh", &mut out).unwrap();
        out[5] ^= 1;
        assert_eq!(decode(&mut out[..written - 1]), None);
    }

    #[test]
    fn rejects_partial_frame() {
        // Tail of an I left in the port, then the same tail glued to a whole I.
        assert_eq!(decode(&mut [0x01, 0x01, 0x01, 0x01, 0x00]), None);
        let mut glued = [
            0x7f, 0x41, 0x42, 0x02, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00,
        ];
        assert_eq!(decode(&mut glued), None);
    }

    #[test]
    fn init_frame_matches_z_serial() {
        // Bytes z-serial 0.3.1 writes for an empty frame with the I flag.
        let mut out = [0u8; MAX_FRAME];
        let written = encode(INIT, &[], &mut out).unwrap();
        assert_eq!(
            &out[..written],
            &[0x02, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00]
        );
    }

    #[test]
    fn reset_frame_matches_zenoh_pico() {
        // The CRC of an empty payload is 0, the same as zenoh-pico sends.
        let mut out = [0u8; MAX_FRAME];
        let written = encode(RESET, &[], &mut out).unwrap();
        assert_eq!(
            &out[..written],
            &[0x02, 0x04, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00]
        );
    }
}
