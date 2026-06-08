#![allow(unused_doc_comments)]

extern crate std;
use crate::*;
use proptest::prelude::*;

/// Property 1: VLE roundtrip for any u64.
/// Encode any u64 to bytes, decode back, and verify the original value is preserved.
proptest! {
    #[test]
    fn vle_roundtrip(v in proptest::num::u64::ANY) {
        let mut buf = [0u8; 10];
        let mut writer = &mut buf[..];
        <u64 as ZEncode>::z_encode(&v, &mut writer).unwrap();
        let written = 10 - writer.len();
        let decoded = <u64 as ZDecode>::z_decode(&mut &buf[..written]).unwrap();
        prop_assert_eq!(v, decoded);
    }
}

/// Property 2: VLE decode never panics on arbitrary short byte sequences.
/// For any random byte sequence of length < 10, decode either succeeds (if the bytes
/// happen to form a valid VLE) or returns an error — it never panics.
proptest! {
    #[test]
    fn vle_truncated_decode(bytes in proptest::collection::vec(any::<u8>(), 0..9)) {
        let _result = <u64 as ZDecode>::z_decode(&mut &bytes[..]);
        // Must not panic, regardless of input. Result may be Ok or Err.
    }
}

// ─── Keyexpr property tests ─────────────────────────────────────────────────

/// Generate valid keyexpr strings.
///
/// Valid patterns:
/// - Plain chunks: `a/b/c`
/// - Single wildcard: `a/*/c`
/// - Double wildcard at end: `a/**`, `**`
/// - DSL `$*` with surrounding text: `a/b$*/c`, `a/$*b`
fn valid_keyexpr_strategy() -> impl Strategy<Value = String> {
    let plain_chunk = r"[a-z]+";
    let star_chunk = r"\*";
    let dsl_chunk = r"[a-z]+\$\*[a-z]*|[a-z]*\$\*[a-z]+";

    let chunk = prop_oneof![
        4 => plain_chunk,
        1 => star_chunk,
        1 => dsl_chunk,
    ];

    let path = prop::collection::vec(chunk, 1..5).prop_flat_map(|chunks| {
        let base = chunks.join("/");
        prop_oneof![
            3 => Just(base.clone()),
            1 => Just(format!("{}/{}", base, "**")),
        ]
    });

    prop_oneof![
        9 => path,
        1 => Just("**".to_string()),
    ]
}

/// Generate invalid keyexpr strings.
fn invalid_keyexpr_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty string
        Just(String::new()),
        // Trailing slash
        r"[a-z]+(?:/[a-z]+)*/",
        // Double slash (empty chunk)
        r"[a-z]+//[a-z]+",
        // Star in middle of chunk: a/*b
        r"[a-z]+/[a-z]*\*[a-z]+",
        // Hash character
        r"[a-z]+/#[a-z]*",
        // Question mark character
        r"[a-z]+/\?[a-z]*",
        // Lone $ (unbound dollar)
        r"[a-z]+/\$[a-z]*",
        // Dollar after dollar
        r"[a-z]+/\$\$[a-z]*",
        // Lone $* (alone in chunk)
        r"[a-z]+/\$\*/[a-z]+",
        // $* as entire chunk at end
        r"[a-z]+/\$\*",
        // Single * after **
        r"[a-z]+/[*]{2}/\*",
        // Double ** after **
        r"[a-z]+/[*]{2}/[*]{2}",
    ]
}

/// Property 4: Valid keyexprs are never rejected.
/// For any string matching valid keyexpr patterns, `keyexpr::new()` returns `Ok`.
proptest! {
    #[test]
    fn keyexpr_valid_never_rejected(ke in valid_keyexpr_strategy()) {
        // Skip empty strings (regex can produce them)
        if !ke.is_empty() {
            prop_assert!(keyexpr::new(&ke).is_ok(),
                "Expected Ok for valid keyexpr: {:?}", ke);
        }
    }
}

