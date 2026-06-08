//! Hex conformance tests for zenoh protocol encoding.
//!
//! These tests verify wire-format stability by comparing encoded bytes
//! against captured golden hex dumps. To capture golden values:
//! ```sh
//! cargo test -p zenoh-proto --features std hex_ -- --nocapture
//! ```
//! Then update the `expected_hex` constants below.
//!
//! For cross-validation with zenoh-pico: encode the same message in pico
//! and compare the hex dump to the golden value here.

use crate::{exts::*, fields::*, msgs::*};

fn hex_encode<'a>(bytes: &[u8], buf: &'a mut [u8]) -> &'a str {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for (i, b) in bytes.iter().enumerate() {
        buf[i * 2] = HEX[(b >> 4) as usize];
        buf[i * 2 + 1] = HEX[(b & 0x0f) as usize];
    }
    core::str::from_utf8(&buf[..bytes.len() * 2]).unwrap()
}

fn zid_from_bytes(bytes: &[u8; 8]) -> ZenohIdProto {
    ZenohIdProto::try_from(&bytes[..]).unwrap()
}

const BUF_SIZE: usize = 512;

// Transport message hex conformance tests

#[test]
fn hex_init_syn() {
    let msg = InitSyn {
        version: 9,
        identifier: InitIdentifier {
            whatami: WhatAmI::Peer,
            zid: zid_from_bytes(&[0, 0, 0, 0, 0, 0, 0, 1]),
        },
        resolution: InitResolution::default(),
        qos: None,
        qos_link: None,
        auth: None,
        mlink: None,
        lowlatency: None,
        compression: None,
        patch: Patch::none(),
    };
    let len = crate::ZLen::z_len(&msg);
    let mut buf = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&msg, &mut &mut buf[..]).unwrap();
    let mut hex_buf = [0u8; BUF_SIZE * 2];
    let hex = hex_encode(&buf[..len], &mut hex_buf);
    let expected = "0109710000000000000001";
    assert_eq!(
        hex, expected,
        "\nInitSyn — actual: {hex}, expected: {expected}"
    );
    let decoded = <InitSyn as crate::ZDecode>::z_decode(&mut &buf[..len]).unwrap();
    assert_eq!(msg, decoded);
    let mut buf2 = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&decoded, &mut &mut buf2[..]).unwrap();
    assert_eq!(&buf[..len], &buf2[..len], "re-encode");
}

#[test]
fn hex_init_ack() {
    let msg = InitAck {
        version: 9,
        identifier: InitIdentifier {
            whatami: WhatAmI::Router,
            zid: zid_from_bytes(&[0, 0, 0, 0, 0, 0, 0, 2]),
        },
        resolution: InitResolution {
            resolution: Resolution::default(),
            batch_size: BatchSize(8192),
        },
        cookie: &[0xca, 0xfe],
        qos: None,
        qos_link: None,
        auth: None,
        mlink: None,
        lowlatency: None,
        compression: None,
        patch: Patch::none(),
    };
    let len = crate::ZLen::z_len(&msg);
    let mut buf = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&msg, &mut &mut buf[..]).unwrap();
    let mut hex_buf = [0u8; BUF_SIZE * 2];
    let hex = hex_encode(&buf[..len], &mut hex_buf);
    let expected = "61097000000000000000020a002002cafe";
    assert_eq!(
        hex, expected,
        "\nInitAck — actual: {hex}, expected: {expected}"
    );
    let decoded = <InitAck as crate::ZDecode>::z_decode(&mut &buf[..len]).unwrap();
    assert_eq!(msg, decoded);
    let mut buf2 = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&decoded, &mut &mut buf2[..]).unwrap();
    assert_eq!(&buf[..len], &buf2[..len], "re-encode");
}

#[test]
fn hex_open_syn() {
    let msg = OpenSyn {
        lease: core::time::Duration::from_secs(30),
        sn: 42,
        cookie: &[],
        qos: None,
        auth: None,
        mlink_syn: None,
        mlink_ack: None,
        lowlatency: None,
        compression: None,
    };
    let len = crate::ZLen::z_len(&msg);
    let mut buf = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&msg, &mut &mut buf[..]).unwrap();
    let mut hex_buf = [0u8; BUF_SIZE * 2];
    let hex = hex_encode(&buf[..len], &mut hex_buf);
    let expected = "421e2a00";
    assert_eq!(
        hex, expected,
        "\nOpenSyn — actual: {hex}, expected: {expected}"
    );
    let decoded = <OpenSyn as crate::ZDecode>::z_decode(&mut &buf[..len]).unwrap();
    assert_eq!(msg, decoded);
    let mut buf2 = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&decoded, &mut &mut buf2[..]).unwrap();
    assert_eq!(&buf[..len], &buf2[..len], "re-encode");
}

