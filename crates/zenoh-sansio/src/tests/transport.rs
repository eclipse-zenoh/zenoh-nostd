use crate::{Transport, ZTransportRx, ZTransportTx, transport::establishment::State};
use core::{cell::RefCell, time::Duration};
use zenoh_proto::{exts::*, fields::*, keyexpr, msgs::*};

fn zid(val: u128) -> ZenohIdProto {
    ZenohIdProto::try_from(&val.to_le_bytes()[..]).unwrap()
}

#[test]
fn transport_state_handshake() {
    let a_zid = ZenohIdProto::default();
    let mut a = State::WaitingInitSyn {
        mine_zid: a_zid,
        mine_whatami: WhatAmI::Client,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    };

    let b_zid = ZenohIdProto::default();
    let mut b = State::WaitingInitAck {
        mine_zid: b_zid,
        mine_whatami: WhatAmI::Client,
        mine_batch_size: 1025,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(37),
    };

    let init = InitSyn {
        identifier: InitIdentifier {
            zid: b_zid,
            ..Default::default()
        },
        resolution: InitResolution {
            resolution: Resolution::default(),
            batch_size: BatchSize(1025),
        },
        ..Default::default()
    };

    let mut buff = [0u8; 128];

    macro_rules! buff {
        ($msg:expr) => {{
            let mut writer = &mut buff[..];
            <InitSyn as zenoh_proto::ZEncode>::z_encode($msg, &mut writer).unwrap();
            let len = 128 - writer.len();

            &buff[..len]
        }};
    }

    let mut buff = buff!(&init);
    let mut next = Some(TransportMessage::InitSyn(init));
    let mut desc = None;
    let mut current = &mut a;
    let mut other = &mut b;

    for _ in 0..4 {
        if let Some(response) = next {
            (next, desc) = current.poll((response, buff));
            core::mem::swap(&mut current, &mut other);

            buff = &[];
        }
    }

    assert!(desc.is_some());
    assert!(a.description().is_some() && b.description().is_some());
    assert_eq!(desc.unwrap().batch_size, 512);
    assert_eq!(desc.unwrap().resolution, Resolution::default());
}

#[test]
fn transport_peer_handshake() {
    let socket = ([0u8; 512], 0usize, 0usize);
    let socket_ref = RefCell::new(socket);

    let a = Transport::builder([0u8; 512]).with_whatami(WhatAmI::Peer);
    let b = Transport::builder([0u8; 512]).with_whatami(WhatAmI::Peer);

    let read = |socket: &mut &RefCell<([u8; 512], usize, usize)>,
                bytes: &mut [u8]|
     -> core::result::Result<usize, i32> {
        let mut borrow_mut = socket.borrow_mut();

        let remaining = borrow_mut.2 - borrow_mut.1;
        if remaining == 0 {
            return Ok(0);
        }

        let to_read = bytes.len().min(remaining);

        let slice = &borrow_mut.0[borrow_mut.1..(to_read + borrow_mut.1)];
        bytes[..slice.len()].copy_from_slice(slice);
        borrow_mut.1 += to_read;

        Ok(to_read)
    };

    let write = |socket: &mut &RefCell<([u8; 512], usize, usize)>,
                 bytes: &[u8]|
     -> core::result::Result<(), i32> {
        let mut borrow_mut = socket.borrow_mut();
        borrow_mut.0[..bytes.len()].copy_from_slice(bytes);
        borrow_mut.1 = 0;
        borrow_mut.2 = bytes.len();
        Ok(())
    };

    let mut ha = a.listen(&socket_ref, &read, &write);
    let mut hb = b.connect(&socket_ref, &read, &write);

    hb.poll().unwrap();

    for _ in 0..2 {
        ha.poll().unwrap();
        hb.poll().unwrap();
    }

    let ta = ha
        .poll()
        .expect("Unexpected Error")
        .expect("Transport A is not opened yet")
        .open();

    let tb = hb
        .poll()
        .expect("Unexpected Error")
        .expect("Transport B is not opened yet")
        .open();

    assert_eq!(ta.mine_zid, tb.other_zid);
    assert_eq!(ta.other_zid, tb.mine_zid);
}