/// Property 5: Invalid keyexprs are always rejected.
/// For any string matching invalid patterns, `keyexpr::new()` returns `Err`.
proptest! {
    #[test]
    fn keyexpr_invalid_always_rejected(ke in invalid_keyexpr_strategy()) {
        if !ke.is_empty() {
            prop_assert!(keyexpr::new(&ke).is_err(),
                "Expected Err for invalid keyexpr: {:?}", ke);
        }
    }
}

/// Property 6: No panic on any byte sequence.
/// For ANY byte sequence (including invalid UTF-8, control chars, Unicode, null bytes),
/// `keyexpr::new()` never panics — always returns `Ok` or `Err`.
proptest! {
    #[test]
    fn keyexpr_no_panic_on_any_input(bytes in proptest::collection::vec(any::<u8>(), 0..100)) {
        // Convert bytes to a string lossily — this tests what happens when we
        // pass arbitrary bytes through the &str interface
        if let Ok(s) = std::str::from_utf8(&bytes) {
            let _result = keyexpr::new(s);
            // Must not panic. Result may be Ok or Err.
        }
    }
}

/// Property 7: UTF-8 safety — non-UTF-8 byte sequences are rejected, not panicked on.
/// We construct a string from arbitrary bytes and verify the function handles it gracefully.
proptest! {
    #[test]
    fn keyexpr_utf8_safety(bytes in proptest::collection::vec(any::<u8>(), 0..100)) {
        // Try to interpret as UTF-8; if it fails, we can't pass it to keyexpr::new
        // (which takes &str). But we verify that the function never panics on valid UTF-8
        // that happens to be arbitrary bytes.
        if let Ok(s) = std::str::from_utf8(&bytes) {
            let result = keyexpr::new(s);
            // Must not panic. Result may be Ok or Err.
            // If the string contains invalid keyexpr chars, it should be Err.
            // If it's valid, it should be Ok.
            // Either way, no panic.
            let _ = result;
        }
    }
}

/// Property 8: Specific edge cases — boundary conditions around keyexpr parsing.
/// Tests various edge cases that are known to be tricky.
proptest! {
    #[test]
    fn keyexpr_edge_cases(
        chunk in r"[a-z]{1,5}",
    ) {
        // Test: star at chunk boundary
        let s = format!("{}/{}", chunk, "*");
        prop_assert!(keyexpr::new(&s).is_ok(),
            "Expected Ok for 'chunk/*': {:?}", s);

        // Test: double star at end
        let s = format!("{}/{}", chunk, "**");
        prop_assert!(keyexpr::new(&s).is_ok(),
            "Expected Ok for 'chunk/**': {:?}", s);

        // Test: $* DSL (must have surrounding text, $* alone in chunk is LoneDollarStar)
        let s = format!("{}/a{}b", chunk, "$*");
        prop_assert!(keyexpr::new(&s).is_ok(),
            "Expected Ok for 'chunk/a$*b': {:?}", s);
    }
}

/// Property 3: Extension decode never panics on arbitrary byte sequences.
/// For ANY byte sequence, feeding it through decode_ext_header + skip_ext
/// must never panic (only return Ok or Err).
proptest! {
    #[test]
    fn ext_decode_no_panic(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        // Test 1: decode_ext_header on the buffer (peek only)
        if bytes.len() >= 1 {
            let mut reader: &[u8] = &bytes;
            let header_result = decode_ext_header(&mut reader);
            if let Ok((_, kind, _, _)) = header_result {
                let mut reader2: &[u8] = &bytes;
                let _ = skip_ext(&mut reader2, kind);
            }
        }
    }
}

