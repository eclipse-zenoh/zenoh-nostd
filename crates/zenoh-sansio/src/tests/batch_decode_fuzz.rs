#[cfg(feature = "proptest")]
#[cfg(test)]
mod batch_decode_fuzz {
    extern crate std;
    use std::vec::Vec;

    use core::time::Duration;
    use proptest::collection::vec as prop_vec;
    use proptest::prelude::*;
    use zenoh_proto::{ZBodyEncode, ZBodyLen, ZHeader, exts::*, fields::*, keyexpr, msgs::*};

    use crate::{Transport, ZTransportRx, ZTransportTx, transport::TransportRx};

    const BATCH: usize = 256;
    const BUFF: usize = 256;

    fn make_prefixed(payload: &[u8]) -> Vec<u8> {
        let capacity = BATCH.min(BUFF).saturating_sub(2);
        let len = payload.len().min(capacity);
        let mut out = Vec::with_capacity(2 + len);
        out.extend_from_slice(&(len as u16).to_le_bytes());
        out.extend_from_slice(&payload[..len]);
        out
    }

    proptest! {
        #[test]
        fn no_panic_arbitrary_input(payload in prop_vec(any::<u8>(), 0..254)) {
            let prefixed = make_prefixed(&payload);
            let mut transport = Transport::builder([0u8; BUFF]).codec();
            let _ = transport.rx.decode_prefixed(&prefixed);
            let _: Vec<_> = transport.rx.flush().collect();
        }
    }

    fn test_truncated_msg(header: u8, encoded: &[u8]) {
        let total_len = 1 + encoded.len();
        for truncate_at in 0..total_len {
            let mut partial = Vec::with_capacity(1 + truncate_at);
            partial.push(header);
            partial.extend_from_slice(&encoded[..truncate_at.min(encoded.len())]);

            let prefixed = make_prefixed(&partial);
            let mut transport = Transport::builder([0u8; BUFF]).codec();
            let _ = transport.rx.decode_prefixed(&prefixed);
            let _: Vec<_> = transport.rx.flush().collect();
        }
    }

    fn encode_msg<E>(msg: &E) -> (u8, Vec<u8>)
    where
        E: ZBodyEncode + ZHeader + ZBodyLen,
    {
        let header = msg.z_header();
        let body_len = msg.z_body_len();
        // Use a generously sized buffer, then truncate to actual encoded size
        let mut encoded = std::vec![0u8; body_len + 256];
        let mut writer = &mut encoded[..];
        msg.z_body_encode(&mut writer).unwrap();
        let actual_len = body_len + 256 - writer.len();
        encoded.truncate(actual_len);
        (header, encoded)
    }

    #[test]
    fn no_panic_truncated_frameheader() {
        let (h, e) = encode_msg(&FrameHeader {
            reliability: Reliability::Reliable,
            sn: 42,
            qos: QoS::default(),
        });
        test_truncated_msg(h, &e);
    }

    #[test]
    fn no_panic_truncated_init_syn() {
        let (h, e) = encode_msg(&InitSyn {
            version: 1,
            identifier: InitIdentifier {
                whatami: WhatAmI::Client,
                zid: ZenohIdProto::default(),
            },
            ..Default::default()
        });
        test_truncated_msg(h, &e);
    }

    #[test]
    fn no_panic_truncated_init_ack() {
        let (h, e) = encode_msg(&InitAck::default());
        test_truncated_msg(h, &e);
    }

    #[test]
    fn no_panic_truncated_open_syn() {
        let (h, e) = encode_msg(&OpenSyn::default());
        test_truncated_msg(h, &e);
    }

    #[test]
    fn no_panic_truncated_open_ack() {
        let (h, e) = encode_msg(&OpenAck::default());
        test_truncated_msg(h, &e);
    }

    #[test]
    fn no_panic_truncated_close() {
        let (h, e) = encode_msg(&Close::default());
        test_truncated_msg(h, &e);
    }

    #[test]
    fn no_panic_truncated_keepalive() {
        let (h, e) = encode_msg(&KeepAlive);
        test_truncated_msg(h, &e);
    }

    fn test_truncated_batch(payload: &[u8]) {
        let ke = keyexpr::from_str_unchecked("a/b");
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

        let mut tx = crate::transport::TransportTx::new(
            [0u8; BUFF],
            BATCH,
            0,
            Resolution::default(),
            Duration::from_secs(10),
        );
        tx.encode_ref(core::iter::once(msg.as_ref()));
        let full_batch = match tx.flush_prefixed() {
            Some(b) => b.to_vec(),
            None => return,
        };

        for truncate_at in 0..full_batch.len() {
            let partial = &full_batch[..truncate_at];
            let prefixed = make_prefixed(partial);
            let mut transport = Transport::builder([0u8; BUFF]).codec();
            let _ = transport.rx.decode_prefixed(&prefixed);
            let _: Vec<_> = transport.rx.flush().collect();
        }
    }

    proptest! {
        #[test]
        fn no_panic_truncated_batch(payload in prop_vec(any::<u8>(), 0..32)) {
            test_truncated_batch(&payload);
        }
    }

