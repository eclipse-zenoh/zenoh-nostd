use zenoh_proto::{
    TransportLinkError,
    msgs::{NetworkMessage, NetworkMessageRef},
};
use zenoh_sansio::{ZTransportRx, ZTransportTx};

use super::{ZLinkInfo, ZLinkRx, ZLinkTx};

pub trait ZTransportLinkTx {
    fn tx(&mut self) -> (&mut impl ZLinkTx, &mut impl ZTransportTx);

    fn send<'a>(
        &mut self,
        msgs: impl Iterator<Item = NetworkMessage<'a>>,
    ) -> core::result::Result<(), zenoh_proto::TransportLinkError> {
        let (link, transport) = self.tx();
        transport.encode(msgs);

        if transport.is_batching() {
            Ok(())
        } else if let Some(bytes) = transport.flush(link.is_streamed()) {
            link.write_all(bytes).map_err(|e| e.into())
        } else {
            Ok(())
        }
    }

    fn batch_flush(&mut self) -> core::result::Result<(), zenoh_proto::TransportLinkError> {
        let (link, transport) = self.tx();

        if let Some(bytes) = transport.flush(link.is_streamed()) {
            link.write_all(bytes).map_err(|e| e.into())
        } else {
            Ok(())
        }
    }

    #[allow(dead_code)]
    fn send_optimized_ref<'a>(
        &mut self,
        msgs: impl Iterator<Item = (NetworkMessageRef<'a>, &'a [u8])>,
    ) -> core::result::Result<(), zenoh_proto::TransportLinkError> {
        let (link, transport) = self.tx();
        transport.encode_optimized_ref(msgs);

        if let Some(bytes) = transport.flush(link.is_streamed()) {
            link.write_all(bytes).map_err(|e| e.into())
        } else {
            Ok(())
        }
    }

    #[allow(dead_code)]
    fn keepalive(&mut self) -> core::result::Result<(), zenoh_proto::TransportLinkError> {
        let (link, transport) = self.tx();
        transport.keepalive();

        if let Some(bytes) = transport.flush(link.is_streamed()) {
            link.write_all(bytes).map_err(|e| e.into())
        } else {
            Ok(())
        }
    }

    /// Send a Close message and flush it to the transport link.
    /// This gracefully terminates the transport session.
    #[allow(dead_code)]
    fn close(&mut self) -> core::result::Result<(), zenoh_proto::TransportLinkError> {
        let (link, transport) = self.tx();
        transport.close();

        if let Some(bytes) = transport.flush(link.is_streamed()) {
            link.write_all(bytes).map_err(|e| e.into())
        } else {
            Ok(())
        }
    }
}

pub trait ZTransportLinkRx {
    fn rx(&mut self) -> (&mut impl ZLinkRx, &mut impl ZTransportRx);

    #[allow(dead_code)]
    fn recv(
        &mut self,
    ) -> core::result::Result<
        impl Iterator<Item = (NetworkMessage<'_>, &'_ [u8])>,
        zenoh_proto::TransportLinkError,
    > {
        let (link, transport) = self.rx();
        let streamed = link.is_streamed();

        transport
            .decode_with(
                |bytes| {
                    if streamed {
                        link.read_exact(bytes).map(|_| bytes.len())
                    } else {
                        link.read(bytes)
                    }
                },
                streamed,
            )
            .map_err(|e| e.flatten_map::<TransportLinkError>())?;

        Ok(transport.flush())
    }
}
