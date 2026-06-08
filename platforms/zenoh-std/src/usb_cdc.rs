use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use zenoh_nostd::platform::*;

pub struct StdUsbCdcLink {
    port: Arc<Mutex<Box<dyn serialport::SerialPort>>>,
    mtu: u16,
}

impl StdUsbCdcLink {
    pub fn new(path: &str) -> core::result::Result<Self, LinkError> {
        let port = serialport::new(path, 115200)
            .timeout(Duration::from_millis(10))
            .open()
            .map_err(|_| LinkError::CouldNotConnect)?;

        Ok(Self {
            port: Arc::new(Mutex::new(port)),
            mtu: u16::MAX,
        })
    }
}

pub struct UsbCdcTx {
    port: Arc<Mutex<Box<dyn serialport::SerialPort>>>,
    mtu: u16,
}

pub struct UsbCdcRx {
    port: Arc<Mutex<Box<dyn serialport::SerialPort>>>,
    mtu: u16,
}

impl ZLinkInfo for StdUsbCdcLink {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkInfo for UsbCdcTx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkInfo for UsbCdcRx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for StdUsbCdcLink {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        let mut port = self.port.lock().map_err(|_| LinkError::LinkTxFailed)?;
        port.write_all(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkTx for UsbCdcTx {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        let mut port = self.port.lock().map_err(|_| LinkError::LinkTxFailed)?;
        port.write_all(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for StdUsbCdcLink {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let mut port = self.port.lock().map_err(|_| LinkError::LinkRxFailed)?;
        port.read(buffer).map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        let mut port = self.port.lock().map_err(|_| LinkError::LinkRxFailed)?;
        port.read_exact(buffer).map_err(|_| LinkError::LinkRxFailed)
    }
}

impl ZLinkRx for UsbCdcRx {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let mut port = self.port.lock().map_err(|_| LinkError::LinkRxFailed)?;
        port.read(buffer).map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        let mut port = self.port.lock().map_err(|_| LinkError::LinkRxFailed)?;
        port.read_exact(buffer).map_err(|_| LinkError::LinkRxFailed)
    }
}

impl ZLink for StdUsbCdcLink {
    type Tx<'a> = UsbCdcTx;
    type Rx<'a> = UsbCdcRx;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (
            UsbCdcTx { port: self.port.clone(), mtu: self.mtu },
            UsbCdcRx { port: self.port.clone(), mtu: self.mtu },
        )
    }
}
