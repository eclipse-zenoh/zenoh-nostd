use std::net::SocketAddr;

use async_native_tls::{TlsConnector, TlsStream};
use async_net::TcpStream;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use zenoh_nostd::platform::*;

pub struct StdTlsLink {
    pub(crate) stream: TlsStream<TcpStream>,
    pub(crate) mtu: u16,
}

impl StdTlsLink {
    pub async fn connect(addr: SocketAddr, domain: &str) -> Result<Self, LinkError> {
        let tcp = TcpStream::connect(addr)
            .await
            .map_err(|_| LinkError::CouldNotConnect)?;
        tcp.set_nodelay(true)
            .map_err(|_| LinkError::CouldNotConnect)?;
        let connector = TlsConnector::default();
        let stream = connector
            .connect(domain, tcp)
            .await
            .map_err(|_| LinkError::CouldNotConnect)?;
        Ok(Self {
            stream,
            mtu: u16::MAX - 60,
        })
    }
}

// TLS streams aren't cloneable, so split isn't supported.
// StdTlsLink itself handles both Tx and Rx; Tx/Rx halves are stubs.
pub struct StdTlsLinkTx;
pub struct StdTlsLinkRx;

impl ZLinkInfo for StdTlsLink {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        true
    }
}

impl ZLinkInfo for StdTlsLinkTx {
    fn mtu(&self) -> u16 {
        u16::MAX
    }

    fn is_streamed(&self) -> bool {
        true
    }
}

impl ZLinkInfo for StdTlsLinkRx {
    fn mtu(&self) -> u16 {
        u16::MAX
    }

    fn is_streamed(&self) -> bool {
        true
    }
}

impl ZLinkTx for StdTlsLink {
    async fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.stream
            .write_all(buffer)
            .await
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkTx for StdTlsLinkTx {
    async fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for StdTlsLink {
    async fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        self.stream
            .read(buffer)
            .await
            .map_err(|_| LinkError::LinkRxFailed)
    }

    async fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        self.stream
            .read_exact(buffer)
            .await
            .map_err(|_| LinkError::LinkRxFailed)
    }
}

impl ZLinkRx for StdTlsLinkRx {
    async fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    async fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for StdTlsLink {
    type Tx<'a> = StdTlsLinkTx;
    type Rx<'a> = StdTlsLinkRx;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        unimplemented!("TLS link does not support split")
    }
}
