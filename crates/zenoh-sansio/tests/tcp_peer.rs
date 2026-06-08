use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use zenoh_proto::fields::WhatAmI;
use zenoh_sansio::Transport;

#[test]
fn peer_transport_handshake_over_tcp() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut stream = stream;

        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();

        let mut h = Transport::builder([0u8; 512])
            .with_whatami(WhatAmI::Peer)
            .listen(
                &mut stream,
                |s: &mut &mut TcpStream, buf| s.read(buf),
                |s: &mut &mut TcpStream, buf| s.write_all(buf),
            );

        loop {
            match h.poll::<std::io::Error>() {
                Ok(Some(ready)) => {
                    ready.open();
                    break;
                }
                Ok(None) => {}
                Err(e) => panic!("listener handshake failed: {:?}", e),
            }
        }
    });

    let stream = TcpStream::connect(addr).unwrap();
    let mut stream = stream;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();

    let mut h = Transport::builder([0u8; 512])
        .with_whatami(WhatAmI::Peer)
        .connect(
            &mut stream,
            |s: &mut &mut TcpStream, buf| s.read(buf),
            |s: &mut &mut TcpStream, buf| s.write_all(buf),
        );

    loop {
        match h.poll::<std::io::Error>() {
            Ok(Some(ready)) => {
                ready.open();
                break;
            }
            Ok(None) => {}
            Err(e) => panic!("connector handshake failed: {:?}", e),
        }
    }

    handle.join().unwrap();
}
