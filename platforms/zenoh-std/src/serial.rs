use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use zenoh_nostd::platform::*;

pub struct StdSerialLink {
    port: Arc<Mutex<Box<dyn serialport::SerialPort>>>,
    mtu: u16,
}

impl StdSerialLink {
    pub fn new(path: &str, baud_rate: u32) -> core::result::Result<Self, LinkError> {
        let port = serialport::new(path, baud_rate)
            .timeout(Duration::from_millis(10))
            .open()
            .map_err(|_| LinkError::CouldNotConnect)?;

        Ok(Self {
            port: Arc::new(Mutex::new(port)),
            mtu: u16::MAX,
        })
    }
}

pub struct SerialTx {
    port: Arc<Mutex<Box<dyn serialport::SerialPort>>>,
    mtu: u16,
}

pub struct SerialRx {
    port: Arc<Mutex<Box<dyn serialport::SerialPort>>>,
    mtu: u16,
}

impl ZLinkInfo for StdSerialLink {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkInfo for SerialTx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkInfo for SerialRx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for StdSerialLink {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        let mut port = self.port.lock().map_err(|_| LinkError::LinkTxFailed)?;
        port.write_all(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkTx for SerialTx {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        let mut port = self.port.lock().map_err(|_| LinkError::LinkTxFailed)?;
        port.write_all(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for StdSerialLink {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let mut port = self.port.lock().map_err(|_| LinkError::LinkRxFailed)?;
        port.read(buffer).map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        let mut port = self.port.lock().map_err(|_| LinkError::LinkRxFailed)?;
        port.read_exact(buffer).map_err(|_| LinkError::LinkRxFailed)
    }
}

impl ZLinkRx for SerialRx {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let mut port = self.port.lock().map_err(|_| LinkError::LinkRxFailed)?;
        port.read(buffer).map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        let mut port = self.port.lock().map_err(|_| LinkError::LinkRxFailed)?;
        port.read_exact(buffer).map_err(|_| LinkError::LinkRxFailed)
    }
}

impl ZLink for StdSerialLink {
    type Tx<'a> = SerialTx;
    type Rx<'a> = SerialRx;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (
            SerialTx { port: self.port.clone(), mtu: self.mtu },
            SerialRx { port: self.port.clone(), mtu: self.mtu },
        )
    }
}