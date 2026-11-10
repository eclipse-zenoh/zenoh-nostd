use crate::*;

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "_:2|S|ID:5=0x03")]
/// Transport closure notification with a reason code and link/session scope.
pub struct Close {
    /// Wire reason code describing closure.
    pub reason: u8,

    #[zenoh(header = S)]
    /// Whether closure applies to the link or entire transport session.
    pub behaviour: CloseBehaviour,
}

#[repr(u8)]
#[derive(ZRU8, Default, Debug, Clone, Copy, PartialEq)]
/// Whether a close message applies to one link or the whole transport session.
pub enum CloseBehaviour {
    #[default]
    /// Close this link only (default).
    Link = 0,
    /// Close the transport session.
    Session = 1,
}
