use crate::{exts::*, fields::*, *};

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|E|T|ID:5=0x1")]
/// Put payload and optional encoding, timestamp, source and attachment metadata.
pub struct Put<'a> {
    #[zenoh(presence = header(T))]
    /// Optional sender-supplied timestamp; absence means no timestamp was encoded.
    pub timestamp: Option<Timestamp>,
    #[zenoh(presence = header(E), default = Encoding::default())]
    /// Payload encoding metadata, not a payload conversion.
    pub encoding: Encoding<'a>,

    #[zenoh(ext = 0x1)]
    /// Optional source entity and source sequence number.
    pub sinfo: Option<SourceInfo>,
    #[zenoh(ext = 0x3)]
    /// Optional opaque application attachment.
    pub attachment: Option<Attachment<'a>>,

    #[zenoh(size = prefixed)]
    /// Borrowed payload or nested operation; the codec does not interpret application bytes.
    pub payload: &'a [u8],
}
