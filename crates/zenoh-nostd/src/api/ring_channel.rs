use core::cell::RefCell;

use heapless::Deque;

#[allow(dead_code)]
const MAX_RING_CAPACITY: usize = 32;

/// A ring (sliding) channel that drops the oldest item when full.
///
/// Unlike embassy's blocking/suspending channels, `RingChannel` never blocks —
/// `try_send` always succeeds (possibly dropping the oldest item) and `try_recv`
/// returns `None` when empty.
///
/// Interior mutability via `RefCell` allows `try_send` and `try_recv` to take
/// `&self`, matching the embassy channel's API. This type is `!Sync` and intended
/// for single-task embedded use.
pub struct RingChannel<T> {
    buffer: RefCell<Deque<T, MAX_RING_CAPACITY>>,
    capacity: usize,
}

#[allow(dead_code)]
impl<T> RingChannel<T> {
    /// Create a ring channel with the given capacity.
    ///
    /// `capacity` is capped at `MAX_RING_CAPACITY` (currently 32).
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: RefCell::new(Deque::new()),
            capacity: capacity.clamp(1, MAX_RING_CAPACITY),
        }
    }

    /// Attempt to send an item into the channel.
    ///
    /// If the buffer is full, the oldest item is dropped and the new item is
    /// appended. This never blocks.
    pub fn try_send(&self, item: T) -> Result<(), T> {
        let mut buf = self.buffer.borrow_mut();
        if buf.len() >= self.capacity {
            let _ = buf.pop_front();
        }
        buf.push_back(item)
    }

    /// Attempt to receive an item from the channel.
    pub fn try_recv(&self) -> Option<T> {
        self.buffer.borrow_mut().pop_front()
    }

    /// Number of items currently in the channel.
    pub fn len(&self) -> usize {
        self.buffer.borrow().len()
    }

    /// Returns `true` if the channel is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.borrow().is_empty()
    }

    /// Returns `true` if the channel is full (len >= capacity).
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }

    /// The maximum number of items this channel can hold.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// The underlying buffer's maximum fixed size.
    pub const fn max_capacity() -> usize {
        MAX_RING_CAPACITY
    }
}

#[cfg(test)]
#[cfg(feature = "std")]
mod tests {
    use super::*;

    #[test]
    fn test_ring_channel_basic_send_recv() {
        let ch = RingChannel::<i32>::new(4);
        assert!(ch.is_empty());
        assert_eq!(ch.capacity(), 4);

        ch.try_send(1).ok();
        assert_eq!(ch.len(), 1);
        ch.try_send(2).ok();
        ch.try_send(3).ok();
        ch.try_send(4).ok();
        assert!(ch.is_full());

        assert_eq!(ch.try_recv(), Some(1));
        assert_eq!(ch.try_recv(), Some(2));
        assert_eq!(ch.try_recv(), Some(3));
        assert_eq!(ch.try_recv(), Some(4));
        assert!(ch.is_empty());
        assert_eq!(ch.try_recv(), None);
    }

    #[test]
    fn test_ring_channel_drops_oldest_when_full() {
        let ch = RingChannel::<i32>::new(3);

        ch.try_send(1).ok();
        ch.try_send(2).ok();
        ch.try_send(3).ok();
        assert!(ch.is_full());

        ch.try_send(4).ok();
        assert_eq!(ch.len(), 3);

        assert_eq!(ch.try_recv(), Some(2));
        assert_eq!(ch.try_recv(), Some(3));
        assert_eq!(ch.try_recv(), Some(4));
        assert_eq!(ch.try_recv(), None);
    }

    #[test]
    fn test_ring_channel_multiple_drops() {
        let ch = RingChannel::<i32>::new(2);

        ch.try_send(1).ok();
        ch.try_send(2).ok();
        ch.try_send(3).ok();
        ch.try_send(4).ok();
        ch.try_send(5).ok();

        assert_eq!(ch.len(), 2);
        assert_eq!(ch.try_recv(), Some(4));
        assert_eq!(ch.try_recv(), Some(5));
        assert_eq!(ch.try_recv(), None);
    }

    #[test]
    fn test_ring_channel_capacity_1() {
        let ch = RingChannel::<i32>::new(1);

        ch.try_send(10).ok();
        assert_eq!(ch.try_recv(), Some(10));

        ch.try_send(20).ok();
        ch.try_send(30).ok();
        assert_eq!(ch.try_recv(), Some(30));
        assert_eq!(ch.try_recv(), None);
    }

    #[test]
    fn test_ring_channel_capacity_clamping() {
        let ch = RingChannel::<i32>::new(100);
        assert_eq!(ch.capacity(), MAX_RING_CAPACITY);

        let ch = RingChannel::<i32>::new(0);
        assert_eq!(ch.capacity(), 1);

        let ch = RingChannel::<i32>::new(MAX_RING_CAPACITY);
        assert_eq!(ch.capacity(), MAX_RING_CAPACITY);
    }
}
