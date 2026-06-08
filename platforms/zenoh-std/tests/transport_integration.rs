use std::net::{TcpListener, TcpStream, UdpSocket};
use std::thread;
use zenoh_nostd::platform::{ZLink, ZLinkInfo, ZLinkRx, ZLinkTx};
use zenoh_std::tcp::StdTcpLink;
use zenoh_std::udp::StdUdpLink;

#[test]
fn tcp_link_read_write_roundtrip() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut link = StdTcpLink::new(stream, u16::MAX);
        let mut buf = [0u8; 32];
        let n = link.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"hello");
        link.write_all(b"world").unwrap();
    });

    let stream = TcpStream::connect(addr).unwrap();
    let mut link = StdTcpLink::new(stream, u16::MAX);
    link.write_all(b"hello").unwrap();
    let mut buf = [0u8; 32];
    let n = link.read(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"world");

    server.join().unwrap();
}

#[test]
fn tcp_link_split_roundtrip() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut link = StdTcpLink::new(stream, u16::MAX);
        let mut buf = [0u8; 32];
        let n = link.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"ping");
        link.write_all(b"pong").unwrap();
    });

    let stream = TcpStream::connect(addr).unwrap();
    let mut link = StdTcpLink::new(stream, u16::MAX);
    let (mut tx, mut rx) = link.split();
    tx.write_all(b"ping").unwrap();
    let mut buf = [0u8; 32];
    let n = rx.read(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"pong");

    server.join().unwrap();
}

#[test]
fn tcp_link_read_exact() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let payload = b"exact_data_16B!!";

    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut link = StdTcpLink::new(stream, u16::MAX);
        link.write_all(payload).unwrap();
    });

    let stream = TcpStream::connect(addr).unwrap();
    let mut link = StdTcpLink::new(stream, u16::MAX);
    let mut buf = [0u8; 16];
    link.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, payload);

    server.join().unwrap();
}

#[test]
fn udp_link_read_write_roundtrip() {
    let server_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let server_addr = server_socket.local_addr().unwrap();

    let server = thread::spawn(move || {
        let mut buf = [0u8; 32];
        let (n, peer) = server_socket.recv_from(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"hello");
        server_socket.send_to(b"world", peer).unwrap();
    });

    let client_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    client_socket.connect(server_addr).unwrap();
    let mut link = StdUdpLink::new(client_socket, 8192);
    link.write_all(b"hello").unwrap();
    let mut buf = [0u8; 32];
    let n = link.read(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"world");

    server.join().unwrap();
}

#[test]
fn udp_link_read_single_datagram() {
    let server_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let server_addr = server_socket.local_addr().unwrap();

    let server = thread::spawn(move || {
        let mut buf = [0u8; 32];
        let (_, peer) = server_socket.recv_from(&mut buf).unwrap();
        server_socket.send_to(b"short", peer).unwrap();
    });

    let client_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    client_socket.connect(server_addr).unwrap();
    let mut link = StdUdpLink::new(client_socket, 8192);
    link.write_all(b"data").unwrap();
    let mut buf = [0u8; 32];
    let n = link.read(&mut buf).unwrap();
    // read on a connected UDP socket returns exactly one datagram
    assert_eq!(n, 5);
    assert_eq!(&buf[..n], b"short");

    server.join().unwrap();
}

#[test]
fn udp_link_mtu() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let link = StdUdpLink::new(socket, 4096);
    assert_eq!(link.mtu(), 4096);
    assert!(!link.is_streamed());
}

#[test]
fn tcp_link_mtu() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let _server = thread::spawn(move || {
        let _ = listener.accept();
    });
    let stream = TcpStream::connect(addr).unwrap();
    let link = StdTcpLink::new(stream, 8192);
    assert_eq!(link.mtu(), 8192);
    assert!(link.is_streamed());
}