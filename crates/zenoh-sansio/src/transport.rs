use core::fmt::Display;
use core::time::Duration;

use establishment::Description;
use zenoh_proto::{fields::*, msgs::*};

pub(crate) mod establishment;

mod handshake;
mod rx;
mod traits;
mod tx;

pub use handshake::*;
pub use rx::*;
pub use traits::*;
pub use tx::*;

use crate::transport::establishment::State;

/// Configure transport framing and handshake proposals using caller-owned codec storage.
///
/// Defaults: random Zenoh ID, ten-second lease, 32-bit resolution, and batch size
/// from the buffer length cast to `u16`. Use a buffer between 2 and 65535 bytes
/// and keep batch sizes compatible with it. No socket, executor or timer is created.
pub struct TransportBuilder<Buff> {
    zid: ZenohIdProto,
    batch_size: u16,
    lease: Duration,
    resolution: Resolution,

    buff: Buff,
}

impl<Buff> TransportBuilder<Buff> {
    fn new(buff: Buff) -> Self
    where
        Buff: AsRef<[u8]>,
    {
        TransportBuilder {
            zid: ZenohIdProto::default(),
            batch_size: buff.as_ref().len() as u16,
            lease: Duration::from_secs(10),
            resolution: Resolution::default(),
            buff,
        }
    }
    /// Set the local node ID instead of the randomly generated default.
    pub fn with_zid(mut self, zid: ZenohIdProto) -> Self {
        self.zid = zid;
        self
    }

    /// Set the proposed batch limit in bytes.
    ///
    /// This does not resize storage: keep it within the buffer capacity and leave
    /// room for transport framing. Incompatible sizes can drop encoded messages or
    /// cause framing failures.
    pub fn with_batch_size(mut self, batch_size: u16) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Set the advertised inactivity lease; ten seconds by default.
    ///
    /// The caller supplies monotonic timestamps to `Transport::sync`; no timer runs
    /// inside this crate.
    pub fn with_lease(mut self, lease: Duration) -> Self {
        self.lease = lease;
        self
    }

    /// Set proposed frame-sequence/request-ID bit widths; defaults to 32 bits each.
    pub fn with_resolution(mut self, resolution: Resolution) -> Self {
        self.resolution = resolution;
        self
    }

    /// Replace codec storage without changing the configured batch limit.
    ///
    /// Adjust the batch limit separately if the replacement buffer has a different capacity.
    pub fn with_buff<NewBuff>(self, buff: NewBuff) -> TransportBuilder<NewBuff> {
        TransportBuilder {
            zid: self.zid,
            batch_size: self.batch_size,
            lease: self.lease,
            resolution: self.resolution,
            buff,
        }
    }

    /// Create a local codec without exchanging a handshake.
    ///
    /// Clones the buffer for independent RX/TX storage, starts sequence numbers at
    /// zero and uses the local ID for both ends. Intended for codec work/tests; use
    /// `connect`/`listen` for a negotiated connection.
    pub fn codec(self) -> Transport<Buff>
    where
        Buff: Clone,
    {
        Transport {
            tx: TransportTx::new(
                self.buff.clone(),
                self.batch_size as usize,
                0,
                self.resolution,
                self.lease,
            ),
            rx: TransportRx::new(
                self.buff,
                self.batch_size as usize,
                0,
                self.resolution,
                self.lease,
            ),
            mine_zid: self.zid,
            other_zid: self.zid,
        }
    }

    /// Prepare an incoming handshake with synchronous application I/O callbacks.
    ///
    /// No I/O occurs until the handshake is polled or finished. Select stream framing
    /// with [`Handshake::prefixed`] before polling TCP-like links. Reads must fill
    /// requested prefix/body slices for streams, or return exactly one datagram for
    /// message-oriented links. Writes must send the entire provided slice. The caller
    /// owns connection setup, deadlines and teardown after failed/interrupted I/O.
    pub fn listen<T, E, Read, Write>(
        self,
        handle: T,
        read: Read,
        write: Write,
    ) -> Handshake<Buff, T, Read, Write>
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: FnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: FnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        let state = State::WaitingInitSyn {
            mine_zid: self.zid,
            mine_batch_size: self.batch_size,
            mine_resolution: self.resolution,
            mine_lease: self.lease,
        };

        let tx = TransportTx::new(
            self.buff.clone(),
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        );

        let rx = TransportRx::new(
            self.buff,
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        );