#[test]
fn hex_open_ack() {
    let msg = OpenAck {
        lease: core::time::Duration::from_secs(60),
        sn: 100,
        qos: None,
        auth: None,
        mlink_syn: None,
        mlink_ack: None,
        lowlatency: None,
        compression: None,
    };
    let len = crate::ZLen::z_len(&msg);
    let mut buf = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&msg, &mut &mut buf[..]).unwrap();
    let mut hex_buf = [0u8; BUF_SIZE * 2];
    let hex = hex_encode(&buf[..len], &mut hex_buf);
    let expected = "623c64";
    assert_eq!(
        hex, expected,
        "\nOpenAck — actual: {hex}, expected: {expected}"
    );
    let decoded = <OpenAck as crate::ZDecode>::z_decode(&mut &buf[..len]).unwrap();
    assert_eq!(msg, decoded);
    let mut buf2 = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&decoded, &mut &mut buf2[..]).unwrap();
    assert_eq!(&buf[..len], &buf2[..len], "re-encode");
}

#[test]
fn hex_keep_alive() {
    let msg = KeepAlive {};
    let len = crate::ZLen::z_len(&msg);
    let mut buf = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&msg, &mut &mut buf[..]).unwrap();
    let mut hex_buf = [0u8; BUF_SIZE * 2];
    let hex = hex_encode(&buf[..len], &mut hex_buf);
    let expected = "04";
    assert_eq!(
        hex, expected,
        "\nKeepAlive — actual: {hex}, expected: {expected}"
    );
    let decoded = <KeepAlive as crate::ZDecode>::z_decode(&mut &buf[..len]).unwrap();
    assert_eq!(msg, decoded);
    let mut buf2 = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&decoded, &mut &mut buf2[..]).unwrap();
    assert_eq!(&buf[..len], &buf2[..len], "re-encode");
}

#[test]
fn hex_close() {
    let msg = Close {
        reason: 0,
        behaviour: CloseBehaviour::Link,
    };
    let len = crate::ZLen::z_len(&msg);
    let mut buf = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&msg, &mut &mut buf[..]).unwrap();
    let mut hex_buf = [0u8; BUF_SIZE * 2];
    let hex = hex_encode(&buf[..len], &mut hex_buf);
    let expected = "0300";
    assert_eq!(
        hex, expected,
        "\nClose — actual: {hex}, expected: {expected}"
    );
    let decoded = <Close as crate::ZDecode>::z_decode(&mut &buf[..len]).unwrap();
    assert_eq!(msg, decoded);
    let mut buf2 = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&decoded, &mut &mut buf2[..]).unwrap();
    assert_eq!(&buf[..len], &buf2[..len], "re-encode");
}

// Network message hex conformance tests

#[test]
fn hex_put() {
    let msg = Put {
        timestamp: None,
        encoding: Encoding::text_plain(),
        sinfo: None,
        attachment: None,
        payload: b"hello",
    };
    let len = crate::ZLen::z_len(&msg);
    let mut buf = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&msg, &mut &mut buf[..]).unwrap();
    let mut hex_buf = [0u8; BUF_SIZE * 2];
    let hex = hex_encode(&buf[..len], &mut hex_buf);
    let expected = "41080568656c6c6f";
    assert_eq!(hex, expected, "\nPut — actual: {hex}, expected: {expected}");
    let decoded = <Put as crate::ZDecode>::z_decode(&mut &buf[..len]).unwrap();
    assert_eq!(msg, decoded);
    let mut buf2 = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&decoded, &mut &mut buf2[..]).unwrap();
    assert_eq!(&buf[..len], &buf2[..len], "re-encode");
}

