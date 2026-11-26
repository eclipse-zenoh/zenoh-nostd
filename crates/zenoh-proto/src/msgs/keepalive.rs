use crate::*;

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "_:3|ID:5=0x04")]
/// Payload-free transport keepalive; its encoded representation includes the message header.
pub struct KeepAlive;
