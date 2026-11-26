use crate::{exts::*, fields::*, *};

#[derive(ZStruct, Clone, Copy, Debug, PartialEq, Default)]
#[zenoh(header = "Z|_|R|ID:5=0x05")]
/// Transport frame header preceding network messages sharing reliability and QoS.
pub struct FrameHeader {
    #[zenoh(header = R)]
    /// Reliability class for the enclosing transport frame.
    pub reliability: Reliability,
    /// Sequence number associated with the source, frame or opening message.
    pub sn: u32,

    #[zenoh(ext = 0x1, default = QoS::default())]
    /// Wire QoS settings or negotiated QoS extension for this message.
    pub qos: QoS,
}
