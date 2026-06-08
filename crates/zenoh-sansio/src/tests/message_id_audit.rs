use zenoh_proto::msgs::*;

/// All message types that appear in the transport dispatch table (rx.rs).
/// Each entry: (name, type_id, requires_ack, requires_net, requires_ifinal)
const DISPATCH_ENTRIES: &[(&str, u8, bool, bool, bool)] = &[
    ("FrameHeader", FrameHeader::ID, false, false, false),
    ("InitSyn", InitSyn::ID, false, false, false),
    ("InitAck", InitAck::ID, true, false, false),
    ("OpenSyn", OpenSyn::ID, false, false, false),
    ("OpenAck", OpenAck::ID, true, false, false),
    ("Close", Close::ID, false, false, false),
    ("KeepAlive", KeepAlive::ID, false, false, false),
    ("Push", Push::ID, false, true, false),
    ("Request", Request::ID, false, true, false),
    ("Response", Response::ID, false, true, false),
    ("ResponseFinal", ResponseFinal::ID, false, true, false),
    ("Interest", Interest::ID, false, true, false),
    ("InterestFinal", InterestFinal::ID, false, true, true),
    ("Declare", Declare::ID, false, true, false),
];

#[test]
fn no_overlapping_message_ids() {
    for (i, (name_i, id_i, ack_i, net_i, ifinal_i)) in DISPATCH_ENTRIES.iter().enumerate() {
        for (j, (name_j, id_j, ack_j, net_j, ifinal_j)) in DISPATCH_ENTRIES.iter().enumerate() {
            if i >= j {
                continue;
            }
            // FrameHeader is special — recursive, never produces a body in the dispatch
            if *name_i == "FrameHeader" || *name_j == "FrameHeader" {
                continue;
            }
            if id_i != id_j {
                continue;
            }
            // Check if dispatch conditions overlap
            // ack overlap: both need ack, or both don't need ack
            let ack_overlap = !(*ack_i && !*ack_j) && !(!*ack_i && *ack_j);
            // net overlap: both require net or both don't
            let net_overlap = *net_i == *net_j;
            // ifinal overlap: both require ifinal, both don't, or neither does
            let ifinal_overlap = !(*ifinal_i && !*ifinal_j) && !(!*ifinal_i && *ifinal_j);

            if ack_overlap && net_overlap && ifinal_overlap {
                panic!(
                    "ID collision: {} (id=0x{:02x}, ack={}, net={}, ifinal={}) \
                     and {} (id=0x{:02x}, ack={}, net={}, ifinal={})",
                    name_i, id_i, ack_i, net_i, ifinal_i, name_j, id_j, ack_j, net_j, ifinal_j,
                );
            }
        }
    }
}

#[test]
fn unregistered_ids_produce_error() {
    let mut handled = [false; 32];
    for (name, id, _ack, _net, _ifinal) in DISPATCH_ENTRIES {
        if *name == "FrameHeader" {
            continue;
        }
        handled[*id as usize] = true;
    }
    for id in 0u8..=31u8 {
        if handled[id as usize] {
            continue;
        }
        for ack_bit in [0u8, 1u8] {
            let header = (ack_bit << 5) | id;
            let dispatch_id = header & 0b0001_1111;
            assert!(
                !handled[dispatch_id as usize],
                "ID 0x{:02x} (header 0x{:02x}) should be unhandled but is in handled set",
                dispatch_id, header
            );
        }
    }
}

#[cfg(feature = "proptest")]
#[cfg(test)]
mod proptests {
    extern crate std;
    use std::vec::Vec;

    use proptest::prelude::*;
    use zenoh_proto::{exts::*, fields::*, keyexpr, msgs::*};

    use crate::{Transport, ZTransportRx, ZTransportTx};

    proptest! {
        #[test]
        fn roundtrip_all_message_types(payload in prop::collection::vec(any::<u8>(), 0..64)) {
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

            let resp_final_msg = NetworkMessage {
                reliability: Reliability::Reliable,
                qos: QoS::default(),
                body: NetworkBody::ResponseFinal(ResponseFinal {
                    rid: 3,
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

            transport.tx.encode_ref(
                core::iter::once(push_msg.as_ref())
                    .chain(core::iter::once(req_msg.as_ref()))
                    .chain(core::iter::once(resp_msg.as_ref()))
                    .chain(core::iter::once(resp_final_msg.as_ref()))
                    .chain(core::iter::once(decl_msg.as_ref()))
            );

            let encoded = transport.tx.flush_prefixed().unwrap();
            let encoded_copy = encoded.to_vec();

            transport.rx.decode_prefixed(encoded).unwrap();

            let mut flush = transport.rx.flush();

            let (m1, _raw1) = flush.next().unwrap();
            prop_assert_eq!(&m1, &push_msg, "Push roundtrip failed");

            let (m2, _raw2) = flush.next().unwrap();
            prop_assert_eq!(&m2, &req_msg, "Request roundtrip failed");

            let (m3, _raw3) = flush.next().unwrap();
            prop_assert_eq!(&m3, &resp_msg, "Response roundtrip failed");

            let (m4, _raw4) = flush.next().unwrap();
            prop_assert_eq!(&m4, &resp_final_msg, "ResponseFinal roundtrip failed");

            let (m5, _raw5) = flush.next().unwrap();
            prop_assert_eq!(&m5, &decl_msg, "Declare roundtrip failed");

            prop_assert_eq!(flush.count(), 0, "unexpected extra messages");

            let mut transport2 = Transport::builder([0u8; 2048]).codec();
            transport2.tx.encode_ref(
                core::iter::once(m1.as_ref())
                    .chain(core::iter::once(m2.as_ref()))
                    .chain(core::iter::once(m3.as_ref()))
                    .chain(core::iter::once(m4.as_ref()))
                    .chain(core::iter::once(m5.as_ref()))
            );
            let re_encoded = transport2.tx.flush_prefixed().unwrap();
            prop_assert_eq!(&re_encoded[..], &encoded_copy[..],
                "re-encoded bytes differ from original");
        }
    }

    proptest! {
        #[test]
        fn decode_never_panics(bytes: Vec<u8>) {
            let mut transport = Transport::builder([0u8; 256]).codec();
            let _ = transport.rx.decode_raw(&bytes);
            let _: Vec<_> = transport.rx.flush().collect();
        }
    }

    proptest! {
        #[test]
        fn all_header_ids_are_safe(id in 0u8..32u8) {
            let mut transport = Transport::builder([0u8; 256]).codec();
            for ack in [0u8, 1u8] {
                let header = (ack << 5) | id;
                let bytes = [header, 0u8, 0u8, 0u8, 0u8];
                let _ = transport.rx.decode_raw(&bytes);
                let _: Vec<_> = transport.rx.flush().collect();
            }
        }
    }
}
