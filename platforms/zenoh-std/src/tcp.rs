use std::io::{Read, Write};
use std::net::TcpStream;

use zenoh_nostd::platform::*;

pub struct StdTcpLink {
    stream: TcpStream,
    mtu: u16,
}

impl StdTcpLink {
    pub fn new(stream: TcpStream, mtu: u16) -> Self {
        Self { stream, mtu }
    }
}

pub struct StdTcpLinkTx {
    stream: TcpStream,
    mtu: u16,
}

pub struct StdTcpLinkRx {
    stream: TcpStream,
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
        self.stream.write_all(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkTx for StdTcpLinkTx {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.stream.write_all(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for StdTcpLink {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        self.stream.read(buffer).map_err(|_| LinkError::LinkTxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        self.stream.read_exact(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for StdTcpLinkRx {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        self.stream.read(buffer).map_err(|_| LinkError::LinkTxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        self.stream.read_exact(buffer).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLink for StdTcpLink {
    type Tx<'a> = StdTcpLinkTx;
    type Rx<'a> = StdTcpLinkRx;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (
            StdTcpLinkTx { stream: self.stream.try_clone().expect("try_clone failed"), mtu: self.mtu },
            StdTcpLinkRx { stream: self.stream.try_clone().expect("try_clone failed"), mtu: self.mtu },
        )
    }
}