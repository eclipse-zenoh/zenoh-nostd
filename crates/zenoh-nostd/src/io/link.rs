use zenoh_proto::{Endpoint, LinkError};

/// Framing properties shared by a link and its split halves.
pub trait ZLinkInfo {
    /// Report the link's frame-size limit in bytes.
    ///
    /// This is a transport/framing property, not a callback or packet-queue count.
    fn mtu(&self) -> u16;
    /// Whether the transport must use length-prefixed byte-stream framing.
    ///
    /// Return `true` for streams such as TCP; return `false` when reads and writes
    /// preserve whole message boundaries, such as UDP datagrams.
    fn is_streamed(&self) -> bool;
}

/// Asynchronous transmission for one side of a connection.
///
/// For a message-oriented link, each `write_all` sends one complete message;
/// for a stream it may require multiple underlying writes to send the buffer.
pub trait ZLinkTx: ZLinkInfo {
    /// Write the entire buffer. Cancellation or a write error may leave a
    /// partial frame on a stream. The caller must stop the driver and discard
    /// the enclosing session/transport, including both link halves, before any
    /// further network I/O. This operation is not generally cancellation-safe.
    fn write_all(
        &mut self,
        buffer: &[u8],
    ) -> impl Future<Output = core::result::Result<(), zenoh_proto::LinkError>>;

    /// Finish any buffered transmission before the link is dropped. Links whose
    /// writes already hand data to an independently owned transport may keep
    /// the default; adapters with application-owned TX buffers must override it.
    fn flush(&mut self) -> impl Future<Output = Result<(), LinkError>> {
        async { Ok(()) }
    }
}

/// Asynchronous receive operations used during handshakes and session driving.
pub trait ZLinkRx: ZLinkInfo {
    /// Read bytes and return the number written into `buffer`.
    ///
    /// For message-oriented links, receive one complete message without combining
    /// packets; report an error when it cannot fit rather than silently truncating it.
    /// For streams, a read may return fewer bytes than the buffer capacity.
    fn read(
        &mut self,
        buffer: &mut [u8],
    ) -> impl Future<Output = core::result::Result<usize, zenoh_proto::LinkError>>;

    /// Fill `buffer` before succeeding, or report a link error.
    ///
    /// The transport uses this for stream framing. Message-oriented transports use
    /// [`Self::read`] instead and must not concatenate datagrams to fill a buffer.
    fn read_exact(
        &mut self,
        buffer: &mut [u8],
    ) -> impl Future<Output = core::result::Result<(), zenoh_proto::LinkError>>;
}

/// A connection that supports independently polled RX and TX halves.
///
/// The halves must describe the same connection and framing properties. They may
/// borrow the original link, which must stay alive until both halves are dropped.
pub trait ZLink: ZLinkInfo + ZLinkTx + ZLinkRx {
    /// Transmit half, optionally borrowing resources from the link.
    type Tx<'link>: ZLinkTx + ZLinkInfo
    where
        Self: 'link;

    /// Receive half, optionally borrowing resources from the link.
    type Rx<'link>: ZLinkRx + ZLinkInfo
    where
        Self: 'link;

    /// Split a mutable link borrow into RX/TX handles for concurrent polling.
    ///
    /// Halves must not alias mutable storage unsafely. Dropping both halves before
    /// the parent link must allow the implementation to release connection resources.
    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>);
}

/// Open raw links for the Zenoh transport layer.
///
/// Implementations own or borrow network state and may reserve bounded socket
/// resources. [`crate::session::TransportLinkManager`] performs the Zenoh handshake
/// after opening a link; these methods only establish the underlying connection.
/// Release reserved resources when opening fails, is cancelled, or the link drops.
/// Return an error for unsupported protocols or listening modes.
pub trait ZLinkManager {
    /// Connection type whose resource borrow cannot outlive this manager.
    type Link<'a>: ZLink
    where
        Self: 'a;

    /// Open an outgoing raw connection to `endpoint`; no Zenoh handshake is done here.
    fn connect(
        &self,
        endpoint: Endpoint<'_>,
    ) -> impl Future<Output = core::result::Result<Self::Link<'_>, LinkError>>;

    /// Wait for an incoming raw connection, or return an error if unsupported.
    fn listen(
        &self,
        endpoint: Endpoint<'_>,
    ) -> impl Future<Output = core::result::Result<Self::Link<'_>, LinkError>>;
}

/// Wrap separate embedded-I/O readers/writers as a Zenoh link.
///
/// The application supplies connection management, framing properties and the MTU.
/// This wrapper does not initialize hardware or establish a network connection.
pub struct EmbeddedIOLink<Tx: embedded_io_async::Write, Rx: embedded_io_async::Read> {
    tx: Tx,
    rx: Rx,

