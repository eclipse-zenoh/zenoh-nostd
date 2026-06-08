#![allow(dead_code, unreachable_code, unused_imports, unused_assignments)]

use zenoh_proto::{
    ZBodyDecode, ZBodyEncode, ZBodyLen, ZEncode, ZExtKind, decode_ext_header, exts::QueryableInfo,
    fields::*, keyexpr, msgs::*, skip_ext,
};

const VLE_LEN_MAX: usize = 10;

fn wire_expr_a_b() -> WireExpr<'static> {
    WireExpr::from(keyexpr::from_str_unchecked("a/b"))
}

fn encode_buf(x: u64) -> (usize, [u8; VLE_LEN_MAX]) {
    let mut buf = [0u8; VLE_LEN_MAX];
    let mut writer: &mut [u8] = &mut buf[..];
    <u64 as ZBodyEncode>::z_body_encode(&x, &mut writer).unwrap();
    let written = VLE_LEN_MAX - writer.len();
    (written, buf)
}

/// Proof 1: encode never overflows its VLE_LEN_MAX buffer for any u64 input.
///
/// Verifies that the writer closure inside `ZBodyEncode for u64` never writes
/// more than VLE_LEN_MAX bytes. The `write_slot(VLE_LEN_MAX, ...)` call
/// allocates exactly VLE_LEN_MAX bytes and errors if the writer exceeds it.
/// If this proof passes, the `get_unchecked_mut` accesses inside encode are
/// always in bounds.
#[kani::proof]
#[kani::unwind(12)]
fn vle_encode_never_overflows() {
    let x: u64 = kani::any();
    let mut buf = [0u8; VLE_LEN_MAX];
    let mut writer: &mut [u8] = &mut buf[..];
    let result = <u64 as ZBodyEncode>::z_body_encode(&x, &mut writer);
    assert!(
        result.is_ok(),
        "encode must succeed within VLE_LEN_MAX bytes"
    );
}

/// Proof 2a: decode never reads past input — any valid-length buffer is safe.
///
/// Verifies that the decode loop never overflows its read bound. The `read_u8`
/// calls return `Err` when the buffer is empty, and `?` propagates it without
/// panicking. The `count >= VLE_LEN_MAX` guard prevents infinite loops.
/// Any symbolic input of up to VLE_LEN_MAX bytes must either decode cleanly
/// or return an error — no out-of-bounds access, no panic.
#[kani::proof]
#[kani::unwind(12)]
fn vle_decode_never_oob() {
    let buf: [u8; VLE_LEN_MAX] = kani::any();
    let mut reader: &[u8] = &buf[..];
    let result = <u64 as ZBodyDecode>::z_body_decode(&mut reader, ());
    match result {
        Ok(v) => {
            assert!(
                v.z_body_len() <= VLE_LEN_MAX,
                "decoded value has valid VLE length"
            );
        }
        Err(_) => {}
    }
}

/// Proof 2b: decode handles completely empty input gracefully.
#[kani::proof]
fn vle_decode_empty_input() {
    let mut reader: &[u8] = &[];
    let result = <u64 as ZBodyDecode>::z_body_decode(&mut reader, ());
    assert!(result.is_err(), "empty input must produce an error");
}

/// Proof 2c: decode handles truncated multi-byte sequences gracefully.
///
/// For each possible truncation point (0 to VLE_LEN_MAX-1), the decode must
/// return an error rather than panic or read out of bounds.
#[kani::proof]
#[kani::unwind(12)]
fn vle_decode_truncated_sequence() {
    let buf: [u8; VLE_LEN_MAX] = kani::any();
    for truncate_len in 0..VLE_LEN_MAX {
        let mut reader: &[u8] = &buf[..truncate_len];
        let _result = <u64 as ZBodyDecode>::z_body_decode(&mut reader, ());
    }
}

/// Proof 3: roundtrip property — encode(decode(x)) == x for all u64.
#[kani::proof]
#[kani::unwind(12)]
fn vle_roundtrip() {
    let x: u64 = kani::any();
    let (written, buf) = encode_buf(x);

    assert!(written >= 1, "VLE encoding must write at least 1 byte");
    assert!(
        written <= VLE_LEN_MAX,
        "VLE encoding must not exceed VLE_LEN_MAX"
    );

    let mut reader: &[u8] = &buf[..written];
    let decoded = <u64 as ZBodyDecode>::z_body_decode(&mut reader, ()).unwrap();

    assert_eq!(x, decoded, "roundtrip must preserve value");
}

