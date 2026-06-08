#![allow(clippy::unwrap_used, clippy::expect_used)]
use core::time::Duration;
use std::fs;
use std::path::Path;

use core::num::NonZeroU128;
use uhlc::{ID, NTP64};
use zenoh_proto::{ZEncode, exts::*, fields::*, keyexpr, msgs::*};
use zenoh_sansio::{Transport, ZTransportTx};

fn make_corpus_dir(name: &str) -> std::path::PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fuzz")
        .join("corpus")
        .join(name);
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("cannot create {dir:?}: {e}"));
    dir
}

fn zid(val: u128) -> ZenohIdProto {
    ZenohIdProto::try_from(&val.to_le_bytes()[..]).unwrap()
}

fn write_seed(dir: &Path, name: &str, data: &[u8]) {
    fs::write(dir.join(name), data).unwrap_or_else(|e| panic!("cannot write {name}: {e}"));
}

fn make_timestamp() -> Timestamp {
    Timestamp::new(NTP64(0), ID::from(NonZeroU128::new(1).unwrap()))
}

fn encode_and_write<M: ZEncode>(dir: &Path, name: &str, msg: &M, buf: &mut [u8]) {
    let orig_len = buf.len();
    let mut w = &mut buf[..];
    msg.z_encode(&mut w).unwrap();
    let used = orig_len - w.len();
    write_seed(dir, name, &buf[..used]);
}

