use crate::{exts::*, fields::*, msgs::*, *};

#[derive(ZEnum, Debug, PartialEq)]
/// Data operation inside a publication or successful reply; currently only puts are represented.
pub enum PushBody<'a> {
    /// Put payload and optional encoding, timestamp, source and attachment metadata.
    Put(Put<'a>),
}

impl Default for PushBody<'_> {
    fn default() -> Self {
        PushBody::Put(Put::default())
    }
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|M|N|ID:5=0x1d")]
/// Unsolicited publication envelope, carrying a wire expression and data operation.
pub struct Push<'a> {
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

    /// Borrowed payload or nested operation; the codec does not interpret application bytes.
    pub payload: PushBody<'a>,
}
