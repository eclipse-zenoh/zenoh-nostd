use core::fmt::Display;

use zenoh_proto::{EitherError, TransportError, msgs::InitSyn};

#[cfg(feature = "auth")]
use zenoh_proto::msgs::{Close, CloseBehaviour, TransportMessage};
#[cfg(feature = "auth")]
use crate::transport::AuthVerifierFn;

use crate::{
    Transport, TransportRx, TransportTx, ZTransportRx, ZTransportTx,
    establishment::{Description, State},
};

pub enum Handshake<Buff, T, Read, Write> {
    PendingInit {
        #[allow(private_interfaces)]
        state: State,

        init: InitSyn<'static>,
        prefixed: bool,

        /// Owned auth bytes kept alive for the `InitSyn`'s auth reference.
        /// Both are dropped together when transitioning to PendingRecv.
        #[cfg(feature = "auth")]
        auth_bytes: Option<heapless::Vec<u8, 256>>,

        tx: TransportTx<Buff>,
        rx: TransportRx<Buff>,

        handle: T,

        read: Read,
        write: Write,
    },

    PendingRecv {
        #[allow(private_interfaces)]
        state: State,

        prefixed: bool,

        /// Optional auth verifier callback for server-side auth checking.
        /// When set, the verifier is called with the auth payload from the
        /// remote InitSyn. If it returns false, a Close message with reason
        /// AuthRejected (0x02) is sent and the handshake terminates.
        #[cfg(feature = "auth")]
        auth_verifier: Option<AuthVerifierFn>,

        tx: TransportTx<Buff>,
        rx: TransportRx<Buff>,

        handle: T,

        read: Read,
        write: Write,
    },
    Ready {
        #[allow(private_interfaces)]
        description: Description,

        tx: TransportTx<Buff>,
        rx: TransportRx<Buff>,
    },
    Opened,
}

pub struct HandshakeReady<'a, Buff, T, Read, Write> {
    handshake: &'a mut Handshake<Buff, T, Read, Write>,
}

impl<'a, Buff, T, Read, Write> HandshakeReady<'a, Buff, T, Read, Write> {
    pub fn open(self) -> core::result::Result<Transport<Buff>, TransportError> {
        if let Handshake::Ready {
            description,
            tx,
            rx,
        } = core::mem::replace(self.handshake, Handshake::Opened)
        {
            Ok(Transport::new(description, tx.into_inner(), rx.into_inner()))
        } else {
            Err(TransportError::InvalidState)
        }
    }
}

impl<Buff, T, Read, Write> Handshake<Buff, T, Read, Write> {
    pub fn prefixed(mut self) -> Self {
        if let Self::PendingRecv { prefixed, .. } | Self::PendingInit { prefixed, .. } = &mut self {
            *prefixed = true;
        }

        self
    }

    pub fn with_prefixed(mut self, p: bool) -> Self {
        if let Self::PendingRecv { prefixed, .. } | Self::PendingInit { prefixed, .. } = &mut self {
            *prefixed = p;
        }

        self
    }

    #[allow(clippy::type_complexity)]
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
        // Extract auth_verifier pointer before match to avoid borrow conflicts
        #[cfg(feature = "auth")]
        let auth_verifier_ptr: *const Option<AuthVerifierFn> = match self {
            &mut Self::PendingRecv { ref auth_verifier, .. } => auth_verifier,
            _ => core::ptr::null(),
        };

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
                        #[cfg(feature = "auth")]
                        auth_verifier: None,
                        tx,
                        rx,
                        handle,
                        read,
                        write,
                    };
                } else {
                    return Err(EitherError::A(TransportError::InvalidState));
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
                ..
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
                        return Err(EitherError::A(TransportError::InvalidState));
                    }
                }

                rx.decode_with(|bytes| read(handle, bytes), *prefixed)?;

                let resp = rx
                    .flush_transport()
                    .map(|(msg, buff)| {
                        #[cfg(feature = "auth")]
                        {
                            let verifier: Option<AuthVerifierFn> = unsafe {
                                auth_verifier_ptr.as_ref().copied().flatten()
                            };
                            if let TransportMessage::InitSyn(ref syn) = msg {
                                if let Some(verifier) = verifier {
                                    if let Some(ref auth) = syn.auth {
                                        if !verifier(auth.payload) {
                                            zenoh_proto::debug!(
                                                "Auth rejected for peer {:?}",
                                                syn.identifier.zid
                                            );
                                            return (
                                                Some(TransportMessage::Close(Close {
                                                    reason: 0x02,
                                                    behaviour: CloseBehaviour::Link,
                                                })),
                                                None,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        state.poll((msg, buff))
                    })
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
        // Extract auth_verifier pointer before match to avoid borrow conflicts
        #[cfg(feature = "auth")]
        let auth_verifier_ptr: *const Option<AuthVerifierFn> = match self {
            &mut Self::PendingRecv { ref auth_verifier, .. } => auth_verifier,
            _ => core::ptr::null(),
        };

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
                        #[cfg(feature = "auth")]
                        auth_verifier: None,
                        tx,
                        rx,
                        handle,
                        read,
                        write,
                    };
                } else {
                    return Err(EitherError::A(TransportError::InvalidState));
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
                ..
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
                        return Err(EitherError::A(TransportError::InvalidState));
                    }
                }

                rx.decode_with_async(async |bytes| read(handle, bytes).await, *prefixed)
                    .await?;

                let resp = rx
                    .flush_transport()
                    .map(|(msg, buff)| {
                        #[cfg(feature = "auth")]
                        {
                            let verifier: Option<AuthVerifierFn> = unsafe {
                                auth_verifier_ptr.as_ref().copied().flatten()
                            };
                            if let TransportMessage::InitSyn(ref syn) = msg {
                                if let Some(verifier) = verifier {
                                    if let Some(ref auth) = syn.auth {
                                        if !verifier(auth.payload) {
                                            zenoh_proto::debug!(
                                                "Auth rejected for peer {:?}",
                                                syn.identifier.zid
                                            );
                                            return (
                                                Some(TransportMessage::Close(Close {
                                                    reason: 0x02,
                                                    behaviour: CloseBehaviour::Link,
                                                })),
                                                None,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        state.poll((msg, buff))
                    })
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
                break Ok(ready.open().map_err(EitherError::A)?);
            }
        }
    }

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
                break Ok(ready.open().map_err(EitherError::A)?);
            }
        }
    }
}
