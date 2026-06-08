#![no_main]

use libfuzzer_sys::fuzz_target;
use zenoh_sansio::{Transport, ZTransportRx};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let buf_size = 65536;
    let mut transport = Transport::builder(vec![0u8; buf_size]).codec();
    let rx = &mut transport.rx;

    for _ in 0..100 {
        if data.len() >= 2 {
            let _ = rx.decode_prefixed(data);
        }
        let _ = rx.decode_raw(data);

        for _ in rx.flush() {}

        if rx.closed() {
            break;
        }
    }
});
