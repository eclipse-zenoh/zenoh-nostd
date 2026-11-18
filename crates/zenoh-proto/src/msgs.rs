/// Wire extensions shared by network and transport messages.
pub mod exts;
/// Shared wire fields, identifiers, encoding metadata and negotiated resolutions.
pub mod fields;

mod err;
mod put;
mod query;
mod reply;

mod declare;
mod interest;
mod push;
mod request;
mod response;

mod close;
mod frame;
mod init;
mod keepalive;
mod open;

pub use err::*;
pub use put::*;
pub use query::*;
pub use reply::*;

pub use declare::*;
pub use interest::*;
pub use push::*;
pub use request::*;
pub use response::*;

pub use close::*;
pub use frame::*;
pub use init::*;
pub use keepalive::*;
pub use open::*;

use zenoh_derive::ZEnum;

#[derive(ZEnum, Debug, PartialEq)]
/// Network-level operation payload. Borrowed strings and bytes retain their input-buffer lifetime.
pub enum NetworkBody<'a> {
    /// Unsolicited publication envelope, carrying a wire expression and data operation.
    Push(Push<'a>),
    /// Outgoing request envelope, carrying a request ID, key and query-routing metadata.
    Request(Request<'a>),
    /// Reply envelope correlated to an outgoing request by `rid`.
    Response(Response<'a>),
    /// Mark the end of responses for a request ID; carries no sample payload.
    ResponseFinal(ResponseFinal),
    /// Declaration interest with an ID, update mode and optional routing metadata.
    Interest(Interest<'a>),
    /// Completion/removal marker for an interest ID.
    InterestFinal(InterestFinal),
    /// Declaration envelope containing a registration, removal or declaration-completion marker.
    Declare(Declare<'a>),
}

impl NetworkBody<'_> {
    /// Borrow a message view without cloning its payload or allocating storage.
    pub fn as_ref(&self) -> NetworkBodyRef<'_> {
        match self {
            NetworkBody::Push(x) => NetworkBodyRef::Push(x),
            NetworkBody::Request(x) => NetworkBodyRef::Request(x),
            NetworkBody::Response(x) => NetworkBodyRef::Response(x),
            NetworkBody::ResponseFinal(x) => NetworkBodyRef::ResponseFinal(x),
            NetworkBody::Interest(x) => NetworkBodyRef::Interest(x),
            NetworkBody::InterestFinal(x) => NetworkBodyRef::InterestFinal(x),
            NetworkBody::Declare(x) => NetworkBodyRef::Declare(x),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
/// Borrowed view of a network operation for forwarding without cloning its payload.
pub enum NetworkBodyRef<'a> {
    /// Unsolicited publication envelope, carrying a wire expression and data operation.
    Push(&'a Push<'a>),
    /// Outgoing request envelope, carrying a request ID, key and query-routing metadata.
    Request(&'a Request<'a>),
    /// Reply envelope correlated to an outgoing request by `rid`.
    Response(&'a Response<'a>),
    /// Mark the end of responses for a request ID; carries no sample payload.
    ResponseFinal(&'a ResponseFinal),
    /// Declaration interest with an ID, update mode and optional routing metadata.
    Interest(&'a Interest<'a>),
    /// Completion/removal marker for an interest ID.
    InterestFinal(&'a InterestFinal),
    /// Declaration envelope containing a registration, removal or declaration-completion marker.
    Declare(&'a Declare<'a>),
}

impl<'a> crate::ZBodyLen for NetworkBodyRef<'a> {
    fn z_body_len(&self) -> usize {
        match self {
            Self::Push(x) => <Push as crate::ZBodyLen>::z_body_len(x),
            Self::Request(x) => <Request as crate::ZBodyLen>::z_body_len(x),
            Self::Response(x) => <Response as crate::ZBodyLen>::z_body_len(x),
            Self::ResponseFinal(x) => <ResponseFinal as crate::ZBodyLen>::z_body_len(x),
            Self::Interest(x) => <Interest as crate::ZBodyLen>::z_body_len(x),
            Self::InterestFinal(x) => <InterestFinal as crate::ZBodyLen>::z_body_len(x),
            Self::Declare(x) => <Declare as crate::ZBodyLen>::z_body_len(x),
        }
    }
}

impl<'a> crate::ZLen for NetworkBodyRef<'a> {
    fn z_len(&self) -> usize {
        1 + <Self as crate::ZBodyLen>::z_body_len(self)
    }
}

impl<'a> crate::ZBodyEncode for NetworkBodyRef<'a> {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        match self {
            Self::Push(x) => <Push as crate::ZBodyEncode>::z_body_encode(x, w),
            Self::Request(x) => <Request as crate::ZBodyEncode>::z_body_encode(x, w),
            Self::Response(x) => <Response as crate::ZBodyEncode>::z_body_encode(x, w),
            Self::ResponseFinal(x) => <ResponseFinal as crate::ZBodyEncode>::z_body_encode(x, w),
            Self::Interest(x) => <Interest as crate::ZBodyEncode>::z_body_encode(x, w),
            Self::InterestFinal(x) => <InterestFinal as crate::ZBodyEncode>::z_body_encode(x, w),
            Self::Declare(x) => <Declare as crate::ZBodyEncode>::z_body_encode(x, w),
        }
    }
}

impl<'a> crate::ZEncode for NetworkBodyRef<'a> {
    fn z_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        match self {
            Self::Push(x) => <Push as crate::ZEncode>::z_encode(x, w),
            Self::Request(x) => <Request as crate::ZEncode>::z_encode(x, w),
            Self::Response(x) => <Response as crate::ZEncode>::z_encode(x, w),
            Self::ResponseFinal(x) => <ResponseFinal as crate::ZEncode>::z_encode(x, w),
            Self::Interest(x) => <Interest as crate::ZEncode>::z_encode(x, w),
            Self::InterestFinal(x) => <InterestFinal as crate::ZEncode>::z_encode(x, w),
            Self::Declare(x) => <Declare as crate::ZEncode>::z_encode(x, w),
        }
    }
}

#[derive(Debug, PartialEq)]
/// Network operation plus framing reliability and QoS. These fields describe wire policy, not delivery acknowledgment.
pub struct NetworkMessage<'a> {
    /// Reliability class for the enclosing transport frame.
    pub reliability: fields::Reliability,
    /// Wire QoS settings or negotiated QoS extension for this message.
    pub qos: exts::QoS,
    /// Operation/value carried by the envelope; borrowed data retains the input lifetime.
    pub body: NetworkBody<'a>,
}

impl NetworkMessage<'_> {
    /// Borrow a message view without cloning its payload or allocating storage.
    pub fn as_ref(&self) -> NetworkMessageRef<'_> {
        NetworkMessageRef {
            reliability: self.reliability,
            qos: self.qos,
            body: self.body.as_ref(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
/// Borrowed network operation with copied framing metadata, used for re-encoding or forwarding.
pub struct NetworkMessageRef<'a> {
    /// Reliability class for the enclosing transport frame.
    pub reliability: fields::Reliability,
    /// Wire QoS settings or negotiated QoS extension for this message.
    pub qos: exts::QoS,
    /// Operation/value carried by the envelope; borrowed data retains the input lifetime.
    pub body: NetworkBodyRef<'a>,
}

#[derive(ZEnum, Debug, PartialEq)]
/// Transport control message used for handshakes, keepalives and closure.
pub enum TransportMessage<'a> {
    /// Transport closure notification with a reason code and link/session scope.
    Close(Close),
    /// Initial outgoing transport proposal. Optional extensions describe wire capabilities, not implemented runtime features.
    InitSyn(InitSyn<'a>),
    /// Transport initialization acknowledgment with negotiated settings and an opaque cookie.
    InitAck(InitAck<'a>),
    /// Payload-free transport keepalive; its encoded representation includes the message header.
    KeepAlive(KeepAlive),
    /// Open request confirming lease, initial sequence number and the initialization cookie.
    OpenSyn(OpenSyn<'a>),
    /// Open acknowledgment confirming lease and initial sequence number.
    OpenAck(OpenAck<'a>),
}

impl TransportMessage<'_> {
    /// Borrow a message view without cloning its payload or allocating storage.
    pub fn as_ref(&self) -> TransportMessageRef<'_> {
        match self {
            TransportMessage::Close(x) => TransportMessageRef::Close(x),
            TransportMessage::InitSyn(x) => TransportMessageRef::InitSyn(x),
            TransportMessage::InitAck(x) => TransportMessageRef::InitAck(x),
            TransportMessage::KeepAlive(x) => TransportMessageRef::KeepAlive(x),
            TransportMessage::OpenSyn(x) => TransportMessageRef::OpenSyn(x),
            TransportMessage::OpenAck(x) => TransportMessageRef::OpenAck(x),
        }
    }
}

#[derive(Debug, PartialEq)]
/// Borrowed transport control message for encoding without cloning payload data.
pub enum TransportMessageRef<'a> {
    /// Transport closure notification with a reason code and link/session scope.
    Close(&'a Close),
    /// Initial outgoing transport proposal. Optional extensions describe wire capabilities, not implemented runtime features.
    InitSyn(&'a InitSyn<'a>),
    /// Transport initialization acknowledgment with negotiated settings and an opaque cookie.
    InitAck(&'a InitAck<'a>),
    /// Payload-free transport keepalive; its encoded representation includes the message header.
    KeepAlive(&'a KeepAlive),
    /// Open request confirming lease, initial sequence number and the initialization cookie.
    OpenSyn(&'a OpenSyn<'a>),
    /// Open acknowledgment confirming lease and initial sequence number.
    OpenAck(&'a OpenAck<'a>),
}

impl<'a> crate::ZBodyLen for TransportMessageRef<'a> {
    fn z_body_len(&self) -> usize {
        match self {
            Self::Close(x) => <Close as crate::ZBodyLen>::z_body_len(x),
            Self::InitSyn(x) => <InitSyn as crate::ZBodyLen>::z_body_len(x),
            Self::InitAck(x) => <InitAck as crate::ZBodyLen>::z_body_len(x),
            Self::KeepAlive(x) => <KeepAlive as crate::ZBodyLen>::z_body_len(x),
            Self::OpenSyn(x) => <OpenSyn as crate::ZBodyLen>::z_body_len(x),
            Self::OpenAck(x) => <OpenAck as crate::ZBodyLen>::z_body_len(x),
        }
    }
}

impl<'a> crate::ZLen for TransportMessageRef<'a> {
    fn z_len(&self) -> usize {
        1 + <Self as crate::ZBodyLen>::z_body_len(self)
    }
}

impl<'a> crate::ZBodyEncode for TransportMessageRef<'a> {
    fn z_body_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        match self {
            Self::Close(x) => <Close as crate::ZBodyEncode>::z_body_encode(x, w),
            Self::InitSyn(x) => <InitSyn as crate::ZBodyEncode>::z_body_encode(x, w),
            Self::InitAck(x) => <InitAck as crate::ZBodyEncode>::z_body_encode(x, w),
            Self::KeepAlive(x) => <KeepAlive as crate::ZBodyEncode>::z_body_encode(x, w),
            Self::OpenSyn(x) => <OpenSyn as crate::ZBodyEncode>::z_body_encode(x, w),
            Self::OpenAck(x) => <OpenAck as crate::ZBodyEncode>::z_body_encode(x, w),
        }
    }
}

impl<'a> crate::ZEncode for TransportMessageRef<'a> {
    fn z_encode(
        &self,
        w: &mut impl crate::ZWriteable,
    ) -> core::result::Result<(), crate::CodecError> {
        match self {
            Self::Close(x) => <Close as crate::ZEncode>::z_encode(x, w),
            Self::InitSyn(x) => <InitSyn as crate::ZEncode>::z_encode(x, w),
            Self::InitAck(x) => <InitAck as crate::ZEncode>::z_encode(x, w),
            Self::KeepAlive(x) => <KeepAlive as crate::ZEncode>::z_encode(x, w),
            Self::OpenSyn(x) => <OpenSyn as crate::ZEncode>::z_encode(x, w),
            Self::OpenAck(x) => <OpenAck as crate::ZEncode>::z_encode(x, w),
        }
    }
}

#[derive(Debug, PartialEq)]
/// Decoded transport control or network message borrowing its input buffer.
pub enum Message<'a> {
    /// Network-level operation and framing metadata.
    Network(NetworkMessage<'a>),
    /// Transport handshake, keepalive or closure message.
    Transport(TransportMessage<'a>),
}

#[derive(Debug, PartialEq)]
/// Borrowed message view used by encoding and forwarding paths.
pub enum MessageRef<'a> {
    /// Network-level operation and framing metadata.
    Network(NetworkMessageRef<'a>),
    /// Transport handshake, keepalive or closure message.
    Transport(TransportMessageRef<'a>),
}

impl Message<'_> {
    /// Borrow a message view without cloning its payload or allocating storage.
    pub fn as_ref(&self) -> MessageRef<'_> {
        match self {
            Message::Transport(msg) => MessageRef::Transport(msg.as_ref()),
            Message::Network(msg) => MessageRef::Network(msg.as_ref()),
        }
    }
}
