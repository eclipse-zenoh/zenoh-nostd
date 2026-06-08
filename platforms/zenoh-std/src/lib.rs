use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};

use tungstenite::client::IntoClientRequest;
use zenoh_nostd::platform::*;

mod bt;
mod serial;
mod tcp;
mod tls;
mod udp;
mod ws;

pub struct StdLinkManager;

#[allow(clippy::large_enum_variant)]
#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (StdLinkTx, StdLinkRx))]
pub enum StdLink {
    Bt(bt::BtStream),
    Serial(serial::StdSerialLink),
    Tcp(tcp::StdTcpLink),
    Tls(tls::StdTlsLink),
    Udp(udp::StdUdpLink),
    Ws(ws::StdWsLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum StdLinkTx {
    Bt(bt::BtStream),
    Serial(serial::SerialTx),
    Tcp(tcp::StdTcpLinkTx),
    Tls(tls::StdTlsLinkTx),
    Udp(udp::StdUdpLinkTx),
    Ws(ws::StdWsLinkTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum StdLinkRx {
    Bt(bt::BtStream),
    Serial(serial::SerialRx),
    Tcp(tcp::StdTcpLinkRx),
    Tls(tls::StdTlsLinkRx),
    Udp(udp::StdUdpLinkRx),
    Ws(ws::StdWsLinkRx),
}

impl ZLinkManager for StdLinkManager {
    type Link<'a>
        = StdLink
    where
        Self: 'a;

    fn connect(
        &self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<Self::Link<'_>, LinkError> {
        let protocol = endpoint.protocol();
        let address = endpoint.address();

        match protocol.as_str() {
            "tcp" => {
                let dst_addr = SocketAddr::try_from(address)?;
                let socket = TcpStream::connect(dst_addr)
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

                #[allow(unused_mut)]
                let mut mtu = u16::MAX - header;

                #[cfg(target_family = "unix")]
                {
                    let socket = socket2::SockRef::from(&socket);
                    let mss = socket.tcp_mss().unwrap_or(mtu as u32) / 2;
                    let mut tgt = mss;
                    while (tgt + mss) < mtu as u32 {
                        tgt += mss;
                    }
                    mtu = (mtu as u32).min(tgt) as u16;
                }

                Ok(Self::Link::Tcp(tcp::StdTcpLink::new(socket, mtu)))
            }
            "tls" => {
                let dst_addr = SocketAddr::try_from(address)?;
                let domain = address
                    .as_str()
                    .rsplitn(2, ':')
                    .last()
                    .unwrap_or(address.as_str());
                let link = tls::StdTlsLink::connect(dst_addr, domain)?;
                Ok(Self::Link::Tls(link))
            }
            "udp" => {
                let dst_addr = SocketAddr::try_from(address)?;
                let socket = UdpSocket::bind("0.0.0.0:0")
                    .map_err(|_| LinkError::CouldNotConnect)?;

                socket
                    .connect(dst_addr)
                    .map_err(|_| LinkError::CouldNotConnect)?;

                Ok(Self::Link::Udp(udp::StdUdpLink::new(socket, 8192)))
            }
            "ws" => {
                let dst_addr = SocketAddr::try_from(address)?;
                let tcp = TcpStream::connect(dst_addr)
                    .map_err(|_| LinkError::CouldNotConnect)?;
                tcp.set_nodelay(true)
                    .map_err(|_| LinkError::CouldNotConnect)?;

                let header = match tcp
                    .local_addr()
                    .map_err(|_| LinkError::CouldNotGetAddrInfo)?
                    .ip()
                {
                    core::net::IpAddr::V4(_) => 40,
                    core::net::IpAddr::V6(_) => 60,
                };

                let mtu = u16::MAX - header;

                let request: tungstenite::http::Request<()> =
                    format!("ws://{}/", dst_addr)
                        .into_client_request()
                        .map_err(|_| LinkError::CouldNotConnect)?;

                let (stream, _) = tungstenite::client::client(request, tcp)
                    .map_err(|_| LinkError::CouldNotConnect)?;

                Ok(Self::Link::Ws(ws::StdWsLink::new(stream, mtu)))
            }
            "serial" => {
                let path = address.as_str();
                let baud = 115200u32;
                let link = serial::StdSerialLink::new(path, baud)?;
                Ok(Self::Link::Serial(link))
            }
            _ => zenoh::zbail!(LinkError::CouldNotParseProtocol),
        }
    }

    fn listen(
        &self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<Self::Link<'_>, LinkError> {
        let protocol = endpoint.protocol();
        let address = endpoint.address();

        match protocol.as_str() {
            "tcp" => {
                let src_addr = SocketAddr::try_from(address)?;
                let listener = TcpListener::bind(src_addr)
                    .map_err(|_| LinkError::CouldNotConnect)?;

                let (socket, _) = listener
                    .accept()
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

                #[allow(unused_mut)]
                let mut mtu = u16::MAX - header;

                #[cfg(target_family = "unix")]
                {
                    let socket = socket2::SockRef::from(&socket);
                    let mss = socket.tcp_mss().unwrap_or(mtu as u32) / 2;
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
                    .map_err(|_| LinkError::CouldNotConnect)?;

                let mut peek_buf = [0u8; 2];
                let (_, dst_addr) = socket
                    .peek_from(&mut peek_buf)
                    .map_err(|_| LinkError::CouldNotConnect)?;

                socket
                    .connect(dst_addr)
                    .map_err(|_| LinkError::CouldNotConnect)?;

                Ok(Self::Link::Udp(udp::StdUdpLink::new(socket, 8192)))
            }
            "serial" => {
                let path = address.as_str();
                let baud = 115200u32;
                let link = serial::StdSerialLink::new(path, baud)?;
                Ok(Self::Link::Serial(link))
            }
            _ => zenoh::zbail!(LinkError::CouldNotParseProtocol),
        }
    }
}