use crate::{exts::*, fields::*, *};

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "ZID:4|_:2|W:2")]
/// Node role and Zenoh ID advertised during transport initialization.
pub struct InitIdentifier {
    #[zenoh(header = W)]
    /// Advertised node role.
    pub whatami: WhatAmI,
    #[zenoh(size = header(ZID))]
    /// Zenoh node identifier.
    pub zid: ZenohIdProto,
}

#[derive(ZStruct, Debug, PartialEq, Default)]
/// Proposed sequence/request-ID resolution and batch byte limit.
pub struct InitResolution {
    /// Negotiated/proposed integer widths and, in initialization messages, batch limit.
    pub resolution: Resolution,
    /// Maximum batch size in bytes.
    pub batch_size: BatchSize,
}

#[derive(ZStruct, Debug, PartialEq)]
#[zenoh(header = "Z|S|A:1=0|ID:5=0x01")]
/// Initial outgoing transport proposal. Optional extensions describe wire capabilities, not implemented runtime features.
pub struct InitSyn<'a> {
    /// Zenoh wire protocol version advertised by the peer.
    pub version: u8,
    /// Node role and Zenoh identifier.
    pub identifier: InitIdentifier,

    #[zenoh(presence = header(S), default = InitResolution::default())]
    /// Negotiated/proposed integer widths and, in initialization messages, batch limit.
    pub resolution: InitResolution,

    #[zenoh(ext = 0x1)]
    /// Wire QoS settings or negotiated QoS extension for this message.
    pub qos: Option<HasQoS>,
    #[zenoh(ext = 0x1)]
    /// Optional link-specific QoS negotiation extension.
    pub qos_link: Option<QoSLink>,
    #[zenoh(ext = 0x3)]
    /// Optional opaque authentication negotiation data.
    pub auth: Option<Auth<'a>>,
    #[zenoh(ext = 0x4)]
    /// Optional opaque multilink negotiation data.
    pub mlink: Option<MultiLink<'a>>,
    #[zenoh(ext = 0x5)]
    /// Optional low-latency capability flag.
    pub lowlatency: Option<HasLowLatency>,
    #[zenoh(ext = 0x6)]
    /// Optional compression capability flag.
    pub compression: Option<HasCompression>,
    #[zenoh(ext = 0x7, default = Patch::none())]
    /// Protocol patch level advertised during initialization.
    pub patch: Patch,
}

impl Default for InitSyn<'_> {
    fn default() -> Self {
        Self {
            version: crate::VERSION,
            identifier: InitIdentifier::default(),
            resolution: InitResolution::default(),
            qos: None,
            qos_link: None,
            auth: None,
            mlink: None,
            lowlatency: None,
            compression: None,
            patch: Patch::default(),
        }
    }
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|S|A:1=1|ID:5=0x01")]
/// Transport initialization acknowledgment with negotiated settings and an opaque cookie.
pub struct InitAck<'a> {
    /// Zenoh wire protocol version advertised by the peer.
    pub version: u8,
    /// Node role and Zenoh identifier.
    pub identifier: InitIdentifier,

    #[zenoh(presence = header(S), default = InitResolution::default())]
    /// Negotiated/proposed integer widths and, in initialization messages, batch limit.
    pub resolution: InitResolution,

    #[zenoh(size = prefixed)]
    /// Opaque initialization cookie, borrowed from input and echoed during opening.
    pub cookie: &'a [u8],

    #[zenoh(ext = 0x1)]
    /// Wire QoS settings or negotiated QoS extension for this message.
    pub qos: Option<HasQoS>,
    #[zenoh(ext = 0x1)]
    /// Optional link-specific QoS negotiation extension.
    pub qos_link: Option<QoSLink>,
    #[zenoh(ext = 0x3)]
    /// Optional opaque authentication negotiation data.
    pub auth: Option<Auth<'a>>,
    #[zenoh(ext = 0x4)]
    /// Optional opaque multilink negotiation data.
    pub mlink: Option<MultiLink<'a>>,
    #[zenoh(ext = 0x5)]
    /// Optional low-latency capability flag.
    pub lowlatency: Option<HasLowLatency>,
    #[zenoh(ext = 0x6)]
    /// Optional compression capability flag.
    pub compression: Option<HasCompression>,
    #[zenoh(ext = 0x7, default = Patch::none())]
    /// Protocol patch level advertised during initialization.
    pub patch: Patch,
}