        Handshake::PendingRecv {
            state,
            prefixed: false,
            tx,
            rx,
            handle,
            read,
            write,
        }
    }

    /// Prepare an incoming handshake with asynchronous application I/O callbacks.
    ///
    /// No I/O occurs until the handshake is polled or finished. Select stream framing
    /// with [`Handshake::prefixed`] before polling TCP-like links. Reads must fill
    /// requested prefix/body slices for streams, or return exactly one datagram for
    /// message-oriented links. Writes must send the entire provided slice. The caller
    /// owns connection setup, deadlines and teardown after failed/interrupted I/O.
    pub fn listen_async<T, E, Read, Write>(
        self,
        handle: T,
        read: Read,
        write: Write,
    ) -> Handshake<Buff, T, Read, Write>
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: AsyncFnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: AsyncFnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        let state = State::WaitingInitSyn {
            mine_zid: self.zid,
            mine_batch_size: self.batch_size,
            mine_resolution: self.resolution,
            mine_lease: self.lease,
        };

        let tx = TransportTx::new(
            self.buff.clone(),
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        );

        let rx = TransportRx::new(
            self.buff,
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        );

        Handshake::PendingRecv {
            state,
            prefixed: false,
            tx,
            rx,
            handle,
            read,
            write,
        }
    }

    /// Prepare an outgoing handshake with synchronous application I/O callbacks.
    ///
    /// No I/O occurs until the handshake is polled or finished. Select stream framing
    /// with [`Handshake::prefixed`] before polling TCP-like links. Reads must fill
    /// requested prefix/body slices for streams, or return exactly one datagram for
    /// message-oriented links. Writes must send the entire provided slice. The caller
    /// owns connection setup, deadlines and teardown after failed/interrupted I/O.
    pub fn connect<T, E, Read, Write>(
        self,
        handle: T,
        read: Read,
        write: Write,
    ) -> Handshake<Buff, T, Read, Write>
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: FnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: FnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        let state = State::WaitingInitAck {
            mine_zid: self.zid,
            mine_batch_size: self.batch_size,
            mine_resolution: self.resolution,
            mine_lease: self.lease,
        };

        let tx = TransportTx::new(
            self.buff.clone(),
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        );

        let rx = TransportRx::new(
            self.buff,
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        );

        Handshake::PendingInit {
            state,
            init: InitSyn {
                identifier: InitIdentifier {
                    zid: self.zid,
                    ..Default::default()
                },
                resolution: InitResolution {
                    resolution: self.resolution,
                    batch_size: BatchSize(self.batch_size),
                },
                ..Default::default()
            },
            prefixed: false,
            tx,
            rx,
            handle,
            read,
            write,
        }
    }

    /// Prepare an outgoing handshake with asynchronous application I/O callbacks.
    ///
    /// No I/O occurs until the handshake is polled or finished. Select stream framing
    /// with [`Handshake::prefixed`] before polling TCP-like links. Reads must fill
    /// requested prefix/body slices for streams, or return exactly one datagram for
    /// message-oriented links. Writes must send the entire provided slice. The caller
    /// owns connection setup, deadlines and teardown after failed/interrupted I/O.
    pub fn connect_async<T, E, Read, Write>(
        self,
        handle: T,
        read: Read,
        write: Write,
    ) -> Handshake<Buff, T, Read, Write>
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: AsyncFnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: AsyncFnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        let state = State::WaitingInitAck {
            mine_zid: self.zid,
            mine_batch_size: self.batch_size,
            mine_resolution: self.resolution,
            mine_lease: self.lease,
        };

        let tx = TransportTx::new(
            self.buff.clone(),
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        );

        let rx = TransportRx::new(
            self.buff,
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        );

        Handshake::PendingInit {
            state,
            init: InitSyn {
                identifier: InitIdentifier {
                    zid: self.zid,
                    ..Default::default()
                },
                resolution: InitResolution {
                    resolution: self.resolution,
                    batch_size: BatchSize(self.batch_size),
                },
                ..Default::default()
            },
            prefixed: false,
            tx,
            rx,
            handle,
            read,
            write,
        }
    }
}

/// Transport codec state with independent RX/TX buffers and negotiated node IDs.
///
/// Drive I/O externally through [`ZTransportRx`] and [`ZTransportTx`], and supply
/// monotonic time with [`Self::sync`]. This type has no networking or executor.
/// A codec flush borrows encoded/decoded data; it does not flush a socket.
pub struct Transport<Buff> {
    /// Transmit codec and outgoing lease bookkeeping.
    pub tx: TransportTx<Buff>,
    /// Receive codec and incoming lease bookkeeping.
    pub rx: TransportRx<Buff>,

    /// Local node identifier.
    pub mine_zid: ZenohIdProto,
    /// Remote node identifier negotiated during opening.
    pub other_zid: ZenohIdProto,
}

impl<Buff> Transport<Buff> {
    /// Create transport settings from caller-owned buffer storage; no I/O occurs yet.
    pub fn builder(buff: Buff) -> TransportBuilder<Buff>
    where
        Buff: AsRef<[u8]>,
    {
        TransportBuilder::new(buff)
    }

    pub(crate) fn new(description: Description, tx: Buff, rx: Buff) -> Self {
        Self {
            tx: TransportTx::new(
                tx,
                description.batch_size as usize,
                description.mine_sn,
                description.resolution,
                description.mine_lease,
            ),
            rx: TransportRx::new(
                rx,
                description.batch_size as usize,
                description.other_sn,
                description.resolution,
                description.other_lease,
            ),
            mine_zid: description.mine_zid,
            other_zid: description.other_zid,
        }
    }

    /// Synchronize RX/TX activity and closure using a monotonic time since a common epoch.
    ///
    /// Call regularly after I/O and at lease deadlines. This updates bookkeeping
    /// only; the caller must send keepalives and handle closed transports.
    pub fn sync(&mut self, now: Duration) {
        let Self { tx, rx, .. } = self;
        rx.sync(Some(tx), now);
        tx.sync(Some(rx), now);
    }

    /// Whether either half has entered its closed state.
    ///
    /// This reflects codec bookkeeping; it does not probe the underlying socket.
    pub fn closed(&self) -> bool {
        self.tx.closed() || self.rx.closed()
    }

    /// Borrow independent TX/RX codecs; keep the parent alive until both borrows end.
    pub fn split(&mut self) -> (&mut TransportTx<Buff>, &mut TransportRx<Buff>) {
        (&mut self.tx, &mut self.rx)
    }
}
