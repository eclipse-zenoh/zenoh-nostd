use crate::{
    api::{
        arg::{GetResponseRef, QueryableQueryRef, SampleRef},
        callbacks::ZCallbacks,
    },
    io::{link::ZLinkManager, transport::TransportLinkManager},
};

pub trait ZSessionConfig: Sized {
    type Buff: AsMut<[u8]> + AsRef<[u8]> + Clone;
    type LinkManager: ZLinkManager;

    type SubCallbacks<'res>: ZCallbacks<'res, SampleRef>;
    type GetCallbacks<'res>: ZCallbacks<'res, GetResponseRef>;
    type QueryableCallbacks<'res>: ZCallbacks<'res, QueryableQueryRef<'res, Self>>
    where
        Self: 'res;

    /// Buffer for messages between entities of the same session: a put that
    /// reaches a local subscriber, a get that reaches a local queryable, and
    /// the replies to it.
    ///
    /// The session holds two of these, so one can fill while the other is
    /// dispatched. A local message that doesn't fit is not delivered
    /// locally. A zero-length buffer such as `[u8; 0]` disables same-session
    /// delivery.
    type LoopbackBuff: AsMut<[u8]> + AsRef<[u8]>;

    fn transports(&self) -> &TransportLinkManager<Self::LinkManager>;
    fn buff(&self) -> Self::Buff;
    fn loopback_buff(&self) -> Self::LoopbackBuff;
}

#[allow(dead_code)]
pub trait ZBrokerConfig {
    type Buff: AsMut<[u8]> + AsRef<[u8]> + Clone;
    type LinkManager: ZLinkManager;

    fn transports(&self) -> &TransportLinkManager<Self::LinkManager>;
    fn buff(&self) -> Self::Buff;
}
