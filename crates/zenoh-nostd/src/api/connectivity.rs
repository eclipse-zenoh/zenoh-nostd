use embassy_sync::channel::DynamicSender;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportEvent {
    Connected,
    Disconnected,
}

pub struct ConnectivityState<'ch> {
    pub transport_senders: heapless::Vec<DynamicSender<'ch, TransportEvent>, 4>,
}

impl<'ch> ConnectivityState<'ch> {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self { transport_senders: heapless::Vec::new() }
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
            let _ = sender.try_send(event);
        }
    }
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;
    use embassy_sync::blocking_mutex::raw::NoopRawMutex;
    use embassy_sync::channel::Channel;

    fn make_channel() -> (DynamicSender<'static, TransportEvent>, embassy_sync::channel::DynamicReceiver<'static, TransportEvent>) {
        let ch: &'static _ = alloc::boxed::Box::leak(alloc::boxed::Box::new(Channel::<NoopRawMutex, TransportEvent, 1>::new()));
        (ch.dyn_sender(), ch.dyn_receiver())
    }

    #[test]
    fn connectivity_emit_receive() {
        let (sender, receiver) = make_channel();
        let mut state = ConnectivityState::new();
        let _ = state.transport_senders.push(sender);

        state.emit(TransportEvent::Connected);
        assert_eq!(receiver.try_receive(), Ok(TransportEvent::Connected));
    }

    #[test]
    fn connectivity_disconnected() {
        let (sender, receiver) = make_channel();
        let mut state = ConnectivityState::new();
        let _ = state.transport_senders.push(sender);

        state.emit(TransportEvent::Disconnected);
        assert_eq!(receiver.try_receive(), Ok(TransportEvent::Disconnected));
    }

    #[test]
    fn connectivity_multiple_listeners() {
        let (s1, r1) = make_channel();
        let (s2, r2) = make_channel();

        let mut state = ConnectivityState::new();
        let _ = state.transport_senders.push(s1);
        let _ = state.transport_senders.push(s2);

        state.emit(TransportEvent::Connected);
        assert_eq!(r1.try_receive(), Ok(TransportEvent::Connected));
        assert_eq!(r2.try_receive(), Ok(TransportEvent::Connected));
    }
}
