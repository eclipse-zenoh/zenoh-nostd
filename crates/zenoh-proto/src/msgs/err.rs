use crate::{exts::*, fields::*, *};

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|E|_|ID:5=0x5")]
/// Application error payload and optional encoding/source metadata, not a transport failure.
pub struct Err<'a> {
    #[zenoh(presence = header(E), default = Encoding::default())]
    /// Payload encoding metadata, not a payload conversion.
    pub encoding: Encoding<'a>,

    #[zenoh(ext = 0x1)]
    /// Optional source entity and source sequence number.
    pub sinfo: Option<SourceInfo>,

    #[zenoh(size = prefixed)]
    /// Borrowed payload or nested operation; the codec does not interpret application bytes.
    pub payload: &'a [u8],
}