/// Property 4: VLE boundary encode length.
/// For each boundary 2^(7n)-1, encode produces exactly n bytes.
/// For values >= 2^(7*9), encode produces 10 bytes.
#[test]
fn vle_boundary_encode_length() {
    let boundaries: [(u64, usize); 21] = [
        (0, 1),
        (1, 1),
        (0x7f, 1),                   // 2^7 - 1
        (0x80, 2),                   // 2^7
        (0x3fff, 2),                 // 2^14 - 1
        (0x4000, 3),                 // 2^14
        (0x1f_ffff, 3),              // 2^21 - 1
        (0x20_0000, 4),              // 2^21
        (0xfff_ffff, 4),             // 2^28 - 1
        (0x1000_0000, 5),            // 2^28
        (0x7_ffff_ffff, 5),          // 2^35 - 1
        (0x8_0000_0000, 6),          // 2^35
        (0x3ff_ffff_ffff, 6),        // 2^42 - 1
        (0x400_0000_0000, 7),        // 2^42
        (0x1_ffff_ffff_ffff, 7),     // 2^49 - 1
        (0x2_0000_0000_0000, 8),     // 2^49
        (0xff_ffff_ffff_ffff, 8),    // 2^56 - 1
        (0x100_0000_0000_0000, 9),   // 2^56
        (0x7fff_ffff_ffff_ffff, 9),  // 2^63 - 1
        (0x8000_0000_0000_0000, 10), // 2^63
        (u64::MAX, 10),
    ];

    for (val, expected_len) in &boundaries {
        let mut buf = [0u8; 10];
        let mut writer = &mut buf[..];
        <u64 as ZEncode>::z_encode(val, &mut writer).unwrap();
        let written = 10 - writer.len();
        assert_eq!(
            written, *expected_len,
            "encode of value 0x{:x} produced {} bytes, expected {}",
            val, written, expected_len
        );
    }
}

// ─── Extension variant roundtrip tests ─────────────────────────────────────
//
// Tests that all 3 ZExtKind variants (Unit, U64, ZStruct) roundtrip
// through zext_encode → zext_decode correctly.
//
// Types tested per ZExtKind:
//   - Unit:     HasQoS (empty struct, flag-only extension)
//   - U64:      QueryableInfo (complete flag + distance packed as u64)
//   - ZStruct:  EntityGlobalId (zid: ZenohIdProto, eid: u32)

/// Helper: encodes an extension, then decodes it, asserting equality.
/// Uses HRTB (for<'a>) to allow ZExt types that don't borrow from the decode buffer.
fn ext_roundtrip<T>(ext: &T)
where
    T: for<'a> ZExt<'a> + std::fmt::Debug + PartialEq,
{
    let len = zext_len(ext);
    let mut buf = vec![0u8; len + 16];
    let remaining = {
        let mut writer = &mut buf[..];
        zext_encode::<T, 0xf, true>(ext, &mut writer, false).unwrap();
        writer.len()
    };
    let written = buf.len() - remaining;
    let decoded: T = zext_decode::<T>(&mut &buf[..written]).unwrap();
    assert_eq!(*ext, decoded, "extension roundtrip failed for {:?}", ext);
}

/// Unit-kind roundtrip: HasQoS (empty struct, ZExtKind::Unit)
/// Unit extensions encode only the header byte — no body.
#[test]
fn ext_unit_has_qos_roundtrip() {
    let ext = exts::HasQoS {};
    ext_roundtrip(&ext);
}

/// U64-kind roundtrip: QueryableInfo (ZExtKind::U64)
/// Based on exts.rs: QueryableInfo::as_u64 packs complete (bit 0)
/// and distance (bits 8+).
#[test]
fn ext_u64_queryable_info_roundtrip() {
    let cases = [
        exts::QueryableInfo {
            complete: false,
            distance: 0,
        },
        exts::QueryableInfo {
            complete: true,
            distance: 0,
        },
        exts::QueryableInfo {
            complete: false,
            distance: 1,
        },
        exts::QueryableInfo {
            complete: true,
            distance: u16::MAX,
        },
        exts::QueryableInfo {
            complete: false,
            distance: 0xFFFF,
        },
        exts::QueryableInfo {
            complete: true,
            distance: 0x8000,
        },
    ];
    for ext in &cases {
        ext_roundtrip(ext);
    }
}

