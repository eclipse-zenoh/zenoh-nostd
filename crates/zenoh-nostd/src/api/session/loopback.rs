//! Delivery between entities of the same session.
//!
//! A router never sends a message back to the session it came from, so a
//! put that should reach a subscriber of the same session, a get that should
//! reach one of its queryables, and the replies to that get, are delivered by
//! the session itself.
//!
//! They are not delivered on the spot. User callbacks run while the session
//! state is locked, so a callback that puts would then wait on a lock its own
//! caller holds. Instead, each local message is encoded into a queue that
//! [`Session::run`](crate::session::Session::run) dispatches like a message
//! from the network. Two buffers alternate, so messages that callbacks send
//! while one buffer is being dispatched go into the other.

use core::cell::{Cell, RefCell};

use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use zenoh_proto::{
    SessionError, TransportError, TransportLinkError, ZDecode, ZEncode,
    msgs::{NetworkBody, NetworkBodyRef},
};

/// Size of the length prefix in front of each queued message.
const PREFIX: usize = size_of::<u32>();

struct Frames<Buff> {
    buff: Buff,
    len: usize,
}

pub(crate) struct Loopback<Buff> {
    queues: [RefCell<Frames<Buff>>; 2],
    /// Index of the queue that new messages go into.
    filling: Cell<usize>,
    ready: Signal<NoopRawMutex, ()>,
}

impl<Buff> Loopback<Buff>
where
    Buff: AsMut<[u8]> + AsRef<[u8]>,
{
    pub(crate) fn new(first: Buff, second: Buff) -> Self {
        Self {
            queues: [
                RefCell::new(Frames {
                    buff: first,
                    len: 0,
                }),
                RefCell::new(Frames {
                    buff: second,
                    len: 0,
                }),
            ],
            filling: Cell::new(0),
            ready: Signal::new(),
        }
    }

    /// Whether the session was configured with room for local messages.
    pub(crate) fn enabled(&self) -> bool {
        self.queues
            .iter()
            .all(|queue| !queue.borrow().buff.as_ref().is_empty())
    }

    /// Queues `body` for local dispatch. Returns `false`, and queues nothing,
    /// when it doesn't fit.
    pub(crate) fn push(&self, body: NetworkBodyRef<'_>) -> bool {
        // `drain` never borrows the queue being filled, so this borrow
        // succeeds. If that ever broke, the message is dropped, not a panic.
        let Ok(mut frames) = self.queues[self.filling.get()].try_borrow_mut() else {
            return false;
        };
        let frames = &mut *frames;

        let start = frames.len;
        let buff = frames.buff.as_mut();
        let Some(space) = buff.get_mut(start + PREFIX..) else {
            return false;
        };

        let available = space.len();
        let mut writer = &mut *space;
        if body.z_encode(&mut writer).is_err() {
            return false;
        }
        let len = available - writer.len();
        let Ok(prefix) = u32::try_from(len) else {
            return false;
        };
        let end = start + PREFIX + len;

        buff[start..start + PREFIX].copy_from_slice(&prefix.to_le_bytes());
        frames.len = end;
        self.ready.signal(());

        true
    }

    /// Waits until a message has been queued.
    pub(crate) async fn ready(&self) {
        self.ready.wait().await
    }

    /// Passes every queued message to `dispatch`, in order. Messages queued
    /// meanwhile go into the other buffer and wait for the next call.
    ///
    /// Callers must not drain concurrently; the session drains only while
    /// holding its state lock.
    // The draining queue stays borrowed while `dispatch` runs callbacks. That
    // is sound: `push` only borrows the other queue, and drains don't overlap.
    #[allow(clippy::await_holding_refcell_ref)]
    pub(crate) async fn drain(
        &self,
        mut dispatch: impl AsyncFnMut(NetworkBody<'_>) -> core::result::Result<(), SessionError>,
    ) -> core::result::Result<(), SessionError> {
        let draining = self.filling.get();
        self.filling.set(1 - draining);

        let result = async {
            let frames = self.queues[draining].borrow();
            let mut bytes = &frames.buff.as_ref()[..frames.len];

            // The frames were written by `push`, so their prefixes are sound.
            while !bytes.is_empty() {
                let (prefix, rest) = bytes.split_at(PREFIX);
                let len = u32::from_le_bytes([prefix[0], prefix[1], prefix[2], prefix[3]]);
                let (mut frame, rest) = rest.split_at(len as usize);
                bytes = rest;

                let body = NetworkBody::z_decode(&mut frame)
                    .map_err(|e| TransportLinkError::from(TransportError::from(e)))?;
                dispatch(body).await?;
            }

            Ok(())
        }
        .await;

        self.queues[draining].borrow_mut().len = 0;
        result
    }
}
