//! Native TCP, UDP and outgoing WebSocket links for `zenoh-nostd`.
//!
//! Construct [`StdLinkManager`] and wrap it with
//! [`TransportLinkManager`](zenoh_nostd::session::TransportLinkManager). Connect
//! using numeric `tcp/IP:port`, `udp/IP:port` or `ws/IP:port` endpoints; DNS names,
//! TLS (`wss`) and WebSocket paths are not supported by this manager. Listening
//! supports TCP and UDP, not WebSocket. A UDP listener selects its first sender.
//!
//! This adapter uses `async-net` sockets but installs no executor or Embassy time
//! driver. The application must provide both, poll the session driver alongside
//! operations, and keep session access within one executor/thread context. Select
//! `alloc` on the application's core dependency for heap-backed Zenoh storage;
//! this adapter has no feature of that name and itself uses host allocations.
//! Socket buffer acceptance does not establish remote delivery.

#![warn(missing_docs)]

use std::net::SocketAddr;

use async_net::{TcpListener, TcpStream, UdpSocket};
use wtx::{
    misc::Uri,
    web_socket::{WebSocketConnector, WebSocketPartsOwned},
};
use zenoh_nostd::platform::*;

mod tcp;
mod udp;
mod ws;

/// Stateless native link factory; use `TransportLinkManager::from(StdLinkManager)` for Zenoh handshakes.
///
/// Raw `connect` supports TCP/UDP/WebSocket; raw `listen` supports TCP/UDP.
/// Both require numeric socket addresses and return one link per invocation.
pub struct StdLinkManager;