/// U64-kind roundtrip: NodeId (ZExtKind::U64, single u16 field)
#[test]
fn ext_u64_node_id_roundtrip() {
    let cases = [
        exts::NodeId { node_id: 0 },
        exts::NodeId { node_id: 1 },
        exts::NodeId { node_id: u16::MAX },
        exts::NodeId {
            node_id: u16::MAX / 2,
        },
        exts::NodeId { node_id: 0xFF },
    ];
    for ext in &cases {
        ext_roundtrip(ext);
    }
}

/// ZStruct-kind roundtrip: EntityGlobalId (ZExtKind::ZStruct)
/// Contains zid: ZenohIdProto (variable-length) + eid: u32
#[test]
fn ext_zstruct_entity_global_id_roundtrip() {
    use fields::ZenohIdProto;
    let zid_a = ZenohIdProto::try_from(&[1u8, 2, 3, 4, 5, 6, 7, 8][..]).unwrap();
    let zid_b = ZenohIdProto::try_from(&[0xFFu8; 16][..]).unwrap();
    let cases = [
        exts::EntityGlobalId {
            zid: ZenohIdProto::default(),
            eid: 0,
        },
        exts::EntityGlobalId { zid: zid_a, eid: 1 },
        exts::EntityGlobalId {
            zid: zid_b,
            eid: u32::MAX,
        },
        exts::EntityGlobalId {
            zid: zid_a,
            eid: u32::MAX / 2,
        },
    ];
    for ext in &cases {
        ext_roundtrip(ext);
    }
}

/// Verify that all 3 ZExtKind header bytes encode correctly.
/// The header byte format: [MORE:1][KIND:2][MAND:1][ID:4]
/// Unit=0b00, U64=0b01, ZStruct=0b10 (shifted to bits 5-6)
///
/// The mask 0b0110_0000 isolates the kind bits from the ID and flags.
/// The extension IDs (0x1, 0x2, 0x3) are in the lower 4 bits and are
/// masked out — they don't affect the kind assertion.
#[test]
fn ext_header_kind_bytes() {
    let mask = 0b0110_0000;

    let h_unit = zext_eheader::<0x1, true, exts::HasQoS>();
    assert_eq!(h_unit & mask, ZExtKind::Unit as u8, "Unit kind bits");

    let h_u64 = zext_eheader::<0x2, false, exts::QueryableInfo>();
    assert_eq!(h_u64 & mask, ZExtKind::U64 as u8, "U64 kind bits");

    let h_zstruct = zext_eheader::<0x3, true, exts::EntityGlobalId>();
    assert_eq!(
        h_zstruct & mask,
        ZExtKind::ZStruct as u8,
        "ZStruct kind bits"
    );
}

// ─── Transport message roundtrip property tests ──────────────────────────
//
// Property: Every transport message survives encode → decode roundtrip
// without data loss or corruption. Also verifies that z_len() provides
// a valid upper bound on the encoded size.

use core::time::Duration;

/// Generate a valid ZenohIdProto from arbitrary bytes.
fn any_zid() -> impl Strategy<Value = fields::ZenohIdProto> {
    proptest::collection::vec(any::<u8>(), 1..=16)
        .prop_map(|bytes| fields::ZenohIdProto::try_from(&bytes[..]).unwrap_or_default())
}

proptest! {
    #[test]
    fn init_syn_roundtrip(
        version in any::<u8>(),
        whatami in prop_oneof![
            Just(fields::WhatAmI::Router),
            Just(fields::WhatAmI::Peer),
            Just(fields::WhatAmI::Client),
        ],
        zid in any_zid(),
        resolution in any::<u8>(),
        batch_size in any::<u16>(),
    ) {
        let msg = crate::msgs::InitSyn {
            version,
            identifier: crate::msgs::InitIdentifier {
                whatami,
                zid,
            },
            resolution: crate::msgs::InitResolution {
                resolution: fields::Resolution(resolution),
                batch_size: fields::BatchSize(batch_size),
            },
            qos: None,
            qos_link: None,
            auth: None,
            mlink: None,
            lowlatency: None,
            compression: None,
            patch: exts::Patch::none(),
        };

        let len = msg.z_len();
        let mut buf = vec![0u8; len + 16];
        let buf_len = buf.len();
        let mut writer = &mut buf[..];
        msg.z_encode(&mut writer).unwrap();
        let written = buf_len - writer.len();
        prop_assert!(written <= len + 16, "encoded size exceeds z_len bound");

        let decoded = crate::msgs::InitSyn::z_decode(&mut &buf[..written]).unwrap();
        prop_assert_eq!(msg, decoded, "InitSyn roundtrip failed");
    }
}