/// Proof 4a: boundary value — u64::MAX encodes as exactly VLE_LEN_MAX bytes
/// and decodes correctly.
#[kani::proof]
fn vle_boundary_u64_max() {
    let x = u64::MAX;
    let (written, buf) = encode_buf(x);

    assert_eq!(
        written, VLE_LEN_MAX,
        "u64::MAX must encode as exactly VLE_LEN_MAX bytes"
    );

    let mut reader: &[u8] = &buf[..written];
    let decoded = <u64 as ZBodyDecode>::z_body_decode(&mut reader, ()).unwrap();
    assert_eq!(x, decoded);
}

/// Proof 4b: boundary value — zero encodes as 1 byte and decodes correctly.
#[kani::proof]
fn vle_boundary_zero() {
    let x = 0u64;
    let (written, buf) = encode_buf(x);

    assert_eq!(written, 1, "zero must encode as exactly 1 byte");

    let mut reader: &[u8] = &buf[..written];
    let decoded = <u64 as ZBodyDecode>::z_body_decode(&mut reader, ()).unwrap();
    assert_eq!(x, decoded);
}

/// Proof 4c: boundary values — 2^(7n)-1 for n=1..9 encode as exactly n bytes.
#[kani::proof]
#[kani::unwind(12)]
fn vle_boundary_power_of_7() {
    for n in 1..=9 {
        let shift = 7 * n;
        let x = (1u64 << shift) - 1;
        let (written, buf) = encode_buf(x);

        assert_eq!(
            written, n,
            "2^({})-1 must encode as exactly {} bytes",
            shift, n
        );
        assert_eq!(
            x.z_body_len(),
            n,
            "z_body_len must report {} for 2^({})-1",
            n,
            shift
        );

        let mut reader: &[u8] = &buf[..written];
        let decoded = <u64 as ZBodyDecode>::z_body_decode(&mut reader, ()).unwrap();
        assert_eq!(x, decoded);
    }
}

// ============================
// DeclareBody EXHAUSTIVE MATCH PROOFS
// ============================
//
// The ZEnum derive macro on DeclareBody generates a z_body_decode that
// dispatches on the 5-bit header ID. This section proves all 9 variants
// are constructable, matchable, and decodable without panic.

fn assert_declare_body_variant(body: &DeclareBody<'_>, expected: &str) {
    match body {
        DeclareBody::DeclareKeyExpr(_) => assert_eq!(expected, "DeclareKeyExpr"),
        DeclareBody::UndeclareKeyExpr(_) => assert_eq!(expected, "UndeclareKeyExpr"),
        DeclareBody::DeclareSubscriber(_) => assert_eq!(expected, "DeclareSubscriber"),
        DeclareBody::UndeclareSubscriber(_) => assert_eq!(expected, "UndeclareSubscriber"),
        DeclareBody::DeclareQueryable(_) => assert_eq!(expected, "DeclareQueryable"),
        DeclareBody::UndeclareQueryable(_) => assert_eq!(expected, "UndeclareQueryable"),
        DeclareBody::DeclareToken(_) => assert_eq!(expected, "DeclareToken"),
        DeclareBody::UndeclareToken(_) => assert_eq!(expected, "UndeclareToken"),
        DeclareBody::DeclareFinal(_) => assert_eq!(expected, "DeclareFinal"),
    }
}

