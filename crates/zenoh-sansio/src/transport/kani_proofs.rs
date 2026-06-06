use core::time::Duration;
use zenoh_proto::{exts::Patch, fields::*, msgs::*};

use crate::transport::establishment::State;

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

fn make_waiting_init_syn_state(zid: ZenohIdProto) -> State {
    State::WaitingInitSyn {
        mine_zid: zid,
        mine_whatami: WhatAmI::Peer,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    }
}

fn make_waiting_init_ack_state(zid: ZenohIdProto) -> State {
    State::WaitingInitAck {
        mine_zid: zid,
        mine_whatami: WhatAmI::Peer,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    }
}

fn make_waiting_open_syn_state(zid: ZenohIdProto) -> State {
    State::WaitingOpenSyn {
        mine_zid: zid,
        mine_whatami: WhatAmI::Peer,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    }
}

fn make_waiting_open_ack_state(zid: ZenohIdProto, other_zid: ZenohIdProto) -> State {
    State::WaitingOpenAck {
        mine_zid: zid,
        batch_size: 512,
        resolution: Resolution::default(),
        sn: 0u32,
        mine_lease: Duration::from_secs(30),
        other_zid,
    }
}

fn make_opened_state(mine: ZenohIdProto, other: ZenohIdProto) -> State {
    State::Opened(crate::transport::establishment::Description {
        mine_zid: mine,
        batch_size: 512,
        resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
        other_lease: Duration::from_secs(30),
        mine_sn: 0u32,
        other_sn: 0u32,
        other_zid: other,
    })
}

// ============================
// ENUM OF ALL STATE TRANSITIONS
// ============================
//
// State         │ Input      │ Result                     │ New State
// ──────────────│────────────│────────────────────────────│────────────────
// WaitingInitSyn│ InitSyn    │ Some(InitAck), None        │ WaitingOpenSyn
// WaitingInitSyn│ InitAck    │ None, None                 │ WaitingInitSyn (unchanged)
// WaitingInitSyn│ OpenSyn    │ None, None                 │ WaitingInitSyn (unchanged)
// WaitingInitSyn│ OpenAck    │ None, None                 │ WaitingInitSyn (unchanged)
// WaitingInitSyn│ Close/etc  │ None, None                 │ WaitingInitSyn (unchanged)
// ──────────────│────────────│────────────────────────────│────────────────
// WaitingInitAck│ InitSyn (peer, mine>other)│ Some(InitAck), None│ WaitingOpenSyn
// WaitingInitAck│ InitSyn (peer, mine<other)│ None, None        │ WaitingInitAck (unchanged)
// WaitingInitAck│ InitSyn (peer, mine==other)│ None, None (err) │ WaitingInitAck (unchanged)
// WaitingInitAck│ InitSyn (non-peer)│ None, None (err)  │ WaitingInitAck (unchanged)
// WaitingInitAck│ InitAck    │ Some(OpenSyn), None       │ WaitingOpenAck
// WaitingInitAck│ OpenSyn    │ None, None                 │ WaitingInitAck (unchanged)
// WaitingInitAck│ OpenAck    │ None, None                 │ WaitingInitAck (unchanged)
// WaitingInitAck│ Close/etc  │ None, None                 │ WaitingInitAck (unchanged)
// ──────────────│────────────│────────────────────────────│────────────────
// WaitingOpenSyn│ InitSyn    │ None, None                 │ WaitingOpenSyn (unchanged)
// WaitingOpenSyn│ InitAck    │ None, None                 │ WaitingOpenSyn (unchanged)
// WaitingOpenSyn│ OpenSyn    │ Some(OpenAck), Some(Desc)  │ Opened
// WaitingOpenSyn│ OpenAck    │ None, None                 │ WaitingOpenSyn (unchanged)
// WaitingOpenSyn│ Close/etc  │ None, None                 │ WaitingOpenSyn (unchanged)
// ──────────────│────────────│────────────────────────────│────────────────
// WaitingOpenAck│ InitSyn    │ None, None                 │ WaitingOpenAck (unchanged)
// WaitingOpenAck│ InitAck    │ None, None                 │ WaitingOpenAck (unchanged)
// WaitingOpenAck│ OpenSyn    │ None, None                 │ WaitingOpenAck (unchanged)
// WaitingOpenAck│ OpenAck    │ None, Some(Desc)           │ Opened
// WaitingOpenAck│ Close/etc  │ None, None                 │ WaitingOpenAck (unchanged)
// ──────────────│────────────│────────────────────────────│────────────────
// Opened        │ *any*      │ None, Some(Desc)           │ Opened (unchanged)

