use crate::*;

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|_|N|ID:5=0x12")]
pub struct Nack {
    pub sn: u32,
}
