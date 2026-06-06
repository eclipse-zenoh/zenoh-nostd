use std::io::{Read, Write};
use std::time::Duration;

use zenoh_nostd::platform::*;

pub struct StdSerialLink {
    port: Box<dyn serialport::SerialPort>,
    mtu: u16,
}

impl StdSerialLink {
    pub fn new(path: &str, baud_rate: u32) -> core::result::Result<Self, LinkError> {
        let port = serialport::new(path, baud_rate)
            .timeout(Duration::from_millis(10))
            .open()
            .map_err(|_| LinkError::CouldNotConnect)?;

        Ok(Self { port, mtu: u16::MAX })
    }
}

impl ZLinkInfo for StdSerialLink {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        false
    }
}

impl ZLinkTx for StdSerialLink {
    async fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.port
            .write_all(buffer)
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for StdSerialLink {
    async fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        self.port
            .read(buffer)
            .map_err(|_| LinkError::LinkRxFailed)
    }

    async fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for StdSerialLink {
    type Tx<'a> = SerialTx;
    type Rx<'a> = SerialRx;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (SerialTx, SerialRx)
    }
}

pub struct SerialTx;
pub struct SerialRx;

impl ZLinkInfo for SerialTx {
    fn mtu(&self) -> u16 { u16::MAX }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkInfo for SerialRx {
    fn mtu(&self) -> u16 { u16::MAX }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for SerialTx {
    async fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for SerialRx {
    async fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    async fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}