#[test]
fn transport_peer_simultaneous_connect_lower_wins() {
    let socket = ([0u8; 512], 0usize, 0usize);
    let socket_ref = RefCell::new(socket);

    let a = Transport::builder([0u8; 512])
        .with_whatami(WhatAmI::Peer)
        .with_zid(zid(2));
    let b = Transport::builder([0u8; 512])
        .with_whatami(WhatAmI::Peer)
        .with_zid(zid(1));

    let read = |socket: &mut &RefCell<([u8; 512], usize, usize)>,
                bytes: &mut [u8]|
     -> core::result::Result<usize, i32> {
        let mut borrow_mut = socket.borrow_mut();
        let remaining = borrow_mut.2 - borrow_mut.1;
        if remaining == 0 {
            return Ok(0);
        }
        let to_read = bytes.len().min(remaining);
        let slice = &borrow_mut.0[borrow_mut.1..(to_read + borrow_mut.1)];
        bytes[..slice.len()].copy_from_slice(slice);
        borrow_mut.1 += to_read;
        Ok(to_read)
    };

    let write = |socket: &mut &RefCell<([u8; 512], usize, usize)>,
                 bytes: &[u8]|
     -> core::result::Result<(), i32> {
        let mut borrow_mut = socket.borrow_mut();
        borrow_mut.0[..bytes.len()].copy_from_slice(bytes);
        borrow_mut.1 = 0;
        borrow_mut.2 = bytes.len();
        Ok(())
    };

    let mut ha = a.connect(&socket_ref, &read, &write);
    let mut hb = b.connect(&socket_ref, &read, &write);

    ha.poll().unwrap();
    hb.poll().unwrap();

    for _ in 0..5 {
        ha.poll().unwrap();
        hb.poll().unwrap();
    }

    let ta = ha
        .poll()
        .expect("Unexpected Error")
        .expect("Transport A is not opened yet")
        .open();

    let tb = hb
        .poll()
        .expect("Unexpected Error")
        .expect("Transport B is not opened yet")
        .open();

    assert_eq!(ta.mine_zid, tb.other_zid);
    assert_eq!(ta.other_zid, tb.mine_zid);
}

#[test]
fn transport_handshake() {
    let socket = ([0u8; 512], 0usize, 0usize);
    let socket_ref = RefCell::new(socket);

    let a = Transport::builder([0u8; 512]);
    let b = Transport::builder([0u8; 512]);

    let read = |socket: &mut &RefCell<([u8; 512], usize, usize)>,
                bytes: &mut [u8]|
     -> core::result::Result<usize, i32> {
        let mut borrow_mut = socket.borrow_mut();

        let to_read = bytes.len().min(borrow_mut.2);

        let slice = &borrow_mut.0[borrow_mut.1..(to_read + borrow_mut.1)];
        bytes[..slice.len()].copy_from_slice(slice);
        borrow_mut.1 += to_read;

        Ok(to_read)
    };

    let write = |socket: &mut &RefCell<([u8; 512], usize, usize)>,
                 bytes: &[u8]|
     -> core::result::Result<(), i32> {
        let mut borrow_mut = socket.borrow_mut();
        borrow_mut.0[..bytes.len()].copy_from_slice(bytes);
        borrow_mut.1 = 0;
        borrow_mut.2 = bytes.len();
        Ok(())
    };

    let mut ha = a.listen(&socket_ref, &read, &write);
    let mut hb = b.connect(&socket_ref, &read, &write);

    hb.poll().unwrap();

    for _ in 0..2 {
        ha.poll().unwrap();
        hb.poll().unwrap();
    }

    ha.poll()
        .expect("Unexpected Error")
        .expect("Transport A is not opened yet")
        .open();

    hb.poll()
        .expect("Unexpected Error")
        .expect("Transport B is not opened yet")
        .open();
}

#[test]
fn transport_handshake_streamed() {
    let socket = ([0u8; 512], 0usize, 0usize);
    let socket_ref = RefCell::new(socket);

    let a = Transport::builder([0u8; 512]);
    let b = Transport::builder([0u8; 512]);

    let read = |socket: &mut &RefCell<([u8; 512], usize, usize)>,
                bytes: &mut [u8]|
     -> core::result::Result<usize, i32> {
        let mut borrow_mut = socket.borrow_mut();

        let to_read = bytes.len().min(borrow_mut.2);

        let slice = &borrow_mut.0[borrow_mut.1..(to_read + borrow_mut.1)];
        bytes[..slice.len()].copy_from_slice(slice);
        borrow_mut.1 += to_read;

        Ok(to_read)
    };

    let write = |socket: &mut &RefCell<([u8; 512], usize, usize)>,
                 bytes: &[u8]|
     -> core::result::Result<(), i32> {
        let mut borrow_mut = socket.borrow_mut();
        borrow_mut.0[..bytes.len()].copy_from_slice(bytes);
        borrow_mut.1 = 0;
        borrow_mut.2 = bytes.len();
        Ok(())
    };

    let mut ha = a.listen(&socket_ref, &read, &write).prefixed();
    let mut hb = b.connect(&socket_ref, &read, &write).prefixed();

    hb.poll().unwrap();

    for _ in 0..2 {
        ha.poll().unwrap();
        hb.poll().unwrap();
    }

    ha.poll()
        .expect("Unexpected Error")
        .expect("Transport A is not opened yet")
        .open();

    hb.poll()
        .expect("Unexpected Error")
        .expect("Transport B is not opened yet")
        .open();
}

