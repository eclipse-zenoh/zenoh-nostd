use zenoh_proto::{exts::Value, msgs::*, *};

use crate::{
    api::{
        callbacks::{ZCallbacks, ZDynCallback},
        query::QueryableQuery,
        session::Session,
    },
    config::ZSessionConfig,
    session::{GetResponse, Sample},
};

impl<'res, Config> Session<'res, Config>
where
    Config: ZSessionConfig,
{
    pub async fn run(&self) -> core::result::Result<(), SessionError> {
        self.driver
            .run(&self.state, async |_, state, msg, _| {
                match msg.body {
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
                        // Tell the get it is complete (this also ends a
                        // channel-mode get's `recv`), then forget it.
                        if let Some(cb) = state.get_callbacks.get(rid) {
                            cb.call(&GetResponse::Final).await;
                        }
                        state.get_callbacks.remove(rid)?;
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

                        // A callback-mode queryable is done with the query
                        // when its callback returns. A channel-mode one hands
                        // the query off, and its owned copy finalizes later.
                        let mut handed_off = 0;
                        for cb in state.queryable_callbacks.intersects(ke) {
                            cb.call(&query).await;
                            if query.take_handed_off() {
                                handed_off += 1;
                            }
                        }

                        // Finalize now unless an owned copy is still out. The
                        // state lock is held, so no owner can finalize before
                        // the counter is in place.
                        let pending = handed_off > 0
                            && state
                                .queryable_callbacks
                                .set_counter(id, handed_off)
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
                            self.response_final(id).await?;
                        }
                    }
                    _ => {}
                }

                Ok::<(), SessionError>(())
            })
            .await
            .map_err(|e| e.flatten_map())
    }
}