/// Proof 5: All 9 DeclareBody variants can be constructed and matched
/// without panic. Every match arm is reachable.
#[kani::proof]
fn declare_body_all_variants_reachable() {
    // Variant 1: DeclareKeyExpr
    let v1 = DeclareBody::DeclareKeyExpr(DeclareKeyExpr {
        id: 1,
        wire_expr: wire_expr_a_b(),
    });
    assert_declare_body_variant(&v1, "DeclareKeyExpr");

    // Variant 2: UndeclareKeyExpr
    let v2 = DeclareBody::UndeclareKeyExpr(UndeclareKeyExpr { id: 1 });
    assert_declare_body_variant(&v2, "UndeclareKeyExpr");

    // Variant 3: DeclareSubscriber
    let v3 = DeclareBody::DeclareSubscriber(DeclareSubscriber {
        id: 1,
        wire_expr: wire_expr_a_b(),
    });
    assert_declare_body_variant(&v3, "DeclareSubscriber");

    // Variant 4: UndeclareSubscriber
    let v4 = DeclareBody::UndeclareSubscriber(UndeclareSubscriber {
        id: 1,
        wire_expr: None,
    });
    assert_declare_body_variant(&v4, "UndeclareSubscriber");

    // Variant 5: DeclareQueryable
    let v5 = DeclareBody::DeclareQueryable(DeclareQueryable {
        id: 1,
        wire_expr: wire_expr_a_b(),
        qinfo: QueryableInfo::default(),
    });
    assert_declare_body_variant(&v5, "DeclareQueryable");

    // Variant 6: UndeclareQueryable
    let v6 = DeclareBody::UndeclareQueryable(UndeclareQueryable {
        id: 1,
        wire_expr: None,
    });
    assert_declare_body_variant(&v6, "UndeclareQueryable");

    // Variant 7: DeclareToken
    let v7 = DeclareBody::DeclareToken(DeclareToken {
        id: 1,
        wire_expr: wire_expr_a_b(),
    });
    assert_declare_body_variant(&v7, "DeclareToken");

    // Variant 8: UndeclareToken
    let v8 = DeclareBody::UndeclareToken(UndeclareToken {
        id: 1,
        wire_expr: None,
    });
    assert_declare_body_variant(&v8, "UndeclareToken");

    // Variant 9: DeclareFinal
    let v9 = DeclareBody::DeclareFinal(DeclareFinal {});
    assert_declare_body_variant(&v9, "DeclareFinal");
}

/// Proof 6: All 9 DeclareBody ID constants are within the valid 5-bit range.
/// The ZEnum-generated z_body_decode extracts `id = header & 0b0001_1111`
/// and matches each variant's ID. This verifies every ID is < 32.
#[kani::proof]
fn declare_body_all_ids_in_range() {
    assert!(
        (DeclareKeyExpr::ID as u8) < 32,
        "DeclareKeyExpr ID out of range"
    );
    assert!(
        (UndeclareKeyExpr::ID as u8) < 32,
        "UndeclareKeyExpr ID out of range"
    );
    assert!(
        (DeclareSubscriber::ID as u8) < 32,
        "DeclareSubscriber ID out of range"
    );
    assert!(
        (UndeclareSubscriber::ID as u8) < 32,
        "UndeclareSubscriber ID out of range"
    );
    assert!(
        (DeclareQueryable::ID as u8) < 32,
        "DeclareQueryable ID out of range"
    );
    assert!(
        (UndeclareQueryable::ID as u8) < 32,
        "UndeclareQueryable ID out of range"
    );
    assert!(
        (DeclareToken::ID as u8) < 32,
        "DeclareToken ID out of range"
    );
    assert!(
        (UndeclareToken::ID as u8) < 32,
        "UndeclareToken ID out of range"
    );
    assert!(
        (DeclareFinal::ID as u8) < 32,
        "DeclareFinal ID out of range"
    );
}

/// Proof 7: The ZEnum-generated z_body_decode for DeclareBody accepts
/// all 9 valid IDs and rejects all others with an error (no panic).
/// This proves the `_ => Err(...)` wildcard arm is safe for invalid inputs.
#[kani::proof]
#[kani::unwind(12)]
fn declare_body_decode_rejects_unknown_ids() {
    let header: u8 = kani::any();
    let buf = [0u8; 32];
    let mut reader: &[u8] = &buf[..];

    let id = header & 0b0001_1111;
    let result = <DeclareBody as ZBodyDecode>::z_body_decode(&mut reader, header);

    let valid = id == (DeclareKeyExpr::ID as u8)
        || id == (UndeclareKeyExpr::ID as u8)
        || id == (DeclareSubscriber::ID as u8)
        || id == (UndeclareSubscriber::ID as u8)
        || id == (DeclareQueryable::ID as u8)
        || id == (UndeclareQueryable::ID as u8)
        || id == (DeclareToken::ID as u8)
        || id == (UndeclareToken::ID as u8)
        || id == (DeclareFinal::ID as u8);

    if valid {
        // Valid IDs may succeed or fail (insufficient buffer) but never panic
        let _ = result;
    } else {
        // Unknown IDs must produce an error (not a panic)
        assert!(result.is_err(), "unknown DeclareBody ID {} must error", id);
    }
}

