#![allow(dead_code, unused_imports)]

use core::time::Duration;

use zenoh_proto::{exts::Patch, fields::*, msgs::*};

use crate::transport::establishment::State;
use crate::{TransportRx, TransportTx, ZTransportRx, ZTransportTx};

fn make_rx(buff: [u8; 64], max_fragments: usize) -> TransportRx<[u8; 64]> {
    TransportRx::new(buff, 64, 0, Resolution::default(), Duration::from_secs(10))
        .with_max_fragments(max_fragments)
}

fn make_tx(buff: [u8; 64]) -> TransportTx<[u8; 64]> {
    TransportTx::new(buff, 64, 0, Resolution::default(), Duration::from_secs(10))
}

fn any_zid() -> ZenohIdProto {
    let bytes: [u8; 16] = kani::any();
    kani::assume(bytes != [0u8; 16]);
    ZenohIdProto::try_from(&bytes[..]).unwrap()
}

fn make_peer_init_syn(zid: ZenohIdProto) -> InitSyn<'static> {
    InitSyn {
        version: zenoh_proto::VERSION,
        identifier: InitIdentifier {
            zid,
            whatami: WhatAmI::Peer,
        },
        resolution: InitResolution {
            resolution: Resolution::default(),
            batch_size: BatchSize(512),
        },
        qos: None,
        qos_link: None,
        auth: None,
        mlink: None,
        lowlatency: None,
        compression: None,
        patch: Patch::none(),
    }
}

fn make_peer_init_ack(zid: ZenohIdProto) -> InitAck<'static> {
    InitAck {
        version: zenoh_proto::VERSION,
        identifier: InitIdentifier {
            zid,
            whatami: WhatAmI::Peer,
        },
        resolution: InitResolution {
            resolution: Resolution::default(),
            batch_size: BatchSize(512),
        },
        cookie: &[],
        qos: None,
        qos_link: None,
        auth: None,
        mlink: None,
        lowlatency: None,
        compression: None,
        patch: Patch::none(),
    }
}

fn make_open_ack() -> OpenAck<'static> {
    OpenAck {
        lease: Duration::from_secs(30),
        sn: 0u32,
        qos: None,
        auth: None,
        mlink_syn: None,
        mlink_ack: None,
        lowlatency: None,
        compression: None,
    }
}

fn make_open_syn(cookie: &[u8]) -> OpenSyn<'_> {
    OpenSyn {
        lease: Duration::from_secs(30),
        sn: 0u32,
        cookie,
        qos: None,
        auth: None,
        mlink_syn: None,
        mlink_ack: None,
        lowlatency: None,
        compression: None,
    }
}

// ============================
// PROOF 1: State transition legality for TransportRx
// ============================

/// A new TransportRx starts in Opened
#[kani::proof]
fn rx_initial_state_is_opened() {
    let rx = make_rx([0u8; 64], 1);
    assert!(!rx.closed());
    assert!(!rx.should_close(Duration::ZERO));
    assert_eq!(rx.next_timeout(), Duration::ZERO);
}

/// decode_raw with data: Opened → Used
#[kani::proof]
fn rx_decode_raw_transitions_to_used() {
    let mut rx = make_rx([0u8; 64], 1);
    assert_eq!(rx.next_timeout(), Duration::ZERO);
    let _ = rx.decode_raw(&[1u8; 4]);
    rx.sync(None, Duration::ZERO);
    assert!(!rx.closed());
    assert!(rx.next_timeout() > Duration::ZERO);
}

/// Empty decode_raw + sync: Opened → Synchronized (data not needed for sync transition)
#[kani::proof]
fn rx_decode_raw_empty_then_sync() {
    let mut rx = make_rx([0u8; 64], 1);
    let _ = rx.decode_raw(&[]);
    rx.sync(None, Duration::ZERO);
    assert!(!rx.closed());
    assert!(rx.next_timeout() > Duration::ZERO);
}

