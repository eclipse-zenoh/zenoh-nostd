use core::time::Duration;

use crate::{exts::*, fields::*, msgs::*, *};

#[derive(ZEnum, Debug, PartialEq)]
/// Operation carried by a request; currently only query payloads are represented.
pub enum RequestBody<'a> {
    /// Query selector parameters, consolidation policy and optional request value/metadata.
    Query(Query<'a>),
}

impl Default for RequestBody<'_> {
    fn default() -> Self {
        RequestBody::Query(Query::default())
    }
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|M|N|ID:5=0x1c")]
/// Outgoing request envelope, carrying a request ID, key and query-routing metadata.
pub struct Request<'a> {
    /// Identifier of this declaration or request, used by matching removal/reply messages.
    pub id: u32,

    #[zenoh(flatten, shift = 5)]
    /// Wire key expression, possibly compressed through a declared scope ID.
    pub wire_expr: WireExpr<'a>,

    #[zenoh(ext = 0x1, default = QoS::default())]
    /// Wire QoS settings or negotiated QoS extension for this message.
    pub qos: QoS,
    #[zenoh(ext = 0x2)]
    /// Optional sender-supplied timestamp; absence means no timestamp was encoded.
    pub timestamp: Option<Timestamp>,
    #[zenoh(ext = 0x3, default = NodeId::default(), mandatory)]
    /// Routing node-ID extension.
    pub nodeid: NodeId,
    #[zenoh(ext = 0x4, default = QueryTarget::default(), mandatory)]
    /// Peer-side query target selection.
    pub target: QueryTarget,
    #[zenoh(ext = 0x5)]
    /// Optional query-routing budget.
    pub budget: Option<Budget>,
    #[zenoh(ext = 0x6)]
    /// Optional requested query timeout; encoding it starts no local timer.
    pub timeout: Option<Duration>,

    /// Borrowed payload or nested operation; the codec does not interpret application bytes.
    pub payload: RequestBody<'a>,
}
