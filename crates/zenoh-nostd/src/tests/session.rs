//! Session behavior against a scripted fake router.
//!
//! Baseline tests pin behavior that works today. Tests marked `#[ignore]` are
//! reproducers for known bugs: they fail on the current code and are
//! un-ignored by the change that fixes them. Run them with
//! `cargo test -p zenoh-nostd -- --ignored`.

use core::{cell::RefCell, time::Duration};
use std::{boxed::Box, vec, vec::Vec};

use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
use embassy_time::{Instant, Timer, with_timeout};
use zenoh_proto::{exts::Value, fields::WireExpr, keyexpr, msgs::*};

use super::harness::{Seen, TestConfig, WAIT, run};
use crate::session::{FixedCapacityGetResponse, FixedCapacityQueryableQuery, GetResponse};

fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

fn ke(s: &'static str) -> &'static keyexpr {
    keyexpr::new(s).unwrap()
}

/// A shared log that `'static` callbacks can append to.
type Log<T> = &'static RefCell<Vec<T>>;

fn log<T>() -> Log<T> {
    leak(RefCell::new(Vec::new()))
}

/// Polls `cond` until it holds, panicking with `what` after [`WAIT`].
async fn eventually(what: &str, cond: impl Fn() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !cond() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        Timer::after_millis(1).await;
    }
}

