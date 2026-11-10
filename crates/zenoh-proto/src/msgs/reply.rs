use crate::{fields::*, msgs::*, *};

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|_|C|ID:5=0x4")]
/// Successful reply carrying a data operation and consolidation mode.
pub struct Reply<'a> {
    #[zenoh(presence = header(C), default = ConsolidationMode::default())]
    /// Requested or reported reply-consolidation mode.
    pub consolidation: ConsolidationMode,

    /// Borrowed payload or nested operation; the codec does not interpret application bytes.
    pub payload: PushBody<'a>,
}
