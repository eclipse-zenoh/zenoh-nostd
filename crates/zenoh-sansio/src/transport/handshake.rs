use core::fmt::Display;

use zenoh_proto::{EitherError, TransportError, msgs::InitSyn};

use crate::{
    Transport, TransportRx, TransportTx, ZTransportRx, ZTransportTx,
    establishment::{Description, State},
};

/// Handshake state driven by application-provided read/write callbacks.
///
/// Create through [`TransportBuilder`](crate::TransportBuilder), then use
/// [`Self::poll`]/[`Self::poll_async`] or a `finish` method. The caller provides
/// connection setup and timeouts. Public variants expose implementation state;
/// prefer builders and methods rather than constructing states directly.
pub enum Handshake<Buff, T, Read, Write> {
    /// Outgoing initialization is ready to be sent.
    PendingInit {
        #[allow(private_interfaces)]
        /// Internal negotiation state; not directly constructible outside the crate.
        state: State,

        /// Initial outgoing proposal, sent on the next poll.
        init: InitSyn<'static>,
        /// Whether I/O uses a two-byte little-endian batch-length prefix.
        prefixed: bool,

        /// Transmit codec holding handshake output.
        tx: TransportTx<Buff>,
        /// Receive codec holding handshake input.
        rx: TransportRx<Buff>,

        /// Application I/O handle passed to read/write callbacks.
        handle: T,

        /// Application read callback; stream mode requires exact requested lengths.
        read: Read,
        /// Application write callback; must transmit the complete supplied slice.
        write: Write,
    },

    /// Waiting for the next handshake message or negotiated-ready transition.
    PendingRecv {
        #[allow(private_interfaces)]
        /// Internal negotiation state; not directly constructible outside the crate.
        state: State,

        /// Whether I/O uses a two-byte little-endian batch-length prefix.
        prefixed: bool,

        /// Transmit codec holding handshake output.
        tx: TransportTx<Buff>,
        /// Receive codec holding handshake input.
        rx: TransportRx<Buff>,

        /// Application I/O handle passed to read/write callbacks.
        handle: T,

        /// Application read callback; stream mode requires exact requested lengths.
        read: Read,
        /// Application write callback; must transmit the complete supplied slice.
        write: Write,
    },
    /// Handshake negotiation is complete; `HandshakeReady::open` takes the codecs.
    Ready {
        #[allow(private_interfaces)]
        /// Internal negotiated parameters used when opening the transport.
        description: Description,

        /// Transmit codec holding handshake output.
        tx: TransportTx<Buff>,
        /// Receive codec holding handshake input.
        rx: TransportRx<Buff>,
    },
    /// The negotiated transport has been taken; subsequent polls return `None`.
    Opened,
}

/// Proof that a polled handshake can be consumed as an established transport.
///
/// Returned only by a successful poll after negotiation; call [`Self::open`] once.
pub struct HandshakeReady<'a, Buff, T, Read, Write> {
    handshake: &'a mut Handshake<Buff, T, Read, Write>,
}

impl<'a, Buff, T, Read, Write> HandshakeReady<'a, Buff, T, Read, Write> {
    /// Take the negotiated transport, leaving the handshake in `Opened` state.
    pub fn open(self) -> Transport<Buff> {
        if let Handshake::Ready {
            description,
            tx,
            rx,
        } = core::mem::replace(self.handshake, Handshake::Opened)
        {
            Transport::new(description, tx.into_inner(), rx.into_inner())
        } else {
            unreachable!()
        }
    }
}

impl<Buff, T, Read, Write> Handshake<Buff, T, Read, Write> {
    /// Enable two-byte little-endian batch-length framing for byte streams.
    ///
    /// Select before the first poll. Use raw framing for datagram/message links.
    pub fn prefixed(mut self) -> Self {
        if let Self::PendingRecv { prefixed, .. } | Self::PendingInit { prefixed, .. } = &mut self {
            *prefixed = true;
        }

        self
    }

    /// Select stream (`true`) or raw message (`false`, default) framing before polling.
    pub fn with_prefixed(mut self, p: bool) -> Self {
        if let Self::PendingRecv { prefixed, .. } | Self::PendingInit { prefixed, .. } = &mut self {
            *prefixed = p;
        }

        self
    }

