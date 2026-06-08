use std::{marker::PhantomData, net::TcpStream};

use tungstenite::{
    Message, WebSocket,
    stream::MaybeTlsStream,
};
use zenoh_nostd::platform::*;

pub struct WasmWsLink {
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
    mtu: u16,
}

impl WasmWsLink {
    pub fn new(socket: WebSocket<MaybeTlsStream<TcpStream>>) -> Self {
        Self {
            socket,
            mtu: u16::MAX,
        }
    }
}

pub struct WasmWsLinkTx<'a> {
    socket: *mut WebSocket<MaybeTlsStream<TcpStream>>,
    mtu: u16,
    _marker: PhantomData<&'a ()>,
}

pub struct WasmWsLinkRx<'a> {
    socket: *mut WebSocket<MaybeTlsStream<TcpStream>>,
    mtu: u16,
    _marker: PhantomData<&'a ()>,
}

impl ZLinkInfo for WasmWsLink {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        false
    }
}

impl ZLinkInfo for WasmWsLinkTx<'_> {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        false
    }
}

impl ZLinkInfo for WasmWsLinkRx<'_> {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        false
    }
}

impl ZLinkTx for WasmWsLink {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.socket
            .send(Message::Binary(buffer.to_vec().into()))
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkTx for WasmWsLinkTx<'_> {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        let socket = unsafe { &mut *self.socket };
        socket
            .send(Message::Binary(buffer.to_vec().into()))
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for WasmWsLink {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        match self.socket.read().map_err(|_| LinkError::LinkRxFailed)? {
            Message::Binary(data) => {
                let len = data.len().min(buffer.len());
                buffer[..len].copy_from_slice(&data[..len]);
                Ok(len)
            }
            Message::Text(data) => {
                let bytes = data.as_bytes();
                let len = bytes.len().min(buffer.len());
                buffer[..len].copy_from_slice(&bytes[..len]);
                Ok(len)
            }
            Message::Ping(_) | Message::Pong(_) => Ok(0),
            Message::Close(_) | Message::Frame(_) => Err(LinkError::LinkRxFailed),
        }
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        self.read(buffer).map(|_| ())
    }
}

impl ZLinkRx for WasmWsLinkRx<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let socket = unsafe { &mut *self.socket };
        match socket.read().map_err(|_| LinkError::LinkRxFailed)? {
            Message::Binary(data) => {
                let len = data.len().min(buffer.len());
                buffer[..len].copy_from_slice(&data[..len]);
                Ok(len)
            }
            Message::Text(data) => {
                let bytes = data.as_bytes();
                let len = bytes.len().min(buffer.len());
                buffer[..len].copy_from_slice(&bytes[..len]);
                Ok(len)
            }
            Message::Ping(_) | Message::Pong(_) => Ok(0),
            Message::Close(_) | Message::Frame(_) => Err(LinkError::LinkRxFailed),
        }
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        self.read(buffer).map(|_| ())
    }
}

impl ZLink for WasmWsLink {
    type Tx<'link>
        = WasmWsLinkTx<'link>
    where
        Self: 'link;

    type Rx<'link>
        = WasmWsLinkRx<'link>
    where
        Self: 'link;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        let ptr: *mut WebSocket<MaybeTlsStream<TcpStream>> = &mut self.socket;
        let tx = WasmWsLinkTx {
            socket: ptr,
            mtu: self.mtu,
            _marker: PhantomData,
        };
        let rx = WasmWsLinkRx {
            socket: ptr,
            mtu: self.mtu,
            _marker: PhantomData,
        };
        (tx, rx)
    }
}