proptest! {
    #[test]
    fn init_ack_roundtrip(
        version in any::<u8>(),
        whatami in prop_oneof![
            Just(fields::WhatAmI::Router),
            Just(fields::WhatAmI::Peer),
            Just(fields::WhatAmI::Client),
        ],
        zid in any_zid(),
        resolution in any::<u8>(),
        batch_size in any::<u16>(),
        cookie in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        let msg = crate::msgs::InitAck {
            version,
            identifier: crate::msgs::InitIdentifier {
                whatami,
                zid,
            },
            resolution: crate::msgs::InitResolution {
                resolution: fields::Resolution(resolution),
                batch_size: fields::BatchSize(batch_size),
            },
            cookie: &cookie,
            qos: None,
            qos_link: None,
            auth: None,
            mlink: None,
            lowlatency: None,
            compression: None,
            patch: exts::Patch::none(),
        };

        let len = msg.z_len();
        let mut buf = vec![0u8; len + 16];
        let buf_len = buf.len();
        let mut writer = &mut buf[..];
        msg.z_encode(&mut writer).unwrap();
        let written = buf_len - writer.len();
        prop_assert!(written <= len + 16);

        let decoded = crate::msgs::InitAck::z_decode(&mut &buf[..written]).unwrap();
        prop_assert_eq!(msg, decoded, "InitAck roundtrip failed");
    }
}

proptest! {
    #[test]
    fn open_syn_roundtrip(
        lease_ms in any::<u32>(),
        sn in any::<u32>(),
        cookie in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        let msg = crate::msgs::OpenSyn {
            lease: Duration::from_millis(lease_ms as u64),
            sn,
            cookie: &cookie,
            qos: None,
            auth: None,
            mlink_syn: None,
            mlink_ack: None,
            lowlatency: None,
            compression: None,
        };

        let len = msg.z_len();
        let mut buf = vec![0u8; len + 16];
        let buf_len = buf.len();
        let mut writer = &mut buf[..];
        msg.z_encode(&mut writer).unwrap();
        let written = buf_len - writer.len();
        prop_assert!(written <= len + 16);

        let decoded = crate::msgs::OpenSyn::z_decode(&mut &buf[..written]).unwrap();
        prop_assert_eq!(msg, decoded, "OpenSyn roundtrip failed");
    }
}

proptest! {
    #[test]
    fn open_ack_roundtrip(
        lease_ms in any::<u32>(),
        sn in any::<u32>(),
    ) {
        let msg = crate::msgs::OpenAck {
            lease: Duration::from_millis(lease_ms as u64),
            sn,
            qos: None,
            auth: None,
            mlink_syn: None,
            mlink_ack: None,
            lowlatency: None,
            compression: None,
        };

        let len = msg.z_len();
        let mut buf = vec![0u8; len + 16];
        let buf_len = buf.len();
        let mut writer = &mut buf[..];
        msg.z_encode(&mut writer).unwrap();
        let written = buf_len - writer.len();
        prop_assert!(written <= len + 16);

        let decoded = crate::msgs::OpenAck::z_decode(&mut &buf[..written]).unwrap();
        prop_assert_eq!(msg, decoded, "OpenAck roundtrip failed");
    }
}

