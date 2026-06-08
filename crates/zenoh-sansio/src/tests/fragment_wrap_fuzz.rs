#[cfg(feature = "proptest")]
#[cfg(test)]
mod fragment_wrap_fuzz {
    extern crate std;
    use std::vec::Vec;

    use core::time::Duration;
    use proptest::collection::vec as prop_vec;
    use proptest::prelude::*;
    use zenoh_proto::{ZEncode, exts::*, fields::*, msgs::*};

    use crate::{ZTransportRx, transport::TransportRx};

    const BATCH: usize = 512;
    const BUFF: usize = 512;

    fn make_fragment_frame(payload: &[u8], sn: u32) -> Vec<u8> {
        let header = FrameHeader {
            reliability: Reliability::Reliable,
            sn,
            qos: QoS::default(),
        };
        let mut buf = std::vec![0u8; 512];
        let mut writer = &mut buf[..];
        header.z_encode(&mut writer).unwrap();
        let header_len = 512 - writer.len();
        buf.truncate(header_len);
        buf.extend_from_slice(payload);
        buf
    }

    proptest! {
        #[test]
        fn fragment_sn_wrap_no_corruption(
            old_payloads in prop_vec(any::<u8>(), 1..64),
            new_payload in prop_vec(any::<u8>(), 1..64),
        ) {
            let mut rx = TransportRx::new(
                [0u8; BUFF], BATCH, u32::MAX - 1,
                Resolution::default(), Duration::from_secs(10),
            ).with_max_fragments(3);

            let frag1 = make_fragment_frame(&old_payloads, u32::MAX);
            let frag2 = make_fragment_frame(&old_payloads, u32::MAX);
            let frag_new = make_fragment_frame(&new_payload, 0);

            let mut batch = Vec::new();
            batch.extend_from_slice(&frag1);
            batch.extend_from_slice(&frag2);
            batch.extend_from_slice(&frag_new);

            let max_batch = BUFF.min(BATCH);
            if batch.len() > max_batch {
                return Ok(());
            }

            rx.decode_raw(&batch).unwrap();
            let messages: Vec<_> = rx.flush().collect();

            for (_msg, raw) in &messages {
                for window in raw.windows(new_payload.len()) {
                    prop_assert_ne!(
                        window, &new_payload[..],
                        "new message payload must not appear in decoded old message"
                    );
                }
            }
        }
    }
}
