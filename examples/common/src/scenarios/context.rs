use super::{KEY_CAPACITY, PAYLOAD_CAPACITY};
use core::{
    cell::Cell,
    fmt::{self, Write as _},
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel, signal::Signal};
use heapless::{String, Vec};
use zenoh_nostd::session::Sample;

pub(super) struct Event {
    pub id: u32,
    pub key: String<KEY_CAPACITY>,
    pub payload: Vec<u8, PAYLOAD_CAPACITY>,
}
/// Application-owned bounded callback state; contains no parser or console I/O.
pub struct Context {
    pub(super) events: Channel<NoopRawMutex, Event, 8>,
    pub(super) received: Cell<u64>,
    pub(super) received_bytes: Cell<u64>,
    pub(super) aggregate: Cell<bool>,
    dropped: Cell<u64>,
    pub(super) stop: Signal<NoopRawMutex, ()>,
    stopping: Cell<bool>,
}
impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
impl Context {
    pub const fn new() -> Self {
        Self {
            events: Channel::new(),
            received: Cell::new(0),
            received_bytes: Cell::new(0),
            aggregate: Cell::new(false),
            dropped: Cell::new(0),
            stop: Signal::new(),
            stopping: Cell::new(false),
        }
    }
    pub fn reset(&self) {
        self.events.clear();
        self.received.set(0);
        self.received_bytes.set(0);
        self.aggregate.set(false);
    }
    pub fn request_stop(&self) {
        self.stopping.set(true);
        self.stop.signal(());
    }
    pub(super) fn finished(&self) {
        self.stop.reset();
        self.stopping.set(false);
        self.reset();
    }
    pub fn dropped(&self) -> u64 {
        self.dropped.get()
    }
    pub(super) fn is_stopping(&self) -> bool {
        self.stopping.get()
    }
    pub(super) fn drop_event(&self) {
        self.dropped.set(self.dropped.get().saturating_add(1));
    }
    pub(super) fn reply(&self, error: bool, sample: &Sample<'_>) {
        if let (Ok(key), Ok(payload)) = (
            String::try_from(sample.keyexpr().as_str()),
            Vec::from_slice(sample.payload()),
        ) {
            if self
                .events
                .try_send(Event {
                    id: u32::from(error),
                    key,
                    payload,
                })
                .is_err()
            {
                self.drop_event();
            }
        } else {
            self.drop_event();
        }
    }
    pub(super) fn sample(&self, sample: &Sample<'_>) {
        self.received.set(self.received.get().saturating_add(1));
        self.received_bytes.set(
            self.received_bytes
                .get()
                .saturating_add(sample.payload().len() as u64),
        );
        if self.aggregate.get() {
            return;
        }
        if let (Ok(key), Ok(payload)) = (
            String::try_from(sample.keyexpr().as_str()),
            Vec::from_slice(sample.payload()),
        ) {
            if self
                .events
                .try_send(Event {
                    id: 0,
                    key,
                    payload,
                })
                .is_err()
            {
                self.drop_event();
            }
        } else {
            self.drop_event();
        }
    }
}
#[derive(Clone, Copy)]
pub enum Ready {
    Subscriber,
    Queryable,
    Broker,
}
impl Ready {
    pub const fn message(self) -> &'static str {
        match self {
            Self::Subscriber => "subscriber ready",
            Self::Queryable => "queryable ready",
            Self::Broker => "broker running",
        }
    }
}
/// Reporting is application policy and runs outside receive callbacks.
/// Callback data is first copied into bounded scenario event storage.
pub trait Reporter {
    fn emit(&self, args: fmt::Arguments<'_>);
    fn ready(&self, ready: Ready) -> impl core::future::Future<Output = ()> {
        async move {
            self.emit(format_args!("{}\r\n", ready.message()));
        }
    }
}
/// Formats bounded diagnostic records for an application-provided log function.
pub struct ReportFn(pub fn(&str));
impl Reporter for ReportFn {
    fn emit(&self, args: fmt::Arguments<'_>) {
        let mut line = String::<384>::new();
        if line.write_fmt(args).is_ok() {
            (self.0)(&line);
        } else {
            (self.0)("[scenario report exceeded display limit]\r\n");
        }
    }
}
