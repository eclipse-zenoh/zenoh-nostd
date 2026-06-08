//! Zenoh platform support for OpenCR board
//!
//! Provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations via
//! LWIP's BSD socket API (`lwip/sockets.h`).
//!
//! OpenCR is STM32F7-based. LWIP provides a standard POSIX-compatible socket
//! API. Ethernet is typically used via STM32 HAL Ethernet driver + LWIP.
//!
//! # Requirements
//!
//! - LWIP stack enabled with `lwipopts.h` configuration
//! - `LWIP_SOCKET=y` for socket API
//! - STM32 HAL Ethernet driver initialized

#![no_std]

use zenoh_nostd::platform::*;

mod serial;
mod sockets;
mod tcp;
mod udp;

pub struct OpencrLinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (OpencrLinkTx, OpencrLinkRx))]
pub enum OpencrLink {
    Tcp(tcp::PlatformTcpLink),
    Udp(udp::PlatformUdpLink),
    Serial(serial::PlatformSerialLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum OpencrLinkTx {
    Tcp(tcp::PlatformTcpTx),
    Udp(udp::PlatformUdpTx),
    Serial(serial::PlatformSerialTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum OpencrLinkRx {
    Tcp(tcp::PlatformTcpRx),
    Udp(udp::PlatformUdpRx),
    Serial(serial::PlatformSerialRx),
}

impl ZLinkManager for OpencrLinkManager {
    type Link<'a>
        = OpencrLink
    where
        Self: 'a;

    fn connect(
        &self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<Self::Link<'_>, LinkError> {
        let protocol = endpoint.protocol();
        let address = endpoint.address();

        match protocol.as_str() {
            "tcp" => Ok(Self::Link::Tcp(tcp::PlatformTcpLink::connect(address.as_str())?)),
            "udp" => Ok(Self::Link::Udp(udp::PlatformUdpLink::connect(address.as_str())?)),
            "serial" => {
                let path = address.as_str();
                let baud = 115200u32;
                Ok(Self::Link::Serial(serial::PlatformSerialLink::new(path, baud)?))
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
            "tcp" => Ok(Self::Link::Tcp(tcp::PlatformTcpLink::listen(address.as_str())?)),
            "udp" => {
                let _ = address;
                Err(LinkError::CouldNotListen)
            }
            "serial" => {
                let path = address.as_str();
                let baud = 115200u32;
                Ok(Self::Link::Serial(serial::PlatformSerialLink::new(path, baud)?))
            }
            _ => zenoh::zbail!(LinkError::CouldNotParseProtocol),
        }
    }
}