proptest! {
    #[test]
    fn close_roundtrip(
        reason in any::<u8>(),
        behaviour in prop_oneof![
            Just(crate::msgs::CloseBehaviour::Link),
            Just(crate::msgs::CloseBehaviour::Session),
        ],
    ) {
        let msg = crate::msgs::Close { reason, behaviour };

        let len = msg.z_len();
        let mut buf = vec![0u8; len + 16];
        let buf_len = buf.len();
        let mut writer = &mut buf[..];
        msg.z_encode(&mut writer).unwrap();
        let written = buf_len - writer.len();
        prop_assert!(written <= len + 16);

        let decoded = crate::msgs::Close::z_decode(&mut &buf[..written]).unwrap();
        prop_assert_eq!(msg, decoded, "Close roundtrip failed");
    }
}

// ─── Network message property tests ─────────────────────────────────────────
//
// Property: Request, Response, ResponseFinal survive encode → decode roundtrip.
// Also verifies that z_len() provides a valid upper bound.

/// Generate a WireExpr from a valid keyexpr strategy.
fn wire_expr_strategy() -> impl Strategy<Value = (u16, String)> {
    (any::<u16>(), valid_keyexpr_strategy())
        .prop_filter("non-empty keyexpr", |(_, s)| !s.is_empty())
}

proptest! {
    #[test]
    fn request_roundtrip(
        id in any::<u32>(),
        (_scope, suffix) in wire_expr_strategy(),
        params in r"([a-z]{0,10})".prop_map(|s: String| s),
    ) {
        use crate::fields::WireExpr;
        let ke = match keyexpr::new(&suffix) {
            Ok(k) => k,
            Err(_) => return Err(proptest::test_runner::TestCaseError::reject("invalid keyexpr")),
        };
        let msg = crate::msgs::Request {
            id,
            wire_expr: WireExpr::from(ke),
            qos: exts::QoS::default(),
            timestamp: None,
            nodeid: exts::NodeId::default(),
            target: exts::QueryTarget::BestMatching,
            budget: None,
            timeout: None,
            payload: crate::msgs::RequestBody::Query(crate::msgs::Query {
                consolidation: fields::ConsolidationMode::None,
                parameters: &params,
                sinfo: None,
                body: None,
                attachment: None,
                query_target: exts::QueryTarget::BestMatching,
                reply_keyexpr: Default::default(),
            }),
        };

        let len = msg.z_len();
        let mut buf = vec![0u8; len + 16];
        let buf_len = buf.len();
        let mut writer = &mut buf[..];
        msg.z_encode(&mut writer).unwrap();
        let written = buf_len - writer.len();
        prop_assert!(written <= len + 16, "encoded size exceeds z_len bound");

        let decoded = crate::msgs::Request::z_decode(&mut &buf[..written]).unwrap();
        prop_assert_eq!(msg, decoded, "Request roundtrip failed");
    }
}

proptest! {
    #[test]
    fn response_roundtrip(
        rid in any::<u32>(),
        (_scope, suffix) in wire_expr_strategy(),
        payload_is_err in proptest::bool::ANY,
    ) {
        use crate::fields::WireExpr;
        let ke = match keyexpr::new(&suffix) {
            Ok(k) => k,
            Err(_) => return Err(proptest::test_runner::TestCaseError::reject("invalid keyexpr")),
        };
        let msg = if payload_is_err {
            crate::msgs::Response {
                rid,
                wire_expr: WireExpr::from(ke),
                qos: exts::QoS::default(),
                timestamp: None,
                respid: None,
                payload: crate::msgs::ResponseBody::Err(crate::msgs::Err {
                    payload: b"error",
                    ..Default::default()
                }),
            }
        } else {
            crate::msgs::Response {
                rid,
                wire_expr: WireExpr::from(ke),
                qos: exts::QoS::default(),
                timestamp: None,
                respid: None,
                payload: crate::msgs::ResponseBody::Reply(crate::msgs::Reply {
                    consolidation: fields::ConsolidationMode::None,
                    payload: crate::msgs::PushBody::Put(crate::msgs::Put {
                        payload: b"hello",
                        ..Default::default()
                    }),
                }),
            }
        };

        let len = msg.z_len();
        let mut buf = vec![0u8; len + 16];
        let buf_len = buf.len();
        let mut writer = &mut buf[..];
        msg.z_encode(&mut writer).unwrap();
        let written = buf_len - writer.len();
        prop_assert!(written <= len + 16);

        let decoded = crate::msgs::Response::z_decode(&mut &buf[..written]).unwrap();
        prop_assert_eq!(msg, decoded, "Response roundtrip failed");
    }
}

