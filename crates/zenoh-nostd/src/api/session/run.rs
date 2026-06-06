use zenoh_proto::{exts::Value, msgs::*, *};

use crate::{
    api::{
        callbacks::{ZCallbacks, ZDynCallback},
        query::QueryableQuery,
        session::{KeMapping, Session},
    },
    config::ZSessionConfig,
    session::{GetResponse, Sample, SampleKind},
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
                        payload,
                        timestamp,
                        qos,
                        ..
                    }) => {
                        let ke_buf = state.resolve_keyexpr(&wire_expr).ok_or(SessionError::TransportClosed)?;
                        let ke_str = ke_buf.as_str();
                        let ke = match keyexpr::new(ke_str) {
                            Ok(k) => k,
                            Err(_) => return Ok(()),
                        };
                        let (kind, payload_bytes, encoding, attachment) = match &payload {
                            PushBody::Put(Put {
                                payload,
                                encoding,
                                attachment,
                                ..
                            }) => (SampleKind::Put, *payload, Some(encoding.clone()), attachment.as_ref().map(|a| a.buffer)),
                            PushBody::Del(_) => (SampleKind::Delete, &[] as &[u8], None, None),
                        };
                        let sample = Sample::new(ke, payload_bytes, kind, encoding, timestamp, attachment, qos, msg.reliability);

                        for cb in state.sub_callbacks.intersects(ke) {
                            cb.call_try_sync(&sample).await;
                        }
                    }
                    NetworkBody::Response(Response {
                        rid,
                        wire_expr,
                        payload,
                        timestamp,
                        qos,
                        ..
                    }) => {
                        let ke_buf = state.resolve_keyexpr(&wire_expr).ok_or(SessionError::TransportClosed)?;
                        let ke = match keyexpr::new(ke_buf.as_str()) {
                            Ok(k) => k,
                            Err(_) => return Ok(()),
                        };
                        let response = match payload {
                            ResponseBody::Reply(Reply {
                                payload:
                                    PushBody::Put(Put {
                                        payload,
                                        encoding,
                                        attachment,
                                        ..
                                    }),
                                ..
                            }) => {
                                let sample = Sample::new(ke, payload, SampleKind::Put, Some(encoding.clone()), timestamp, attachment.as_ref().map(|a| a.buffer), qos, msg.reliability);
                                GetResponse::Ok(sample)
                            }
                            ResponseBody::Reply(Reply {
                                payload: PushBody::Del(del),
                                ..
                            }) => {
                                let sample = Sample::new(ke, &[], SampleKind::Delete, None, timestamp, del.attachment.as_ref().map(|a| a.buffer), qos, msg.reliability);
                                GetResponse::Ok(sample)
                            }
                            ResponseBody::Err(Err {
                                payload,
                                encoding,
                                ..
                            }) => {
                                let sample = Sample::new(ke, payload, SampleKind::Delete, Some(encoding.clone()), timestamp, None, qos, msg.reliability);
                                GetResponse::Err(sample)
                            }
                        };

                        if let Some(cb) = state.get_callbacks.get(rid) {
                            cb.call_try_sync(&response).await;
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
                                parameters, body, query_target, ..
                            }),
                        target: _,
                        qos,
                        ..
                    }) => {
                        let ke_buf = state.resolve_keyexpr(&wire_expr).ok_or(SessionError::TransportClosed)?;
                        let ke = match keyexpr::new(ke_buf.as_str()) {
                            Ok(k) => k,
                            Err(_) => return Ok(()),
                        };
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
                            query_target,
                            qos,
                        );

                        let count = state.queryable_callbacks.intersects(ke).count();
                        state.queryable_callbacks.set_counter(id, count)?;
                        for cb in state.queryable_callbacks.intersects(ke) {
                            cb.call(&query).await;
                        }
                    }
                    NetworkBody::Declare(Declare { body, .. }) => {
                        if let DeclareBody::DeclareKeyExpr(ke) = body {
                            let suffix = ke.wire_expr.suffix;
                            if !suffix.is_empty() {
                                let mut s = heapless::String::<64>::new();
                                let _ = s.push_str(suffix);
                                let _ = state.ke_mappings.push(KeMapping { scope: ke.id, keyexpr: s });
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
