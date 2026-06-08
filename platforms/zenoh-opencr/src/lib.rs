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
use zenoh_platform_posix::tcp::{PosixTcpLink, PosixTcpTx, PosixTcpRx};
use zenoh_platform_posix::udp::{PosixUdpLink, PosixUdpTx, PosixUdpRx};

mod serial;

pub struct OpencrLinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (OpencrLinkTx, OpencrLinkRx))]
pub enum OpencrLink {
    Tcp(PosixTcpLink),
    Udp(PosixUdpLink),
    Serial(serial::PlatformSerialLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum OpencrLinkTx {
    Tcp(PosixTcpTx),
    Udp(PosixUdpTx),
    Serial(serial::PlatformSerialTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum OpencrLinkRx {
    Tcp(PosixTcpRx),
    Udp(PosixUdpRx),
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
            "tcp" => Ok(Self::Link::Tcp(PosixTcpLink::connect(address.as_str())?)),
            "udp" => Ok(Self::Link::Udp(PosixUdpLink::connect(address.as_str())?)),
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
            "tcp" => Ok(Self::Link::Tcp(PosixTcpLink::listen(address.as_str())?)),
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