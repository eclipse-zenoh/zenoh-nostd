use std::sync::{Arc, Mutex};

use zenoh_nostd::platform::*;

pub struct BtStream {
    inner: Arc<Mutex<()>>,
    mtu: u16,
}

impl BtStream {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(())),
            mtu: u16::MAX,
        }
    }
}

#[allow(dead_code)]
pub struct BtStreamTx {
    inner: Arc<Mutex<()>>,
    mtu: u16,
}

#[allow(dead_code)]
pub struct BtStreamRx {
    inner: Arc<Mutex<()>>,
    mtu: u16,
}

impl ZLinkInfo for BtStream {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkInfo for BtStreamTx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkInfo for BtStreamRx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for BtStream {
    fn write_all(&mut self, _buffer: &[u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkTx for BtStreamTx {
    fn write_all(&mut self, _buffer: &[u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for BtStream {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }
    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLinkRx for BtStreamRx {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }
    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for BtStream {
    type Tx<'a> = BtStreamTx;
    type Rx<'a> = BtStreamRx;
    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (
            BtStreamTx { inner: self.inner.clone(), mtu: self.mtu },
            BtStreamRx { inner: self.inner.clone(), mtu: self.mtu },
        )
    }
}