    #[allow(clippy::type_complexity)]
    /// Advance a handshake step using synchronous I/O callbacks.
    ///
    /// Returns `Some(ready)` when negotiation completes, `None` while progressing
    /// or after opening, and `EitherError` for protocol or callback failures. This
    /// is not a `Future::poll`: callbacks can block and are called directly.
    pub fn poll<E>(
        &mut self,
    ) -> core::result::Result<
        Option<HandshakeReady<'_, Buff, T, Read, Write>>,
        EitherError<TransportError, E>,
    >
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: FnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: FnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        match self {
            Self::Opened => Ok(None),
            Self::Ready { .. } => Ok(Some(HandshakeReady { handshake: self })),
            Self::PendingInit {
                init,
                prefixed,
                tx,
                handle,
                write,
                ..
            } => {
                tx.init_syn(init);

                if let Some(bytes) = tx.flush(*prefixed) {
                    write(handle, bytes).map_err(EitherError::B)?;
                }

                if let Self::PendingInit {
                    state,
                    prefixed,
                    tx,
                    rx,
                    handle,
                    read,
                    write,
                    ..
                } = core::mem::replace(self, Self::Opened)
                {
                    *self = Self::PendingRecv {
                        state,
                        prefixed,
                        tx,
                        rx,
                        handle,
                        read,
                        write,
                    };
                } else {
                    unreachable!()
                }

                Ok(None)
            }
            Self::PendingRecv {
                state,
                prefixed,
                tx,
                rx,
                handle,
                read,
                write,
            } => {
                if let Some(description) = state.description() {
                    if let Self::PendingRecv { tx, rx, .. } = core::mem::replace(self, Self::Opened)
                    {
                        *self = Self::Ready {
                            description,
                            tx,
                            rx,
                        };

                        return Ok(Some(HandshakeReady { handshake: self }));
                    } else {
                        unreachable!()
                    }
                }

                rx.decode_with(|bytes| read(handle, bytes), *prefixed)?;
                let resp = rx
                    .flush_transport()
                    .map(|msg| state.poll(msg))
                    .filter_map(|response| response.0);

                for resp in resp {
                    tx.transport(resp)
                }

                if let Some(bytes) = tx.flush(*prefixed) {
                    write(handle, bytes).map_err(EitherError::B)?;
                }

                Ok(None)
            }
        }
    }

    /// Advance a handshake step by awaiting the supplied I/O callbacks.
    ///
    /// Returns `Some(ready)` on negotiation, `None` while progressing or after
    /// opening. Discard the connection after a failed or cancelled I/O operation;
    /// this method does not provide resumable partial writes.
    pub async fn poll_async<E>(
        &mut self,
    ) -> core::result::Result<
        Option<HandshakeReady<'_, Buff, T, Read, Write>>,
        EitherError<TransportError, E>,
    >
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: AsyncFnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: AsyncFnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        match self {
            Self::Opened => Ok(None),
            Self::Ready { .. } => Ok(Some(HandshakeReady { handshake: self })),
            Self::PendingInit {
                init,
                prefixed,
                tx,
                handle,
                write,
                ..
            } => {
                tx.init_syn(init);

                if let Some(bytes) = tx.flush(*prefixed) {
                    write(handle, bytes).await.map_err(EitherError::B)?;
                }

                if let Self::PendingInit {
                    state,
                    prefixed,
                    tx,
                    rx,
                    handle,
                    read,
                    write,
                    ..
                } = core::mem::replace(self, Self::Opened)
                {
                    *self = Self::PendingRecv {
                        state,
                        prefixed,
                        tx,
                        rx,
                        handle,
                        read,
                        write,
                    };
                } else {
                    unreachable!()
                }

                Ok(None)
            }
            Self::PendingRecv {
                state,
                prefixed,
                tx,
                rx,
                handle,
                read,
                write,
            } => {
                if let Some(description) = state.description() {
                    if let Self::PendingRecv { tx, rx, .. } = core::mem::replace(self, Self::Opened)
                    {
                        *self = Self::Ready {
                            description,
                            tx,
                            rx,
                        };

                        return Ok(Some(HandshakeReady { handshake: self }));
                    } else {
                        unreachable!()
                    }
                }

                rx.decode_with_async(async |bytes| read(handle, bytes).await, *prefixed)
                    .await?;

                let resp = rx
                    .flush_transport()
                    .map(|msg| state.poll(msg))
                    .filter_map(|response| response.0);
                for resp in resp {
                    tx.transport(resp);
                }

                if let Some(bytes) = tx.flush(*prefixed) {
                    write(handle, bytes).await.map_err(EitherError::B)?;
                }

                Ok(None)
            }
        }
    }

    /// Poll synchronously until negotiation succeeds or a protocol/I/O error occurs.
    ///
    /// No deadline or sleeping is added; callbacks control blocking. The caller
    /// must enforce a timeout where required.
    pub fn finish<E>(
        mut self,
    ) -> core::result::Result<Transport<Buff>, EitherError<TransportError, E>>
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: FnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: FnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        loop {
            if let Some(ready) = self.poll()? {
                break Ok(ready.open());
            }
        }
    }

    /// Await handshake steps until negotiation succeeds or a protocol/I/O error occurs.
    ///
    /// No timeout is installed. If this future is cancelled, discard the underlying
    /// connection before attempting another handshake.
    pub async fn finish_async<E>(
        mut self,
    ) -> core::result::Result<Transport<Buff>, EitherError<TransportError, E>>
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: AsyncFnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: AsyncFnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        loop {
            if let Some(ready) = self.poll_async().await? {
                break Ok(ready.open());
            }
        }
    }
}
