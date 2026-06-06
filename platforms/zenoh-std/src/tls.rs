use std::net::SocketAddr;
use std::sync::Arc;

use async_native_tls::{TlsConnector, TlsStream};
use async_net::TcpStream;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use zenoh_nostd::platform::*;

pub struct StdTlsLink {
    pub(crate) stream: Arc<Mutex<TlsStream<TcpStream>>>,
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
            stream: Arc::new(Mutex::new(stream)),
            mtu: u16::MAX - 60,
        })
    }
}

pub struct StdTlsLinkTx {
    pub(crate) stream: Arc<Mutex<TlsStream<TcpStream>>>,
    pub(crate) mtu: u16,
}

pub struct StdTlsLinkRx {
    pub(crate) stream: Arc<Mutex<TlsStream<TcpStream>>>,
    pub(crate) mtu: u16,
}

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
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        true
    }
}

impl ZLinkInfo for StdTlsLinkRx {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        true
    }
}

impl ZLinkTx for StdTlsLink {
    async fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        let mut stream = self.stream.lock().await;
        stream
            .write_all(buffer)
            .await
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkTx for StdTlsLinkTx {
    async fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        let mut stream = self.stream.lock().await;
        stream
            .write_all(buffer)
            .await
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for StdTlsLink {
    async fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let mut stream = self.stream.lock().await;
        stream
            .read(buffer)
            .await
            .map_err(|_| LinkError::LinkRxFailed)
    }

    async fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        let mut stream = self.stream.lock().await;
        stream
            .read_exact(buffer)
            .await
            .map_err(|_| LinkError::LinkRxFailed)
    }
}

impl ZLinkRx for StdTlsLinkRx {
    async fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let mut stream = self.stream.lock().await;
        stream
            .read(buffer)
            .await
            .map_err(|_| LinkError::LinkRxFailed)
    }

    async fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        let mut stream = self.stream.lock().await;
        stream
            .read_exact(buffer)
            .await
            .map_err(|_| LinkError::LinkRxFailed)
    }
}

impl ZLink for StdTlsLink {
    type Tx<'a> = StdTlsLinkTx;
    type Rx<'a> = StdTlsLinkRx;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        let tx = StdTlsLinkTx {
            stream: self.stream.clone(),
            mtu: self.mtu,
        };
        let rx = StdTlsLinkRx {
            stream: self.stream.clone(),
            mtu: self.mtu,
        };
        (tx, rx)
    }
}