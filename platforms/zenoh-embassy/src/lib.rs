#![no_std]

use core::{cell::RefCell, net::SocketAddr};
#[cfg(feature = "tcp")]
use embassy_net::tcp::TcpSocket;
#[cfg(feature = "udp")]
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{IpAddress, IpEndpoint, Stack};

#[cfg(not(any(feature = "tcp", feature = "udp")))]
compile_error!("zenoh-embassy requires at least one transport: tcp or udp");
use zenoh_nostd::platform::*;

#[cfg(feature = "tcp")]
pub mod tcp;
#[cfg(feature = "udp")]
pub mod udp;

#[cfg(feature = "tcp")]
fn tcp_listen_endpoint(address: SocketAddr) -> Result<embassy_net::IpListenEndpoint, LinkError> {
    let addr = match address.ip() {
        core::net::IpAddr::V4(ip) if ip.is_unspecified() => None,
        core::net::IpAddr::V4(ip) => Some(IpAddress::Ipv4(ip)),
        core::net::IpAddr::V6(_) => return Err(LinkError::CouldNotConnect),
    };
    Ok(embassy_net::IpListenEndpoint {
        addr,
        port: address.port(),
    })
}

struct BufferPool<T, const MTU: usize, const SOCKS: usize> {
    tx_buffers: [[T; MTU]; SOCKS],
    rx_buffers: [[T; MTU]; SOCKS],
    used: [bool; SOCKS],
}

impl<T, const MTU: usize, const SOCKS: usize> BufferPool<T, MTU, SOCKS> {
    fn new(value: T) -> Self
    where
        T: Copy,
    {
        Self {
            tx_buffers: [[value; MTU]; SOCKS],
            rx_buffers: [[value; MTU]; SOCKS],
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

/// Return a buffer slot on errors, cancellation, and normal link destruction.
pub(crate) struct PoolLease<'a> {
    idx: usize,
    pool: &'a RefCell<dyn BufferPoolDrop>,
}
impl<'a> PoolLease<'a> {
    fn new(pool: &'a RefCell<dyn BufferPoolDrop>, idx: usize) -> Self {
        Self { idx, pool }
    }
}
impl Drop for PoolLease<'_> {
    fn drop(&mut self) {
        self.pool.borrow_mut().release(self.idx);
    }
}

impl<T, const MTU: usize, const SOCKS: usize> BufferPoolDrop for BufferPool<T, MTU, SOCKS> {
    fn release(&mut self, idx: usize) {
        if idx < SOCKS {
            self.used[idx] = false;
        }
    }
}

pub struct EmbassyLinkManager<'net, const MTU: usize, const SOCKS: usize> {
    stack: Stack<'net>,
    buffers: RefCell<BufferPool<u8, MTU, SOCKS>>,
    #[cfg(feature = "udp")]
    metadatas: RefCell<BufferPool<PacketMetadata, MTU, SOCKS>>,
}

impl<'net, const MTU: usize, const SOCKS: usize> EmbassyLinkManager<'net, MTU, SOCKS> {
    pub fn new(stack: Stack<'net>) -> Self {
        Self {
            stack,
            buffers: RefCell::new(BufferPool::new(0)),
            #[cfg(feature = "udp")]
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

    #[cfg(feature = "udp")]
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
    #[cfg(feature = "tcp")]
    Tcp(tcp::EmbassyTcpLink<'net>),
    #[cfg(feature = "udp")]
    Udp(udp::EmbassyUdpLink<'net>),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum EmbassyLinkTx<'link> {
    #[cfg(feature = "tcp")]
    Tcp(tcp::EmbassyTcpLinkTx<'link>),
    #[cfg(feature = "udp")]
    Udp(udp::EmbassyUdpLinkTx<'link>),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum EmbassyLinkRx<'link> {
    #[cfg(feature = "tcp")]
    Tcp(tcp::EmbassyTcpLinkRx<'link>),
    #[cfg(feature = "udp")]
    Udp(udp::EmbassyUdpLinkRx<'link>),
}

impl<'net, const MTU: usize, const SOCKS: usize> ZLinkManager
    for EmbassyLinkManager<'net, MTU, SOCKS>
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
            #[cfg(feature = "tcp")]
            "tcp" => {
                let dst_addr = SocketAddr::try_from(address)?;
                let (idx, tx, rx) = self.allocate_buffers().ok_or(LinkError::CouldNotConnect)?;
                let lease = PoolLease::new(&self.buffers, idx);
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
                    socket, MTU as u16, lease,
                )))
            }
            #[cfg(feature = "udp")]
            "udp" => {
                let dst_addr = SocketAddr::try_from(address)?;
                let (idx1, tx, rx) = self.allocate_buffers().ok_or(LinkError::CouldNotConnect)?;
                let buffers = PoolLease::new(&self.buffers, idx1);

                let (idx2, tm, rm) = self
                    .allocate_metadatas()
                    .ok_or(LinkError::CouldNotConnect)?;

                let metadatas = PoolLease::new(&self.metadatas, idx2);
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
                    buffers,
                    metadatas,
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
        #[cfg(feature = "tcp")]
        let address = endpoint.address();

        match protocol.as_str() {
            #[cfg(feature = "tcp")]
            "tcp" => {
                let src_addr = SocketAddr::try_from(address)?;
                let ip_endpoint = tcp_listen_endpoint(src_addr)?;
                let (idx, tx, rx) = self.allocate_buffers().ok_or(LinkError::CouldNotConnect)?;
                let lease = PoolLease::new(&self.buffers, idx);
                let mut socket = TcpSocket::new(self.stack, rx, tx);

                socket
                    .accept(ip_endpoint)
                    .await
                    .map_err(|_| LinkError::CouldNotConnect)?;

                Ok(Self::Link::Tcp(tcp::EmbassyTcpLink::new(
                    socket, MTU as u16, lease,
                )))
            }
            _ => zenoh::zbail!(LinkError::CouldNotParseProtocol),
        }
    }
}

#[cfg(all(test, feature = "tcp"))]
mod tests {
    use super::*;

    #[test]
    fn cancelled_allocation_returns_its_pool_slot() {
        let pool = RefCell::new(BufferPool::<u8, 16, 1>::new(0));
        for _ in 0..1000 {
            let idx = pool.borrow_mut().allocate().unwrap();
            let lease = PoolLease::new(&pool, idx);
            assert!(pool.borrow_mut().allocate().is_none());
            drop(lease);
        }
        assert_eq!(pool.borrow_mut().allocate(), Some(0));
    }

    #[test]
    fn wildcard_listener_accepts_any_local_address() {
        let endpoint = tcp_listen_endpoint("0.0.0.0:7444".parse().unwrap()).unwrap();
        assert_eq!(endpoint.addr, None);
        assert_eq!(endpoint.port, 7444);
    }

    #[test]
    fn explicit_listener_keeps_its_address() {
        let endpoint = tcp_listen_endpoint("192.0.2.1:7445".parse().unwrap()).unwrap();
        assert_eq!(
            endpoint.addr,
            Some(IpAddress::Ipv4(core::net::Ipv4Addr::new(192, 0, 2, 1)))
        );
        assert_eq!(endpoint.port, 7445);
    }

    #[test]
    fn ipv6_listener_is_rejected() {
        assert!(matches!(
            tcp_listen_endpoint("[::1]:7444".parse().unwrap()),
            Err(LinkError::CouldNotConnect)
        ));
    }
}