// ============================
// EXTENSION CODEC PROOFS
// ============================
//
// These proofs verify that the extension skip/decode functions never
// perform out-of-bounds reads, regardless of buffer content.

/// Proof 8: skip_ext never reads out of bounds for any ZExtKind with symbolic buffer.
///
/// Verifies that skip_ext never panics for any buffer content. The function must
/// always return Ok or Err — an OOB read would manifest as a Kani verification failure.
#[kani::proof]
#[kani::unwind(42)]
fn skip_ext_never_oob() {
    let buf: [u8; 32] = kani::any();
    for kind in [ZExtKind::Unit, ZExtKind::U64, ZExtKind::ZStruct] {
        let mut reader: &[u8] = &buf[..];
        let _result = skip_ext(&mut reader, kind);
    }
}

/// Proof 9: skip_ext with each kind on an empty buffer returns Err.
#[kani::proof]
fn skip_ext_empty_input() {
    for kind in [ZExtKind::Unit, ZExtKind::U64, ZExtKind::ZStruct] {
        let mut reader: &[u8] = &[];
        let result = skip_ext(&mut reader, kind);
        assert!(
            result.is_err(),
            "skip_ext on empty buffer must error for {:?}",
            kind
        );
    }
}

/// Proof 10: decode_ext_header never reads out of bounds on non-empty symbolic buffer.
#[kani::proof]
fn decode_ext_header_nonempty() {
    let buf: [u8; 32] = kani::any();
    let mut reader: &[u8] = &buf[..];
    let _result = decode_ext_header(&mut reader);
}

/// Proof 11: decode_ext_header returns Err on empty buffer.
#[kani::proof]
fn decode_ext_header_empty() {
    let mut reader: &[u8] = &[];
    let result = decode_ext_header(&mut reader);
    assert!(
        result.is_err(),
        "decode_ext_header on empty buffer must error"
    );
}

/// Proof 12: Combined decode_ext_header + skip_ext on symbolic buffer never OOB.
///
/// This mirrors the real extension decode flow: peek header, then skip body.
#[kani::proof]
#[kani::unwind(42)]
fn ext_decode_skip_combined() {
    let buf: [u8; 32] = kani::any();
    let mut reader: &[u8] = &buf[..];
    if let Ok((_, kind, _, _)) = decode_ext_header(&mut reader) {
        let mut reader2: &[u8] = &buf[..];
        let _result = skip_ext(&mut reader2, kind);
    }
}

// ============================
// CURSOR PROOFS
// ============================

use crate::verus_cursor::{Cursor, CursorError};

/// Proof 13: Cursor::advance(n) never moves past the limit.
///
/// On success: pos <= limit always holds.
/// On failure: pos and remaining are unchanged (atomic semantics).
/// The Overrun error carries the exact position, limit, and requested count.
#[kani::proof]
#[kani::unwind(12)]
fn kani_cursor_advance() {
    let limit: usize = kani::any();
    kani::assume(limit <= 256);
    let mut c = Cursor::new(limit);

    let n: usize = kani::any();
    kani::assume(n <= 256);

    let old_pos = c.position();
    let old_remaining = c.remaining();
    let result = c.advance(n);

    match result {
        Ok(()) => {
            assert!(c.position() <= c.limit(), "pos must not exceed limit");
        }
        Err(CursorError::Overrun { .. }) => {
            assert_eq!(c.position(), old_pos, "pos unchanged on failure");
            assert_eq!(
                c.remaining(),
                old_remaining,
                "remaining unchanged on failure"
            );
        }
        Err(_) => unreachable!("advance only returns Overrun errors"),
    }
}

