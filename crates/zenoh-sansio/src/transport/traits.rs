use core::fmt::Display;

use zenoh_proto::{
    EitherError, TransportError,
    msgs::{
        InitAck, InitSyn, NetworkMessage, NetworkMessageRef, OpenAck, OpenSyn, TransportMessage,
        TransportMessageRef,
    },
};

/// Feed transport batches into codec storage and borrow decoded network messages.
///
/// Drain [`Self::flush`] before further decoding; the iterator borrows the
/// receive buffer. Methods do not operate sockets or schedule lease timers.
pub trait ZTransportRx {
    /// Copy one complete length-prefixed batch from `read` into receive storage.
    ///
    /// The two-byte little-endian prefix and the indicated body must both be present.
    /// This does not accumulate arbitrarily fragmented stream reads across calls.
    fn decode_prefixed(&mut self, read: &[u8]) -> core::result::Result<(), TransportError>;
    /// Append raw batch bytes into receive storage, returning `TransportRxFull` when they do not fit.
    ///
    /// For datagrams pass exactly one complete datagram, then drain it before the next.
    fn decode_raw(&mut self, read: &[u8]) -> core::result::Result<(), TransportError>;

    /// Select prefixed stream or raw message decoding using `prefixed`.
    fn decode(&mut self, read: &[u8], prefixed: bool) -> core::result::Result<(), TransportError> {
        if prefixed {
            self.decode_prefixed(read)
        } else {
            self.decode_raw(read)
        }
    }

    /// Read a prefixed batch with a synchronous callback.
    ///
    /// The callback must fill the two-byte prefix, then the requested body exactly.
    /// Short nonempty reads return `InvalidAttribute`; protocol and callback errors
    /// are preserved as separate `EitherError` branches.
    fn decode_prefixed_with<E>(
        &mut self,
        read: impl FnMut(&mut [u8]) -> core::result::Result<usize, E>,
    ) -> core::result::Result<(), EitherError<TransportError, E>>
    where
        E: Display;

    /// Read raw bytes through one synchronous callback invocation.
    ///
    /// The callback must return a length within the supplied buffer, preserving one
    /// complete message for datagram links. Returns protocol or callback errors.
    fn decode_raw_with<E>(
        &mut self,
        read: impl FnMut(&mut [u8]) -> core::result::Result<usize, E>,
    ) -> core::result::Result<(), EitherError<TransportError, E>>
    where
        E: Display;

    /// Select synchronous callback-based stream/raw decoding using `prefixed`.
    fn decode_with<E>(
        &mut self,
        read: impl FnMut(&mut [u8]) -> core::result::Result<usize, E>,
        prefixed: bool,
    ) -> core::result::Result<(), EitherError<TransportError, E>>
    where
        E: Display,
    {
        if prefixed {
            self.decode_prefixed_with(read)
        } else {
            self.decode_raw_with(read)
        }
    }

    /// Await a prefixed batch using an asynchronous exact-read callback.
    ///
    /// The prefix/body length rules match [`Self::decode_prefixed_with`]. Interrupted
    /// stream reads are not retained for retry; discard the connection on cancellation.
    fn decode_prefixed_with_async<E>(
        &mut self,
        read: impl AsyncFnMut(&mut [u8]) -> core::result::Result<usize, E>,
    ) -> impl Future<Output = core::result::Result<(), EitherError<TransportError, E>>>
    where
        E: Display;

    /// Await one raw-message read callback; preserve the entire message boundary.
    ///
    /// Return protocol or callback errors in separate `EitherError` branches.
    fn decode_raw_with_async<E>(
        &mut self,
        read: impl AsyncFnMut(&mut [u8]) -> core::result::Result<usize, E>,
    ) -> impl Future<Output = core::result::Result<(), EitherError<TransportError, E>>>
    where
        E: Display;

    /// Select asynchronous callback-based stream/raw decoding using `prefixed`.
    fn decode_with_async<E>(
        &mut self,
        read: impl AsyncFnMut(&mut [u8]) -> core::result::Result<usize, E>,
        prefixed: bool,
    ) -> impl Future<Output = core::result::Result<(), EitherError<TransportError, E>>>
    where
        E: Display,
    {
        async move {
            if prefixed {
                self.decode_prefixed_with_async(read).await
            } else {
                self.decode_raw_with_async(read).await
            }
        }
    }

