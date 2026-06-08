#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportEvent {
    Connected,
    Disconnected,
}

#[allow(clippy::type_complexity)]
pub struct ConnectivityState<'ch> {
    pub transport_senders: heapless::Vec<core::cell::Cell<Option<&'ch dyn Fn(TransportEvent)>>, 4>,
}

impl<'ch> ConnectivityState<'ch> {
    pub fn new() -> Self {
        Self {
            transport_senders: heapless::Vec::new(),
        }
    }

    #[allow(clippy::result_unit_err)]
    pub fn on_event(&mut self, f: &'ch dyn Fn(TransportEvent)) -> core::result::Result<(), ()> {
        self.transport_senders
            .push(core::cell::Cell::new(Some(f)))
            .map_err(|_| ())
    }
}

impl<'ch> Default for ConnectivityState<'ch> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ch> ConnectivityState<'ch> {
    pub fn emit(&self, event: TransportEvent) {
        for sender in &self.transport_senders {
            if let Some(f) = sender.get() {
                f(event);
            }
        }
    }
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;

    #[test]
    fn connectivity_emit_receive() {
        let called = core::cell::Cell::new(false);
        let f = |_event: TransportEvent| {
            called.set(true);
        };
        let mut state = ConnectivityState::new();
        let _ = state.on_event(&f);

        state.emit(TransportEvent::Connected);
        assert!(called.get());
    }

    #[test]
    fn connectivity_disconnected() {
        let event = core::cell::Cell::new(None);
        let f = |e: TransportEvent| {
            event.set(Some(e));
        };
        let mut state = ConnectivityState::new();
        let _ = state.on_event(&f);

        state.emit(TransportEvent::Disconnected);
        assert_eq!(event.get(), Some(TransportEvent::Disconnected));
    }

    #[test]
    fn connectivity_multiple_listeners() {
        let c1 = core::cell::Cell::new(false);
        let c2 = core::cell::Cell::new(false);
        let f1 = |_: TransportEvent| {
            c1.set(true);
        };
        let f2 = |_: TransportEvent| {
            c2.set(true);
        };
        let mut state = ConnectivityState::new();
        let _ = state.on_event(&f1);
        let _ = state.on_event(&f2);

        state.emit(TransportEvent::Connected);
        assert!(c1.get());
        assert!(c2.get());
    }
}
