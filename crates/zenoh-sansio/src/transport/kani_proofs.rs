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

fn make_waiting_init_ack_state(zid: ZenohIdProto) -> State {
    State::WaitingInitAck {
        mine_zid: zid,
        mine_whatami: WhatAmI::Peer,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
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