    /// Borrow an iterator of decoded network messages and their original encoded bytes.
    ///
    /// Consume it before refilling the buffer. The iterator discards transport-only
    /// messages; malformed or invalid messages are logged and stop decoding the batch.
    /// Dropping it early discards unconsumed batch bytes.
    fn flush(&mut self) -> impl Iterator<Item = (NetworkMessage<'_>, &'_ [u8])>;
    /// Discard buffered receive bytes without resetting negotiated transport state.
    fn clear(&mut self);
}

/// Queue transport/network messages and borrow their encoded batches.
///
/// Encoding is bounded by the negotiated batch and buffer sizes. The current
/// implementation consumes and silently skips messages that cannot be encoded;
/// it does not return per-message errors or fragment oversized messages. Size
/// messages before enqueueing, and do not treat a flush as delivery confirmation.
pub trait ZTransportTx {
    /// Queue a keepalive control message; flush and write the resulting bytes separately.
    fn keepalive(&mut self);
    /// Queue an initialization proposal.
    fn init_syn(&mut self, syn: &InitSyn);
    /// Queue an initialization acknowledgment.
    fn init_ack(&mut self, ack: &InitAck);
    /// Queue an opening proposal.
    fn open_syn(&mut self, syn: &OpenSyn);
    /// Queue an opening acknowledgment.
    fn open_ack(&mut self, ack: &OpenAck);
    /// Queue a default close notification; this does not close a socket or commit codec closure.
    fn close(&mut self);

    /// Encode a transport control message into the current batch.
    fn transport(&mut self, msg: TransportMessage);
    /// Encode a borrowed transport control message without cloning it.
    fn transport_ref(&mut self, msg: TransportMessageRef);

    /// Consume network messages and encode those that fit; oversized/unencodable messages are skipped.
    fn encode<'a>(&mut self, msgs: impl Iterator<Item = NetworkMessage<'a>>);
    /// Encode borrowed network messages that fit, without cloning payload data.
    fn encode_ref<'a>(&mut self, msgs: impl Iterator<Item = NetworkMessageRef<'a>>);
    /// Forward messages using their supplied already-encoded network body bytes.
    ///
    /// Each byte slice must encode exactly the accompanying network body. Frame
    /// reliability/QoS are taken from the message; mismatched bytes are not validated.
    fn encode_optimized<'a>(&mut self, msgs: impl Iterator<Item = (NetworkMessage<'a>, &'a [u8])>);
    /// Forward borrowed messages with matching pre-encoded network body bytes.
    ///
    /// The supplied bytes must match the message metadata; no consistency check is made.
    fn encode_optimized_ref<'a>(
        &mut self,
        msgs: impl Iterator<Item = (NetworkMessageRef<'a>, &'a [u8])>,
    );

    /// Borrow the encoded batch with a two-byte little-endian length prefix, resetting the write cursor.
    ///
    /// Write the entire returned slice before borrowing the codec mutably again.
    /// An empty batch can still return the zero-length prefix; this performs no I/O.
    fn flush_prefixed(&mut self) -> Option<&'_ [u8]>;
    /// Borrow encoded batch bytes excluding the stream prefix, resetting the write cursor.
    ///
    /// An empty batch can yield `Some(&[])`. Send the entire slice as one message;
    /// this performs no I/O. Buffer/batch capacity must include the two reserved bytes.
    fn flush_raw(&mut self) -> Option<&'_ [u8]>;

    /// Borrow a stream-prefixed or raw encoded batch according to `prefixed`.
    fn flush(&mut self, prefixed: bool) -> Option<&'_ [u8]> {
        if prefixed {
            self.flush_prefixed()
        } else {
            self.flush_raw()
        }
    }
    /// Discard pending output and frame grouping, retaining sequence/lease bookkeeping.
    fn clear(&mut self);
}