// ============================
// EXISTING PROOFS (preserved)
// ============================

/// Proof: WaitingInitSyn always responds with Some(InitAck) and transitions to WaitingOpenSyn
#[kani::proof]
#[kani::unwind(32)]
fn init_syn_to_init_ack_proof() {
    let mine = any_zid();
    let other = any_zid();

    let mut state = make_waiting_init_syn_state(mine);
    let init_syn = make_peer_init_syn(other);

    let (response, _) = state.poll((TransportMessage::InitSyn(init_syn), &[] as &[u8]));
    assert!(response.is_some());
    assert!(matches!(state, State::WaitingOpenSyn { .. }));
}

/// Proof: State transitions are deterministic — same input → same output
#[kani::proof]
#[kani::unwind(32)]
fn state_transition_determinism_proof() {
    let mine = any_zid();
    let other = any_zid();

    let mut state_a = make_waiting_init_ack_state(mine);
    let mut state_b = make_waiting_init_ack_state(mine);

    let init_a = make_peer_init_syn(other);
    let init_b = make_peer_init_syn(other);

    let (resp_a, _) = state_a.poll((TransportMessage::InitSyn(init_a), &[] as &[u8]));
    let (resp_b, _) = state_b.poll((TransportMessage::InitSyn(init_b), &[] as &[u8]));

    assert_eq!(resp_a, resp_b);
    assert_eq!(state_a, state_b);
}

/// Proof: For ANY two distinct peer ZIDs, exactly one wins and one yields
#[kani::proof]
#[kani::unwind(32)]
fn zid_comparison_exhaustive_proof() {
    let mine = any_zid();
    let other = any_zid();
    kani::assume(mine != other);

    let mut state = make_waiting_init_ack_state(mine);
    let init_syn = make_peer_init_syn(other);

    let (response, _) = state.poll((TransportMessage::InitSyn(init_syn), &[] as &[u8]));

    assert!(
        (mine > other && response.is_some()) || (mine < other && response.is_none()),
    );
}

// ============================
// TRANSITION: WaitingInitSyn
// ============================

/// Proof: WaitingInitSyn rejects InitAck (wrong message type) → (None, None), state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn waiting_init_syn_rejects_init_ack() {
    let mine = any_zid();
    let other = any_zid();

    let mut state = make_waiting_init_syn_state(mine);
    let original = state;
    let ack = make_peer_init_ack(other);

    let (response, description) = state.poll((TransportMessage::InitAck(ack), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_none());
    assert_eq!(state, original);
}

/// Proof: WaitingInitSyn rejects OpenSyn (wrong message type) → (None, None), state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn waiting_init_syn_rejects_open_syn() {
    let mine = any_zid();

    let mut state = make_waiting_init_syn_state(mine);
    let original = state;
    let open_syn = make_open_syn(&[]);

    let (response, description) = state.poll((TransportMessage::OpenSyn(open_syn), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_none());
    assert_eq!(state, original);
}

/// Proof: WaitingInitSyn rejects OpenAck (wrong message type) → (None, None), state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn waiting_init_syn_rejects_open_ack() {
    let mine = any_zid();

    let mut state = make_waiting_init_syn_state(mine);
    let original = state;
    let ack = make_open_ack();

    let (response, description) = state.poll((TransportMessage::OpenAck(ack), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_none());
    assert_eq!(state, original);
}

