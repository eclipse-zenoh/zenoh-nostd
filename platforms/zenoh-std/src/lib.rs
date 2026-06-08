use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::Arc;

use native_tls::Identity;
use tungstenite::client::IntoClientRequest;
use zenoh_nostd::platform::*;

#[cfg(feature = "bt")]
mod bt;
mod mcast;
mod raw_eth;
mod serial;
pub mod tcp;
mod tls;
pub mod udp;
mod usb_cdc;
mod ws;

pub struct StdLinkManager;

#[allow(clippy::large_enum_variant)]
#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (StdLinkTx, StdLinkRx))]
pub enum StdLink {
    #[cfg(feature = "bt")]
    Bt(bt::BtStream),
    Mcast(mcast::McLink),
    RawEth(raw_eth::StdRawEthLink),
    Serial(serial::StdSerialLink),
    Tcp(tcp::StdTcpLink),
    Tls(tls::StdTlsLink),
    Udp(udp::StdUdpLink),
    UsbCdc(usb_cdc::StdUsbCdcLink),
    Ws(ws::StdWsLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum StdLinkTx {
    #[cfg(feature = "bt")]
    Bt(bt::BtStreamTx),
    Mcast(mcast::McLinkTx),
    RawEth(raw_eth::RawEthTx),
    Serial(serial::SerialTx),
    Tcp(tcp::StdTcpLinkTx),
    Tls(tls::StdTlsLinkTx),
    Udp(udp::StdUdpLinkTx),
    UsbCdc(usb_cdc::UsbCdcTx),
    Ws(ws::StdWsLinkTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum StdLinkRx {
    #[cfg(feature = "bt")]
    Bt(bt::BtStreamRx),
    Mcast(mcast::McLinkRx),
    RawEth(raw_eth::RawEthRx),
    Serial(serial::SerialRx),
    Tcp(tcp::StdTcpLinkRx),
    Tls(tls::StdTlsLinkRx),
    Udp(udp::StdUdpLinkRx),
    UsbCdc(usb_cdc::UsbCdcRx),
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
            "raw-eth" => {
                let iface = address.as_str();
                let link = raw_eth::StdRawEthLink::new(iface)?;
                Ok(Self::Link::RawEth(link))
            }
            "usb-cdc" => {
                let path = address.as_str();
                let link = usb_cdc::StdUsbCdcLink::new(path)?;
                Ok(Self::Link::UsbCdc(link))
            }
            #[cfg(feature = "bt")]
            "bt" => {
                let link = bt::BtStream::new();
                Ok(Self::Link::Bt(link))
            }
            "udp-mcast" => {
                let dst_addr = SocketAddr::try_from(address)?;
                let bind_addr = "0.0.0.0:0".parse::<SocketAddr>()
                    .map_err(|_| LinkError::CouldNotConnect)?;
                let std_socket = std::net::UdpSocket::bind(bind_addr)
                    .map_err(|_| LinkError::CouldNotConnect)?;
                std_socket.connect(dst_addr)
                    .map_err(|_| LinkError::CouldNotConnect)?;
                let socket = Arc::new(std_socket);
                Ok(Self::Link::Mcast(mcast::McLink { socket, mtu: 8192, multicast_addr: dst_addr }))
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
            "tls" => {
                let src_addr = SocketAddr::try_from(address)?;
                let identity_pem = std::env::var("ZENOH_TLS_IDENTITY")
                    .map_err(|_| LinkError::CouldNotListen)?;
                let password = std::env::var("ZENOH_TLS_PASSWORD")
                    .unwrap_or_default();
                let identity = Identity::from_pkcs12(identity_pem.as_bytes(), &password)
                    .map_err(|_| LinkError::CouldNotListen)?;
                let listener = TcpListener::bind(src_addr)
                    .map_err(|_| LinkError::CouldNotConnect)?;
                let (socket, _) = listener
                    .accept()
                    .map_err(|_| LinkError::CouldNotConnect)?;
                let link = tls::StdTlsLink::accept(socket, &identity)?;
                Ok(Self::Link::Tls(link))
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
            "ws" => {
                let src_addr = SocketAddr::try_from(address)?;
                let listener = TcpListener::bind(src_addr)
                    .map_err(|_| LinkError::CouldNotConnect)?;
                let (socket, _) = listener
                    .accept()
                    .map_err(|_| LinkError::CouldNotConnect)?;
                let link = ws::StdWsLink::accept(socket)?;
                Ok(Self::Link::Ws(link))
            }
            "serial" => {
                let path = address.as_str();
                let baud = 115200u32;
                let link = serial::StdSerialLink::new(path, baud)?;
                Ok(Self::Link::Serial(link))
            }
            "raw-eth" => {
                let iface = address.as_str();
                let link = raw_eth::StdRawEthLink::new(iface)?;
                Ok(Self::Link::RawEth(link))
            }
            "usb-cdc" => {
                let path = address.as_str();
                let link = usb_cdc::StdUsbCdcLink::new(path)?;
                Ok(Self::Link::UsbCdc(link))
            }
            "udp-mcast" => {
                let dst_addr = SocketAddr::try_from(address)?;
                let std_socket = std::net::UdpSocket::bind(dst_addr)
                    .map_err(|_| LinkError::CouldNotConnect)?;
                let mut peek_buf = [0u8; 2];
                let (_, src_addr) = std_socket.peek_from(&mut peek_buf)
                    .map_err(|_| LinkError::CouldNotConnect)?;
                std_socket.connect(src_addr)
                    .map_err(|_| LinkError::CouldNotConnect)?;
                let socket = Arc::new(std_socket);
                Ok(Self::Link::Mcast(mcast::McLink { socket, mtu: 8192, multicast_addr: dst_addr }))
            }
            _ => zenoh::zbail!(LinkError::CouldNotParseProtocol),
        }
    }
}