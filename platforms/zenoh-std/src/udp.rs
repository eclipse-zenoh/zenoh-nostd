use std::net::UdpSocket;

use zenoh_nostd::platform::*;

pub struct StdUdpLink {
    socket: UdpSocket,
    mtu: u16,
}

impl StdUdpLink {
    pub fn new(socket: UdpSocket, mtu: u16) -> Self {
        Self { socket, mtu }
    }
}

pub struct StdUdpLinkTx {
    socket: UdpSocket,
    mtu: u16,
}

pub struct StdUdpLinkRx {
    socket: UdpSocket,
    mtu: u16,
}

impl ZLinkInfo for StdUdpLink {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkInfo for StdUdpLinkTx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkInfo for StdUdpLinkRx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for StdUdpLink {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.socket.send(buffer).map_err(|_| LinkError::LinkTxFailed).map(|_| ())
    }
}

impl ZLinkTx for StdUdpLinkTx {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.socket.send(buffer).map_err(|_| LinkError::LinkTxFailed).map(|_| ())
    }
}

impl ZLinkRx for StdUdpLink {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        self.socket.recv(buffer).map_err(|_| LinkError::LinkTxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let n = self.read(&mut buffer[offset..])?;
            if n == 0 {
                return Err(LinkError::LinkRxFailed);
            }
            offset += n;
        }
        Ok(())
    }
}

impl ZLinkRx for StdUdpLinkRx {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        self.socket.recv(buffer).map_err(|_| LinkError::LinkTxFailed)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let n = self.read(&mut buffer[offset..])?;
            if n == 0 {
                return Err(LinkError::LinkRxFailed);
            }
            offset += n;
        }
        Ok(())
    }
}

impl ZLink for StdUdpLink {
    type Tx<'a> = StdUdpLinkTx;
    type Rx<'a> = StdUdpLinkRx;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (
            StdUdpLinkTx { socket: self.socket.try_clone().expect("try_clone failed"), mtu: self.mtu },
            StdUdpLinkRx { socket: self.socket.try_clone().expect("try_clone failed"), mtu: self.mtu },
        )
    }
}