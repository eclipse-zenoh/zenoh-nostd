use crate::{exts::*, fields::*, *};

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|_|T|ID:5=0x2")]
pub struct Del<'a> {
    #[zenoh(presence = header(T))]
    pub timestamp: Option<Timestamp>,
    #[zenoh(ext = 0x1)]
    pub sinfo: Option<SourceInfo>,
    #[zenoh(ext = 0x3)]
    pub attachment: Option<Attachment<'a>>,
}
