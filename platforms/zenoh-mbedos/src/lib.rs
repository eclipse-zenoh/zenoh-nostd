//! Zenoh platform support for Mbed OS
//!
//! Provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations via
//! Mbed OS POSIX socket API (`mbed-os/rtos` and `mbed-os/netsocket`).
//!
//! # Requirements
//!
//! - Mbed OS with `rtos` and `netsocket` libraries
//! - Ethernet or WiFi interface enabled in `mbed_app.json`
//! - For serial: Mbed OS `Serial` or `UnbufferedSerial` driver

#![no_std]

use zenoh_nostd::platform::*;

mod serial;
mod sockets;
mod tcp;
mod udp;

pub struct MbedLinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (MbedLinkTx, MbedLinkRx))]
pub enum MbedLink {
    Tcp(tcp::PlatformTcpLink),
    Udp(udp::PlatformUdpLink),
    Serial(serial::PlatformSerialLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum MbedLinkTx {
    Tcp(tcp::PlatformTcpTx),
    Udp(udp::PlatformUdpTx),
    Serial(serial::PlatformSerialTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum MbedLinkRx {
    Tcp(tcp::PlatformTcpRx),
    Udp(udp::PlatformUdpRx),
    Serial(serial::PlatformSerialRx),
}

impl ZLinkManager for MbedLinkManager {
    type Link<'a>
        = MbedLink
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