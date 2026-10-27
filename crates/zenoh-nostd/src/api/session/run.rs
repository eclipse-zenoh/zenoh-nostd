use zenoh_proto::{exts::Value, msgs::*, *};

use crate::io::transport::ZTransportLinkTx;

use crate::{
    api::{
        callbacks::{ZCallbacks, ZDynCallback},
        query::QueryableQuery,
        session::Session,
    },
    config::ZSessionConfig,
    session::{GetResponse, Sample},
};

impl<'transport, 'resources: 'transport, Config> Session<'transport, 'resources, Config>
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
                        state.get_callbacks.remove(rid)?;
                        // TODO: also close channels
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

                        let count = state.queryable_callbacks.intersects(ke).count();
                        // Bounded handlers may decline work instead of blocking RX.
                        // A saturated request table must not terminate the session.
                        if count == 0 || state.queryable_callbacks.set_counter(id, count).is_err() {
                            self.driver
                                .tx()
                                .await
                                .send(core::iter::once(NetworkMessage {
                                    reliability: fields::Reliability::default(),
                                    qos: exts::QoS::default(),
                                    body: NetworkBody::ResponseFinal(ResponseFinal {
                                        rid: id,
                                        ..Default::default()
                                    }),
                                }))
                                .await?;
                        } else {
                            let mut rejected = 0;
                            for cb in state.queryable_callbacks.intersects(ke) {
                                query.rejected.set(false);
                                cb.call(&query).await;
                                if query.rejected.get() {
                                    rejected += 1;
                                }
                            }
                            for _ in 0..rejected {
                                if state.queryable_callbacks.decrease(id) {
                                    self.driver
                                        .tx()
                                        .await
                                        .send(core::iter::once(NetworkMessage {
                                            reliability: fields::Reliability::default(),
                                            qos: exts::QoS::default(),
                                            body: NetworkBody::ResponseFinal(ResponseFinal {
                                                rid: id,
                                                ..Default::default()
                                            }),
                                        }))
                                        .await?;
                                }
                            }
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