#[test]
fn hex_del() {
    let msg = Del {
        timestamp: None,
        sinfo: None,
        attachment: None,
    };
    let len = crate::ZLen::z_len(&msg);
    let mut buf = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&msg, &mut &mut buf[..]).unwrap();
    let mut hex_buf = [0u8; BUF_SIZE * 2];
    let hex = hex_encode(&buf[..len], &mut hex_buf);
    let expected = "02";
    assert_eq!(hex, expected, "\nDel — actual: {hex}, expected: {expected}");
    let decoded = <Del as crate::ZDecode>::z_decode(&mut &buf[..len]).unwrap();
    assert_eq!(msg, decoded);
    let mut buf2 = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&decoded, &mut &mut buf2[..]).unwrap();
    assert_eq!(&buf[..len], &buf2[..len], "re-encode");
}

// Declare message hex conformance tests

#[test]
fn hex_declare_subscriber() {
    let body = DeclareBody::DeclareSubscriber(DeclareSubscriber {
        id: 1,
        wire_expr: WireExpr {
            scope: 0,
            mapping: Mapping::Sender,
            suffix: "example/sub",
        },
    });
    let msg = Declare {
        id: Some(42),
        qos: QoS::default(),
        timestamp: None,
        nodeid: NodeId::default(),
        body,
    };
    let len = crate::ZLen::z_len(&msg);
    let mut buf = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&msg, &mut &mut buf[..]).unwrap();
    let mut hex_buf = [0u8; BUF_SIZE * 2];
    let hex = hex_encode(&buf[..len], &mut hex_buf);
    let expected = "3e2a6201000b6578616d706c652f737562";
    assert_eq!(
        hex, expected,
        "\nDeclareSubscriber — actual: {hex}, expected: {expected}"
    );
    let decoded = <Declare as crate::ZDecode>::z_decode(&mut &buf[..len]).unwrap();
    assert_eq!(msg, decoded);
    let mut buf2 = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&decoded, &mut &mut buf2[..]).unwrap();
    assert_eq!(&buf[..len], &buf2[..len], "re-encode");
}

#[test]
fn hex_declare_key_expr() {
    let body = DeclareBody::DeclareKeyExpr(DeclareKeyExpr {
        id: 7,
        wire_expr: WireExpr {
            scope: 0,
            mapping: Mapping::Sender,
            suffix: "example/ke",
        },
    });
    let msg = Declare {
        id: None,
        qos: QoS::default(),
        timestamp: None,
        nodeid: NodeId::default(),
        body,
    };
    let len = crate::ZLen::z_len(&msg);
    let mut buf = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&msg, &mut &mut buf[..]).unwrap();
    let mut hex_buf = [0u8; BUF_SIZE * 2];
    let hex = hex_encode(&buf[..len], &mut hex_buf);
    let expected = "1e6007000a6578616d706c652f6b65";
    assert_eq!(
        hex, expected,
        "\nDeclareKeyExpr — actual: {hex}, expected: {expected}"
    );
    let decoded = <Declare as crate::ZDecode>::z_decode(&mut &buf[..len]).unwrap();
    assert_eq!(msg, decoded);
    let mut buf2 = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&decoded, &mut &mut buf2[..]).unwrap();
    assert_eq!(&buf[..len], &buf2[..len], "re-encode");
}

// Batch (concatenated messages) hex conformance test

#[test]
fn hex_transport_batch() {
    let keepalive = KeepAlive {};
    let close = Close {
        reason: 0,
        behaviour: CloseBehaviour::Link,
    };
    let len_ka = crate::ZLen::z_len(&keepalive);
    let len_cl = crate::ZLen::z_len(&close);
    let total = len_ka + len_cl;
    let mut buf = [0u8; BUF_SIZE];
    crate::ZEncode::z_encode(&keepalive, &mut &mut buf[..]).unwrap();
    crate::ZEncode::z_encode(&close, &mut &mut buf[len_ka..]).unwrap();
    let mut hex_buf = [0u8; BUF_SIZE * 2];
    let hex = hex_encode(&buf[..total], &mut hex_buf);
    let expected = "040300";
    assert_eq!(
        hex, expected,
        "\nTransport batch — actual: {hex}, expected: {expected}"
    );
    let decoded_ka = <KeepAlive as crate::ZDecode>::z_decode(&mut &buf[..]).unwrap();
    assert_eq!(keepalive, decoded_ka);
    let decoded_cl = <Close as crate::ZDecode>::z_decode(&mut &buf[len_ka..]).unwrap();
    assert_eq!(close, decoded_cl);
}
