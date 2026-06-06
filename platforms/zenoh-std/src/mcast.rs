use std::net::SocketAddr;
use std::sync::Arc;

use async_net::UdpSocket;
use zenoh_nostd::platform::*;

/// A UDP multicast link for sending and receiving datagrams.
///
/// Used for SCOUT/HELLO peer discovery. Not streamed — each read/write
/// is a discrete datagram.
pub struct McLink {
    socket: Arc<UdpSocket>,
    mtu: u16,
    multicast_addr: SocketAddr,
}

impl McLink {
    pub async fn new(
        multicast_addr: SocketAddr,
        bind_addr: SocketAddr,
    ) -> core::result::Result<Self, LinkError> {
        let socket = Arc::new(
            UdpSocket::bind(bind_addr)
                .await
                .map_err(|_| LinkError::CouldNotConnect)?,
        );

        let multi_ip = match multicast_addr.ip() {
            std::net::IpAddr::V4(ip) => ip,
            _ => return Err(LinkError::CouldNotConnect),
        };
        let bind_ip = match bind_addr.ip() {
            std::net::IpAddr::V4(ip) => ip,
            _ => return Err(LinkError::CouldNotConnect),
        };
        socket
            .join_multicast_v4(multi_ip, bind_ip)
            .map_err(|_| LinkError::CouldNotConnect)?;

        Ok(Self {
            socket,
            mtu: 8192,
            multicast_addr,
        })
    }
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
    async fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.socket
            .send_to(buffer, self.multicast_addr)
            .await
            .map_err(|_| LinkError::LinkTxFailed)?;
        Ok(())
    }
}

impl ZLinkTx for McLinkTx {
    async fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.socket
            .send_to(buffer, self.multicast_addr)
            .await
            .map_err(|_| LinkError::LinkTxFailed)?;
        Ok(())
    }
}

impl ZLinkRx for McLink {
    async fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let (len, _) = self
            .socket
            .recv_from(buffer)
            .await
            .map_err(|_| LinkError::LinkRxFailed)?;
        Ok(len)
    }

    async fn read_exact(&mut self, _: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLinkRx for McLinkRx {
    async fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let (len, _) = self
            .socket
            .recv_from(buffer)
            .await
            .map_err(|_| LinkError::LinkRxFailed)?;
        Ok(len)
    }

    async fn read_exact(&mut self, _: &mut [u8]) -> core::result::Result<(), LinkError> {
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
