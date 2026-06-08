use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

use native_tls::{Identity, TlsConnector, TlsStream};
use zenoh_nostd::platform::*;

pub struct StdTlsLink {
    stream: Arc<Mutex<TlsStream<TcpStream>>>,
    mtu: u16,
}

impl StdTlsLink {
    pub fn connect(addr: SocketAddr, domain: &str) -> Result<Self, LinkError> {
        let tcp = TcpStream::connect(addr)
            .map_err(|_| LinkError::CouldNotConnect)?;
        tcp.set_nodelay(true)
            .map_err(|_| LinkError::CouldNotConnect)?;
        let connector = TlsConnector::builder()
            .build()
            .map_err(|_| LinkError::CouldNotConnect)?;
        let stream = connector
            .connect(domain, tcp)
            .map_err(|_| LinkError::CouldNotConnect)?;
        Ok(Self {
            stream: Arc::new(Mutex::new(stream)),
            mtu: u16::MAX - 60,
        })
    }

    pub fn connect_with_cert(
        addr: SocketAddr,
        domain: &str,
        pkcs12_der: &[u8],
        password: &str,
    ) -> Result<Self, LinkError> {
        let tcp = TcpStream::connect(addr)
            .map_err(|_| LinkError::CouldNotConnect)?;
        tcp.set_nodelay(true)
            .map_err(|_| LinkError::CouldNotConnect)?;

        let identity =
            Identity::from_pkcs12(pkcs12_der, password).map_err(|_| LinkError::CouldNotConnect)?;

        let connector = TlsConnector::builder()
            .identity(identity)
            .build()
            .map_err(|_| LinkError::CouldNotConnect)?;

        let stream = connector
            .connect(domain, tcp)
            .map_err(|_| LinkError::CouldNotConnect)?;

        Ok(Self {
            stream: Arc::new(Mutex::new(stream)),
            mtu: u16::MAX - 60,
        })
    }
}

pub struct StdTlsLinkTx {
    stream: Arc<Mutex<TlsStream<TcpStream>>>,
    mtu: u16,
}

pub struct StdTlsLinkRx {
    stream: Arc<Mutex<TlsStream<TcpStream>>>,
    mtu: u16,
}

impl ZLinkInfo for StdTlsLink {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkInfo for StdTlsLinkTx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkInfo for StdTlsLinkRx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for StdTlsLink {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        let mut stream = self.stream.lock().map_err(|_| LinkError::LinkTxFailed)?;
        stream.write_all(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkTx for StdTlsLinkTx {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        let mut stream = self.stream.lock().map_err(|_| LinkError::LinkTxFailed)?;
        stream.write_all(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for StdTlsLink {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let mut stream = self.stream.lock().map_err(|_| LinkError::LinkRxFailed)?;
        stream.read(buffer).map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        let mut stream = self.stream.lock().map_err(|_| LinkError::LinkRxFailed)?;
        stream.read_exact(buffer).map_err(|_| LinkError::LinkRxFailed)
    }
}

impl ZLinkRx for StdTlsLinkRx {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let mut stream = self.stream.lock().map_err(|_| LinkError::LinkRxFailed)?;
        stream.read(buffer).map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        let mut stream = self.stream.lock().map_err(|_| LinkError::LinkRxFailed)?;
        stream.read_exact(buffer).map_err(|_| LinkError::LinkRxFailed)
    }
}

impl ZLink for StdTlsLink {
    type Tx<'a> = StdTlsLinkTx;
    type Rx<'a> = StdTlsLinkRx;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (
            StdTlsLinkTx { stream: self.stream.clone(), mtu: self.mtu },
            StdTlsLinkRx { stream: self.stream.clone(), mtu: self.mtu },
        )
    }
}