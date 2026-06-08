//! Zenoh platform support for Raspberry Pi Pico (RP2040/RP2350)
//!
//! Provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations via
//! Pico SDK's LWIP socket API (standard BSD sockets on RP2040).
//!
//! The Raspberry Pi Pico W uses the CYW43439 WiFi chip (cyw43 driver)
//! with LWIP for TCP/IP networking. This provides a POSIX-compatible
//! socket API via FFI.
//!
//! # Pico SDK Requirements (CMakeLists.txt / pico_sdk_import.cmake)
//!
//! - `pico_cyw43_arch_lwip_threadsafe_backoff` or `pico_cyw43_arch_lwip_sys_freertos`
//! - `lwip` (included with Pico SDK)
//! - `pico_stdlib`

#![no_std]

use zenoh_nostd::platform::*;

mod serial;
mod sockets;
mod tcp;
mod udp;

pub struct Rp2040LinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (Rp2040LinkTx, Rp2040LinkRx))]
pub enum Rp2040Link {
    Tcp(tcp::Rp2040TcpLink),
    Udp(udp::Rp2040UdpLink),
    Serial(serial::Rp2040SerialLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum Rp2040LinkTx {
    Tcp(tcp::Rp2040TcpTx),
    Udp(udp::Rp2040UdpTx),
    Serial(serial::Rp2040SerialTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum Rp2040LinkRx {
    Tcp(tcp::Rp2040TcpRx),
    Udp(udp::Rp2040UdpRx),
    Serial(serial::Rp2040SerialRx),
}

impl ZLinkManager for Rp2040LinkManager {
    type Link<'a>
        = Rp2040Link
    where
        Self: 'a;

    fn connect(
        &self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<Self::Link<'_>, LinkError> {
        let protocol = endpoint.protocol();
        let address = endpoint.address();

        match protocol.as_str() {
            "tcp" => Ok(Self::Link::Tcp(tcp::Rp2040TcpLink::connect(address.as_str())?)),
            "udp" => Ok(Self::Link::Udp(udp::Rp2040UdpLink::connect(address.as_str())?)),
            "serial" => {
                let path = address.as_str();
                let baud = 115200u32;
                Ok(Self::Link::Serial(serial::Rp2040SerialLink::new(path, baud)?))
            }
            _ => Err(LinkError::CouldNotParseProtocol),
        }
    }

    fn listen(
        &self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<Self::Link<'_>, LinkError> {
        let protocol = endpoint.protocol();
        let address = endpoint.address();

        match protocol.as_str() {
            "tcp" => Ok(Self::Link::Tcp(tcp::Rp2040TcpLink::listen(address.as_str())?)),
            "udp" => {
                let _ = address;
                Err(LinkError::CouldNotListen)
            }
            "serial" => {
                let path = address.as_str();
                let baud = 115200u32;
                Ok(Self::Link::Serial(serial::Rp2040SerialLink::new(path, baud)?))
            }
            _ => Err(LinkError::CouldNotParseProtocol),
        }
    }
}