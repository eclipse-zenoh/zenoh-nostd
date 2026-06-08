//! Zenoh platform support for FreeRTOS+TCP
//!
//! Provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations via
//! FreeRTOS+TCP's BSD socket API (`FreeRTOS_socket.h`).
//!
//! FreeRTOS+TCP provides a BSD sockets-compatible API with `FreeRTOS_`-prefixed
//! function names (e.g., `FreeRTOS_socket`, `FreeRTOS_send`, `FreeRTOS_recv`).
//!
//! # Requirements
//!
//! - FreeRTOS+TCP component enabled in FreeRTOSConfig.h
//! - `ipconfigUSE_TCP=y` for TCP support
//! - `ipconfigUSE_UDP=y` for UDP support

#![no_std]

use zenoh_nostd::platform::*;
use zenoh_platform_posix::tcp::{PosixTcpLink, PosixTcpTx, PosixTcpRx};
use zenoh_platform_posix::udp::{PosixUdpLink, PosixUdpTx, PosixUdpRx};

mod serial;

pub struct FreeRtosLinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (FreeRtosLinkTx, FreeRtosLinkRx))]
pub enum FreeRtosLink {
    Tcp(PosixTcpLink),
    Udp(PosixUdpLink),
    Serial(serial::PlatformSerialLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum FreeRtosLinkTx {
    Tcp(PosixTcpTx),
    Udp(PosixUdpTx),
    Serial(serial::PlatformSerialTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum FreeRtosLinkRx {
    Tcp(PosixTcpRx),
    Udp(PosixUdpRx),
    Serial(serial::PlatformSerialRx),
}

impl ZLinkManager for FreeRtosLinkManager {
    type Link<'a>
        = FreeRtosLink
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