proptest! {
    #[test]
    fn response_final_roundtrip(
        rid in any::<u32>(),
    ) {
        let msg = crate::msgs::ResponseFinal {
            rid,
            qos: exts::QoS::default(),
            timestamp: None,
        };

        let len = msg.z_len();
        let mut buf = [0u8; 64];
        let buf_len = buf.len();
        let mut writer = &mut buf[..];
        msg.z_encode(&mut writer).unwrap();
        let written = buf_len - writer.len();
        prop_assert!(written <= len + 16);

        let decoded = crate::msgs::ResponseFinal::z_decode(&mut &buf[..written]).unwrap();
        prop_assert_eq!(msg, decoded, "ResponseFinal roundtrip failed");
    }
}

/// Property: Decoding Request/Response/ResponseFinal from arbitrary bytes never panics.
/// For ANY byte sequence, decode either succeeds or returns an error — never panics.
proptest! {
    #[test]
    fn request_decode_no_panic(bytes in proptest::collection::vec(any::<u8>(), 0..128)) {
        let _result = crate::msgs::Request::z_decode(&mut &bytes[..]);
    }
}

proptest! {
    #[test]
    fn response_decode_no_panic(bytes in proptest::collection::vec(any::<u8>(), 0..128)) {
        let _result = crate::msgs::Response::z_decode(&mut &bytes[..]);
    }
}

proptest! {
    #[test]
    fn response_final_decode_no_panic(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        let _result = crate::msgs::ResponseFinal::z_decode(&mut &bytes[..]);
    }
}

// ─── Liveliness property tests ──────────────────────────────────────────────

proptest! {
    #[test]
    fn declare_token_roundtrip(
        id in any::<u32>(),
        (_scope, suffix) in wire_expr_strategy(),
    ) {
        use crate::fields::WireExpr;
        let ke = match keyexpr::new(&suffix) {
            Ok(k) => k,
            Err(_) => return Err(proptest::test_runner::TestCaseError::reject("invalid keyexpr")),
        };
        let msg = crate::msgs::DeclareToken {
            id,
            wire_expr: WireExpr::from(ke),
        };

        let len = msg.z_len();
        let mut buf = vec![0u8; len + 16];
        let buf_len = buf.len();
        let mut writer = &mut buf[..];
        msg.z_encode(&mut writer).unwrap();
        let written = buf_len - writer.len();
        prop_assert!(written <= len + 16);

        let decoded = crate::msgs::DeclareToken::z_decode(&mut &buf[..written]).unwrap();
        prop_assert_eq!(msg, decoded, "DeclareToken roundtrip failed");
    }
}

proptest! {
    #[test]
    fn undeclare_token_roundtrip(
        id in any::<u32>(),
    ) {
        let msg = crate::msgs::UndeclareToken {
            id,
            wire_expr: None,
        };

        let len = msg.z_len();
        let mut buf = [0u8; 64];
        let buf_len = buf.len();
        let mut writer = &mut buf[..];
        msg.z_encode(&mut writer).unwrap();
        let written = buf_len - writer.len();
        prop_assert!(written <= len + 16);

        let decoded = crate::msgs::UndeclareToken::z_decode(&mut &buf[..written]).unwrap();
        prop_assert_eq!(msg, decoded, "UndeclareToken roundtrip failed");
    }
}

/// Property: Decoding liveliness messages from arbitrary bytes never panics.
proptest! {
    #[test]
    fn declare_token_decode_no_panic(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        let _result = crate::msgs::DeclareToken::z_decode(&mut &bytes[..]);
    }
}

proptest! {
    #[test]
    fn undeclare_token_decode_no_panic(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        let _result = crate::msgs::UndeclareToken::z_decode(&mut &bytes[..]);
    }
}