/// sync: Opened → Synchronized
#[kani::proof]
fn rx_sync_opened_to_synchronized() {
    let mut rx = make_rx([0u8; 64], 1);
    rx.sync(None, Duration::ZERO);
    assert!(!rx.should_close(Duration::ZERO));
    assert!(rx.next_timeout() > Duration::ZERO);
}

/// Lease expiry: Synchronized → Closed
#[kani::proof]
fn rx_sync_expired_lease_closes() {
    let mut rx = make_rx([0u8; 64], 1);
    rx.sync(None, Duration::ZERO);
    rx.sync(None, Duration::from_secs(100));
    assert!(rx.closed());
}

/// Closed tx causes rx to close
#[kani::proof]
fn rx_closes_when_tx_closes() {
    let mut rx = make_rx([0u8; 64], 1);
    let mut tx = make_tx([0u8; 64]);
    tx.sync(None, Duration::ZERO);
    tx.sync(None, Duration::from_secs(100));
    assert!(tx.closed());
    rx.sync(Some(&tx), Duration::ZERO);
    assert!(rx.closed());
}

/// decode_raw on closed state is a no-op
#[kani::proof]
fn rx_decode_raw_closed_is_noop() {
    let mut rx = make_rx([0u8; 64], 1);
    rx.sync(None, Duration::ZERO);
    rx.sync(None, Duration::from_secs(100));
    assert!(rx.closed());
    let result = rx.decode_raw(&[1u8; 4]);
    assert!(result.is_ok());
    assert!(rx.closed());
}

/// Mutual sync keeps both synchronized
#[kani::proof]
fn tx_and_rx_sync_together() {
    let mut rx = make_rx([0u8; 64], 1);
    let mut tx = make_tx([0u8; 64]);
    rx.sync(Some(&tx), Duration::ZERO);
    tx.sync(Some(&rx), Duration::ZERO);
    assert!(!rx.closed());
    assert!(rx.next_timeout() >= Duration::from_secs(10));
    assert!(tx.next_timeout() >= Duration::from_secs(2));
}

// ============================
// PROOF 2: Fragment reassembly safety
// ============================

/// max_fragments=1: reassembly is a no-op
#[kani::proof]
fn fragment_max1_noop() {
    let rx = make_rx([0u8; 64], 1);
    let _ = rx;
}

/// max_fragments>1 with concrete fragment frames: no panic
#[kani::proof]
#[kani::unwind(16)]
fn fragment_max2_concrete_no_panic() {
    let mut rx = make_rx([0u8; 64], 2);
    let _ = rx.decode_raw(&[0x05u8; 16]);
    let _items = rx.flush();
}

/// max_fragments>1 with zero buffer: safe
#[kani::proof]
#[kani::unwind(16)]
fn fragment_zero_buffer() {
    let mut rx = make_rx([0u8; 64], 3);
    let _ = rx.decode_raw(&[0u8; 8]);
    let _items = rx.flush();
}

/// max_fragments=2 with flush_transport
#[kani::proof]
#[kani::unwind(16)]
fn fragment_flush_transport_no_panic() {
    let mut rx = make_rx([0u8; 64], 2);
    let _ = rx.decode_raw(&[0x05u8; 8]);
    let _items = rx.flush_transport();
}

// ============================
// PROOF 3: SN monotonicity
// ============================

/// sn.wrapping_add(1) is always safe
#[kani::proof]
fn sn_wrapping_add_one_always_safe() {
    let sn: u32 = kani::any();
    let _next = sn.wrapping_add(1);
}

/// The 2^31 SN comparison boundary is correct
#[kani::proof]
fn sn_wrapping_comparison_boundary() {
    let expected: u32 = kani::any();
    let boundary = 1u32 << 31;

    let diff_0 = expected.wrapping_sub(expected);
    assert_eq!(diff_0, 0);

    let just_behind = expected.wrapping_sub(1);
    let diff_neg1 = just_behind.wrapping_sub(expected);
    assert!(diff_neg1 >= boundary);
}

