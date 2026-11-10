use core::time::Duration;

use crate::{exts::*, *};

#[derive(ZStruct, Debug, PartialEq)]
#[zenoh(header = "Z|T|A:1=0|ID:5=0x02")]
/// Open request confirming lease, initial sequence number and the initialization cookie.
pub struct OpenSyn<'a> {
    #[zenoh(flatten, shift = 6)]
    /// Advertised inactivity lease as a duration.
    pub lease: Duration,
    /// Sequence number associated with the source, frame or opening message.
    pub sn: u32,

    #[zenoh(size = prefixed)]
    /// Opaque initialization cookie, borrowed from input and echoed during opening.
    pub cookie: &'a [u8],

    #[zenoh(ext = 0x1)]
    /// Wire QoS settings or negotiated QoS extension for this message.
    pub qos: Option<HasQoS>,
    #[zenoh(ext = 0x3)]
    /// Optional opaque authentication negotiation data.
    pub auth: Option<Auth<'a>>,
    #[zenoh(ext = 0x4)]
    /// Optional multilink opening data.
    pub mlink_syn: Option<MultiLinkSyn<'a>>,
    #[zenoh(ext = 0x4)]
    /// Optional multilink acknowledgment flag.
    pub mlink_ack: Option<HasMultiLinkAck>,
    #[zenoh(ext = 0x5)]
    /// Optional low-latency capability flag.
    pub lowlatency: Option<HasLowLatency>,
    #[zenoh(ext = 0x6)]
    /// Optional compression capability flag.
    pub compression: Option<HasCompression>,
}

impl Default for OpenSyn<'_> {
    fn default() -> Self {
        Self {
            lease: Duration::from_secs(10),
            sn: u32::default(),
            cookie: &[],
            qos: None,
            auth: None,
            mlink_syn: None,
            mlink_ack: None,
            lowlatency: None,
            compression: None,
        }
    }
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|T|A:1=1|ID:5=0x02")]
/// Open acknowledgment confirming lease and initial sequence number.
pub struct OpenAck<'a> {
    #[zenoh(flatten, shift = 6)]
    /// Advertised inactivity lease as a duration.
    pub lease: Duration,
    /// Sequence number associated with the source, frame or opening message.
    pub sn: u32,

    #[zenoh(ext = 0x1)]
    /// Wire QoS settings or negotiated QoS extension for this message.
    pub qos: Option<HasQoS>,
    #[zenoh(ext = 0x3)]
    /// Optional opaque authentication negotiation data.
    pub auth: Option<Auth<'a>>,
    #[zenoh(ext = 0x4)]
    /// Optional multilink opening data.
    pub mlink_syn: Option<MultiLinkSyn<'a>>,
    #[zenoh(ext = 0x4)]
    /// Optional multilink acknowledgment flag.
    pub mlink_ack: Option<HasMultiLinkAck>,
    #[zenoh(ext = 0x5)]
    /// Optional low-latency capability flag.
    pub lowlatency: Option<HasLowLatency>,
    #[zenoh(ext = 0x6)]
    /// Optional compression capability flag.
    pub compression: Option<HasCompression>,
}