fn reply<'a>(rid: u32, ke: &'a keyexpr, payload: &'a [u8]) -> NetworkBody<'a> {
    NetworkBody::Response(Response {
        rid,
        wire_expr: WireExpr::from(ke),
        payload: ResponseBody::Reply(Reply {
            payload: PushBody::Put(Put {
                payload,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    })
}

fn request<'a>(id: u32, ke: &'a keyexpr) -> NetworkBody<'a> {
    NetworkBody::Request(Request {
        id,
        wire_expr: WireExpr::from(ke),
        ..Default::default()
    })
}

fn request_id(seen: &Seen) -> u32 {
    match seen {
        Seen::Request { id, .. } => *id,
        other => panic!("expected a Request, got {other:?}"),
    }
}

// ── Baseline: behavior that works today ─────────────────────────────────────

#[test]
fn put_sends_push() {
    run(|session, router| async move {
        session.put(ke("t/put"), b"hello").finish().await.unwrap();

        let seen = router
            .wait_for("Push", |s| matches!(s, Seen::Push { .. }))
            .await;
        assert_eq!(
            seen,
            Seen::Push {
                ke: "t/put".into(),
                payload: b"hello".to_vec(),
                encoding: 0,
                qos: zenoh_proto::exts::QoS::default().inner,
                reliable: true,
            }
        );
    });
}

#[test]
fn subscriber_receives_remote_push() {
    run(|session, router| async move {
        let got: Log<Vec<u8>> = log();
        let _sub = session
            .declare_subscriber(ke("t/sub/*"))
            .callback_sync(move |s| got.borrow_mut().push(s.payload().to_vec()))
            .finish()
            .await
            .unwrap();

        router
            .wait_for(
                "DeclareSubscriber",
                |s| matches!(s, Seen::DeclareSubscriber { ke, .. } if ke == "t/sub/*"),
            )
            .await;

        router.push("t/sub/a", b"one").await;
        router.push("t/other", b"ignored").await;
        router.push("t/sub/b", b"two").await;

        eventually("two samples", || got.borrow().len() == 2).await;
        assert_eq!(*got.borrow(), vec![b"one".to_vec(), b"two".to_vec()]);
    });
}

#[test]
fn get_callback_receives_remote_reply() {
    run(|session, router| async move {
        let got: Log<Vec<u8>> = log();
        session
            .get(ke("t/get"))
            .callback_sync(move |r| {
                if let GetResponse::Ok(s) = r {
                    got.borrow_mut().push(s.payload().to_vec());
                }
            })
            .finish()
            .await
            .unwrap();

        let id = request_id(
            &router
                .wait_for("Request", |s| matches!(s, Seen::Request { .. }))
                .await,
        );
        router.send(reply(id, ke("t/get"), b"answer")).await;

        eventually("one reply", || got.borrow().len() == 1).await;
        assert_eq!(*got.borrow(), vec![b"answer".to_vec()]);
    });
}

#[test]
fn queryable_channel_replies_and_finalizes() {
    run(|session, router| async move {
        type Q = FixedCapacityQueryableQuery<TestConfig, 64, 64, 64>;
        let ch: &'static Channel<NoopRawMutex, Q, 4> = leak(Channel::new());
        let queryable = session
            .declare_queryable(ke("t/q/*"))
            .channel(ch.dyn_sender(), ch.dyn_receiver())
            .finish()
            .await
            .unwrap();

        router
            .wait_for("DeclareQueryable", |s| {
                matches!(s, Seen::DeclareQueryable { .. })
            })
            .await;
        router.send(request(42, ke("t/q/1"))).await;

        let mut query = queryable.recv().await.unwrap();
        assert_eq!(query.keyexpr().as_str(), "t/q/1");
        query.reply(ke("t/q/1"), b"value").await.unwrap();
        query.finalize().await.unwrap();

        router
            .wait_for("Reply", |s| {
                *s == Seen::Reply {
                    rid: 42,
                    ke: "t/q/1".into(),
                    payload: b"value".to_vec(),
                }
            })
            .await;
        router
            .wait_for("ResponseFinal", |s| *s == Seen::ResponseFinal { rid: 42 })
            .await;
    });
}

// ── Reproducers: these fail on the current code ─────────────────────────────

/// A get's channel must end once the router signals `ResponseFinal`, not
/// when the get's timeout expires. Once ended, it stays ended.
#[test]
fn get_channel_closes_on_response_final() {
    run(|session, router| async move {
        type R = FixedCapacityGetResponse<64, 64>;
        let ch: &'static Channel<NoopRawMutex, R, 4> = leak(Channel::new());
        let responses = session
            .get(ke("t/get"))
            .timeout(Duration::from_secs(3))
            .channel(ch.dyn_sender(), ch.dyn_receiver())
            .finish()
            .await
            .unwrap();

        let id = request_id(
            &router
                .wait_for("Request", |s| matches!(s, Seen::Request { .. }))
                .await,
        );
        router.send(reply(id, ke("t/get"), b"answer")).await;
        router
            .send(NetworkBody::ResponseFinal(ResponseFinal {
                rid: id,
                ..Default::default()
            }))
            .await;

        assert!(responses.recv().await.is_some(), "expected the reply");
        let end = with_timeout(embassy_time::Duration::from_millis(500), responses.recv()).await;
        assert!(
            matches!(end, Ok(None)),
            "receiver should close right after ResponseFinal, not at the timeout"
        );

        // Ended for good: no waiting for the timeout on later calls either.
        assert!(responses.try_recv().is_none());
        let again = with_timeout(embassy_time::Duration::from_millis(100), responses.recv()).await;
        assert!(matches!(again, Ok(None)));
    });
}

/// A get callback sees its replies, then exactly one `Final`, then nothing:
/// a reply that arrives after `ResponseFinal` is not delivered.
#[test]
fn get_callback_receives_final_after_replies() {
    #[derive(Debug, PartialEq)]
    enum Event {
        Ok(Vec<u8>),
        Final,
    }

    run(|session, router| async move {
        let events: Log<Event> = log();
        session
            .get(ke("t/get"))
            .callback_sync(move |r| {
                events.borrow_mut().push(match r {
                    GetResponse::Ok(s) => Event::Ok(s.payload().to_vec()),
                    GetResponse::Final => Event::Final,
                    GetResponse::Err(_) => panic!("unexpected Err reply"),
                })
            })
            .finish()
            .await
            .unwrap();

        let id = request_id(
            &router
                .wait_for("Request", |s| matches!(s, Seen::Request { .. }))
                .await,
        );
        router.send(reply(id, ke("t/get"), b"one")).await;
        router.send(reply(id, ke("t/get"), b"two")).await;
        router
            .send(NetworkBody::ResponseFinal(ResponseFinal {
                rid: id,
                ..Default::default()
            }))
            .await;
        router.send(reply(id, ke("t/get"), b"late")).await;

        eventually("Final", || events.borrow().contains(&Event::Final)).await;
        // Let the late reply be processed, if it were going to be delivered.
        Timer::after_millis(20).await;
        assert_eq!(
            *events.borrow(),
            vec![
                Event::Ok(b"one".to_vec()),
                Event::Ok(b"two".to_vec()),
                Event::Final
            ]
        );
    });
}

/// A callback-mode queryable must be able to finish a query. The callback
/// gets `&QueryableQuery` but `finalize` needs `&mut self`, and nothing
/// finalizes on its behalf, so `ResponseFinal` is never sent and the remote
/// querier waits out its whole timeout.
#[test]
fn callback_queryable_sends_response_final() {
    run(|session, router| async move {
        let _queryable = session
            .declare_queryable(ke("t/q/*"))
            .callback(async |q| {
                q.reply(ke("t/q/1"), b"value").await.unwrap();
            })
            .finish()
            .await
            .unwrap();

        router
            .wait_for("DeclareQueryable", |s| {
                matches!(s, Seen::DeclareQueryable { .. })
            })
            .await;
        router.send(request(7, ke("t/q/1"))).await;

        router
            .wait_for("Reply", |s| matches!(s, Seen::Reply { rid: 7, .. }))
            .await;
        router
            .wait_for("ResponseFinal", |s| *s == Seen::ResponseFinal { rid: 7 })
            .await;
    });
}

/// A queryable must keep serving queries for as long as it is declared. The
/// session tracks each in-flight query, and that tracking must be released
/// once the query is finalized, or the table fills up after `CAPACITY` (8 in
/// the test config) queries and the next one kills the session.
#[test]
fn queryable_serves_more_queries_than_capacity() {
    run(|session, router| async move {
        type Q = FixedCapacityQueryableQuery<TestConfig, 64, 64, 64>;
        let ch: &'static Channel<NoopRawMutex, Q, 4> = leak(Channel::new());
        let queryable = session
            .declare_queryable(ke("t/q/*"))
            .channel(ch.dyn_sender(), ch.dyn_receiver())
            .finish()
            .await
            .unwrap();

        router
            .wait_for("DeclareQueryable", |s| {
                matches!(s, Seen::DeclareQueryable { .. })
            })
            .await;

        for rid in 100..120 {
            router.send(request(rid, ke("t/q/1"))).await;
            let mut query = queryable.recv().await.unwrap();
            query.finalize().await.unwrap();
            router
                .wait_for("ResponseFinal", |s| *s == Seen::ResponseFinal { rid })
                .await;
        }
    });
}

/// A query that matches none of the session's queryables (for instance one
/// that raced with an undeclare) must still be finalized, or the remote
/// querier waits out its whole timeout.
#[test]
fn unmatched_query_sends_response_final() {
    run(|session, router| async move {
        let _queryable = session
            .declare_queryable(ke("t/q/*"))
            .callback_sync(|_| {})
            .finish()
            .await
            .unwrap();

        router
            .wait_for("DeclareQueryable", |s| {
                matches!(s, Seen::DeclareQueryable { .. })
            })
            .await;
        router.send(request(9, ke("t/elsewhere"))).await;

        router
            .wait_for("ResponseFinal", |s| *s == Seen::ResponseFinal { rid: 9 })
            .await;
    });
}

/// When a channel-mode queryable can't take a query (here, its payload is
/// larger than the owned query's buffer), the query is dropped. It must
/// still be finalized, since nobody else ever will.
#[test]
fn dropped_channel_query_sends_response_final() {
    run(|session, router| async move {
        type Q = FixedCapacityQueryableQuery<TestConfig, 64, 64, 4>;
        let ch: &'static Channel<NoopRawMutex, Q, 4> = leak(Channel::new());
        let queryable = session
            .declare_queryable(ke("t/q/*"))
            .channel(ch.dyn_sender(), ch.dyn_receiver())
            .finish()
            .await
            .unwrap();

        router
            .wait_for("DeclareQueryable", |s| {
                matches!(s, Seen::DeclareQueryable { .. })
            })
            .await;
        router
            .send(NetworkBody::Request(Request {
                id: 5,
                wire_expr: WireExpr::from(ke("t/q/1")),
                payload: RequestBody::Query(Query {
                    body: Some(Value {
                        payload: b"far too large for four bytes",
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }))
            .await;

        router
            .wait_for("ResponseFinal", |s| *s == Seen::ResponseFinal { rid: 5 })
            .await;
        assert!(queryable.try_recv().is_none());
    });
}

/// A publication must reach subscribers declared on the same session. A
/// router never echoes a message back to its sender, so the session has to
/// deliver locally.
#[test]
#[ignore = "reproducer: no session-local delivery for put"]
fn put_reaches_same_session_subscriber() {
    run(|session, router| async move {
        let got: Log<Vec<u8>> = log();
        let _sub = session
            .declare_subscriber(ke("t/local"))
            .callback_sync(move |s| got.borrow_mut().push(s.payload().to_vec()))
            .finish()
            .await
            .unwrap();

        session.put(ke("t/local"), b"mine").finish().await.unwrap();

        // It still goes to the network for remote subscribers.
        router
            .wait_for("Push", |s| matches!(s, Seen::Push { .. }))
            .await;
        eventually("local delivery", || got.borrow().len() == 1).await;
        assert_eq!(*got.borrow(), vec![b"mine".to_vec()]);
    });
}

/// A get must reach queryables declared on the same session, and complete
/// once they finalize. A router never routes a Request back to its sender.
#[test]
#[ignore = "reproducer: no session-local delivery for get"]
fn get_reaches_same_session_queryable() {
    run(|session, router| async move {
        type Q = FixedCapacityQueryableQuery<TestConfig, 64, 64, 64>;
        let qch: &'static Channel<NoopRawMutex, Q, 4> = leak(Channel::new());
        let queryable = session
            .declare_queryable(ke("t/lq/*"))
            .channel(qch.dyn_sender(), qch.dyn_receiver())
            .finish()
            .await
            .unwrap();

        let got: Log<Vec<u8>> = log();
        session
            .get(ke("t/lq/1"))
            .timeout(Duration::from_secs(3))
            .callback_sync(move |r| {
                if let GetResponse::Ok(s) = r {
                    got.borrow_mut().push(s.payload().to_vec());
                }
            })
            .finish()
            .await
            .unwrap();

        let mut query = with_timeout(embassy_time::Duration::from_millis(500), queryable.recv())
            .await
            .expect("local queryable never received the query")
            .unwrap();
        query.reply(ke("t/lq/1"), b"local").await.unwrap();
        query.finalize().await.unwrap();

        eventually("local reply", || got.borrow().len() == 1).await;
        assert_eq!(*got.borrow(), vec![b"local".to_vec()]);
        // The router is not told about a local reply.
        assert!(
            !router
                .seen()
                .await
                .iter()
                .any(|s| matches!(s, Seen::Reply { .. })),
            "a local reply must not be sent to the network"
        );
    });
}