#[allow(clippy::large_enum_variant)]
#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (StdLinkTx<'link>, StdLinkRx<'link>))]
/// Connection selected by the link manager; split it through the `ZLink` trait.
pub enum StdLink {
    /// TCP byte stream.
    Tcp(tcp::StdTcpLink),
    /// UDP datagrams.
    Udp(udp::StdUdpLink),
    /// Binary WebSocket messages.
    Ws(ws::StdWsLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
/// Transmit half produced by `ZLink::split`; retains the selected framing policy.
pub enum StdLinkTx<'link> {
    /// TCP byte stream.
    Tcp(tcp::StdTcpLinkTx),
    /// UDP datagrams.
    Udp(udp::StdUdpLinkTx),
    /// Binary WebSocket messages.
    Ws(ws::StdWsLinkTx<'link>),
}

#[derive(ZLinkInfo, ZLinkRx)]
/// Receive half produced by `ZLink::split`; retains the selected framing policy.
pub enum StdLinkRx<'link> {
    /// TCP byte stream.
    Tcp(tcp::StdTcpLinkRx),
    /// UDP datagrams.
    Udp(udp::StdUdpLinkRx),
    /// Binary WebSocket messages.
    Ws(ws::StdWsLinkRx<'link>),
}

impl ZLinkManager for StdLinkManager {
    type Link<'a>
        = StdLink
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
                let socket = TcpStream::connect(dst_addr)
                    .await
                    .map_err(|_| LinkError::CouldNotConnect)?;

                socket
                    .set_nodelay(true)
                    .map_err(|_| LinkError::CouldNotConnect)?;

                let header = match socket
                    .local_addr()
                    .map_err(|_| LinkError::CouldNotGetAddrInfo)?
                    .ip()
                {
                    core::net::IpAddr::V4(_) => 40,
                    core::net::IpAddr::V6(_) => 60,
                };

                #[allow(unused_mut)] // mut is not needed when target_family != unix
                let mut mtu = u16::MAX - header;

                // target limitation of socket2: https://docs.rs/socket2/latest/src/socket2/sys/unix.rs.html#1544
                #[cfg(target_family = "unix")]
                {
                    let socket = socket2::SockRef::from(&socket);
                    // Get the MSS and divide it by 2 to ensure we can at least fill half the MSS
                    let mss = socket.tcp_mss().unwrap_or(mtu as u32) / 2;
                    // Compute largest multiple of TCP MSS that is smaller of default MTU
                    let mut tgt = mss;
                    while (tgt + mss) < mtu as u32 {
                        tgt += mss;
                    }
                    mtu = (mtu as u32).min(tgt) as u16;
                }

                Ok(Self::Link::Tcp(tcp::StdTcpLink::new(socket, mtu)))
            }
            "udp" => {
                let dst_addr = SocketAddr::try_from(address)?;
                let socket = UdpSocket::bind("0.0.0.0:0")
                    .await
                    .map_err(|_| LinkError::CouldNotConnect)?;

                socket
                    .connect(dst_addr)
                    .await
                    .map_err(|_| LinkError::CouldNotConnect)?;

                Ok(Self::Link::Udp(udp::StdUdpLink::new(socket, 8192)))
            }
            "ws" => {
                let dst_addr = SocketAddr::try_from(address)?;
                let uri = Uri::new(format!("ws://{}", dst_addr));

                let socket = TcpStream::connect(uri.hostname_with_implied_port())
                    .await
                    .map_err(|_| {
                        zenoh::error!("Could not connect to TcpStream");
                        LinkError::CouldNotConnect
                    })?;

                socket
                    .set_nodelay(true)
                    .map_err(|_| LinkError::CouldNotConnect)?;

                let header = match socket
                    .local_addr()
                    .map_err(|_| LinkError::CouldNotGetAddrInfo)?
                    .ip()
                {
                    core::net::IpAddr::V4(_) => 40,
                    core::net::IpAddr::V6(_) => 60,
                };

                #[allow(unused_mut)] // mut is not needed when target_family != unix
                let mut mtu = u16::MAX - header;

                // target limitation of socket2: https://docs.rs/socket2/latest/src/socket2/sys/unix.rs.html#1544
                #[cfg(target_family = "unix")]
                {
                    let socket = socket2::SockRef::from(&socket);
                    // Get the MSS and divide it by 2 to ensure we can at least fill half the MSS
                    let mss = socket.tcp_mss().unwrap_or(mtu as u32) / 2;
                    // Compute largest multiple of TCP MSS that is smaller of default MTU
                    let mut tgt = mss;
                    while (tgt + mss) < mtu as u32 {
                        tgt += mss;
                    }
                    mtu = (mtu as u32).min(tgt) as u16;
                }

                let stream = WebSocketConnector::default()
                    .connect(socket, &uri.to_ref())
                    .await
                    .map_err(|_| LinkError::CouldNotConnect)?;

                let WebSocketPartsOwned { reader, writer, .. } = stream
                    .into_parts(|s| (s.clone(), s))
                    .map_err(|_| LinkError::CouldNotConnect)?;

                Ok(Self::Link::Ws(ws::StdWsLink::new(reader, writer, mtu)))
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
                let socket = TcpListener::bind(src_addr)
                    .await
                    .map_err(|_| LinkError::CouldNotConnect)?;

                let (socket, _) = socket
                    .accept()
                    .await
                    .map_err(|_| LinkError::CouldNotConnect)?;

                socket
                    .set_nodelay(true)
                    .map_err(|_| LinkError::CouldNotConnect)?;

                let header = match socket
                    .local_addr()
                    .map_err(|_| LinkError::CouldNotGetAddrInfo)?
                    .ip()
                {
                    core::net::IpAddr::V4(_) => 40,
                    core::net::IpAddr::V6(_) => 60,
                };

                #[allow(unused_mut)] // mut is not needed when target_family != unix
                let mut mtu = u16::MAX - header;

                // target limitation of socket2: https://docs.rs/socket2/latest/src/socket2/sys/unix.rs.html#1544
                #[cfg(target_family = "unix")]
                {
                    let socket = socket2::SockRef::from(&socket);
                    // Get the MSS and divide it by 2 to ensure we can at least fill half the MSS
                    let mss = socket.tcp_mss().unwrap_or(mtu as u32) / 2;
                    // Compute largest multiple of TCP MSS that is smaller of default MTU
                    let mut tgt = mss;
                    while (tgt + mss) < mtu as u32 {
                        tgt += mss;
                    }
                    mtu = (mtu as u32).min(tgt) as u16;
                }

                Ok(Self::Link::Tcp(tcp::StdTcpLink::new(socket, mtu)))
            }
            "udp" => {
                let src_addr = SocketAddr::try_from(address)?;
                let socket = UdpSocket::bind(src_addr)
                    .await
                    .map_err(|_| LinkError::CouldNotConnect)?;

                let (_, dst_addr) = socket
                    .peek_from(&mut [0u8, 0])
                    .await
                    .map_err(|_| LinkError::CouldNotConnect)?;

                socket
                    .connect(dst_addr)
                    .await
                    .map_err(|_| LinkError::CouldNotConnect)?;

                Ok(Self::Link::Udp(udp::StdUdpLink::new(socket, 8192)))
            }
            _ => zenoh::zbail!(LinkError::CouldNotParseProtocol),
        }
    }
}