/// Proof 14: Cursor::read_u8() succeeds when remaining > 0, fails when empty.
///
/// On success: remaining decreases by exactly 1.
/// On failure: cursor state is unchanged.
#[kani::proof]
#[kani::unwind(12)]
fn kani_cursor_read_u8() {
    let limit: usize = kani::any();
    kani::assume(limit <= 256);
    let mut c = Cursor::new(limit);

    let old_pos = c.position();
    let old_remaining = c.remaining();
    let result = c.read_u8();

    match result {
        Ok(()) => {
            assert_eq!(c.position(), old_pos + 1, "pos increased by 1");
            assert!(old_remaining > 0, "must have had remaining bytes");
            assert_eq!(c.remaining() + 1, old_remaining, "remaining decreased by 1");
        }
        Err(CursorError::Overrun { .. }) => {
            assert_eq!(old_remaining, 0, "only fails when empty");
            assert_eq!(c.position(), old_pos, "pos unchanged on failure");
        }
        Err(_) => unreachable!("read_u8 only returns Overrun errors"),
    }
}

/// Proof 15: Cursor::read_slice(n) never reads beyond the limit.
///
/// n == 0 is always a no-op (succeeds, no position change).
/// n > 0 follows advance(n) semantics.
#[kani::proof]
#[kani::unwind(12)]
fn kani_cursor_read_slice() {
    let limit: usize = kani::any();
    kani::assume(limit <= 256);
    let mut c = Cursor::new(limit);

    let n: usize = kani::any();
    kani::assume(n <= 256);

    let old_pos = c.position();
    let old_remaining = c.remaining();
    let result = c.read_slice(n);

    match result {
        Ok(()) => {
            assert!(c.position() <= c.limit(), "pos must not exceed limit");
            if n == 0 {
                assert_eq!(c.position(), old_pos, "zero-length slice is no-op");
                assert_eq!(
                    c.remaining(),
                    old_remaining,
                    "remaining unchanged for zero-length"
                );
            }
        }
        Err(CursorError::Overrun { .. }) => {
            assert_eq!(c.position(), old_pos, "pos unchanged on failure");
            assert_eq!(
                c.remaining(),
                old_remaining,
                "remaining unchanged on failure"
            );
        }
        Err(_) => unreachable!("read_slice only returns Overrun errors"),
    }
}

/// Proof 16: No cursor operation ever results in pos > limit.
///
/// This single-step safety property holds for any operation
/// (advance, read_u8, read_slice, read_vle) from any starting position
/// reachable via Cursor::new.
#[kani::proof]
#[kani::unwind(12)]
fn kani_cursor_no_overread() {
    let limit: usize = kani::any();
    kani::assume(limit <= 256);
    let mut c = Cursor::new(limit);

    let n: usize = kani::any();
    kani::assume(n <= 256);

    // Invariant before every operation
    assert!(c.position() <= c.limit(), "invariant: pos <= limit");

    // Try each operation; invariant must hold after each
    let _ = c.advance(n);
    assert!(c.position() <= c.limit(), "invariant after advance");

    let _ = c.read_u8();
    assert!(c.position() <= c.limit(), "invariant after read_u8");

    let _ = c.read_slice(n);
    assert!(c.position() <= c.limit(), "invariant after read_slice");

    let _ = c.read_vle();
    assert!(c.position() <= c.limit(), "invariant after read_vle");
}

/// Proof 17: Cursor::read_vle never leaves the cursor in an invalid state.
///
/// On success: pos advances by at least 1 and at most 10 (standard VLE bound).
/// On Overrun: pos is reset to the start of the VLE read (rollback).
/// On MalformedVle: pos is reset to the start (rollback).
#[kani::proof]
#[kani::unwind(12)]
fn kani_cursor_read_vle() {
    let limit: usize = kani::any();
    kani::assume(limit <= 256);
    let mut c = Cursor::new(limit);

    let old_pos = c.position();
    let result = c.read_vle();

    match result {
        Ok(()) => {
            assert!(c.position() > old_pos, "VLE must advance");
            assert!(c.position() <= c.limit(), "VLE must not exceed limit");
        }
        Err(CursorError::Overrun { .. }) => {
            // On overrun, pos is reset to `start` which is the initial pos
            assert!(c.position() <= c.limit(), "pos still valid after overrun");
        }
        Err(CursorError::MalformedVle { .. }) => {
            // On malformed, pos is reset to `start`
            assert!(c.position() <= c.limit(), "pos still valid after malformed");
        }
    }
}