fn main() {
    // ── decode_message corpus ──────────────────────────────────────────────
    let msg_dir = make_corpus_dir("decode_message");

    let mut buf256 = [0u8; 256];
    let mut buf128 = [0u8; 128];
    let mut buf16 = [0u8; 16];
    let mut buf512 = [0u8; 512];

    // Put (minimal)
    encode_and_write(
        &msg_dir,
        "put_empty.bin",
        &Put {
            payload: &[],
            ..Default::default()
        },
        &mut buf128,
    );

    // Put (with payload and string encoding)
    encode_and_write(
        &msg_dir,
        "put_hello.bin",
        &Put {
            encoding: Encoding::string(),
            payload: b"hello",
            ..Default::default()
        },
        &mut buf256,
    );

    // Put (with timestamp)
    encode_and_write(
        &msg_dir,
        "put_timestamp.bin",
        &Put {
            timestamp: Some(make_timestamp()),
            payload: b"data",
            ..Default::default()
        },
        &mut buf256,
    );

    // Del (minimal)
    encode_and_write(&msg_dir, "del_empty.bin", &Del::default(), &mut buf128);

    // Del (with timestamp)
    encode_and_write(
        &msg_dir,
        "del_timestamp.bin",
        &Del {
            timestamp: Some(make_timestamp()),
            ..Default::default()
        },
        &mut buf128,
    );

    // Declare (DeclareKeyExpr)
    let ke = keyexpr::from_str_unchecked("a/b");
    encode_and_write(
        &msg_dir,
        "declare_keyexpr.bin",
        &Declare {
            body: DeclareBody::DeclareKeyExpr(DeclareKeyExpr {
                id: 1,
                wire_expr: WireExpr::from(ke),
            }),
            ..Default::default()
        },
        &mut buf256,
    );

    // Declare (UndeclareKeyExpr)
    encode_and_write(
        &msg_dir,
        "undeclare_keyexpr.bin",
        &Declare {
            body: DeclareBody::UndeclareKeyExpr(UndeclareKeyExpr { id: 1 }),
            ..Default::default()
        },
        &mut buf256,
    );

    // Declare (DeclareSubscriber)
    encode_and_write(
        &msg_dir,
        "declare_subscriber.bin",
        &Declare {
            body: DeclareBody::DeclareSubscriber(DeclareSubscriber {
                id: 42,
                wire_expr: WireExpr::from(ke),
            }),
            ..Default::default()
        },
        &mut buf256,
    );

    // Declare (UndeclareSubscriber)
    encode_and_write(
        &msg_dir,
        "undeclare_subscriber.bin",
        &Declare {
            body: DeclareBody::UndeclareSubscriber(UndeclareSubscriber {
                id: 42,
                ..Default::default()
            }),
            ..Default::default()
        },
        &mut buf256,
    );

    // Declare (DeclareQueryable)
    encode_and_write(
        &msg_dir,
        "declare_queryable.bin",
        &Declare {
            body: DeclareBody::DeclareQueryable(DeclareQueryable {
                id: 99,
                wire_expr: WireExpr::from(ke),
                ..Default::default()
            }),
            ..Default::default()
        },
        &mut buf256,
    );

    // Declare (UndeclareQueryable)
    encode_and_write(
        &msg_dir,
        "undeclare_queryable.bin",
        &Declare {
            body: DeclareBody::UndeclareQueryable(UndeclareQueryable {
                id: 99,
                ..Default::default()
            }),
            ..Default::default()
        },
        &mut buf256,
    );

    // Declare (DeclareToken)
    encode_and_write(
        &msg_dir,
        "declare_token.bin",
        &Declare {
            body: DeclareBody::DeclareToken(DeclareToken {
                id: 7,
                wire_expr: WireExpr::from(ke),
            }),
            ..Default::default()
        },
        &mut buf256,
    );

    // Declare (UndeclareToken)
    encode_and_write(
        &msg_dir,
        "undeclare_token.bin",
        &Declare {
            body: DeclareBody::UndeclareToken(UndeclareToken {
                id: 7,
                ..Default::default()
            }),
            ..Default::default()
        },
        &mut buf256,
    );

    // Declare (DeclareFinal)
    encode_and_write(
        &msg_dir,
        "declare_final.bin",
        &Declare {
            body: DeclareBody::DeclareFinal(DeclareFinal {}),
            ..Default::default()
        },
        &mut buf256,
    );

    // Request (Query)
    encode_and_write(
        &msg_dir,
        "request_query.bin",
        &Request {
            id: 1,
            wire_expr: WireExpr::from(ke),
            payload: RequestBody::Query(Query::default()),
            ..Default::default()
        },
        &mut buf512,
    );

    // Response (Err)
    encode_and_write(
        &msg_dir,
        "response_err.bin",
        &Response {
            rid: 1,
            wire_expr: WireExpr::from(ke),
            payload: ResponseBody::Err(Err::default()),
            ..Default::default()
        },
        &mut buf512,
    );

    // Response (Reply)
    encode_and_write(
        &msg_dir,
        "response_reply.bin",
        &Response {
            rid: 1,
            wire_expr: WireExpr::from(ke),
            payload: ResponseBody::Reply(Reply::default()),
            ..Default::default()
        },
        &mut buf512,
    );

    // ResponseFinal
    encode_and_write(
        &msg_dir,
        "response_final.bin",
        &ResponseFinal {
            rid: 1,
            ..Default::default()
        },
        &mut buf256,
    );

    // KeepAlive
    encode_and_write(&msg_dir, "keepalive.bin", &KeepAlive, &mut buf16);

    // Close (Link)
    encode_and_write(
        &msg_dir,
        "close_link.bin",
        &Close {
            reason: 0,
            behaviour: CloseBehaviour::Link,
        },
        &mut buf16,
    );

    // Close (Session)
    encode_and_write(
        &msg_dir,
        "close_session.bin",
        &Close {
            reason: 1,
            behaviour: CloseBehaviour::Session,
        },
        &mut buf16,
    );

    // ── decode_frame & transport_decode corpus ────────────────────────────
    let frame_dir = make_corpus_dir("decode_frame");
    let trans_dir = make_corpus_dir("transport_decode");

    let encode_transport = |msg: TransportMessage| -> Vec<u8> {
        let mut transport = Transport::builder(vec![0u8; 4096]).codec();
        match msg {
            TransportMessage::InitSyn(ref syn) => transport.tx.init_syn(syn),
            TransportMessage::InitAck(ref ack) => transport.tx.init_ack(ack),
            TransportMessage::OpenSyn(ref syn) => transport.tx.open_syn(syn),
            TransportMessage::OpenAck(ref ack) => transport.tx.open_ack(ack),
            TransportMessage::Close(_) => transport.tx.close(),
            TransportMessage::KeepAlive(_) => transport.tx.keepalive(),
        };
        transport.tx.flush_raw().unwrap_or_default().to_vec()
    };

    let encode_frame = |body: NetworkBody<'_>| -> Vec<u8> {
        let mut transport = Transport::builder(vec![0u8; 4096]).codec();
        let msg = NetworkMessage {
            reliability: Reliability::Reliable,
            qos: QoS::default(),
            body,
        };
        transport.tx.encode_ref(core::iter::once(msg.as_ref()));
        transport.tx.flush_raw().unwrap_or_default().to_vec()
    };

    // Push(Put) frame
    {
        let body = NetworkBody::Push(Push {
            wire_expr: WireExpr::from(ke),
            payload: PushBody::Put(Put {
                payload: b"hello world",
                ..Default::default()
            }),
            ..Default::default()
        });
        let data = encode_frame(body);
        write_seed(&frame_dir, "push_put.bin", &data);
        write_seed(&trans_dir, "push_put.bin", &data);
    }

    // Push(Del) frame
    {
        let body = NetworkBody::Push(Push {
            wire_expr: WireExpr::from(ke),
            payload: PushBody::Del(Del::default()),
            ..Default::default()
        });
        let data = encode_frame(body);
        write_seed(&frame_dir, "push_del.bin", &data);
        write_seed(&trans_dir, "push_del.bin", &data);
    }

    // Request(Query) frame
    {
        let body = NetworkBody::Request(Request {
            id: 1,
            wire_expr: WireExpr::from(ke),
            payload: RequestBody::Query(Query::default()),
            ..Default::default()
        });
        let data = encode_frame(body);
        write_seed(&frame_dir, "request.bin", &data);
        write_seed(&trans_dir, "request.bin", &data);
    }

    // Response(Reply) frame
    {
        let body = NetworkBody::Response(Response {
            rid: 1,
            wire_expr: WireExpr::from(ke),
            payload: ResponseBody::Reply(Reply::default()),
            ..Default::default()
        });
        let data = encode_frame(body);
        write_seed(&frame_dir, "response_reply.bin", &data);
        write_seed(&trans_dir, "response_reply.bin", &data);
    }

    // ResponseFinal frame
    {
        let body = NetworkBody::ResponseFinal(ResponseFinal {
            rid: 1,
            ..Default::default()
        });
        let data = encode_frame(body);
        write_seed(&frame_dir, "response_final.bin", &data);
        write_seed(&trans_dir, "response_final.bin", &data);
    }

    // Declare(DeclareKeyExpr) frame
    {
        let body = NetworkBody::Declare(Declare {
            body: DeclareBody::DeclareKeyExpr(DeclareKeyExpr {
                id: 1,
                wire_expr: WireExpr::from(ke),
            }),
            ..Default::default()
        });
        let data = encode_frame(body);
        write_seed(&frame_dir, "declare_keyexpr.bin", &data);
        write_seed(&trans_dir, "declare_keyexpr.bin", &data);
    }

    // Interest frame
    {
        let body = NetworkBody::Interest(Interest {
            id: 1,
            mode: InterestMode::Final,
            inner: InterestInner::default(),
            ..Default::default()
        });
        let data = encode_frame(body);
        write_seed(&frame_dir, "interest_final.bin", &data);
        write_seed(&trans_dir, "interest_final.bin", &data);
    }

    // KeepAlive transport message
    {
        let data = encode_transport(TransportMessage::KeepAlive(KeepAlive));
        write_seed(&frame_dir, "keepalive.bin", &data);
        write_seed(&trans_dir, "keepalive.bin", &data);
    }

    // Close transport message
    {
        let data = encode_transport(TransportMessage::Close(Close::default()));
        write_seed(&frame_dir, "close.bin", &data);
        write_seed(&trans_dir, "close.bin", &data);
    }

    // ── establishment corpus ──────────────────────────────────────────────
    let est_dir = make_corpus_dir("establishment");
    let est_client_dir = make_corpus_dir("establishment_client");

    let a_zid = zid(1);
    let b_zid = zid(2);

    let encode_init_syn = |whatami: WhatAmI, z: ZenohIdProto, batch: u16| -> Vec<u8> {
        let mut buf = [0u8; 512];
        let msg = InitSyn {
            version: zenoh_proto::VERSION,
            identifier: InitIdentifier { whatami, zid: z },
            resolution: InitResolution {
                resolution: Resolution::default(),
                batch_size: BatchSize(batch),
            },
            ..Default::default()
        };
        let orig = buf.len();
        let mut w = &mut buf[..];
        msg.z_encode(&mut w).unwrap();
        let used = orig - w.len();
        buf[..used].to_vec()
    };

    let encode_init_ack =
        |whatami: WhatAmI, z: ZenohIdProto, batch: u16, cookie: &[u8]| -> Vec<u8> {
            let mut buf = [0u8; 1024];
            let msg = InitAck {
                version: zenoh_proto::VERSION,
                identifier: InitIdentifier { whatami, zid: z },
                resolution: InitResolution {
                    resolution: Resolution::default(),
                    batch_size: BatchSize(batch),
                },
                cookie,
                ..Default::default()
            };
            let orig = buf.len();
            let mut w = &mut buf[..];
            msg.z_encode(&mut w).unwrap();
            let used = orig - w.len();
            buf[..used].to_vec()
        };

    let encode_open_syn = |sn: u32, cookie: &[u8], lease_secs: u64| -> Vec<u8> {
        let mut buf = [0u8; 1024];
        let msg = OpenSyn {
            lease: Duration::from_secs(lease_secs),
            sn,
            cookie,
            ..Default::default()
        };
        let orig = buf.len();
        let mut w = &mut buf[..];
        msg.z_encode(&mut w).unwrap();
        let used = orig - w.len();
        buf[..used].to_vec()
    };

    let encode_open_ack = |sn: u32, lease_secs: u64| -> Vec<u8> {
        let mut buf = [0u8; 1024];
        let msg = OpenAck {
            lease: Duration::from_secs(lease_secs),
            sn,
            ..Default::default()
        };
        let orig = buf.len();
        let mut w = &mut buf[..];
        msg.z_encode(&mut w).unwrap();
        let used = orig - w.len();
        buf[..used].to_vec()
    };

    // Handshake variant 1: Client ↔ Server
    {
        let init_syn = encode_init_syn(WhatAmI::Client, b_zid, 1024);
        let open_syn = encode_open_syn(0, &init_syn, 30);
        let mut server_input = Vec::new();
        server_input.extend_from_slice(&init_syn);
        server_input.extend_from_slice(&open_syn);
        write_seed(&est_dir, "handshake_client_to_server.bin", &server_input);

        let init_ack = encode_init_ack(WhatAmI::Peer, a_zid, 512, &init_syn);
        let open_ack = encode_open_ack(0, 30);
        let mut client_input = Vec::new();
        client_input.extend_from_slice(&init_ack);
        client_input.extend_from_slice(&open_ack);
        write_seed(
            &est_client_dir,
            "handshake_server_to_client.bin",
            &client_input,
        );
    }

    // Handshake variant 2: Peer ↔ Peer
    {
        let init_syn = encode_init_syn(WhatAmI::Peer, b_zid, 2048);
        let open_syn = encode_open_syn(42, &init_syn, 37);
        let mut server_input = Vec::new();
        server_input.extend_from_slice(&init_syn);
        server_input.extend_from_slice(&open_syn);
        write_seed(&est_dir, "handshake_peer.bin", &server_input);

        let init_ack = encode_init_ack(WhatAmI::Peer, a_zid, 2048, &init_syn);
        let open_ack = encode_open_ack(42, 37);
        let mut client_input = Vec::new();
        client_input.extend_from_slice(&init_ack);
        client_input.extend_from_slice(&open_ack);
        write_seed(&est_client_dir, "handshake_peer.bin", &client_input);
    }

    // ── keyexpr corpus ────────────────────────────────────────────────────
    let ke_dir = make_corpus_dir("keyexpr");

    let keyexprs = [
        "a",
        "a/b",
        "a/b/c",
        "demo/example",
        "hello/world",
        "a/**",
        "*/b",
        "a/*/c",
        "**",
        "*",
        "a/b/c/d/e/f",
        "test/path/with/many/segments",
        "a/b?",
        "a/b$",
        "a/b[c]",
        "a/b[c-e]",
        "single",
        "with-dashes",
        "with_underscores",
    ];
    for (i, ke) in keyexprs.iter().enumerate() {
        write_seed(&ke_dir, &format!("keyexpr_{i:02}.bin"), ke.as_bytes());
    }

    println!("Corpus generation complete.");
    println!(
        "  decode_message: {} seeds",
        fs::read_dir(&msg_dir).unwrap().count()
    );
    println!(
        "  decode_frame:   {} seeds",
        fs::read_dir(&frame_dir).unwrap().count()
    );
    println!(
        "  transport_decode: {} seeds",
        fs::read_dir(&trans_dir).unwrap().count()
    );
    println!(
        "  establishment:    {} seeds",
        fs::read_dir(&est_dir).unwrap().count()
    );
    println!(
        "  establishment_client: {} seeds",
        fs::read_dir(&est_client_dir).unwrap().count()
    );
    println!(
        "  keyexpr:          {} seeds",
        fs::read_dir(&ke_dir).unwrap().count()
    );
}
