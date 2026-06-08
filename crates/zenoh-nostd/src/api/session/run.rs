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
    pub fn run(&mut self) -> core::result::Result<(), SessionError> {
        // SAFETY: We hold &mut self, guaranteeing exclusive access.
        // Raw pointers allow us to pass self reference into the poll closure.
        let self_ptr: *mut Session<'res, Config> = self;
        let state_ptr = self.state.get();
        let driver_ptr = self.driver.get();

        let connectivity = &self.connectivity;

        loop {
            let state = unsafe { &mut *state_ptr };
            let driver = unsafe { &mut *driver_ptr };

            let result = driver.poll(
                state,
                connectivity,
                |_, s, msg, _| {
                    match msg.body {
                        NetworkBody::Push(Push {
                            wire_expr,
                            payload,
                            timestamp,
                            qos,
                            ..
                        }) => {
                            let ke_buf = s.resolve_keyexpr(&wire_expr).ok_or(SessionError::TransportClosed)?;
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

                            for cb in s.sub_callbacks.intersects(ke) {
                                cb.call(&sample);
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
                            let ke_buf = s.resolve_keyexpr(&wire_expr).ok_or(SessionError::TransportClosed)?;
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

                            if let Some(cb) = s.get_callbacks.get(rid) {
                                cb.call(&response);
                            }
                        }
                        NetworkBody::ResponseFinal(ResponseFinal { rid, .. }) => {
                            s.get_callbacks.remove(rid)?;
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
                            let ke_buf = s.resolve_keyexpr(&wire_expr).ok_or(SessionError::TransportClosed)?;
                            let ke = match keyexpr::new(ke_buf.as_str()) {
                                Ok(k) => k,
                                Err(_) => return Ok(()),
                            };
                            // SAFETY: self_ptr is valid and non-aliasing (we're in run() which holds &mut self).
                            let session = unsafe { &*self_ptr };
                            let query = QueryableQuery::new(
                                session,
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

                            let count = s.queryable_callbacks.intersects(ke).count();
                            s.queryable_callbacks.set_counter(id, count)?;
                            for cb in s.queryable_callbacks.intersects(ke) {
                                cb.call(&query);
                            }
                        }
                        NetworkBody::Declare(Declare { body: DeclareBody::DeclareKeyExpr(ke), .. }) => {
                            let suffix = ke.wire_expr.suffix;
                            if !suffix.is_empty() {
                                let mut s_buf = heapless::String::<64>::new();
                                if s_buf.push_str(suffix).is_err() {
                                    zenoh_proto::error!("Keyexpr suffix too long, dropping: {}", suffix);
                                } else if s.ke_mappings.push(KeMapping { scope: ke.id, keyexpr: s_buf }).is_err() {
                                    zenoh_proto::error!("Keyexpr mapping table full, dropping scope={}", ke.id);
                                }
                            }
                        }
                        _ => {}
                    }

                    Ok::<(), SessionError>(())
                },
            );

            match result {
                Err(e) => return Err(e.flatten_map()),
                Ok(false) => break,
                Ok(true) => {}
            }
        }

        Ok(())
    }
}