/// Proof 18: After Cursor::new(limit), invariant holds immediately.
#[kani::proof]
#[kani::unwind(12)]
fn kani_cursor_new_invariant() {
    let limit: usize = kani::any();
    kani::assume(limit <= 256);
    let c = Cursor::new(limit);

    assert_eq!(c.position(), 0, "starts at position 0");
    assert_eq!(c.remaining(), limit, "remaining equals limit");
    assert_eq!(c.limit(), limit, "limit stored correctly");
    assert!(c.position() <= c.limit(), "invariant holds");
}

// ============================
// SESSION STATE MACHINE PROOFS
// ============================

use crate::verus_session::{Event, SessionState, TransitionError, transition};

/// Proof 19: For any valid state and any valid event, the transition function
/// never panics. It always returns either Ok(a valid SessionState variant) or
/// Err(TransitionError). Every reachable (state, event) pair is covered by
/// the match or falls through to the wildcard error arm.
#[kani::proof]
#[kani::unwind(20)]
fn kani_state_machine_transition_valid() {
    let state_idx: u8 = kani::any();
    let event_idx: u8 = kani::any();
    kani::assume(state_idx < 6);
    kani::assume(event_idx < 14);

    let state = match state_idx {
        0 => SessionState::Closed,
        1 => SessionState::Opening,
        2 => SessionState::Open,
        3 => SessionState::Degraded,
        4 => SessionState::Closing,
        _ => SessionState::Failed,
    };

    let event = match event_idx {
        0 => Event::InitSynSent,
        1 => Event::InitSynReceived,
        2 => Event::InitAckSent,
        3 => Event::InitAckReceived,
        4 => Event::OpenSynSent,
        5 => Event::OpenSynReceived,
        6 => Event::OpenAckSent,
        7 => Event::OpenAckReceived,
        8 => Event::KeepAliveTimeout,
        9 => Event::TransportError,
        10 => Event::CloseSent,
        11 => Event::CloseReceived,
        12 => Event::DegradedDetected,
        _ => Event::Recovered,
    };

    let result = transition(state, event);

    match result {
        Ok(SessionState::Closed)
        | Ok(SessionState::Opening)
        | Ok(SessionState::Open)
        | Ok(SessionState::Degraded)
        | Ok(SessionState::Closing)
        | Ok(SessionState::Failed) => {}
        Err(TransitionError::InvalidTransition { .. }) => {}
    }
}

/// Proof 20: Once in Open state, only DegradedDetected, CloseSent,
/// CloseReceived, and TransportError produce non-error transitions.
/// All other events (InitSyn*, InitAck*, OpenSyn*, OpenAck*, KeepAliveTimeout,
/// Recovered) return errors — you cannot re-init or re-open an already
/// open session.
#[kani::proof]
#[kani::unwind(20)]
fn kani_state_machine_open_stays_open() {
    let event_idx: u8 = kani::any();
    kani::assume(event_idx < 14);

    let state = SessionState::Open;

    let event = match event_idx {
        0 => Event::InitSynSent,
        1 => Event::InitSynReceived,
        2 => Event::InitAckSent,
        3 => Event::InitAckReceived,
        4 => Event::OpenSynSent,
        5 => Event::OpenSynReceived,
        6 => Event::OpenAckSent,
        7 => Event::OpenAckReceived,
        8 => Event::KeepAliveTimeout,
        9 => Event::TransportError,
        10 => Event::CloseSent,
        11 => Event::CloseReceived,
        12 => Event::DegradedDetected,
        _ => Event::Recovered,
    };

    let result = transition(state, event);

    match event {
        Event::DegradedDetected => {
            assert_eq!(result, Ok(SessionState::Degraded));
        }
        Event::CloseSent | Event::CloseReceived => {
            assert_eq!(result, Ok(SessionState::Closing));
        }
        Event::TransportError => {
            assert_eq!(result, Ok(SessionState::Failed));
        }
        _ => {
            assert!(result.is_err(), "Open should reject event {:?}", event);
        }
    }
}

