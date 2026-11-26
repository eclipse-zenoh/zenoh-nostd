use zenoh_proto::{
    TransportLinkError,
    msgs::{NetworkMessage, NetworkMessageRef},
};
use zenoh_sansio::{ZTransportRx, ZTransportTx};

use super::{ZLinkInfo, ZLinkRx, ZLinkTx};

pub trait ZTransportLinkTx {
    fn tx(&mut self) -> (&mut impl ZLinkTx, &mut impl ZTransportTx);

    fn flush(&mut self) -> impl Future<Output = Result<(), TransportLinkError>> {
        let (link, _) = self.tx();
        async move { link.flush().await.map_err(Into::into) }
    }

    fn send<'a>(
        &mut self,
        msgs: impl Iterator<Item = NetworkMessage<'a>>,
    ) -> impl Future<Output = core::result::Result<(), zenoh_proto::TransportLinkError>> {
        let (link, transport) = self.tx();
        transport.encode(msgs);

        async move {
            if let Some(bytes) = transport.flush(link.is_streamed()) {
                link.write_all(bytes).await.map_err(|e| e.into())
            } else {
                Ok(())
            }
        }
    }

    #[allow(dead_code)]
    fn send_optimized_ref<'a>(
        &mut self,
        msgs: impl Iterator<Item = (NetworkMessageRef<'a>, &'a [u8])>,
    ) -> impl Future<Output = core::result::Result<(), zenoh_proto::TransportLinkError>> {
        let (link, transport) = self.tx();
        transport.encode_optimized_ref(msgs);

        async move {
            if let Some(bytes) = transport.flush(link.is_streamed()) {
                link.write_all(bytes).await.map_err(|e| e.into())
            } else {
                Ok(())
            }
        }
    }

    fn keepalive(
        &mut self,
    ) -> impl Future<Output = core::result::Result<(), zenoh_proto::TransportLinkError>> {
        let (link, transport) = self.tx();
        transport.keepalive();

        async move {
            if let Some(bytes) = transport.flush(link.is_streamed()) {
                link.write_all(bytes).await.map_err(|e| e.into())
            } else {
                Ok(())
            }
        }
    }
}

pub trait ZTransportLinkRx {
    fn rx(&mut self) -> (&mut impl ZLinkRx, &mut impl ZTransportRx);

    fn recv(
        &mut self,
    ) -> impl core::future::Future<
        Output = core::result::Result<
            impl Iterator<Item = (NetworkMessage<'_>, &'_ [u8])>,
            zenoh_proto::TransportLinkError,
        >,
    > {
        let (link, transport) = self.rx();
        let streamed = link.is_streamed();

        async move {
            transport
                .decode_with_async(
                    async |bytes| {
                        if streamed {
                            link.read_exact(bytes).await.map(|_| bytes.len())
                        } else {
                            link.read(bytes).await
                        }
                    },
                    streamed,
                )
                .await
                .map_err(|e| e.flatten_map::<TransportLinkError>())?;

            Ok(transport.flush())
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use core::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct BufferedWrite(Arc<AtomicUsize>);
    impl ZLinkInfo for BufferedWrite {
        fn mtu(&self) -> u16 {
            64
        }
        fn is_streamed(&self) -> bool {
            true
        }
    }
    impl ZLinkTx for BufferedWrite {
        async fn write_all(&mut self, _: &[u8]) -> Result<(), zenoh_proto::LinkError> {
            Ok(())
        }
        async fn flush(&mut self) -> Result<(), zenoh_proto::LinkError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }
    #[test]
    fn explicit_flush_drains_the_adapter_after_a_completed_frame() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut link = crate::io::transport::TransportLink::new(
            BufferedWrite(count.clone()),
            zenoh_sansio::Transport::builder([0u8; 64]).codec(),
        );
        let mut cx = Context::from_waker(Waker::noop());
        {
            let mut send = core::pin::pin!(link.keepalive());
            assert!(matches!(send.as_mut().poll(&mut cx), Poll::Ready(Ok(()))));
        }
        assert_eq!(count.load(Ordering::SeqCst), 0);
        let mut flush = core::pin::pin!(link.flush());
        assert!(matches!(flush.as_mut().poll(&mut cx), Poll::Ready(Ok(()))));
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}