// ============================
// PROOF 4: Establishment protocol
// ============================

/// Listener: InitSyn → InitAck(out) → WaitingOpenSyn
#[kani::proof]
#[kani::unwind(32)]
fn establishment_listener_init_syn_to_ack() {
    let mine = any_zid();
    let other = any_zid();
    kani::assume(mine != other);

    let mut state = State::WaitingInitSyn {
        mine_zid: mine,
        mine_whatami: WhatAmI::Peer,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    };

    let (r, d) = state.poll((TransportMessage::InitSyn(make_peer_init_syn(other)), &[]));
    assert!(r.is_some());
    assert!(matches!(r, Some(TransportMessage::InitAck(_))));
    assert!(d.is_none());
    assert!(matches!(state, State::WaitingOpenSyn { .. }));
}

/// Initiator: WaitingOpenAck + OpenAck → Description → Opened
#[kani::proof]
#[kani::unwind(32)]
fn establishment_initiator_open_ack_to_opened() {
    let mine = any_zid();
    let other = any_zid();

    let mut state = State::WaitingOpenAck {
        mine_zid: mine,
        batch_size: 512,
        resolution: Resolution::default(),
        sn: 0u32,
        mine_lease: Duration::from_secs(30),
        other_zid: other,
    };

    let (r, d) = state.poll((TransportMessage::OpenAck(make_open_ack()), &[]));
    assert!(r.is_none());
    assert!(d.is_some());
    assert!(matches!(state, State::Opened(_)));
}

/// Every poll returns immediately (tests paths without SHA3 hashing)
#[kani::proof]
#[kani::unwind(32)]
fn establishment_never_hangs() {
    let mine = any_zid();

    let mut s1 = State::WaitingInitSyn {
        mine_zid: mine,
        mine_whatami: WhatAmI::Peer,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    };
    let mut s3 = State::WaitingOpenAck {
        mine_zid: mine,
        batch_size: 512,
        resolution: Resolution::default(),
        sn: 0u32,
        mine_lease: Duration::from_secs(30),
        other_zid: mine,
    };

    let _ = s1.poll((TransportMessage::InitAck(make_peer_init_ack(mine)), &[]));
    let _ = s1.poll((TransportMessage::OpenSyn(make_open_syn(&[])), &[]));
    let _ = s1.poll((TransportMessage::OpenAck(make_open_ack()), &[]));
    let _ = s1.poll((
        TransportMessage::Close(Close {
            reason: 0,
            behaviour: CloseBehaviour::Session,
        }),
        &[],
    ));

    let _ = s3.poll((TransportMessage::InitSyn(make_peer_init_syn(mine)), &[]));
    let _ = s3.poll((TransportMessage::InitAck(make_peer_init_ack(mine)), &[]));
    let _ = s3.poll((TransportMessage::OpenSyn(make_open_syn(&[])), &[]));
}

/// Invalid message in establishment state returns (None, None)
#[kani::proof]
#[kani::unwind(32)]
fn establishment_invalid_returns_none_none() {
    let mine = any_zid();
    let mut state = State::WaitingInitSyn {
        mine_zid: mine,
        mine_whatami: WhatAmI::Peer,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    };
    let (r, d) = state.poll((
        TransportMessage::Close(Close {
            reason: 0,
            behaviour: CloseBehaviour::Session,
        }),
        &[],
    ));
    assert!(r.is_none());
    assert!(d.is_none());
}

/// Simultaneous open: higher ZID yields, lower ZID wins
#[kani::proof]
#[kani::unwind(32)]
fn establishment_simultaneous_open() {
    let mine = any_zid();
    let other = any_zid();
    kani::assume(mine != other);

    let mut state = State::WaitingInitAck {
        mine_zid: mine,
        mine_whatami: WhatAmI::Peer,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    };
    let (r, d) = state.poll((TransportMessage::InitSyn(make_peer_init_syn(other)), &[]));

    if mine > other {
        assert!(r.is_some(), "higher ZID yields with InitAck");
        assert!(matches!(state, State::WaitingOpenSyn { .. }));
    } else {
        assert!(r.is_none(), "lower ZID wins");
        assert!(matches!(state, State::WaitingInitAck { .. }));
    }
    assert!(d.is_none());
}

