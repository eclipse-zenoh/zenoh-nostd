#![no_main]

use libfuzzer_sys::fuzz_target;
use zenoh_sansio::{Handshake, Transport};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    struct Handle<'a> {
        input: &'a [u8],
        pos: usize,
    }

    let buf_size = 65536;
    let handle = Handle {
        input: data,
        pos: 0,
    };

    let read = |h: &mut Handle, buf: &mut [u8]| -> Result<usize, core::convert::Infallible> {
        let len = buf.len().min(h.input.len().saturating_sub(h.pos));
        if len > 0 {
            buf[..len].copy_from_slice(&h.input[h.pos..h.pos + len]);
            h.pos += len;
        }
        Ok(len)
    };

    let write = |_: &mut Handle, _: &[u8]| -> Result<(), core::convert::Infallible> { Ok(()) };

    // Test client-side handshake: connect sends InitSyn, then waits for
    // InitAck → OpenSyn → OpenAck from the peer.
    let builder = Transport::builder(vec![0u8; buf_size]);
    let mut handshake: Handshake<_, _, _, _> = builder.connect(handle, read, write);

    for _ in 0..50 {
        match handshake.poll::<core::convert::Infallible>() {
            Ok(Some(ready)) => {
                let _ = ready.open();
                break;
            }
            Ok(None) => continue,
            Err(_) => break,
        }
    }
});
