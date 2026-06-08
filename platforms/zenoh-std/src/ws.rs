use std::sync::{Arc, Mutex};

use tungstenite::{accept, Message, WebSocket};
use zenoh_nostd::platform::*;

pub struct StdWsLink {
    stream: Arc<Mutex<WebSocket<std::net::TcpStream>>>,
    mtu: u16,
}

impl StdWsLink {
    pub fn accept(tcp: std::net::TcpStream) -> Result<Self, LinkError> {
        let stream = accept(tcp).map_err(|_| LinkError::CouldNotListen)?;
        Ok(Self::new(stream, u16::MAX))
    }

    pub fn new(stream: WebSocket<std::net::TcpStream>, mtu: u16) -> Self {
        Self { stream: Arc::new(Mutex::new(stream)), mtu }
    }
}

pub struct StdWsLinkTx {
    stream: Arc<Mutex<WebSocket<std::net::TcpStream>>>,
    mtu: u16,
}

pub struct StdWsLinkRx {
    stream: Arc<Mutex<WebSocket<std::net::TcpStream>>>,
    mtu: u16,
}

impl ZLinkInfo for StdWsLink {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkInfo for StdWsLinkTx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkInfo for StdWsLinkRx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for StdWsLink {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        let mut stream = self.stream.lock().map_err(|_| LinkError::LinkTxFailed)?;
        stream.send(Message::Binary(buffer.to_vec())).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkTx for StdWsLinkTx {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        let mut stream = self.stream.lock().map_err(|_| LinkError::LinkTxFailed)?;
        stream.send(Message::Binary(buffer.to_vec())).map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for StdWsLink {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let mut stream = self.stream.lock().map_err(|_| LinkError::LinkRxFailed)?;
        match stream.read().map_err(|_| LinkError::LinkRxFailed)? {
            Message::Binary(data) => {
                let len = data.len().min(buffer.len());
                buffer[..len].copy_from_slice(&data[..len]);
                Ok(len)
            }
            _ => Err(LinkError::LinkRxFailed),
        }
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        self.read(buffer).map(|_| ())
    }
}

impl ZLinkRx for StdWsLinkRx {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let mut stream = self.stream.lock().map_err(|_| LinkError::LinkRxFailed)?;
        match stream.read().map_err(|_| LinkError::LinkRxFailed)? {
            Message::Binary(data) => {
                let len = data.len().min(buffer.len());
                buffer[..len].copy_from_slice(&data[..len]);
                Ok(len)
            }
            _ => Err(LinkError::LinkRxFailed),
        }
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        self.read(buffer).map(|_| ())
    }
}

impl ZLink for StdWsLink {
    type Tx<'a> = StdWsLinkTx;
    type Rx<'a> = StdWsLinkRx;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (
            StdWsLinkTx { stream: self.stream.clone(), mtu: self.mtu },
            StdWsLinkRx { stream: self.stream.clone(), mtu: self.mtu },
        )
    }
}