    mtu: u16,
    is_streamed: bool,
}

impl<Tx: embedded_io_async::Write, Rx: embedded_io_async::Read> EmbeddedIOLink<Tx, Rx> {
    /// Wrap an established connection with its frame-size limit and framing mode.
    ///
    /// Set `is_streamed` for byte streams; message-oriented readers/writers must
    /// preserve one message per operation as required by [`ZLinkRx`] and [`ZLinkTx`].
    pub fn new(tx: Tx, rx: Rx, mtu: u16, is_streamed: bool) -> Self {
        Self {
            tx,
            rx,
            mtu,
            is_streamed,
        }
    }
}

pub struct EmbeddedIOLinkTx<'a, Tx: embedded_io_async::Write> {
    tx: &'a mut Tx,

    mtu: u16,
    is_streamed: bool,
}

pub struct EmbeddedIOLinkRx<'a, Rx: embedded_io_async::Read> {
    rx: &'a mut Rx,

    mtu: u16,
    is_streamed: bool,
}

impl<Tx: embedded_io_async::Write, Rx: embedded_io_async::Read> ZLinkInfo
    for EmbeddedIOLink<Tx, Rx>
{
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        self.is_streamed
    }
}

impl<Tx: embedded_io_async::Write> ZLinkInfo for EmbeddedIOLinkTx<'_, Tx> {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        self.is_streamed
    }
}

impl<Rx: embedded_io_async::Read> ZLinkInfo for EmbeddedIOLinkRx<'_, Rx> {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        self.is_streamed
    }
}

impl<Tx: embedded_io_async::Write, Rx: embedded_io_async::Read> ZLinkTx for EmbeddedIOLink<Tx, Rx> {
    async fn flush(&mut self) -> Result<(), LinkError> {
        self.tx.flush().await.map_err(|_| LinkError::LinkTxFailed)
    }

    async fn write_all(
        &mut self,
        buffer: &[u8],
    ) -> core::result::Result<(), zenoh_proto::LinkError> {
        self.tx
            .write_all(buffer)
            .await
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl<Tx: embedded_io_async::Write> ZLinkTx for EmbeddedIOLinkTx<'_, Tx> {
    async fn flush(&mut self) -> Result<(), LinkError> {
        self.tx.flush().await.map_err(|_| LinkError::LinkTxFailed)
    }

    async fn write_all(
        &mut self,
        buffer: &[u8],
    ) -> core::result::Result<(), zenoh_proto::LinkError> {
        self.tx
            .write_all(buffer)
            .await
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl<Tx: embedded_io_async::Write, Rx: embedded_io_async::Read> ZLinkRx for EmbeddedIOLink<Tx, Rx> {
    async fn read(
        &mut self,
        buffer: &mut [u8],
    ) -> core::result::Result<usize, zenoh_proto::LinkError> {
        self.rx
            .read(buffer)
            .await
            .map_err(|_| LinkError::LinkRxFailed)
    }

    async fn read_exact(
        &mut self,
        buffer: &mut [u8],
    ) -> core::result::Result<(), zenoh_proto::LinkError> {
        self.rx
            .read_exact(buffer)
            .await
            .map_err(|_| LinkError::LinkRxFailed)
    }
}

impl<Rx: embedded_io_async::Read> ZLinkRx for EmbeddedIOLinkRx<'_, Rx> {
    async fn read(
        &mut self,
        buffer: &mut [u8],
    ) -> core::result::Result<usize, zenoh_proto::LinkError> {
        self.rx
            .read(buffer)
            .await
            .map_err(|_| LinkError::LinkRxFailed)
    }

    async fn read_exact(
        &mut self,
        buffer: &mut [u8],
    ) -> core::result::Result<(), zenoh_proto::LinkError> {
        self.rx
            .read_exact(buffer)
            .await
            .map_err(|_| LinkError::LinkRxFailed)
    }
}

impl<Tx: embedded_io_async::Write, Rx: embedded_io_async::Read> ZLink for EmbeddedIOLink<Tx, Rx> {
    type Tx<'link>
        = EmbeddedIOLinkTx<'link, Tx>
    where
        Self: 'link;

    type Rx<'link>
        = EmbeddedIOLinkRx<'link, Rx>
    where
        Self: 'link;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        let Self {
            tx,
            rx,
            mtu,
            is_streamed,
        } = self;

        (
            EmbeddedIOLinkTx {
                tx,
                mtu: *mtu,
                is_streamed: *is_streamed,
            },
            EmbeddedIOLinkRx {
                rx,
                mtu: *mtu,
                is_streamed: *is_streamed,
            },
        )
    }
}