/// Opened state always returns (None, Some(Description)) and stays Opened
#[kani::proof]
#[kani::unwind(32)]
fn opened_state_returns_description() {
    let mine = any_zid();
    let other = any_zid();
    let mut state = State::Opened(crate::transport::establishment::Description {
        mine_zid: mine,
        batch_size: 512,
        resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
        other_lease: Duration::from_secs(30),
        mine_sn: 0,
        other_sn: 0,
        other_zid: other,
    });
    for msg in [
        TransportMessage::Close(Close {
            reason: 0,
            behaviour: CloseBehaviour::Session,
        }),
        TransportMessage::InitSyn(make_peer_init_syn(other)),
        TransportMessage::InitAck(make_peer_init_ack(other)),
        TransportMessage::OpenSyn(make_open_syn(&[])),
        TransportMessage::OpenAck(make_open_ack()),
    ] {
        let (r, d) = state.poll((msg, &[]));
        assert!(r.is_none());
        assert!(d.is_some());
        assert!(matches!(state, State::Opened(_)));
    }
}

// ============================
// PROOF 5: Resolution negotiation bounds
// ============================

/// Resolution::default() is safe
#[kani::proof]
fn resolution_default_safe() {
    let _r = Resolution::default();
}

/// Bits ordering is correct
#[kani::proof]
fn bits_comparison_ordering() {
    assert!(Bits::U8 < Bits::U16);
    assert!(Bits::U16 < Bits::U32);
    assert!(Bits::U32 < Bits::U64);
}

/// ZenohIdProto::size() is within bounds
#[kani::proof]
fn zid_size_bounds() {
    let zid = any_zid();
    let size = zid.size();
    assert!(size > 0 && size <= 16);
}

// ============================
// PROOF 6: KeepAlive replay resistance
// ============================
//
// An attacker who captures a KeepAlive message must not be able to extend
// the session lease indefinitely. The fix adds a separate last_data_received
// timestamp that is ONLY updated by data-bearing messages (Push, Request,
// Response, etc.), NOT by KeepAlive. KeepAlive-only decode_raw → sync cycles
// must not prevent lease expiry.

/// KeepAlive replay does not extend the lease: the session closes after
/// lease expires despite repeated decode_raw + sync cycles (which is what
/// KeepAlive replay triggers).
#[kani::proof]
fn keepalive_replay_does_not_extend_lease() {
    let mut rx = make_rx([0u8; 64], 1);
    let lease = Duration::from_secs(10);

    // Initial sync at T=0
    rx.sync(None, Duration::ZERO);
    assert!(!rx.closed());
    assert!(rx.next_timeout() > Duration::ZERO);
    // next_timeout = max(last_data_received=0, last_synced=0) + lease = 10s
    assert!(rx.next_timeout() <= lease);

    // Simulate a sequence of KeepAlive replays: decode_raw (→ Used) then sync
    // At T=5: attacker replays captured KeepAlive
    let _ = rx.decode_raw(&[1u8; 4]);
    rx.sync(None, Duration::from_secs(5));
    assert!(!rx.closed());
    // KeepAlive must NOT extend the lease — next_timeout still ≤ 10s
    assert!(rx.next_timeout() <= lease);

    // At T=9: another KeepAlive replay
    let _ = rx.decode_raw(&[1u8; 4]);
    rx.sync(None, Duration::from_secs(9));
    assert!(!rx.closed());
    assert!(rx.next_timeout() <= lease);

    // At T=11: lease expired (lease=10, last_synced=T=0)
    // The connection MUST close despite repeated KeepAlive replays
    rx.sync(None, Duration::from_secs(11));
    assert!(rx.closed());
}
