use crate::{exts::*, fields::*, msgs::*, *};

#[derive(ZEnum, Debug, PartialEq)]
/// Successful data reply or application error carried by a response envelope.
pub enum ResponseBody<'a> {
    /// Application error payload and optional encoding/source metadata, not a transport failure.
    Err(Err<'a>),
    /// Successful reply carrying a data operation and consolidation mode.
    Reply(Reply<'a>),
}

impl Default for ResponseBody<'_> {
    fn default() -> Self {
        ResponseBody::Err(Err::default())
    }
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|M|N|ID:5=0x1b")]
/// Reply envelope correlated to an outgoing request by `rid`.
pub struct Response<'a> {
    /// Request ID being answered or finalized.
    pub rid: u32,

    #[zenoh(flatten, shift = 5)]
    /// Wire key expression, possibly compressed through a declared scope ID.
    pub wire_expr: WireExpr<'a>,

    #[zenoh(ext = 0x1, default = QoS::default())]
    /// Wire QoS settings or negotiated QoS extension for this message.
    pub qos: QoS,
    #[zenoh(ext = 0x2)]
    /// Optional sender-supplied timestamp; absence means no timestamp was encoded.
    pub timestamp: Option<Timestamp>,
    #[zenoh(ext = 0x3)]
    /// Optional globally identified responder.
    pub respid: Option<EntityGlobalId>,

    /// Borrowed payload or nested operation; the codec does not interpret application bytes.
    pub payload: ResponseBody<'a>,
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|_:2|ID:5=0x1a")]
/// Mark the end of responses for a request ID; carries no sample payload.
pub struct ResponseFinal {
    /// Request ID being answered or finalized.
    pub rid: u32,

    #[zenoh(ext = 0x1, default = QoS::default())]
    /// Wire QoS settings or negotiated QoS extension for this message.
    pub qos: QoS,
    #[zenoh(ext = 0x2)]
    /// Optional sender-supplied timestamp; absence means no timestamp was encoded.
    pub timestamp: Option<Timestamp>,
}
