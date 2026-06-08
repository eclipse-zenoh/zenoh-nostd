use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use zenoh_nostd::platform::*;

pub struct StdTcpLink {
    inner: Arc<Mutex<TcpStream>>,
    mtu: u16,
}

impl StdTcpLink {
    pub fn new(stream: TcpStream, mtu: u16) -> Self {
        Self {
            inner: Arc::new(Mutex::new(stream)),
            mtu,
        }
    }
}

pub struct StdTcpLinkTx {
    inner: Arc<Mutex<TcpStream>>,
    mtu: u16,
}

pub struct StdTcpLinkRx {
    inner: Arc<Mutex<TcpStream>>,
    mtu: u16,
}

impl ZLinkInfo for StdTcpLink {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkInfo for StdTcpLinkTx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkInfo for StdTcpLinkRx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for StdTcpLink {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.inner
            .lock()
            .map_err(|_| LinkError::LinkTxFailed)?
            .write_all(buffer)
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkTx for StdTcpLinkTx {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.inner
            .lock()
            .map_err(|_| LinkError::LinkTxFailed)?
            .write_all(buffer)
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for StdTcpLink {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        self.inner
            .lock()
            .map_err(|_| LinkError::LinkRxFailed)?
            .read(buffer)
            .map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        self.inner
            .lock()
            .map_err(|_| LinkError::LinkRxFailed)?
            .read_exact(buffer)
            .map_err(|_| LinkError::LinkRxFailed)
    }
}

impl ZLinkRx for StdTcpLinkRx {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        self.inner
            .lock()
            .map_err(|_| LinkError::LinkRxFailed)?
            .read(buffer)
            .map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        self.inner
            .lock()
            .map_err(|_| LinkError::LinkRxFailed)?
            .read_exact(buffer)
            .map_err(|_| LinkError::LinkRxFailed)
    }
}

impl ZLink for StdTcpLink {
    type Tx<'a> = StdTcpLinkTx;
    type Rx<'a> = StdTcpLinkRx;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (
            StdTcpLinkTx { inner: self.inner.clone(), mtu: self.mtu },
            StdTcpLinkRx { inner: self.inner.clone(), mtu: self.mtu },
        )
    }
}