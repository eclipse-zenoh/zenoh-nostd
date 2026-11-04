#![no_std]

use core::{cell::RefCell, net::SocketAddr};
use embassy_net::{
    IpAddress, IpEndpoint, Stack,
    tcp::TcpSocket,
    udp::{PacketMetadata, UdpSocket},
};
use zenoh_nostd::platform::*;

pub mod tcp;
pub mod udp;

struct BufferPool<T, const TX: usize, const RX: usize, const SOCKS: usize> {
    tx_buffers: [[T; TX]; SOCKS],
    rx_buffers: [[T; RX]; SOCKS],
    used: [bool; SOCKS],
}

impl<T, const TX: usize, const RX: usize, const SOCKS: usize> BufferPool<T, TX, RX, SOCKS> {
    fn new(value: T) -> Self
    where
        T: Copy,
    {
        Self {
            tx_buffers: [[value; TX]; SOCKS],
            rx_buffers: [[value; RX]; SOCKS],
            used: [false; SOCKS],
        }
    }

    fn allocate(&mut self) -> Option<usize> {
        self.used.iter().position(|&used| !used).inspect(|&idx| {
            self.used[idx] = true;
        })
    }
}

pub(crate) trait BufferPoolDrop {
    fn release(&mut self, idx: usize);
}

impl<T, const TX: usize, const RX: usize, const SOCKS: usize> BufferPoolDrop
    for BufferPool<T, TX, RX, SOCKS>
{
    fn release(&mut self, idx: usize) {
        if idx < SOCKS {
            self.used[idx] = false;
        }
    }
}

/// `MTU` is the size of each socket's tx buffer (the largest datagram or segment sent) and `SOCKS`
/// how many sockets can be open. `RX` is each socket's receive buffer: a peer may send several
/// datagrams back to back, and UDP does not resend what an undersized buffer drops, so keep it at
/// several MTUs.
/// `META` is how many packets each socket can queue (embassy's `PacketMetadata` slots); it has
/// nothing to do with the MTU, and sizing it by the MTU made this struct about 120 KB.
pub struct EmbassyLinkManager<
    'net,
    const MTU: usize,
    const SOCKS: usize,
    const META: usize = 8,
    const RX: usize = 4096,
> {
    stack: Stack<'net>,
    buffers: RefCell<BufferPool<u8, MTU, RX, SOCKS>>,
    metadatas: RefCell<BufferPool<PacketMetadata, META, META, SOCKS>>,
}

