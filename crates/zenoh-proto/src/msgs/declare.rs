use crate::{exts::*, fields::*, *};

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|_|I|ID:5=0x1e")]
/// Declaration envelope containing a registration, removal or declaration-completion marker.
pub struct Declare<'a> {
    #[zenoh(presence = header(I))]
    /// Optional interest ID associating this declaration with a requested sequence.
    pub id: Option<u32>,

    #[zenoh(ext = 0x1, default = QoS::default())]
    /// Wire QoS settings or negotiated QoS extension for this message.
    pub qos: QoS,
    #[zenoh(ext = 0x2)]
    /// Optional sender-supplied timestamp; absence means no timestamp was encoded.
    pub timestamp: Option<Timestamp>,
    #[zenoh(ext = 0x3, default = NodeId::default(), mandatory)]
    /// Routing node-ID extension.
    pub nodeid: NodeId,

    /// Operation/value carried by the envelope; borrowed data retains the input lifetime.
    pub body: DeclareBody<'a>,
}

#[derive(ZEnum, Debug, PartialEq)]
/// Registration, removal or completion carried by a declaration envelope.
pub enum DeclareBody<'a> {
    /// Associate a key-expression ID with a wire expression for later compression.
    DeclareKeyExpr(DeclareKeyExpr<'a>),
    /// Remove a previously declared key-expression ID.
    UndeclareKeyExpr(UndeclareKeyExpr),
    /// Announce a subscriber ID and its matching wire expression.
    DeclareSubscriber(DeclareSubscriber<'a>),
    /// Withdraw a subscriber ID, optionally supplying its wire expression.
    UndeclareSubscriber(UndeclareSubscriber<'a>),
    /// Announce a queryable ID, matching expression and completeness/distance information.
    DeclareQueryable(DeclareQueryable<'a>),
    /// Withdraw a queryable ID, optionally supplying its wire expression.
    UndeclareQueryable(UndeclareQueryable<'a>),
    /// Announce a liveliness token associated with a wire expression.
    DeclareToken(DeclareToken<'a>),
    /// Withdraw a liveliness token by ID, optionally supplying its expression.
    UndeclareToken(UndeclareToken<'a>),
    /// Mark the end of a declaration sequence.
    DeclareFinal(DeclareFinal),
}

impl Default for DeclareBody<'_> {
    fn default() -> Self {
        DeclareBody::DeclareFinal(DeclareFinal::default())
    }
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "_|M|N|ID:5=0x00")]
/// Associate a key-expression ID with a wire expression for later compression.
pub struct DeclareKeyExpr<'a> {
    /// Identifier of this declaration or request, used by matching removal/reply messages.
    pub id: u16,
    #[zenoh(flatten, shift = 5)]
    /// Wire key expression, possibly compressed through a declared scope ID.
    pub wire_expr: WireExpr<'a>,
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "_:3|ID:5=0x01")]
/// Remove a previously declared key-expression ID.
pub struct UndeclareKeyExpr {
    /// Identifier of this declaration or request, used by matching removal/reply messages.
    pub id: u16,
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "_|M|N|ID:5=0x02")]
/// Announce a subscriber ID and its matching wire expression.
pub struct DeclareSubscriber<'a> {
    /// Identifier of this declaration or request, used by matching removal/reply messages.
    pub id: u32,
    #[zenoh(flatten, shift = 5)]
    /// Wire key expression, possibly compressed through a declared scope ID.
    pub wire_expr: WireExpr<'a>,
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|_:2|ID:5=0x03")]
/// Withdraw a subscriber ID, optionally supplying its wire expression.
pub struct UndeclareSubscriber<'a> {
    /// Identifier of this declaration or request, used by matching removal/reply messages.
    pub id: u32,
    #[zenoh(ext = 0x0f)]
    /// Wire key expression, possibly compressed through a declared scope ID.
    pub wire_expr: Option<WireExpr<'a>>,
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|M|N|ID:5=0x04")]
/// Announce a queryable ID, matching expression and completeness/distance information.
pub struct DeclareQueryable<'a> {
    /// Identifier of this declaration or request, used by matching removal/reply messages.
    pub id: u32,
    #[zenoh(flatten, shift = 5)]
    /// Wire key expression, possibly compressed through a declared scope ID.
    pub wire_expr: WireExpr<'a>,

    #[zenoh(ext = 0x01, default = QueryableInfo::default())]
    /// Advertised queryable completeness and distance.
    pub qinfo: QueryableInfo,
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|_:2|ID:5=0x05")]
/// Withdraw a queryable ID, optionally supplying its wire expression.
pub struct UndeclareQueryable<'a> {
    /// Identifier of this declaration or request, used by matching removal/reply messages.
    pub id: u32,
    #[zenoh(ext = 0x0f)]
    /// Wire key expression, possibly compressed through a declared scope ID.
    pub wire_expr: Option<WireExpr<'a>>,
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|M|N|ID:5=0x06")]
/// Announce a liveliness token associated with a wire expression.
pub struct DeclareToken<'a> {
    /// Identifier of this declaration or request, used by matching removal/reply messages.
    pub id: u32,
    #[zenoh(flatten, shift = 5)]
    /// Wire key expression, possibly compressed through a declared scope ID.
    pub wire_expr: WireExpr<'a>,
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|_:2|ID:5=0x07")]
/// Withdraw a liveliness token by ID, optionally supplying its expression.
pub struct UndeclareToken<'a> {
    /// Identifier of this declaration or request, used by matching removal/reply messages.
    pub id: u32,
    #[zenoh(ext = 0x0f)]
    /// Wire key expression, possibly compressed through a declared scope ID.
    pub wire_expr: Option<WireExpr<'a>>,
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|_:2|ID:5=0x1A")]
/// Mark the end of a declaration sequence.
pub struct DeclareFinal {}