// ============================
// TRANSITION: WaitingInitAck
// ============================

/// Proof: WaitingInitAck rejects OpenSyn (wrong message type) → (None, None), state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn waiting_init_ack_rejects_open_syn() {
    let mine = any_zid();

    let mut state = make_waiting_init_ack_state(mine);
    let original = state;
    let open_syn = make_open_syn(&[]);

    let (response, description) = state.poll((TransportMessage::OpenSyn(open_syn), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_none());
    assert_eq!(state, original);
}

/// Proof: WaitingInitAck rejects OpenAck (wrong message type) → (None, None), state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn waiting_init_ack_rejects_open_ack() {
    let mine = any_zid();

    let mut state = make_waiting_init_ack_state(mine);
    let original = state;
    let ack = make_open_ack();

    let (response, description) = state.poll((TransportMessage::OpenAck(ack), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_none());
    assert_eq!(state, original);
}

// ============================
// TRANSITION: WaitingOpenSyn
// ============================

/// Proof: WaitingOpenSyn rejects InitSyn (wrong message type) → (None, None), state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn waiting_open_syn_rejects_init_syn() {
    let mine = any_zid();
    let other = any_zid();

    let mut state = make_waiting_open_syn_state(mine);
    let original = state;
    let syn = make_peer_init_syn(other);

    let (response, description) = state.poll((TransportMessage::InitSyn(syn), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_none());
    assert_eq!(state, original);
}

/// Proof: WaitingOpenSyn rejects OpenAck (wrong message type) → (None, None), state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn waiting_open_syn_rejects_open_ack() {
    let mine = any_zid();

    let mut state = make_waiting_open_syn_state(mine);
    let original = state;
    let ack = make_open_ack();

    let (response, description) = state.poll((TransportMessage::OpenAck(ack), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_none());
    assert_eq!(state, original);
}

// ============================
// TRANSITION: WaitingOpenAck
// ============================

/// Proof: WaitingOpenAck rejects InitSyn (wrong message type) → (None, None), state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn waiting_open_ack_rejects_init_syn() {
    let mine = any_zid();
    let other = any_zid();

    let mut state = make_waiting_open_ack_state(mine, other);
    let original = state;
    let syn = make_peer_init_syn(other);

    let (response, description) = state.poll((TransportMessage::InitSyn(syn), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_none());
    assert_eq!(state, original);
}

/// Proof: WaitingOpenAck rejects InitAck (wrong message type) → (None, None), state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn waiting_open_ack_rejects_init_ack() {
    let mine = any_zid();
    let other = any_zid();

    let mut state = make_waiting_open_ack_state(mine, other);
    let original = state;
    let ack = make_peer_init_ack(other);

    let (response, description) = state.poll((TransportMessage::InitAck(ack), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_none());
    assert_eq!(state, original);
}

/// Proof: WaitingOpenAck rejects OpenSyn (wrong message type) → (None, None), state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn waiting_open_ack_rejects_open_syn() {
    let mine = any_zid();
    let other = any_zid();

    let mut state = make_waiting_open_ack_state(mine, other);
    let original = state;
    let open_syn = make_open_syn(&[]);

    let (response, description) = state.poll((TransportMessage::OpenSyn(open_syn), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_none());
    assert_eq!(state, original);
}

