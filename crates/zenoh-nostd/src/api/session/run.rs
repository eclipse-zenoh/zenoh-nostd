use core::convert::Infallible;

use embassy_futures::select::{Either, select};
use zenoh_proto::{exts::Value, msgs::*, *};

use crate::{
    api::{
        callbacks::{ZCallbacks, ZDynCallback},
        query::QueryableQuery,
        session::{Origin, Session, SessionState, queryable::query_key},
    },
    config::ZSessionConfig,
    session::{GetResponse, Sample},
};

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    /// Drives the session: dispatches messages from the network and between
    /// the session's own entities, and keeps the link alive. Returns when the
    /// link closes or fails.
    pub async fn run(&self) -> core::result::Result<(), SessionError> {
        let network = async {
            self.driver
                .run(&self.state, async |_, state, msg, _| {
                    self.dispatch(state, msg.body, Origin::Remote).await
                })
                .await
                .map_err(|e| e.flatten_map())
        };

        match select(network, self.run_loopback()).await {
            Either::First(result) => result,
            Either::Second(Err(e)) => Err(e),
            Either::Second(Ok(never)) => match never {},
        }
    }

    async fn run_loopback(&self) -> core::result::Result<Infallible, SessionError> {
        loop {
            self.loopback.ready().await;
            let mut state = self.state().await;
            self.loopback
                .drain(async |body| self.dispatch(&mut state, body, Origin::Local).await)
                .await?;
        }
    }

    async fn dispatch(
        &self,
        state: &mut SessionState<'res, Config>,
        body: NetworkBody<'_>,
        origin: Origin,
    ) -> core::result::Result<(), SessionError> {
        match body {
            NetworkBody::Push(Push {
                wire_expr,
                payload: PushBody::Put(Put { payload, .. }),
                ..
            }) => {
                let ke = wire_expr.suffix;
                let ke = keyexpr::new(ke)?;
                let sample = Sample::new(ke, payload);

                for cb in state.sub_callbacks.intersects(ke) {
                    cb.call(&sample).await;
                }
            }
            NetworkBody::Response(Response {
                rid,
                wire_expr,
                payload,
                ..
            }) => {
                let ke = wire_expr.suffix;
                let ke = keyexpr::new(ke)?;
                let response = match payload {
                    ResponseBody::Reply(Reply {
                        payload: PushBody::Put(Put { payload, .. }),
                        ..
                    }) => GetResponse::Ok(Sample::new(ke, payload)),
                    ResponseBody::Err(Err { payload, .. }) => {
                        GetResponse::Err(Sample::new(ke, payload))
                    }
                };

                if let Some(cb) = state.get_callbacks.get(rid) {
                    cb.call(&response).await;
                }
            }
            NetworkBody::ResponseFinal(ResponseFinal { rid, .. }) => {
                // A get waits for one ResponseFinal from the network, plus
                // one from the session itself if it reached local queryables.
                // Once all are in, tell the get it is complete (this also
                // ends a channel-mode get's `recv`), then forget it.
                if state.get_callbacks.decrease(rid) {
                    if let Some(cb) = state.get_callbacks.get(rid) {
                        cb.call(&GetResponse::Final).await;
                    }
                    state.get_callbacks.remove(rid)?;
                }
            }
            NetworkBody::Request(Request {
                id,
                wire_expr,
                payload:
                    RequestBody::Query(Query {
                        parameters, body, ..
                    }),
                ..
            }) => {
                let ke = wire_expr.suffix;
                let ke = keyexpr::new(ke)?;
                let query = QueryableQuery::new(
                    self,
                    id,
                    origin,
                    ke,
                    if parameters.is_empty() {
                        None
                    } else {
                        Some(parameters)
                    },
                    match body {
                        Some(Value { payload, .. }) => Some(payload),
                        None => None,
                    },
                );

                // A callback-mode queryable is done with the query when its
                // callback returns. A channel-mode one hands the query off,
                // and its owned copy finalizes later.
                let mut handed_off = 0;
                for cb in state.queryable_callbacks.intersects(ke) {
                    cb.call(&query).await;
                    if query.take_handed_off() {
                        handed_off += 1;
                    }
                }

                // Finalize now unless an owned copy is still out. The state
                // lock is held, so no owner can finalize before the counter is
                // in place.
                let pending = handed_off > 0
                    && state
                        .queryable_callbacks
                        .set_counter(query_key(id, origin), handed_off)
                        .inspect_err(|e| {
                            zenoh_proto::error!(
                                "{}: Too many queries in flight, finalizing query {} early: {:?}",
                                zenoh_proto::zctx!(),
                                id,
                                e
                            )
                        })
                        .is_ok();

                if !pending {
                    match self.response_final(id, origin).await {
                        // The local get then completes at its timeout. That
                        // is no reason to stop the session.
                        Err(SessionError::LoopbackFull) => zenoh_proto::error!(
                            "{}: Couldn't finalize local query {}: {}",
                            zenoh_proto::zctx!(),
                            id,
                            SessionError::LoopbackFull
                        ),
                        result => result?,
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