impl<'net, const MTU: usize, const SOCKS: usize, const META: usize, const RX: usize>
    EmbassyLinkManager<'net, MTU, SOCKS, META, RX>
{
    pub fn new(stack: Stack<'net>) -> Self {
        Self {
            stack,
            buffers: RefCell::new(BufferPool::new(0)),
            metadatas: RefCell::new(BufferPool::new(PacketMetadata::EMPTY)),
        }
    }

    #[allow(clippy::mut_from_ref)]
    fn allocate_buffers(&self) -> Option<(usize, &mut [u8], &mut [u8])> {
        let idx = self.buffers.borrow_mut().allocate()?;

        // SAFETY: This pool is simple, I should not have made any mistake. The reference will still be valid
        // because Tcp borrows EmbassyLinkManager and so EmbassyLinkManager can't be moved.
        let buffers = unsafe { &mut *self.buffers.as_ptr() };
        let tx = &mut buffers.tx_buffers[idx];
        let rx = &mut buffers.rx_buffers[idx];

        Some((idx, tx, rx))
    }

    #[allow(clippy::mut_from_ref)]
    fn allocate_metadatas(&self) -> Option<(usize, &mut [PacketMetadata], &mut [PacketMetadata])> {
        let idx = self.metadatas.borrow_mut().allocate()?;

        // SAFETY: This pool is simple, I should not have made any mistake. The reference will still be valid
        // because Tcp borrows EmbassyLinkManager and so EmbassyLinkManager can't be moved.
        let buffers = unsafe { &mut *self.metadatas.as_ptr() };
        let tx = &mut buffers.tx_buffers[idx];
        let rx = &mut buffers.rx_buffers[idx];

        Some((idx, tx, rx))
    }
}

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (EmbassyLinkTx<'link>, EmbassyLinkRx<'link>))]
pub enum EmbassyLink<'net> {
    Tcp(tcp::EmbassyTcpLink<'net>),
    Udp(udp::EmbassyUdpLink<'net>),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum EmbassyLinkTx<'link> {
    Tcp(tcp::EmbassyTcpLinkTx<'link>),
    Udp(udp::EmbassyUdpLinkTx<'link>),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum EmbassyLinkRx<'link> {
    Tcp(tcp::EmbassyTcpLinkRx<'link>),
    Udp(udp::EmbassyUdpLinkRx<'link>),
}

impl<'net, const MTU: usize, const SOCKS: usize, const META: usize, const RX: usize> ZLinkManager
    for EmbassyLinkManager<'net, MTU, SOCKS, META, RX>
{
    type Link<'a>
        = EmbassyLink<'a>
    where
        Self: 'a;

    async fn connect(
        &self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<Self::Link<'_>, LinkError> {
        let protocol = endpoint.protocol();
        let address = endpoint.address();

        match protocol.as_str() {
            "tcp" => {
                let dst_addr = SocketAddr::try_from(address)?;
                let (idx, tx, rx) = self.allocate_buffers().ok_or(LinkError::CouldNotConnect)?;
                let mut socket = TcpSocket::new(self.stack, rx, tx);

                let address: IpAddress = match dst_addr.ip() {
                    core::net::IpAddr::V4(v4) => IpAddress::Ipv4(v4),
                    core::net::IpAddr::V6(_) => {
                        zenoh::zbail!(LinkError::CouldNotConnect)
                    }
                };

                let ip_endpoint = IpEndpoint::new(address, dst_addr.port());

                socket
                    .connect(ip_endpoint)
                    .await
                    .map_err(|_| LinkError::CouldNotConnect)?;

                Ok(Self::Link::Tcp(tcp::EmbassyTcpLink::new(
                    socket,
                    MTU as u16,
                    idx,
                    &self.buffers,
                )))
            }
            "udp" => {
                let dst_addr = SocketAddr::try_from(address)?;
                let (idx1, tx, rx) = self.allocate_buffers().ok_or(LinkError::CouldNotConnect)?;

                let (idx2, tm, rm) = self
                    .allocate_metadatas()
                    .ok_or(LinkError::CouldNotConnect)?;

                let mut socket = UdpSocket::new(self.stack, rm, rx, tm, tx);
                socket.bind(0).map_err(|_| LinkError::CouldNotConnect)?;

                let address: IpAddress = match dst_addr.ip() {
                    core::net::IpAddr::V4(v4) => IpAddress::Ipv4(v4),
                    core::net::IpAddr::V6(_) => {
                        zenoh::zbail!(LinkError::CouldNotConnect)
                    }
                };

                let ip_endpoint = IpEndpoint::new(address, dst_addr.port());

                Ok(Self::Link::Udp(udp::EmbassyUdpLink::new(
                    socket,
                    ip_endpoint.into(),
                    MTU as u16,
                    idx1,
                    &self.buffers,
                    idx2,
                    &self.metadatas,
                )))
            }
            _ => zenoh::zbail!(LinkError::CouldNotParseProtocol),
        }
    }

    async fn listen(
        &self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<Self::Link<'_>, LinkError> {
        let protocol = endpoint.protocol();
        let address = endpoint.address();

        match protocol.as_str() {
            "tcp" => {
                let src_addr = SocketAddr::try_from(address)?;
                let (idx, tx, rx) = self.allocate_buffers().ok_or(LinkError::CouldNotConnect)?;
                let mut socket = TcpSocket::new(self.stack, rx, tx);

                let address: IpAddress = match src_addr.ip() {
                    core::net::IpAddr::V4(v4) => IpAddress::Ipv4(v4),
                    core::net::IpAddr::V6(_) => {
                        zenoh::zbail!(LinkError::CouldNotConnect)
                    }
                };

                let ip_endpoint = IpEndpoint::new(address, src_addr.port());

                socket
                    .accept(ip_endpoint)
                    .await
                    .map_err(|_| LinkError::CouldNotConnect)?;

                Ok(Self::Link::Tcp(tcp::EmbassyTcpLink::new(
                    socket,
                    MTU as u16,
                    idx,
                    &self.buffers,
                )))
            }
            _ => zenoh::zbail!(LinkError::CouldNotParseProtocol),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The manager is built by value, often on a small stack, and kept in a static. A metadata pool
    /// with one entry per MTU byte once made it about 120 KB and overflowed the main task's stack.
    /// A UDP peer can send datagrams back to back (a ROS node announces its liveliness tokens that
    /// way); an rx buffer of one MTU drops all but the one being read, and UDP does not resend.
    #[test]
    fn the_rx_buffer_can_hold_several_datagrams() {
        let pool = BufferPool::<u8, 512, 4096, 2>::new(0);
        assert_eq!(pool.tx_buffers[0].len(), 512);
        assert_eq!(pool.rx_buffers[0].len(), 4096);
        let default_rx = core::mem::size_of::<EmbassyLinkManager<'static, 512, 3>>();
        assert!(
            default_rx >= 3 * 4096,
            "the default rx buffer is smaller than 8 datagrams"
        );
    }

    #[test]
    fn the_link_manager_stays_small() {
        let size = core::mem::size_of::<EmbassyLinkManager<'static, 512, 3>>();
        assert!(
            size < 32 * 1024,
            "EmbassyLinkManager<512, 3> is {size} bytes"
        );
    }
}