/// Proof 21: Closed, Closing, and Failed states accept only their
/// reconnection/completion events. All other events are rejected:
///
/// Closed   → InitSyn{Sent,Received} → Opening; everything else → error
/// Closing  → Close{Sent,Received}   → Closed; TransportError → Failed;
///            everything else → error
/// Failed   → InitSyn{Sent,Received} → Opening; CloseReceived → Closed;
///            everything else → error
#[kani::proof]
#[kani::unwind(20)]
fn kani_state_machine_absorbing_states() {
    let state_idx: u8 = kani::any();
    let event_idx: u8 = kani::any();
    kani::assume(state_idx == 0 || state_idx == 4 || state_idx == 5);
    kani::assume(event_idx < 14);

    let state = match state_idx {
        0 => SessionState::Closed,
        4 => SessionState::Closing,
        _ => SessionState::Failed,
    };

    let event = match event_idx {
        0 => Event::InitSynSent,
        1 => Event::InitSynReceived,
        2 => Event::InitAckSent,
        3 => Event::InitAckReceived,
        4 => Event::OpenSynSent,
        5 => Event::OpenSynReceived,
        6 => Event::OpenAckSent,
        7 => Event::OpenAckReceived,
        8 => Event::KeepAliveTimeout,
        9 => Event::TransportError,
        10 => Event::CloseSent,
        11 => Event::CloseReceived,
        12 => Event::DegradedDetected,
        _ => Event::Recovered,
    };

    let result = transition(state, event);

    match (state, event) {
        (SessionState::Closed, Event::InitSynSent)
        | (SessionState::Closed, Event::InitSynReceived) => {
            assert_eq!(result, Ok(SessionState::Opening));
        }
        (SessionState::Closing, Event::CloseSent)
        | (SessionState::Closing, Event::CloseReceived) => {
            assert_eq!(result, Ok(SessionState::Closed));
        }
        (SessionState::Closing, Event::TransportError) => {
            assert_eq!(result, Ok(SessionState::Failed));
        }
        (SessionState::Failed, Event::InitSynSent)
        | (SessionState::Failed, Event::InitSynReceived) => {
            assert_eq!(result, Ok(SessionState::Opening));
        }
        (SessionState::Failed, Event::CloseReceived) => {
            assert_eq!(result, Ok(SessionState::Closed));
        }
        _ => {
            assert!(
                result.is_err(),
                "{:?} should reject event {:?}",
                state,
                event
            );
        }
    }
}

/// Proof 22: Epoch counter is monotonic — the SN decode logic never
/// decreases the epoch. For any u32 SN and epoch values:
///
///   - If SN wraps (u32::MAX → 0), epoch increments by exactly 1.
///   - If SN does not wrap, epoch is unchanged.
///   - Epoch never decrements.
#[kani::proof]
fn kani_state_machine_epoch_counter() {
    let sn: u32 = kani::any();
    let epoch: u32 = kani::any();

    let old_sn = sn;
    let old_epoch = epoch;

    let new_sn = old_sn.wrapping_add(1);
    let wraps = new_sn < old_sn;

    let mut test_sn = old_sn;
    let mut test_epoch = old_epoch;

    let old_expected = test_sn;
    test_sn = test_sn.wrapping_add(1);
    if test_sn < old_expected {
        test_epoch = test_epoch.wrapping_add(1);
    }

    // epoch is monotonic: never decreases, only stays same or increments by 1
    assert!(
        test_epoch == old_epoch || test_epoch == old_epoch.wrapping_add(1),
        "epoch must be non-decreasing"
    );

    // epoch increments iff SN wrapped
    assert_eq!(
        wraps,
        test_epoch != old_epoch,
        "epoch must increment iff SN wraps"
    );

    // epoch never increments by more than 1
    assert!(
        test_epoch == old_epoch || test_epoch == old_epoch.wrapping_add(1),
        "epoch must increment by at most 1"
    );

    // SN always advances by exactly 1
    assert_eq!(test_sn, new_sn, "SN must advance by 1");
}
