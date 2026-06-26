#[cfg(kani)]
#[kani::proof]
#[kani::unwind(4)]
fn simultaneous_open_proof() {
    use core::time::Duration;
    use zenoh_proto::{
        fields::*,
        msgs::*,
    };

    use crate::transport::establishment::State;

    // Use concrete ZIDs: 16-byte fixed arrays always succeed
    if let (Ok(zid), Ok(lower_zid), Ok(higher_zid)) = (
        ZenohIdProto::try_from(&[0u8; 16][..]),
        ZenohIdProto::try_from(&[1u8; 16][..]),
        ZenohIdProto::try_from(&[2u8; 16][..]),
    ) {

    // Scenario 1: lower ZID receives InitSyn from higher ZID → wins, returns (None, None)
    let mut state_lower = State::WaitingInitAck {
        mine_zid: lower_zid,
        mine_whatami: WhatAmI::Peer,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    };

    let init_higher = InitSyn {
        identifier: InitIdentifier {
            zid: higher_zid,
            whatami: WhatAmI::Peer,
        },
        resolution: InitResolution {
            resolution: Resolution::default(),
            batch_size: BatchSize(512),
        },
        ..Default::default()
    };

    let (resp_lower_wins, _) = state_lower.poll((
        TransportMessage::InitSyn(init_higher),
        &[] as &[u8],
    ));
    // Lower ZID wins → returns None (no InitAck)
    assert!(resp_lower_wins.is_none());

    // Scenario 2: higher ZID receives InitSyn from lower ZID → yields InitAck
    let mut state_higher = State::WaitingInitAck {
        mine_zid: higher_zid,
        mine_whatami: WhatAmI::Peer,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    };

    let init_lower = InitSyn {
        identifier: InitIdentifier {
            zid: lower_zid,
            whatami: WhatAmI::Peer,
        },
        resolution: InitResolution {
            resolution: Resolution::default(),
            batch_size: BatchSize(512),
        },
        ..Default::default()
    };

    let (resp_higher_yields, _) = state_higher.poll((
        TransportMessage::InitSyn(init_lower),
        &[] as &[u8],
    ));
    // Higher ZID yields → returns Some(InitAck)
    assert!(resp_higher_yields.is_some());
    }
}
