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
use zenoh_platform_posix::tcp::{PosixTcpLink, PosixTcpTx, PosixTcpRx};
use zenoh_platform_posix::udp::{PosixUdpLink, PosixUdpTx, PosixUdpRx};

mod serial;

pub struct MbedLinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (MbedLinkTx, MbedLinkRx))]
pub enum MbedLink {
    Tcp(PosixTcpLink),
    Udp(PosixUdpLink),
    Serial(serial::PlatformSerialLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum MbedLinkTx {
    Tcp(PosixTcpTx),
    Udp(PosixUdpTx),
    Serial(serial::PlatformSerialTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum MbedLinkRx {
    Tcp(PosixTcpRx),
    Udp(PosixUdpRx),
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