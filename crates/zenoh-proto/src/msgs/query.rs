use crate::{exts::*, fields::*, *};

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|P|C|ID:5=0x3")]
/// Query selector parameters, consolidation policy and optional request value/metadata.
pub struct Query<'a> {
    #[zenoh(presence = header(C), default = ConsolidationMode::default())]
    /// Requested or reported reply-consolidation mode.
    pub consolidation: ConsolidationMode,
    #[zenoh(presence = header(P), size = prefixed, default = "")]
    /// Borrowed query selector parameters; an empty string means none were supplied.
    pub parameters: &'a str,

    #[zenoh(ext = 0x1)]
    /// Optional source entity and source sequence number.
    pub sinfo: Option<SourceInfo>,
    #[zenoh(ext = 0x3)]
    /// Optional borrowed request value including encoding and payload.
    pub body: Option<Value<'a>>,
    #[zenoh(ext = 0x5)]
    /// Optional opaque application attachment.
    pub attachment: Option<Attachment<'a>>,
}