#[test]
fn transport_streamed_codec() {
    let mut transport = Transport::builder([0u8; 512]).codec();

    let msg = NetworkMessage {
        reliability: Reliability::Reliable,
        qos: QoS::declare(),
        body: NetworkBody::Push(Push {
            wire_expr: WireExpr::from(keyexpr::from_str_unchecked("abc/def")),
            payload: PushBody::Put(Put {
                payload: &[1, 2, 3, 4],
                ..Default::default()
            }),
            ..Default::default()
        }),
    };

    transport.tx.encode_ref(core::iter::once(msg.as_ref()));
    transport
        .rx
        .decode_prefixed(transport.tx.flush_prefixed().unwrap())
        .unwrap();

    let mut flush = transport.rx.flush();
    let m = flush.next().unwrap().0;

    assert_eq!(flush.count(), 0);
    assert_eq!(m, msg);
}

#[test]
fn transport_builder_peer_initsyn_has_peer_whatami() {
    let socket = ([0u8; 512], 0usize, 0usize);
    let socket_ref = RefCell::new(socket);

    let a = Transport::builder([0u8; 512]).with_whatami(WhatAmI::Peer);

    let write = |socket: &mut &RefCell<([u8; 512], usize, usize)>,
                 bytes: &[u8]|
     -> core::result::Result<(), i32> {
        let mut borrow_mut = socket.borrow_mut();
        borrow_mut.0[..bytes.len()].copy_from_slice(bytes);
        borrow_mut.1 = 0;
        borrow_mut.2 = bytes.len();
        Ok(())
    };

    let mut h = a.connect(&socket_ref, |_, _| Ok(0usize), write);
    h.poll().unwrap();

    let whatami = {
        let borrow = socket_ref.borrow();
        <InitSyn as zenoh_proto::ZDecode>::z_decode(&mut &borrow.0[..borrow.2])
            .unwrap()
            .identifier
            .whatami
    };

    assert_eq!(whatami, WhatAmI::Peer);
}

#[test]
fn transport_peer_simultaneous_connect_equal_zid_errors() {
    let zid = ZenohIdProto::default();

    let mut a = State::WaitingInitAck {
        mine_zid: zid,
        mine_whatami: WhatAmI::Peer,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    };

    let init = InitSyn {
        identifier: InitIdentifier {
            zid,
            whatami: WhatAmI::Peer,
        },
        ..Default::default()
    };

    let mut buff = [0u8; 128];
    let mut writer = &mut buff[..];
    <InitSyn as zenoh_proto::ZEncode>::z_encode(&init, &mut writer).unwrap();
    let len = 128 - writer.len();
    let encoded = &buff[..len];

    let (response, desc) = a.poll((TransportMessage::InitSyn(init), encoded));

    assert!(response.is_none(), "expected no response for equal ZIDs");
    assert!(desc.is_none(), "expected no description for equal ZIDs");
}

#[test]
fn transport_peer_simultaneous_connect_lower_zid_wins() {
    let higher_zid = zid(2);
    let lower_zid = zid(1);

    let mut a = State::WaitingInitAck {
        mine_zid: lower_zid,
        mine_whatami: WhatAmI::Peer,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    };

    let init = InitSyn {
        identifier: InitIdentifier {
            zid: higher_zid,
            whatami: WhatAmI::Peer,
        },
        ..Default::default()
    };

    let mut buff = [0u8; 128];
    let mut writer = &mut buff[..];
    <InitSyn as zenoh_proto::ZEncode>::z_encode(&init, &mut writer).unwrap();
    let len = 128 - writer.len();
    let encoded = &buff[..len];

    let (response, desc) = a.poll((TransportMessage::InitSyn(init), encoded));

    assert!(response.is_none(), "lower ZID should not yield, expected no response");
    assert!(desc.is_none(), "expected no description while waiting for InitAck");
}

