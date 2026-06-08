use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;

use zenoh_nostd::platform::*;

/// A UDP multicast link for sending and receiving datagrams.
///
/// Used for SCOUT/HELLO peer discovery. Not streamed — each read/write
/// is a discrete datagram.
pub struct McLink {
    pub(crate) socket: Arc<UdpSocket>,
    pub(crate) mtu: u16,
    pub(crate) multicast_addr: SocketAddr,
}

pub struct McLinkTx {
    socket: Arc<UdpSocket>,
    multicast_addr: SocketAddr,
    mtu: u16,
}

pub struct McLinkRx {
    socket: Arc<UdpSocket>,
    mtu: u16,
}

impl ZLinkInfo for McLink {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        false
    }
}

impl ZLinkInfo for McLinkTx {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        false
    }
}

impl ZLinkInfo for McLinkRx {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        false
    }
}

impl ZLinkTx for McLink {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.socket
            .send_to(buffer, self.multicast_addr)
            .map_err(|_| LinkError::LinkTxFailed)?;
        Ok(())
    }
}

impl ZLinkTx for McLinkTx {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.socket
            .send_to(buffer, self.multicast_addr)
            .map_err(|_| LinkError::LinkTxFailed)?;
        Ok(())
    }
}

impl ZLinkRx for McLink {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let (len, _) = self
            .socket
            .recv_from(buffer)
            .map_err(|_| LinkError::LinkRxFailed)?;
        Ok(len)
    }

    fn read_exact(&mut self, _: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLinkRx for McLinkRx {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let (len, _) = self
            .socket
            .recv_from(buffer)
            .map_err(|_| LinkError::LinkRxFailed)?;
        Ok(len)
    }

    fn read_exact(&mut self, _: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for McLink {
    type Tx<'a> = McLinkTx;
    type Rx<'a> = McLinkRx;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        let tx = McLinkTx {
            socket: self.socket.clone(),
            multicast_addr: self.multicast_addr,
            mtu: self.mtu,
        };

        let rx = McLinkRx {
            socket: self.socket.clone(),
            mtu: self.mtu,
        };

        (tx, rx)
    }
}
