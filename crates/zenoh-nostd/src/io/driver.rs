use zenoh_proto::{
    EitherError, TransportLinkError,
    fields::ZenohIdProto,
    msgs::NetworkMessage,
};
use zenoh_sansio::{ZTransportRx, ZTransportTx};

use crate::{
    api::connectivity::{ConnectivityState, TransportEvent},
    io::link::{ZLinkInfo, ZLinkRx, ZLinkTx},
    io::transport::{
        TransportLink, TransportLinkRx, TransportLinkTx, ZTransportLinkRx, ZTransportLinkTx,
    },
    platform::ZLink,
};

pub struct Driver<'res, Link, Buff>
where
    Link: ZLink + 'res,
    Buff: Clone,
{
    zid: ZenohIdProto,
    tx: TransportLinkTx<'res, Link::Tx<'res>, Buff>,
    rx: TransportLinkRx<'res, Link::Rx<'res>, Buff>,
}

impl<'res, Link, Buff> Driver<'res, Link, Buff>
where
    Link: ZLink,
    Buff: Clone,
{
    pub fn new(transport: &'res mut TransportLink<Link, Buff>) -> Self {
        let zid = transport.transport().other_zid;
        let (tx, rx) = transport.split();
        Self { zid, tx, rx }
    }

    pub fn zid(&self) -> ZenohIdProto {
        self.zid
    }

    pub fn tx_mut(&mut self) -> &mut TransportLinkTx<'res, Link::Tx<'res>, Buff> {
        &mut self.tx
    }

    /// Drive IO: send pending frames, receive and dispatch messages, handle timeouts.
    /// Returns `true` if any work was done, `false` if idle.
    /// Call this in your main loop. Non-blocking.
    pub fn poll<State, E>(
        &mut self,
        state: &mut State,
        connectivity: &ConnectivityState<'_>,
        mut update: impl FnMut(
            ZenohIdProto,
            &mut State,
            NetworkMessage<'_>,
            &[u8],
        ) -> core::result::Result<(), E>,
    ) -> core::result::Result<bool, EitherError<TransportLinkError, E>>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]>,
    {
        let mut did_work = false;

        // 1. Sync transport state machines (lease, keepalive timers)
        self.rx.transport_mut().sync(None, core::time::Duration::ZERO);
        self.tx
            .transport_mut()
            .sync(Some(self.rx.transport()), core::time::Duration::ZERO);

        // 2. Check if transport is closed
        if self.rx.transport().closed() {
            connectivity.emit(TransportEvent::Disconnected);
            return Err(EitherError::A(TransportLinkError::TransportClosed));
        }

        // 3. Send keepalive if needed
        if self
            .tx
            .transport()
            .should_send_keepalive(core::time::Duration::ZERO)
        {
            let (link, transport) = self.tx.tx();
            transport.keepalive();
            let streamed = link.is_streamed();
            if let Some(bytes) = transport.flush(streamed) {
                link.write_all(bytes)
                    .map_err(TransportLinkError::from)?;
            }
            did_work = true;
        }

        // 4. Try to receive frames (non-blocking)
        {
            let (link, transport) = self.rx.rx();
            let streamed = link.is_streamed();

            match transport.decode_with(
                |bytes| {
                    if streamed {
                        link.read_exact(bytes).map(|_| bytes.len())
                    } else {
                        link.read(bytes)
                    }
                },
                streamed,
            ) {
                Ok(()) => {
                    for msg in transport.flush() {
                        update(self.zid, state, msg.0, msg.1).map_err(EitherError::B)?;
                        did_work = true;
                    }
                }
                Err(e) => {
                    connectivity.emit(TransportEvent::Disconnected);
                    return Err(EitherError::A(
                        e.flatten_map::<TransportLinkError>(),
                    ));
                }
            }
        }

        // 5. Check close condition
        if self.rx.transport().should_close(core::time::Duration::ZERO) {
            let (link, transport) = self.tx.tx();
            transport.close();
            let streamed = link.is_streamed();
            if let Some(bytes) = transport.flush(streamed) {
                link.write_all(bytes)
                    .map_err(TransportLinkError::from)?;
            }
            connectivity.emit(TransportEvent::Disconnected);
            return Err(EitherError::A(TransportLinkError::TransportClosed));
        }

        Ok(did_work)
    }
}