#[test]
fn transport_peer_simultaneous_connect_higher_zid_yields() {
    let higher_zid = zid(2);
    let lower_zid = zid(1);

    let mut a = State::WaitingInitAck {
        mine_zid: higher_zid,
        mine_whatami: WhatAmI::Peer,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
    };

    let init = InitSyn {
        identifier: InitIdentifier {
            zid: lower_zid,
            whatami: WhatAmI::Peer,
        },
        ..Default::default()
    };

    let mut buff = [0u8; 128];
    let mut writer = &mut buff[..];
    <InitSyn as zenoh_proto::ZEncode>::z_encode(&init, &mut writer).unwrap();
    let len = 128 - writer.len();
    let encoded = &buff[..len];

    let (response, desc) = a.poll((TransportMessage::InitSyn(init), encoded));

    assert!(response.is_some(), "higher ZID should yield with InitAck");
    assert!(desc.is_none(), "description only set after OpenSyn/OpenAck");
}

#[cfg(feature = "std")]
#[cfg(test)]
mod proptests {
    use core::time::Duration;
    use proptest::prelude::*;
    use zenoh_proto::{fields::*, msgs::*};

    use crate::transport::establishment::State;

    /// Property: ZID comparison for simultaneous-open is deterministic and
    /// consistent. For any two ZIDs, one must win and the other must yield
    /// (or equal ZIDs produce no response from either side).
    proptest! {
        #[test]
        fn zid_comparison_deterministic(a: u128, b: u128) {
            let zid_a = ZenohIdProto::try_from(&a.to_le_bytes()[..]).unwrap();
            let zid_b = ZenohIdProto::try_from(&b.to_le_bytes()[..]).unwrap();

            // Poll with A as mine, B as theirs
            let mut state_a = State::WaitingInitAck {
                mine_zid: zid_a,
                mine_whatami: WhatAmI::Peer,
                mine_batch_size: 512,
                mine_resolution: Resolution::default(),
                mine_lease: Duration::from_secs(30),
            };
            let init_b = InitSyn {
                identifier: InitIdentifier { zid: zid_b, whatami: WhatAmI::Peer },
                resolution: InitResolution { resolution: Resolution::default(), batch_size: BatchSize(512) },
                ..Default::default()
            };
            let (resp_a, _) = state_a.poll((TransportMessage::InitSyn(init_b), &[] as &[u8]));

            // Poll with B as mine, A as theirs
            let mut state_b = State::WaitingInitAck {
                mine_zid: zid_b,
                mine_whatami: WhatAmI::Peer,
                mine_batch_size: 512,
                mine_resolution: Resolution::default(),
                mine_lease: Duration::from_secs(30),
            };
            let init_a = InitSyn {
                identifier: InitIdentifier { zid: zid_a, whatami: WhatAmI::Peer },
                resolution: InitResolution { resolution: Resolution::default(), batch_size: BatchSize(512) },
                ..Default::default()
            };
            let (resp_b, _) = state_b.poll((TransportMessage::InitSyn(init_a), &[] as &[u8]));

            // One must yield (Some), the other must win (None), or both None for equal ZIDs
            if zid_a == zid_b {
                prop_assert!(resp_a.is_none() && resp_b.is_none(), "equal ZIDs: both should produce None");
            } else {
                prop_assert!(resp_a.is_some() || resp_b.is_some(), "one side must yield");
                prop_assert!(resp_a.is_none() || resp_b.is_none(), "one side must win");
                // Verify consistency: same side wins every time
                if resp_a.is_some() {
                    // A yielded → A's ZID was higher than B's
                    let mut state_a2 = State::WaitingInitAck {
                        mine_zid: zid_a,
                        mine_whatami: WhatAmI::Peer,
                        mine_batch_size: 512,
                        mine_resolution: Resolution::default(),
                        mine_lease: Duration::from_secs(30),
                    };
                    let init_b2 = InitSyn {
                        identifier: InitIdentifier { zid: zid_b, whatami: WhatAmI::Peer },
                        resolution: InitResolution { resolution: Resolution::default(), batch_size: BatchSize(512) },
                        ..Default::default()
                    };
                    let (resp_a2, _) = state_a2.poll((TransportMessage::InitSyn(init_b2), &[] as &[u8]));
                    prop_assert_eq!(resp_a.is_some(), resp_a2.is_some(), "comparison must be deterministic");
                }
            }
        }
    }
}
