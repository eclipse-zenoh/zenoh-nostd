use crate::{exts::*, fields::*, *};

#[repr(u8)]
#[derive(ZRU8, Debug, Clone, Copy, PartialEq, Default)]
/// Which declaration updates an interest requests: current, future, both or final.
pub enum InterestMode {
    #[default]
    /// Final interest marker.
    Final = 0b00,
    /// Request existing declarations.
    Current = 0b01,
    /// Request subsequent declaration changes.
    Future = 0b10,
    /// Request existing declarations and future changes.
    CurrentFuture = 0b11,
}

#[derive(ZStruct, Debug, Default)]
#[zenoh(header = "A|M|N|R|T|Q|S|K")]
/// Interest option bits and optional key-expression restriction.
pub struct InterestInner<'a> {
    #[zenoh(header = FULL)]
    /// Raw option/flag byte; see [`InterestOptions`] for declaration-interest bits.
    pub options: u8,

    #[zenoh(presence = header(R), flatten, shift = 5)]
    /// Wire key expression, possibly compressed through a declared scope ID.
    pub wire_expr: Option<WireExpr<'a>>,
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|MODE:2|ID:5=0x19")]
/// Declaration interest with an ID, update mode and optional routing metadata.
pub struct Interest<'a> {
    /// Identifier of this declaration or request, used by matching removal/reply messages.
    pub id: u32,

    #[zenoh(header = MODE)]
    /// Which current/future declaration updates this interest requests.
    pub mode: InterestMode,

    /// Packed interest selection flags and optional expression restriction.
    pub inner: InterestInner<'a>,

    #[zenoh(ext = 0x1, default = QoS::default())]
    /// Wire QoS settings or negotiated QoS extension for this message.
    pub qos: QoS,
    #[zenoh(ext = 0x2)]
    /// Optional sender-supplied timestamp; absence means no timestamp was encoded.
    pub timestamp: Option<Timestamp>,
    #[zenoh(ext = 0x3, default = NodeId::default(), mandatory)]
    /// Routing node-ID extension.
    pub nodeid: NodeId,
}

#[derive(ZStruct, Debug, PartialEq, Default)]
#[zenoh(header = "Z|MODE:2=0x0|ID:5=0x19")]
/// Completion/removal marker for an interest ID.
pub struct InterestFinal {
    /// Identifier of this declaration or request, used by matching removal/reply messages.
    pub id: u32,

    #[zenoh(ext = 0x1, default = QoS::default())]
    /// Wire QoS settings or negotiated QoS extension for this message.
    pub qos: QoS,
    #[zenoh(ext = 0x2)]
    /// Optional sender-supplied timestamp; absence means no timestamp was encoded.
    pub timestamp: Option<Timestamp>,
    #[zenoh(ext = 0x3, default = NodeId::default(), mandatory)]
    /// Routing node-ID extension.
    pub nodeid: NodeId,
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
/// Raw declaration-interest mask. Combine the public constants through their `options` bytes.
pub struct InterestOptions {
    /// Raw option/flag byte; see [`InterestOptions`] for declaration-interest bits.
    pub options: u8,
}

impl PartialEq for InterestInner<'_> {
    fn eq(&self, other: &Self) -> bool {
        let options = InterestOptions::options(self.options);
        let other_options = InterestOptions::options(other.options);

        options == other_options && self.wire_expr == other.wire_expr
    }
}

impl PartialEq for InterestOptions {
    fn eq(&self, other: &Self) -> bool {
        self.keyexprs() == other.keyexprs()
            && self.subscribers() == other.subscribers()
            && self.queryables() == other.queryables()
            && self.tokens() == other.tokens()
            && self.aggregate() == other.aggregate()
    }
}

impl InterestOptions {
    /// Select key-expression declarations.
    pub const KEYEXPRS: InterestOptions = InterestOptions::options(1);
    /// Select subscriber declarations.
    pub const SUBSCRIBERS: InterestOptions = InterestOptions::options(1 << 1);
    /// Select queryable declarations.
    pub const QUERYABLES: InterestOptions = InterestOptions::options(1 << 2);
    /// Select liveliness-token declarations.
    pub const TOKENS: InterestOptions = InterestOptions::options(1 << 3);

    /// Request aggregated declaration information.
    pub const AGGREGATE: InterestOptions = InterestOptions::options(1 << 7);

    const fn options(options: u8) -> Self {
        Self { options }
    }

    /// Whether this mask selects key expressions.
    pub const fn keyexprs(&self) -> bool {
        self.options & Self::KEYEXPRS.options != 0
    }

    /// Whether this mask selects subscribers.
    pub const fn subscribers(&self) -> bool {
        self.options & Self::SUBSCRIBERS.options != 0
    }

    /// Whether this mask selects queryables.
    pub const fn queryables(&self) -> bool {
        self.options & Self::QUERYABLES.options != 0
    }

    /// Whether this mask selects liveliness tokens.
    pub const fn tokens(&self) -> bool {
        self.options & Self::TOKENS.options != 0
    }

    /// Whether this mask selects aggregation.
    pub const fn aggregate(&self) -> bool {
        self.options & Self::AGGREGATE.options != 0
    }
}
