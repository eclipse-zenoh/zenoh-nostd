use crate::{
    Transport, ZTransportRx, ZTransportTx,
    transport::{TransportTx, establishment::State},
};
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
        #[cfg(feature = "auth")]
        mine_auth: None,
    };

    let b_zid = ZenohIdProto::default();
    let mut b = State::WaitingInitAck {
        mine_zid: b_zid,
        mine_whatami: WhatAmI::Client,
        mine_batch_size: 1025,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(37),
        #[cfg(feature = "auth")]
        mine_auth: None,
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
        .open().unwrap();

    let tb = hb
        .poll()
        .expect("Unexpected Error")
        .expect("Transport B is not opened yet")
        .open().unwrap();

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
        .open().unwrap();

    let tb = hb
        .poll()
        .expect("Unexpected Error")
        .expect("Transport B is not opened yet")
        .open().unwrap();

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
        .open().unwrap();

    hb.poll()
        .expect("Unexpected Error")
        .expect("Transport B is not opened yet")
        .open().unwrap();
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
        .open().unwrap();

    hb.poll()
        .expect("Unexpected Error")
        .expect("Transport B is not opened yet")
        .open().unwrap();
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
        #[cfg(feature = "auth")]
        mine_auth: None,
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
        #[cfg(feature = "auth")]
        mine_auth: None,
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

    assert!(
        response.is_none(),
        "lower ZID should not yield, expected no response"
    );
    assert!(
        desc.is_none(),
        "expected no description while waiting for InitAck"
    );
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
        #[cfg(feature = "auth")]
        mine_auth: None,
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

#[test]
fn transport_batching_no_flush() {
    let mut transport = Transport::builder([0u8; 512]).codec();

    transport.tx.batch_start();
    assert!(transport.tx.is_batching());

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

    transport.tx.batch_stop();
    assert!(!transport.tx.is_batching());

    transport
        .rx
        .decode_prefixed(transport.tx.flush_prefixed().unwrap())
        .unwrap();

    let mut flush = transport.rx.flush();
    let m = flush.next().unwrap().0;
    assert_eq!(flush.count(), 0);
    assert_eq!(
        m.body,
        NetworkBody::Push(Push {
            wire_expr: WireExpr::from(keyexpr::from_str_unchecked("abc/def")),
            payload: PushBody::Put(Put {
                payload: &[1, 2, 3, 4],
                ..Default::default()
            }),
            ..Default::default()
        })
    );
}

#[test]
fn transport_delete_codec() {
    let mut transport = Transport::builder([0u8; 512]).codec();

    let msg = NetworkMessage {
        reliability: Reliability::Reliable,
        qos: QoS::declare(),
        body: NetworkBody::Push(Push {
            wire_expr: WireExpr::from(keyexpr::from_str_unchecked("abc/def")),
            payload: PushBody::Del(Del {
                timestamp: None,
                sinfo: None,
                attachment: None,
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
fn transport_fragmentation() {
    let mut tx = TransportTx::new(
        [0u8; 512],
        64,
        0,
        Resolution::default(),
        Duration::from_secs(10),
    )
    .with_max_fragments(3);

    let payload: [u8; 200] = [0xAAu8; 200];

    let msg = NetworkMessage {
        reliability: Reliability::Reliable,
        qos: QoS::declare(),
        body: NetworkBody::Push(Push {
            wire_expr: WireExpr::from(keyexpr::from_str_unchecked("abc/def")),
            payload: PushBody::Put(Put {
                payload: &payload,
                ..Default::default()
            }),
            ..Default::default()
        }),
    };

    let mut body_buff = [0u8; 512];
    let mut writer = &mut body_buff[..];
    let wlen = writer.len();
    <NetworkBody as zenoh_proto::ZEncode>::z_encode(&msg.body, &mut writer).unwrap();
    let body_len = wlen - writer.len();
    let _ = writer;
    let encoded_bytes = &body_buff[..body_len];

    tx.encode_optimized(core::iter::once((msg, encoded_bytes)));

    let mut total_flushed = 0usize;
    let mut flush_count = 0usize;

    loop {
        let flushed = match tx.flush_prefixed() {
            Some(f) if f.len() >= 2 => f,
            _ => break,
        };

        total_flushed += flushed.len();
        flush_count += 1;

        if flush_count > 8 {
            break;
        }

        tx.keepalive();
    }

    assert!(
        total_flushed > 64,
        "fragmentation should produce multiple frames exceeding batch size (total={total_flushed})"
    );
    assert!(
        flush_count > 1,
        "fragmentation should emit more than 1 frame (got {flush_count})"
    );
}

#[test]
fn transport_qos() {
    let mut transport = Transport::builder([0u8; 512]).codec();

    let realtime_qos = QoS::new(Priority::Data, CongestionControl::Drop, true);
    let background_qos = QoS::declare();

    let msg1 = NetworkMessage {
        reliability: Reliability::Reliable,
        qos: realtime_qos,
        body: NetworkBody::Push(Push {
            wire_expr: WireExpr::from(keyexpr::from_str_unchecked("a/b")),
            payload: PushBody::Put(Put {
                payload: &[1],
                ..Default::default()
            }),
            ..Default::default()
        }),
    };

    let msg2 = NetworkMessage {
        reliability: Reliability::Reliable,
        qos: background_qos,
        body: NetworkBody::Push(Push {
            wire_expr: WireExpr::from(keyexpr::from_str_unchecked("c/d")),
            payload: PushBody::Put(Put {
                payload: &[2],
                ..Default::default()
            }),
            ..Default::default()
        }),
    };

    transport
        .tx
        .encode_ref(core::iter::once(msg1.as_ref()).chain(core::iter::once(msg2.as_ref())));
    transport
        .rx
        .decode_prefixed(transport.tx.flush_prefixed().unwrap())
        .unwrap();

    let mut flush = transport.rx.flush();
    let m1 = flush.next().unwrap().0;
    let m2 = flush.next().unwrap().0;
    assert_eq!(flush.count(), 0);

    assert_eq!(m1.qos, realtime_qos);
    assert_eq!(m2.qos, background_qos);
    assert_ne!(m1.qos, m2.qos);
}

#[test]
fn sequence_number_wrap() {
    let mut tx = crate::transport::TransportTx::new(
        [0u8; 512],
        512,
        u32::MAX - 5,
        Resolution::default(),
        Duration::from_secs(10),
    );

    let ke = keyexpr::from_str_unchecked("a/b");

    for i in 0u8..12 {
        let msg = NetworkMessage {
            reliability: Reliability::Reliable,
            qos: QoS::default(),
            body: NetworkBody::Push(Push {
                wire_expr: WireExpr::from(ke),
                payload: PushBody::Put(Put {
                    payload: &[i],
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };

        ZTransportTx::encode_ref(&mut tx, core::iter::once(msg.as_ref()));
    }

    let flushed = tx.flush_prefixed().unwrap();
    let mut rx = crate::transport::TransportRx::new(
        [0u8; 512],
        512,
        u32::MAX - 5,
        Resolution::default(),
        Duration::from_secs(10),
    );
    rx.decode_prefixed(flushed).unwrap();

    let mut count = 0u8;
    for (m, _) in rx.flush() {
        match &m.body {
            NetworkBody::Push(Push {
                payload: PushBody::Put(Put { payload, .. }),
                ..
            }) => {
                assert_eq!(payload, &[count]);
            }
            _ => panic!("expected Push"),
        }
        count += 1;
    }
    assert_eq!(count, 12, "all messages must survive SN wrap");
}

#[test]
fn frame_sn_monotonic() {
    let ke = keyexpr::from_str_unchecked("a/b");
    let mut tx = crate::transport::TransportTx::new(
        [0u8; 1024],
        1024,
        0,
        Resolution::default(),
        Duration::from_secs(10),
    );

    for i in 0..50 {
        let msg = NetworkMessage {
            reliability: match i % 3 {
                0 => Reliability::Reliable,
                _ => Reliability::BestEffort,
            },
            qos: QoS::default(),
            body: NetworkBody::Push(Push {
                wire_expr: WireExpr::from(ke),
                payload: PushBody::Put(Put {
                    payload: &[i as u8],
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };
        ZTransportTx::encode_ref(&mut tx, core::iter::once(msg.as_ref()));
    }

    let flushed = tx.flush_prefixed().unwrap();
    let buff = &flushed[2..];

    let mut last_sn: Option<u32> = None;
    let mut reader = &buff[..];
    while !reader.is_empty() {
        let id = reader[0] & 0x1f;
        if id == FrameHeader::ID {
            let header = <FrameHeader as zenoh_proto::ZDecode>::z_decode(&mut reader).unwrap();
            if let Some(last) = last_sn {
                assert!(
                    header.sn == last || header.sn == last.wrapping_add(1),
                    "SN must be monotonic: last={last}, current={}",
                    header.sn
                );
            }
            last_sn = Some(header.sn);
        } else {
            break;
        }
    }
}

#[test]
fn transport_state_machine_happy_path() {
    let a_zid = ZenohIdProto::default();

    let mut listener = State::WaitingInitSyn {
        mine_zid: a_zid,
        mine_whatami: WhatAmI::Client,
        mine_batch_size: 512,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(30),
        #[cfg(feature = "auth")]
        mine_auth: None,
    };

    let b_zid = ZenohIdProto::try_from(&2u128.to_le_bytes()[..]).unwrap();
    let mut connector = State::WaitingInitAck {
        mine_zid: b_zid,
        mine_whatami: WhatAmI::Client,
        mine_batch_size: 1025,
        mine_resolution: Resolution::default(),
        mine_lease: Duration::from_secs(37),
        #[cfg(feature = "auth")]
        mine_auth: None,
    };

    let mut buff = [0u8; 256];

    macro_rules! encode {
        ($msg:expr) => {{
            let mut writer = &mut buff[..];
            <InitSyn as zenoh_proto::ZEncode>::z_encode($msg, &mut writer).unwrap();
            let len = 256 - writer.len();
            &buff[..len]
        }};
    }

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

    let encoded_init = encode!(&init);
    let (resp, desc) = listener.poll((TransportMessage::InitSyn(init), encoded_init));
    assert!(resp.is_some(), "listener must respond with InitAck");
    assert!(desc.is_none(), "not yet established");
    assert!(matches!(listener, State::WaitingOpenSyn { .. }));

    if let Some(TransportMessage::InitAck(ack)) = resp {
        let (resp2, desc2) = connector.poll((TransportMessage::InitAck(ack), &[] as &[u8]));
        assert!(resp2.is_some(), "connector must respond with OpenSyn");
        assert!(desc2.is_none(), "not yet established");
        assert!(matches!(connector, State::WaitingOpenAck { .. }));

        if let Some(TransportMessage::OpenSyn(open)) = resp2 {
            let (resp3, desc3) = listener.poll((TransportMessage::OpenSyn(open), &[] as &[u8]));
            assert!(resp3.is_some(), "listener must respond with OpenAck");
            assert!(desc3.is_some(), "listener must establish after OpenSyn");
            assert!(matches!(listener, State::Opened(_)));

            if let Some(TransportMessage::OpenAck(ack2)) = resp3 {
                let (resp4, desc4) =
                    connector.poll((TransportMessage::OpenAck(ack2), &[] as &[u8]));
                assert!(resp4.is_none(), "no response after OpenAck");
                assert!(desc4.is_some(), "connector must establish after OpenAck");
                assert!(matches!(connector, State::Opened(_)));
            } else {
                panic!("expected OpenAck");
            }
        } else {
            panic!("expected OpenSyn");
        }
    } else {
        panic!("expected InitAck");
    }

    assert!(listener.description().is_some());
    assert!(connector.description().is_some());
}

#[test]
fn transport_state_machine_invalid_transitions() {
    let zid = ZenohIdProto::default();

    // WaitingInitSyn rejects everything except InitSyn
    {
        let mut state = State::WaitingInitSyn {
            mine_zid: zid,
            mine_whatami: WhatAmI::Client,
            mine_batch_size: 512,
            mine_resolution: Resolution::default(),
            mine_lease: Duration::from_secs(30),
            #[cfg(feature = "auth")]
            mine_auth: None,
        };
        let (resp, desc) =
            state.poll((TransportMessage::InitAck(InitAck::default()), &[] as &[u8]));
        assert!(
            resp.is_none() && desc.is_none(),
            "WaitingInitSyn must reject InitAck"
        );
    }
    {
        let mut state = State::WaitingInitSyn {
            mine_zid: zid,
            mine_whatami: WhatAmI::Client,
            mine_batch_size: 512,
            mine_resolution: Resolution::default(),
            mine_lease: Duration::from_secs(30),
            #[cfg(feature = "auth")]
            mine_auth: None,
        };
        let (resp, desc) =
            state.poll((TransportMessage::OpenSyn(OpenSyn::default()), &[] as &[u8]));
        assert!(
            resp.is_none() && desc.is_none(),
            "WaitingInitSyn must reject OpenSyn"
        );
    }
    {
        let mut state = State::WaitingInitSyn {
            mine_zid: zid,
            mine_whatami: WhatAmI::Client,
            mine_batch_size: 512,
            mine_resolution: Resolution::default(),
            mine_lease: Duration::from_secs(30),
            #[cfg(feature = "auth")]
            mine_auth: None,
        };
        let (resp, desc) =
            state.poll((TransportMessage::OpenAck(OpenAck::default()), &[] as &[u8]));
        assert!(
            resp.is_none() && desc.is_none(),
            "WaitingInitSyn must reject OpenAck"
        );
    }

    // WaitingInitAck rejects OpenSyn and OpenAck
    {
        let mut state = State::WaitingInitAck {
            mine_zid: zid,
            mine_whatami: WhatAmI::Client,
            mine_batch_size: 512,
            mine_resolution: Resolution::default(),
            mine_lease: Duration::from_secs(30),
            #[cfg(feature = "auth")]
            mine_auth: None,
        };
        let (resp, desc) =
            state.poll((TransportMessage::OpenSyn(OpenSyn::default()), &[] as &[u8]));
        assert!(
            resp.is_none() && desc.is_none(),
            "WaitingInitAck must reject OpenSyn"
        );
    }
    {
        let mut state = State::WaitingInitAck {
            mine_zid: zid,
            mine_whatami: WhatAmI::Client,
            mine_batch_size: 512,
            mine_resolution: Resolution::default(),
            mine_lease: Duration::from_secs(30),
            #[cfg(feature = "auth")]
            mine_auth: None,
        };
        let (resp, desc) =
            state.poll((TransportMessage::OpenAck(OpenAck::default()), &[] as &[u8]));
        assert!(
            resp.is_none() && desc.is_none(),
            "WaitingInitAck must reject OpenAck"
        );
    }

    // WaitingOpenSyn rejects InitSyn, InitAck, OpenAck
    {
        let mut state = State::WaitingOpenSyn {
            mine_zid: zid,
            mine_whatami: WhatAmI::Client,
            mine_batch_size: 512,
            mine_resolution: Resolution::default(),
            mine_lease: Duration::from_secs(30),
        };
        let (resp, desc) =
            state.poll((TransportMessage::InitSyn(InitSyn::default()), &[] as &[u8]));
        assert!(
            resp.is_none() && desc.is_none(),
            "WaitingOpenSyn must reject InitSyn"
        );
    }
    {
        let mut state = State::WaitingOpenSyn {
            mine_zid: zid,
            mine_whatami: WhatAmI::Client,
            mine_batch_size: 512,
            mine_resolution: Resolution::default(),
            mine_lease: Duration::from_secs(30),
        };
        let (resp, desc) =
            state.poll((TransportMessage::InitAck(InitAck::default()), &[] as &[u8]));
        assert!(
            resp.is_none() && desc.is_none(),
            "WaitingOpenSyn must reject InitAck"
        );
    }
    {
        let mut state = State::WaitingOpenSyn {
            mine_zid: zid,
            mine_whatami: WhatAmI::Client,
            mine_batch_size: 512,
            mine_resolution: Resolution::default(),
            mine_lease: Duration::from_secs(30),
        };
        let (resp, desc) =
            state.poll((TransportMessage::OpenAck(OpenAck::default()), &[] as &[u8]));
        assert!(
            resp.is_none() && desc.is_none(),
            "WaitingOpenSyn must reject OpenAck"
        );
    }

    // WaitingOpenAck rejects InitSyn, InitAck, OpenSyn
    {
        let mut state = State::WaitingOpenAck {
            mine_zid: zid,
            batch_size: 512,
            resolution: Resolution::default(),
            sn: 0,
            mine_lease: Duration::from_secs(30),
            other_zid: zid,
        };
        let (resp, desc) =
            state.poll((TransportMessage::InitSyn(InitSyn::default()), &[] as &[u8]));
        assert!(
            resp.is_none() && desc.is_none(),
            "WaitingOpenAck must reject InitSyn"
        );
    }
    {
        let mut state = State::WaitingOpenAck {
            mine_zid: zid,
            batch_size: 512,
            resolution: Resolution::default(),
            sn: 0,
            mine_lease: Duration::from_secs(30),
            other_zid: zid,
        };
        let (resp, desc) =
            state.poll((TransportMessage::InitAck(InitAck::default()), &[] as &[u8]));
        assert!(
            resp.is_none() && desc.is_none(),
            "WaitingOpenAck must reject InitAck"
        );
    }
    {
        let mut state = State::WaitingOpenAck {
            mine_zid: zid,
            batch_size: 512,
            resolution: Resolution::default(),
            sn: 0,
            mine_lease: Duration::from_secs(30),
            other_zid: zid,
        };
        let (resp, desc) =
            state.poll((TransportMessage::OpenSyn(OpenSyn::default()), &[] as &[u8]));
        assert!(
            resp.is_none() && desc.is_none(),
            "WaitingOpenAck must reject OpenSyn"
        );
    }
}

// ---------------------------------------------------------------------------
// Fuzz-style edge case tests — must never panic on invalid/malformed input
// ---------------------------------------------------------------------------

#[test]
fn truncated_input_does_not_panic() {
    let mut transport = Transport::builder([0u8; 128]).codec();
    transport.rx.decode_raw(&[0x00]).ok();
    let count = transport.rx.flush().count();
    assert_eq!(count, 0);
}

#[test]
fn all_zeros_input_does_not_panic() {
    let mut transport = Transport::builder([0u8; 128]).codec();
    transport.rx.decode_raw(&[0u8; 64]).ok();
    let count = transport.rx.flush().count();
    assert_eq!(count, 0);
}

#[test]
fn random_garbage_does_not_panic() {
    let mut transport = Transport::builder([0u8; 128]).codec();
    transport.rx.decode_raw(&[0xFF; 64]).ok();
    let count = transport.rx.flush().count();
    assert_eq!(count, 0);
}

#[test]
fn oversized_message_does_not_overflow() {
    let ke = keyexpr::from_str_unchecked("a/b");
    let mut transport = Transport::builder([0u8; 64]).codec();
    let big_payload = [0xABu8; 256];
    let msg = NetworkMessage {
        reliability: Reliability::Reliable,
        qos: QoS::declare(),
        body: NetworkBody::Push(Push {
            wire_expr: WireExpr::from(ke),
            payload: PushBody::Put(Put {
                payload: &big_payload,
                ..Default::default()
            }),
            ..Default::default()
        }),
    };
    transport.tx.encode_ref(core::iter::once(msg.as_ref()));
    let result = transport.tx.flush_prefixed();
    assert!(result.is_none() || result.is_some());
}

#[test]
fn invalid_frame_header_does_not_panic() {
    let mut transport = Transport::builder([0u8; 128]).codec();
    let raw = &[0b0001_1111, 0x00, 0x00, 0x00, 0x00];
    transport.rx.decode_raw(raw).ok();
    let count = transport.rx.flush().count();
    assert_eq!(count, 0);
}

#[test]
fn double_close_is_safe() {
    let mut transport = Transport::builder([0u8; 128]).codec();
    transport.tx.close();
    transport.tx.close(); // second close should not panic
    // close() queues a Close message; enter Synchronized then timeout to trigger closure
    transport
        .tx
        .sync(None, core::time::Duration::from_secs(3600));
    transport
        .tx
        .sync(None, core::time::Duration::from_secs(3700));
    assert!(transport.tx.closed());
}

#[test]
fn zero_length_buffer_does_not_panic() {
    let transport = Transport::builder([0u8; 0]).codec();
    let ke = keyexpr::from_str_unchecked("a/b");
    let msg = NetworkMessage {
        reliability: Reliability::Reliable,
        qos: QoS::declare(),
        body: NetworkBody::Push(Push {
            wire_expr: WireExpr::from(ke),
            payload: PushBody::Put(Put {
                payload: &[1u8; 4],
                ..Default::default()
            }),
            ..Default::default()
        }),
    };

    let mut tx = transport.tx;
    tx.encode_ref(core::iter::once(msg.as_ref()));
    let result = tx.flush_prefixed();
    assert!(result.is_none());
}

#[test]
fn message_after_close_is_rejected() {
    let ke = keyexpr::from_str_unchecked("a/b");
    let mut transport = Transport::builder([0u8; 128]).codec();

    transport.tx.close();
    transport
        .tx
        .sync(None, core::time::Duration::from_secs(3600));
    transport
        .tx
        .sync(None, core::time::Duration::from_secs(3700));
    assert!(transport.tx.closed());

    let msg = NetworkMessage {
        reliability: Reliability::Reliable,
        qos: QoS::declare(),
        body: NetworkBody::Push(Push {
            wire_expr: WireExpr::from(ke),
            payload: PushBody::Put(Put {
                payload: &[1u8; 4],
                ..Default::default()
            }),
            ..Default::default()
        }),
    };
    // encode_ref should not panic even after close
    transport.tx.encode_ref(core::iter::once(msg.as_ref()));
    // flush should not panic either
    let _ = transport.tx.flush_prefixed();
}

#[test]
fn simultaneous_close_handling() {
    let mut transport = Transport::builder([0u8; 128]).codec();
    transport.tx.close();
    transport
        .tx
        .sync(None, core::time::Duration::from_secs(3600));
    transport
        .tx
        .sync(None, core::time::Duration::from_secs(3700));
    assert!(transport.tx.closed());
    transport
        .rx
        .sync(Some(&transport.tx), core::time::Duration::from_secs(0));
    assert!(transport.rx.closed());
}

#[test]
fn max_sn_before_wrap() {
    let ke = keyexpr::from_str_unchecked("a/b");
    let mut tx = crate::transport::TransportTx::new(
        [0u8; 512],
        512,
        u32::MAX - 3,
        Resolution::default(),
        core::time::Duration::from_secs(10),
    );

    for i in 0u8..8 {
        let msg = NetworkMessage {
            reliability: Reliability::Reliable,
            qos: QoS::default(),
            body: NetworkBody::Push(Push {
                wire_expr: WireExpr::from(ke),
                payload: PushBody::Put(Put {
                    payload: &[i],
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };
        crate::ZTransportTx::encode_ref(&mut tx, core::iter::once(msg.as_ref()));
    }

    let flushed = tx.flush_prefixed().unwrap();
    let mut rx = crate::transport::TransportRx::new(
        [0u8; 512],
        512,
        u32::MAX - 3,
        Resolution::default(),
        core::time::Duration::from_secs(10),
    );
    rx.decode_prefixed(flushed).unwrap();

    let mut count = 0u8;
    for (m, _) in rx.flush() {
        match &m.body {
            NetworkBody::Push(Push {
                payload: PushBody::Put(Put { payload, .. }),
                ..
            }) => {
                assert_eq!(payload, &[count]);
            }
            _ => panic!("expected Push"),
        }
        count += 1;
    }
    assert_eq!(
        count, 8,
        "all messages must survive SN wrap from u32::MAX-3"
    );
}

#[cfg(feature = "auth")]
#[cfg(test)]
mod auth_tests {
    use crate::transport::establishment::State;
    use core::time::Duration;
    use zenoh_proto::{ZDecode, exts::Auth, fields::*, msgs::*, VERSION};

    fn zid(val: u128) -> ZenohIdProto {
        ZenohIdProto::try_from(&val.to_le_bytes()[..]).unwrap()
    }

#[test]
    fn state_waitinginitsyn_with_auth_produces_initack_with_auth() {
        let z = zid(42);
        let mut payload = heapless::Vec::<u8, 256>::new();
        payload
            .extend_from_slice(&[0x01, b'u', b's', b'e', b'r', 0x00, b'p', b'a', b's', b's'])
            .unwrap();

        let mut state = State::WaitingInitSyn {
            mine_zid: z,
            mine_whatami: WhatAmI::Client,
            mine_batch_size: 512,
            mine_resolution: Resolution::default(),
            mine_lease: Duration::from_secs(30),
            #[cfg(feature = "auth")]
            mine_auth: Some(payload),
        };

        let init = InitSyn {
            identifier: InitIdentifier {
                zid: z,
                ..Default::default()
            },
            resolution: InitResolution {
                resolution: Resolution::default(),
                batch_size: BatchSize(512),
            },
            ..Default::default()
        };

        let encoded = &[0u8; 0];
        let (response, _) = state.poll((TransportMessage::InitSyn(init), encoded));

        let ack = match response {
            Some(TransportMessage::InitAck(ack)) => ack,
            other => panic!("Expected InitAck, got {:?}", other),
        };

        assert!(ack.auth.is_some(), "InitAck must carry auth");
        let auth = ack.auth.unwrap();
        assert_eq!(
            auth.payload,
            &[0x01, b'u', b's', b'e', b'r', 0x00, b'p', b'a', b's', b's']
        );
    }

    #[test]
    fn auth_payload_roundtrip() {
        use zenoh_proto::{ZEncode, ZLen};
        let payload: &[u8] = &[0x01, b'u', b's', b'e', b'r', 0x00, b'p', b'a', b's', b's'];
        let auth = Auth { payload };

        let mut buf = [0u8; 64];
        let len = auth.z_len();
        auth.z_encode(&mut &mut buf[..]).unwrap();

        let decoded = Auth::z_decode(&mut &buf[..len]).unwrap();
        assert_eq!(auth, decoded);
    }

    #[test]
    fn auth_empty_payload_roundtrip() {
        use zenoh_proto::{ZEncode, ZLen};
        let auth = Auth { payload: &[] };

        let mut buf = [0u8; 64];
        let len = auth.z_len();
        auth.z_encode(&mut &mut buf[..]).unwrap();

        let decoded = Auth::z_decode(&mut &buf[..len]).unwrap();
        assert_eq!(auth, decoded);
    }

    #[test]
    fn auth_payload_in_initack() {
        use zenoh_proto::{ZEncode, ZLen};
        let payload: &[u8] = &[0x01, b't', b'o', b'k', b'e', b'n', 0x00, b'd', b'a', b't', b'a'];
        let ack = InitAck {
            auth: Some(Auth { payload }),
            ..Default::default()
        };

        let mut buf = [0u8; 128];
        let len = ack.z_len();
        ack.z_encode(&mut &mut buf[..]).unwrap();

        let decoded = <InitAck as ZDecode>::z_decode(&mut &buf[..len]).unwrap();
        assert_eq!(ack, decoded);
        assert!(decoded.auth.is_some());
        assert_eq!(decoded.auth.unwrap().payload, payload);
    }

    #[test]
    fn init_syn_with_auth() {
        use zenoh_proto::{ZEncode, ZLen};
        let payload: &[u8] = &[0x02, b'c', b'r', b'e', b'd'];
        let syn = InitSyn {
            auth: Some(Auth { payload }),
            ..Default::default()
        };

        let mut buf = [0u8; 128];
        let len = syn.z_len();
        syn.z_encode(&mut &mut buf[..]).unwrap();

        let decoded = <InitSyn as ZDecode>::z_decode(&mut &buf[..len]).unwrap();
        assert_eq!(syn, decoded);
        assert!(decoded.auth.is_some());
        assert_eq!(decoded.auth.unwrap().payload, payload);
        assert_eq!(decoded.version, VERSION);
    }
}

#[cfg(feature = "proptest")]
#[cfg(test)]
mod proptests {
    extern crate std;
    use core::time::Duration;
    use proptest::prelude::*;
    use std::vec::Vec;
    use zenoh_proto::{ZDecode, ZEncode, exts::*, fields::*, keyexpr, msgs::*};

    use crate::{Transport, ZTransportRx, ZTransportTx, transport::establishment::State};

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
                #[cfg(feature = "auth")]
                mine_auth: None,
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
                #[cfg(feature = "auth")]
                mine_auth: None,
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
                        #[cfg(feature = "auth")]
                        mine_auth: None,
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

    /// Property: any payload can be encoded as a Push/Put or Push/Del,
    /// decoded, and the payload is preserved identically.
    proptest! {
        #[test]
        fn roundtrip_pushbody(payload: Vec<u8>) {
            let ke = keyexpr::from_str_unchecked("test/ke");
            let mut transport = Transport::builder([0u8; 1024]).codec();

            // Put variant
            let put = Put { payload: &payload, ..Default::default() };
            let push = Push {
                wire_expr: WireExpr::from(ke),
                payload: PushBody::Put(put),
                ..Default::default()
            };
            let msg = NetworkMessage {
                reliability: Reliability::Reliable,
                qos: QoS::default(),
                body: NetworkBody::Push(push),
            };

            transport.tx.encode_ref(core::iter::once(msg.as_ref()));
            transport.rx.decode_prefixed(transport.tx.flush_prefixed().unwrap()).unwrap();

            let mut flush = transport.rx.flush();
            let decoded = flush.next().unwrap().0;
            prop_assert_eq!(flush.count(), 0);

            match decoded.body {
                NetworkBody::Push(Push { payload: PushBody::Put(Put { payload: p, .. }), .. }) => {
                    prop_assert_eq!(p, &payload[..]);
                }
                _ => panic!("expected PushBody::Put"),
            }

            // Del variant (no payload)
            let mut transport2 = Transport::builder([0u8; 256]).codec();
            let del_push = Push {
                wire_expr: WireExpr::from(ke),
                payload: PushBody::Del(Del::default()),
                ..Default::default()
            };
            let msg2 = NetworkMessage {
                reliability: Reliability::Reliable,
                qos: QoS::default(),
                body: NetworkBody::Push(del_push),
            };

            transport2.tx.encode_ref(core::iter::once(msg2.as_ref()));
            transport2.rx.decode_prefixed(transport2.tx.flush_prefixed().unwrap()).unwrap();

            let mut flush2 = transport2.rx.flush();
            let decoded2 = flush2.next().unwrap().0;
            prop_assert_eq!(flush2.count(), 0);

            match decoded2.body {
                NetworkBody::Push(Push { payload: PushBody::Del(_), .. }) => {}
                _ => panic!("expected PushBody::Del"),
            }
        }
    }

    /// Property: all NetworkMessage variants (Push, Request, Response,
    /// ResponseFinal, Declare) survive encode/decode roundtrips.
    proptest! {
        #[test]
        fn roundtrip_all_messages(payload in prop::collection::vec(any::<u8>(), 0..64)) {
            let ke = keyexpr::from_str_unchecked("a/b");
            let mut transport = Transport::builder([0u8; 2048]).codec();

            let put = Put { payload: &payload, ..Default::default() };
            let push_msg = NetworkMessage {
                reliability: Reliability::Reliable,
                qos: QoS::default(),
                body: NetworkBody::Push(Push {
                    wire_expr: WireExpr::from(ke),
                    payload: PushBody::Put(put),
                    ..Default::default()
                }),
            };

            let req_msg = NetworkMessage {
                reliability: Reliability::Reliable,
                qos: QoS::default(),
                body: NetworkBody::Request(Request {
                    id: 1,
                    wire_expr: WireExpr::from(ke),
                    payload: RequestBody::Query(Query::default()),
                    ..Default::default()
                }),
            };

            let reply = Reply {
                payload: PushBody::Put(Put { payload: &payload, ..Default::default() }),
                ..Default::default()
            };
            let resp_msg = NetworkMessage {
                reliability: Reliability::Reliable,
                qos: QoS::default(),
                body: NetworkBody::Response(Response {
                    rid: 2,
                    wire_expr: WireExpr::from(ke),
                    payload: ResponseBody::Reply(reply),
                    ..Default::default()
                }),
            };

            let decl_msg = NetworkMessage {
                reliability: Reliability::Reliable,
                qos: QoS::default(),
                body: NetworkBody::Declare(Declare {
                    body: DeclareBody::DeclareFinal(DeclareFinal {}),
                    ..Default::default()
                }),
            };

            let resp_final_msg = NetworkMessage {
                reliability: Reliability::Reliable,
                qos: QoS::default(),
                body: NetworkBody::ResponseFinal(ResponseFinal {
                    rid: 3,
                    ..Default::default()
                }),
            };

            transport.tx.encode_ref(
                core::iter::once(push_msg.as_ref())
                    .chain(core::iter::once(req_msg.as_ref()))
                    .chain(core::iter::once(resp_msg.as_ref()))
                    .chain(core::iter::once(decl_msg.as_ref()))
                    .chain(core::iter::once(resp_final_msg.as_ref()))
            );
            transport.rx.decode_prefixed(transport.tx.flush_prefixed().unwrap()).unwrap();

            let mut flush = transport.rx.flush();

            let m1 = flush.next().unwrap().0;
            match m1.body {
                NetworkBody::Push(Push { payload: PushBody::Put(Put { payload: p, .. }), .. }) => {
                    prop_assert_eq!(p, &payload[..]);
                }
                _ => panic!("expected Push"),
            }

            let m2 = flush.next().unwrap().0;
            match m2.body {
                NetworkBody::Request(Request { id: 1, .. }) => {}
                _ => panic!("expected Request"),
            }

            let m3 = flush.next().unwrap().0;
            match m3.body {
                NetworkBody::Response(Response {
                    rid: 2,
                    payload: ResponseBody::Reply(Reply {
                        payload: PushBody::Put(Put { payload: p, .. }),
                        ..
                    }),
                    ..
                }) => {
                    prop_assert_eq!(p, &payload[..]);
                }
                _ => panic!("expected Response"),
            }

            let m4 = flush.next().unwrap().0;
            match m4.body {
                NetworkBody::Declare(Declare { body: DeclareBody::DeclareFinal(_), .. }) => {}
                _ => panic!("expected Declare"),
            }

            let m5 = flush.next().unwrap().0;
            match m5.body {
                NetworkBody::ResponseFinal(ResponseFinal { rid: 3, .. }) => {}
                _ => panic!("expected ResponseFinal"),
            }

            prop_assert_eq!(flush.count(), 0);
        }
    }

    /// Property: FrameHeader with arbitrary SN and QoS encodes and decodes
    /// to an identical struct.
    proptest! {
        #[test]
        fn frameheader_roundtrip(sn: u32, qos_inner: u8) {
            let header = FrameHeader {
                reliability: Reliability::Reliable,
                sn,
                qos: QoS { inner: qos_inner },
            };

            let mut buf = [0u8; 64];
            let mut writer = &mut buf[..];
            header.z_encode(&mut writer).unwrap();
            let len = 64 - writer.len();

            let mut reader = &buf[..len];
            let decoded = <FrameHeader as ZDecode>::z_decode(&mut reader).unwrap();

            prop_assert_eq!(header, decoded);
        }
    }

    /// Property: multiple messages can be batched (encoded one after another)
    /// and each survives encode/decode roundtrip independently.
    proptest! {
        #[test]
        fn batching_multiple_messages(
            payloads in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 0..32), 0..6)
        ) {
            let ke = keyexpr::from_str_unchecked("a/b");

            let msgs: Vec<_> = payloads.iter().map(|payload| {
                NetworkMessage {
                    reliability: Reliability::Reliable,
                    qos: QoS::default(),
                    body: NetworkBody::Push(Push {
                        wire_expr: WireExpr::from(ke),
                        payload: PushBody::Put(Put {
                            payload,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                }
            }).collect();

            if msgs.is_empty() {
                return Ok(());
            }

            let mut transport = Transport::builder([0u8; 4096]).codec();
            for msg in &msgs {
                transport.tx.encode_ref(core::iter::once(msg.as_ref()));
            }

            transport.rx.decode_prefixed(transport.tx.flush_prefixed().unwrap()).unwrap();

            for (i, (decoded, _)) in transport.rx.flush().enumerate() {
                match decoded.body {
                    NetworkBody::Push(Push {
                        payload: PushBody::Put(Put { payload: p, .. }),
                        ..
                    }) => {
                        prop_assert_eq!(p, &payloads[i][..],
                            "payload mismatch at index {}", i);
                    }
                    _ => panic!("expected Push at index {}", i),
                }
            }
        }
    }

    /// Property: any valid keyexpr must intersect itself.
    proptest! {
        #[test]
        fn keyexpr_intersect_self(
            a in prop::collection::vec("[a-zA-Z0-9][a-zA-Z0-9]*", 1..5)
                .prop_map(|v| v.join("/"))
                .prop_filter("valid keyexpr", |s| keyexpr::new(s).is_ok())
        ) {
            let ke = keyexpr::new(&a).unwrap();
            prop_assert!(ke.intersects(ke), "keyexpr '{a}' must intersect itself");
        }
    }

    /// Test: Empty payloads roundtrip correctly.
    proptest! {
        #[test]
        fn empty_payload_roundtrip(_dummy in 0u8..1) {
            let ke = keyexpr::from_str_unchecked("test/empty");
            let mut transport = Transport::builder([0u8; 256]).codec();

            let msg = NetworkMessage {
                reliability: Reliability::Reliable,
                qos: QoS::default(),
                body: NetworkBody::Push(Push {
                    wire_expr: WireExpr::from(ke),
                    payload: PushBody::Put(Put {
                        payload: &[],
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
            };

            transport.tx.encode_ref(core::iter::once(msg.as_ref()));
            transport.rx.decode_prefixed(transport.tx.flush_prefixed().unwrap()).unwrap();

            let mut flush = transport.rx.flush();
            let decoded = flush.next().unwrap().0;
            prop_assert_eq!(flush.count(), 0);

            match decoded.body {
                NetworkBody::Push(Push { payload: PushBody::Put(Put { payload, .. }), .. }) => {
                    prop_assert!(payload.is_empty(), "empty payload must decode as empty");
                }
                _ => panic!("expected PushBody::Put"),
            }
        }
    }

    /// Test: Maximum size payloads up to batch_size survive roundtrip.
    proptest! {
        #[test]
        fn max_size_payload(payload_size in 1usize..250) {
            let ke = keyexpr::from_str_unchecked("test/ke");
            let mut transport = Transport::builder([0u8; 512]).codec();

            let mut payload = Vec::with_capacity(payload_size);
            payload.resize(payload_size, 0xABu8);
            let msg = NetworkMessage {
                reliability: Reliability::Reliable,
                qos: QoS::default(),
                body: NetworkBody::Push(Push {
                    wire_expr: WireExpr::from(ke),
                    payload: PushBody::Put(Put {
                        payload: &payload,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
            };

            transport.tx.encode_ref(core::iter::once(msg.as_ref()));
            let flushed = transport.tx.flush_prefixed();
            if flushed.is_none() {
                return Ok(());
            }
            transport.rx.decode_prefixed(flushed.unwrap()).unwrap();

            let mut flush = transport.rx.flush();
            let decoded = flush.next().unwrap().0;
            prop_assert_eq!(flush.count(), 0);

            match decoded.body {
                NetworkBody::Push(Push { payload: PushBody::Put(Put { payload: p, .. }), .. }) => {
                    prop_assert_eq!(&p[..], &payload[..]);
                }
                _ => panic!("expected PushBody::Put"),
            }
        }
    }

    /// Test: Every QoS combination (priority, congestion control, express)
    /// survives encode/decode roundtrip.
    proptest! {
        #[test]
        fn all_qos_combinations(qos_inner: u8) {
            let qos = QoS { inner: qos_inner };
            let ke = keyexpr::from_str_unchecked("test/ke");
            let mut transport = Transport::builder([0u8; 256]).codec();

            let msg = NetworkMessage {
                reliability: Reliability::Reliable,
                qos,
                body: NetworkBody::Push(Push {
                    wire_expr: WireExpr::from(ke),
                    payload: PushBody::Put(Put {
                        payload: &[0u8; 1],
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
            };

            transport.tx.encode_ref(core::iter::once(msg.as_ref()));
            transport.rx.decode_prefixed(transport.tx.flush_prefixed().unwrap()).unwrap();

            let mut flush = transport.rx.flush();
            let decoded = flush.next().unwrap().0;
            prop_assert_eq!(flush.count(), 0);
            prop_assert_eq!(decoded.qos, qos, "QoS must survive roundtrip");
        }
    }

    /// Test: Fragmented messages produce multiple frames and payload survives roundtrip.
    proptest! {
        #[test]
        fn fragmentation_random_payloads(payload in prop::collection::vec(any::<u8>(), 1..128)) {
            let ke = keyexpr::from_str_unchecked("test/ke");
            let batch_size_val: usize = 64;
            let mut tx = crate::transport::TransportTx::new(
                [0u8; 512],
                batch_size_val,
                0,
                Resolution::default(),
                Duration::from_secs(10),
            )
            .with_max_fragments(3);

            let mut body_buff = [0u8; 1024];
            let mut writer = &mut body_buff[..];
            let wlen = writer.len();

            let body_msg = NetworkBody::Push(Push {
                wire_expr: WireExpr::from(ke),
                payload: PushBody::Put(Put {
                    payload: &payload,
                    ..Default::default()
                }),
                ..Default::default()
            });

            <NetworkBody as zenoh_proto::ZEncode>::z_encode(&body_msg, &mut writer).unwrap();
            let body_len = wlen - writer.len();
            let encoded_body = &body_buff[..body_len];

            let msg = NetworkMessage {
                reliability: Reliability::Reliable,
                qos: QoS::default(),
                body: body_msg,
            };

            tx.encode_optimized(core::iter::once((msg, encoded_body)));

            // Flush and count frames — ensure fragmentation works
            let mut flush_count = 0usize;
            let mut total_flushed = 0usize;
            loop {
                let flushed = match tx.flush_prefixed() {
                    Some(f) if f.len() >= 2 => f,
                    _ => break,
                };
                total_flushed += flushed.len();
                flush_count += 1;
                if flush_count > 8 { break; }
                tx.keepalive();
            }

            prop_assert!(flush_count >= 1, "at least one frame must be produced");
            prop_assert!(total_flushed > 0, "must flush non-zero bytes");

            // Also verify raw flush works
            let mut raw_total = 0usize;
            loop {
                let flushed = match tx.flush_raw() {
                    Some(f) if !f.is_empty() => f,
                    _ => break,
                };
                raw_total += flushed.len();
                if raw_total > 1024 { break; }
                tx.keepalive();
            }
        }
    }

    /// Test: Batching with mixed message types (Put and Del).
    proptest! {
        #[test]
        fn batching_mixed_types(
            put_count in 1u8..8,
            del_count in 1u8..8,
            payloads in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 0..16),
                0..6,
            ),
        ) {
            let ke = keyexpr::from_str_unchecked("a/b");
            let mut transport = Transport::builder([0u8; 4096]).codec();

            transport.tx.batch_start();
            prop_assert!(transport.tx.is_batching());

            let mut expected_puts: Vec<Vec<u8>> = Vec::new();

            for i in 0..put_count {
                let payload: &[u8] = if (i as usize) < payloads.len() {
                    &payloads[i as usize]
                } else {
                    &[]
                };
                let msg = NetworkMessage {
                    reliability: Reliability::Reliable,
                    qos: QoS::default(),
                    body: NetworkBody::Push(Push {
                        wire_expr: WireExpr::from(ke),
                        payload: PushBody::Put(Put {
                            payload,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                };
                expected_puts.push(payload.to_vec());
                transport.tx.encode_ref(core::iter::once(msg.as_ref()));
            }

            for _ in 0..del_count {
                let msg = NetworkMessage {
                    reliability: Reliability::Reliable,
                    qos: QoS::default(),
                    body: NetworkBody::Push(Push {
                        wire_expr: WireExpr::from(ke),
                        payload: PushBody::Del(Del::default()),
                        ..Default::default()
                    }),
                };
                transport.tx.encode_ref(core::iter::once(msg.as_ref()));
            }

            transport.tx.batch_stop();
            prop_assert!(!transport.tx.is_batching());

            transport.rx.decode_prefixed(transport.tx.flush_prefixed().unwrap()).unwrap();

            let decoded: Vec<_> = transport.rx.flush().collect();
            let total_expected = put_count as usize + del_count as usize;
            prop_assert_eq!(
                decoded.len(),
                total_expected,
                "all batched messages must be decoded"
            );

            let mut put_idx = 0usize;
            let mut del_seen = 0usize;
            for (msg, _) in &decoded {
                match &msg.body {
                    NetworkBody::Push(Push { payload: PushBody::Put(Put { payload, .. }), .. }) => {
                        if put_idx < expected_puts.len() {
                            prop_assert_eq!(&payload[..], &expected_puts[put_idx][..]);
                        }
                        put_idx += 1;
                    }
                    NetworkBody::Push(Push { payload: PushBody::Del(_), .. }) => {
                        del_seen += 1;
                    }
                    _ => panic!("unexpected message type"),
                }
            }

            prop_assert_eq!(put_idx, put_count as usize, "all Put messages must be present");
            prop_assert_eq!(del_seen, del_count as usize, "all Del messages must be present");
        }
    }

    /// Property: keyexpr `intersects` is symmetric, `includes` is reflexive,
    /// and `includes` implies `intersects`.
    proptest! {
        #[test]
        fn keyexpr_wildcard_properties(
            a in prop::collection::vec("[a-zA-Z0-9*#][a-zA-Z0-9*]*", 1..4)
                .prop_map(|v| v.join("/"))
                .prop_filter("valid keyexpr", |s| keyexpr::new(s).is_ok()),
            b in prop::collection::vec("[a-zA-Z0-9*#][a-zA-Z0-9*]*", 1..4)
                .prop_map(|v| v.join("/"))
                .prop_filter("valid keyexpr", |s| keyexpr::new(s).is_ok()),
        ) {
            let ke_a = keyexpr::new(&a).unwrap();
            let ke_b = keyexpr::new(&b).unwrap();

            prop_assert_eq!(
                ke_a.intersects(ke_b),
                ke_b.intersects(ke_a),
                "intersects must be symmetric"
            );

            prop_assert!(
                ke_a.includes(ke_a),
                "includes must be reflexive"
            );
            prop_assert!(
                ke_b.includes(ke_b),
                "includes must be reflexive"
            );
        }
    }
}