    fn test_corrupted_msg(header: u8, encoded: &[u8]) {
        let mut full = Vec::with_capacity(1 + encoded.len());
        full.push(header);
        full.extend_from_slice(encoded);

        for flip_at in 0..full.len() {
            let mut corrupted = full.clone();
            corrupted[flip_at] ^= 0xFF;

            let prefixed = make_prefixed(&corrupted);
            let mut transport = Transport::builder([0u8; BUFF]).codec();
            let _ = transport.rx.decode_prefixed(&prefixed);
            let _: Vec<_> = transport.rx.flush().collect();
        }
    }

    #[test]
    fn no_panic_corrupted_frameheader() {
        let (h, e) = encode_msg(&FrameHeader {
            reliability: Reliability::Reliable,
            sn: 42,
            qos: QoS::default(),
        });
        test_corrupted_msg(h, &e);
    }

    #[test]
    fn no_panic_corrupted_init_syn() {
        let (h, e) = encode_msg(&InitSyn {
            version: 1,
            identifier: InitIdentifier {
                whatami: WhatAmI::Client,
                zid: ZenohIdProto::default(),
            },
            ..Default::default()
        });
        test_corrupted_msg(h, &e);
    }

    #[test]
    fn no_panic_corrupted_init_ack() {
        let (h, e) = encode_msg(&InitAck::default());
        test_corrupted_msg(h, &e);
    }

    #[test]
    fn no_panic_corrupted_open_syn() {
        let (h, e) = encode_msg(&OpenSyn::default());
        test_corrupted_msg(h, &e);
    }

    #[test]
    fn no_panic_corrupted_open_ack() {
        let (h, e) = encode_msg(&OpenAck::default());
        test_corrupted_msg(h, &e);
    }

    #[test]
    fn no_panic_corrupted_close() {
        let (h, e) = encode_msg(&Close::default());
        test_corrupted_msg(h, &e);
    }

    #[test]
    fn no_panic_corrupted_keepalive() {
        let (h, e) = encode_msg(&KeepAlive);
        test_corrupted_msg(h, &e);
    }

    proptest! {
        #[test]
        fn sn_monotonicity(
            payloads in prop_vec(prop_vec(any::<u8>(), 0..16), 1..8),
            initial_sn: u32,
        ) {
            let ke = keyexpr::from_str_unchecked("a/b");
            const BATCH_SN: usize = 1024;
            const BUFF_SN: usize = 1024;

            let mut tx = crate::transport::TransportTx::new(
                [0u8; BUFF_SN], BATCH_SN, initial_sn,
                Resolution::default(), Duration::from_secs(10),
            );

            for payload in &payloads {
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
                tx.encode_ref(core::iter::once(msg.as_ref()));
            }

            let flushed = match tx.flush_prefixed() {
                Some(b) => b.to_vec(),
                None => return Ok(()),
            };

            let mut rx = TransportRx::new(
                [0u8; BUFF_SN], BATCH_SN, initial_sn,
                Resolution::default(), Duration::from_secs(10),
            );

            rx.decode_prefixed(&flushed).unwrap();

            let decoded_count = rx.flush().count();
            prop_assert_eq!(
                decoded_count, payloads.len(),
                "all encoded messages must be decoded (SN monotonicity check)"
            );
        }
    }

    #[test]
    fn empty_input_no_panic() {
        let mut transport = Transport::builder([0u8; 128]).codec();
        let _ = transport.rx.decode_prefixed(&[]);
        let _: Vec<_> = transport.rx.flush().collect();
    }

    #[test]
    fn zero_length_no_panic() {
        let mut transport = Transport::builder([0u8; 128]).codec();
        let _ = transport.rx.decode_prefixed(&[0x00, 0x00]);
        let _: Vec<_> = transport.rx.flush().collect();
    }

    #[test]
    fn oversized_length_no_panic() {
        let mut transport = Transport::builder([0u8; 64]).codec();
        let _ = transport.rx.decode_prefixed(&[0xFF, 0xFF]);
        let _: Vec<_> = transport.rx.flush().collect();
    }

    proptest! {
        #[test]
        fn no_panic_decode_raw(payload in prop_vec(any::<u8>(), 0..256)) {
            let mut transport = Transport::builder([0u8; 256]).codec();
            let _ = transport.rx.decode_raw(&payload);
            let _: Vec<_> = transport.rx.flush().collect();
        }
    }

    #[test]
    fn all_single_byte_headers_no_panic() {
        for header in 0u8..=255 {
            let mut transport = Transport::builder([0u8; 256]).codec();
            let _ = transport.rx.decode_raw(&[header]);
            let _: Vec<_> = transport.rx.flush().collect();
        }
    }

    #[test]
    fn all_two_byte_sequences_no_panic() {
        for h1 in 0u8..=255u8 {
            for h2 in [0u8, 1, 2, 3] {
                let mut transport = Transport::builder([0u8; 256]).codec();
                let _ = transport.rx.decode_raw(&[h1, h2]);
                let _: Vec<_> = transport.rx.flush().collect();
            }
        }
    }
}
