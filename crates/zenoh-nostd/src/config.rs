use crate::{
    api::{
        arg::{GetResponseRef, QueryableQueryRef, SampleRef},
        callbacks::ZCallbacks,
    },
    io::{link::ZLinkManager, transport::TransportLinkManager},
};

/// Application-owned transport, buffer, and callback storage policy.
///
/// Implement this trait to choose each registry independently. Capacities count
/// active registrations, not messages: one wildcard subscription uses one slot,
/// while registering it twice uses two. Get slots count pending outgoing requests;
/// queryable slots count declarations and, in fixed storage, separately bound the
/// number of incoming requests tracked for finalization.
///
/// Registered key text is copied by the provided registries, but operation handles
/// may still borrow the original key. Callback captures must live for `'transport`.
/// For queryables, `'resources: 'transport` keeps link resources alive for deferred
/// replies. Callback object and returned-future storage can be selected separately
/// through the aliases' storage parameters.
///
/// # Example
///
/// This configuration reserves eight subscriptions, four gets and two queryables.
/// The aliases default to a 128-byte key limit; increase their `MAX_KEYEXPR` argument
/// for longer UTF-8 keys. Full fixed tables reject insertion rather than growing.
/// Enabling `alloc` alone does not replace fixed tables: select the `Alloc*Callbacks`
/// aliases to use heap-backed maps. Socket buffers and UDP metadata queues belong
/// to the platform adapter and are configured separately.
///
/// ```
/// use zenoh_nostd::{platform::ZLinkManager, session::*};
///
/// struct AppConfig<M: ZLinkManager> {
///     transports: TransportLinkManager<M>,
/// }
///
/// impl<M: ZLinkManager> ZSessionConfig for AppConfig<M> {
///     type LinkManager = M;
///     type Buff = [u8; 512];
///     type SubCallbacks<'transport> = FixedCapacitySubCallbacks<'transport, 8>;
///     type GetCallbacks<'transport> = FixedCapacityGetCallbacks<'transport, 4>;
///     type QueryableCallbacks<'transport, 'resources> = FixedCapacityQueryableCallbacks<'transport, 'resources, Self, 2>
///     where
///         Self: 'resources, 'resources: 'transport;
///
///     fn transports(&self) -> &TransportLinkManager<M> {
///         &self.transports
///     }
///
///     fn buff(&self) -> Self::Buff {
///         [0; 512]
///     }
/// }
/// ```
pub trait ZSessionConfig: Sized {
    /// Owned protocol buffer type, cloned for the transport codec.
    ///
    /// Choose capacity for negotiated frame sizes and application payloads. Socket
    /// buffers, callback counts and owned sample storage are configured separately.
    type Buff: AsMut<[u8]> + AsRef<[u8]> + Clone;
    /// Platform-specific connection manager; its links may borrow its resources.
    type LinkManager: ZLinkManager;

    /// Registry for active subscriptions and their sample callbacks.
    type SubCallbacks<'transport>: ZCallbacks<'transport, SampleRef>;
    /// Registry for pending outgoing gets and their reply callbacks.
    type GetCallbacks<'transport>: ZCallbacks<'transport, GetResponseRef>;
    /// Registry for queryable declarations and incoming-request finalization counters.
    ///
    /// The transport borrow may be shorter than the lifetime of the link resources.
    type QueryableCallbacks<'transport, 'resources>: ZCallbacks<'transport, QueryableQueryRef<'transport, 'resources, Self>>
    where
        Self: 'resources,
        'resources: 'transport;

    /// Return the configured transport manager used for connection handshakes.
    fn transports(&self) -> &TransportLinkManager<Self::LinkManager>;
    /// Create an independently owned protocol buffer for a new connection.
    fn buff(&self) -> Self::Buff;
}

#[allow(dead_code)]
pub trait ZBrokerConfig {
    type Buff: AsMut<[u8]> + AsRef<[u8]> + Clone;
    type LinkManager: ZLinkManager;

    fn transports(&self) -> &TransportLinkManager<Self::LinkManager>;
    fn buff(&self) -> Self::Buff;
}