/// Proof: WaitingOpenAck with OpenAck → Opened + (None, Some(Description))
#[kani::proof]
#[kani::unwind(32)]
fn waiting_open_ack_with_open_ack() {
    let mine = any_zid();
    let other = any_zid();

    let mut state = make_waiting_open_ack_state(mine, other);
    let ack = make_open_ack();

    let (response, description) = state.poll((TransportMessage::OpenAck(ack), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_some());
    assert!(matches!(state, State::Opened(_)));
}

// ============================
// TRANSITION: Opened
// ============================

/// Proof: Opened state returns (None, Some(Description)) for Close message — state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn opened_state_returns_description_for_close() {
    let mine = any_zid();
    let other = any_zid();

    let mut state = make_opened_state(mine, other);

    let close = Close {
        reason: 0,
        behaviour: CloseBehaviour::Session,
    };
    let (response, description) = state.poll((TransportMessage::Close(close), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_some());
    assert!(matches!(state, State::Opened(_)));
}

/// Proof: Opened state returns (None, Some(Description)) for InitSyn — state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn opened_state_returns_description_for_init_syn() {
    let mine = any_zid();
    let other = any_zid();

    let mut state = make_opened_state(mine, other);

    let syn = make_peer_init_syn(other);
    let (response, description) = state.poll((TransportMessage::InitSyn(syn), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_some());
    assert!(matches!(state, State::Opened(_)));
}

/// Proof: Opened state returns (None, Some(Description)) for OpenSyn — state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn opened_state_returns_description_for_open_syn() {
    let mine = any_zid();
    let other = any_zid();

    let mut state = make_opened_state(mine, other);

    let open_syn = make_open_syn(&[]);
    let (response, description) = state.poll((TransportMessage::OpenSyn(open_syn), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_some());
    assert!(matches!(state, State::Opened(_)));
}

/// Proof: Opened state returns (None, Some(Description)) for OpenAck — state unchanged
#[kani::proof]
#[kani::unwind(32)]
fn opened_state_returns_description_for_open_ack() {
    let mine = any_zid();
    let other = any_zid();

    let mut state = make_opened_state(mine, other);

    let open_ack = make_open_ack();
    let (response, description) = state.poll((TransportMessage::OpenAck(open_ack), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_some());
    assert!(matches!(state, State::Opened(_)));
}

// ============================
// EDGE CASES
// ============================

/// Edge case: Equal ZIDs in simultaneous open → (None, None) with InvalidAttribute error
#[kani::proof]
#[kani::unwind(32)]
fn simultaneous_open_equal_zid_returns_none() {
    let zid = any_zid();

    let mut state = make_waiting_init_ack_state(zid);
    let init_syn = make_peer_init_syn(zid);

    let (response, description) = state.poll((TransportMessage::InitSyn(init_syn), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_none());
    // State unchanged for equal ZIDs
    assert!(matches!(state, State::WaitingInitAck { .. }));
}

/// Edge case: All-zero ZID (invalid) should error when creating state
/// Note: any_zid() already assumes non-zero bytes, so this tests that TryFrom rejects zeros
#[kani::proof]
#[kani::unwind(4)]
fn invalid_zid_all_zeros_rejected() {
    let bytes = [0u8; 16];
    let result = ZenohIdProto::try_from(&bytes[..]);
    assert!(result.is_err());
}

/// Edge case: WaitingInitAck with non-Peer WhatAmI should error on InitSyn
#[kani::proof]
#[kani::unwind(32)]
fn waiting_init_ack_non_peer_whatami_ignores_init_syn() {
    let mine = any_zid();
    let other = any_zid();

    let mut state = State::WaitingInitAck {
        mine_zid: mine,
        mine_whatami: WhatAmI::Client,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    };
    let original = state;
    let init_syn = InitSyn {
        version: zenoh_proto::VERSION,
        identifier: InitIdentifier {
            zid: other,
            whatami: WhatAmI::Client,
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
    };

    let (response, description) = state.poll((TransportMessage::InitSyn(init_syn), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_none());
    assert_eq!(state, original);
}

/// Edge case: OpenSyn with invalid (empty) cookie in WaitingOpenSyn → error handled gracefully (returns None, None)
#[kani::proof]
#[kani::unwind(32)]
fn open_syn_invalid_cookie_handled_gracefully() {
    let mine = any_zid();

    let mut state = make_waiting_open_syn_state(mine);
    let original = state;
    let open_syn = make_open_syn(&[]);

    let (response, description) = state.poll((TransportMessage::OpenSyn(open_syn), &[] as &[u8]));
    assert!(response.is_none());
    assert!(description.is_none());
    assert_eq!(state, original);
}