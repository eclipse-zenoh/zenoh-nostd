//! Zenoh platform support for Zephyr RTOS
//!
//! Provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations via
//! Zephyr's POSIX socket API (`zephyr/posix/sys/socket.h`).
//!
//! # Kconfig Requirements
//!
//! - `CONFIG_NETWORKING=y` — enables networking stack
//! - `CONFIG_POSIX_API=y` — enables POSIX socket API
//! - `CONFIG_NET_TCP=y` — for TCP support
//! - `CONFIG_NET_UDP=y` — for UDP support
//! - For serial: `CONFIG_SERIAL=y`, `CONFIG_UART_INTERRUPT_DRIVEN=y`

#![no_std]

use zenoh_nostd::platform::*;
use zenoh_platform_posix::tcp::{PosixTcpLink, PosixTcpTx, PosixTcpRx};
use zenoh_platform_posix::udp::{PosixUdpLink, PosixUdpTx, PosixUdpRx};

mod serial;

pub struct ZephyrLinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (ZephyrLinkTx, ZephyrLinkRx))]
pub enum ZephyrLink {
    Tcp(PosixTcpLink),
    Udp(PosixUdpLink),
    Serial(serial::PlatformSerialLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum ZephyrLinkTx {
    Tcp(PosixTcpTx),
    Udp(PosixUdpTx),
    Serial(serial::PlatformSerialTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum ZephyrLinkRx {
    Tcp(PosixTcpRx),
    Udp(PosixUdpRx),
    Serial(serial::PlatformSerialRx),
}

impl ZLinkManager for ZephyrLinkManager {
    type Link<'a>
        = ZephyrLink
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
                // TODO: Implement UDP bind
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