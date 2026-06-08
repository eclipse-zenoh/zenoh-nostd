//! Zenoh platform support for Arduino framework (ESP32, etc.)
//!
//! Provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations via
//! ESP-IDF's LWIP socket API (standard BSD sockets on ESP32).
//!
//! On ESP32 Arduino, the ESP-IDF provides a POSIX-compatible socket API
//! via LWIP. This module uses FFI to access those C functions.
//!
//! # Requirements
//!
//! - ESP32 Arduino core with LWIP enabled (default)
//! - For ESP8266: different WiFi library, but same socket pattern

#![no_std]

use zenoh_nostd::platform::*;
use zenoh_platform_posix::tcp::{PosixTcpLink, PosixTcpTx, PosixTcpRx};
use zenoh_platform_posix::udp::{PosixUdpLink, PosixUdpTx, PosixUdpRx};

mod serial;

pub struct ArduinoLinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (ArduinoLinkTx, ArduinoLinkRx))]
pub enum ArduinoLink {
    Tcp(PosixTcpLink),
    Udp(PosixUdpLink),
    Serial(serial::ArduinoSerialLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum ArduinoLinkTx {
    Tcp(PosixTcpTx),
    Udp(PosixUdpTx),
    Serial(serial::ArduinoSerialTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum ArduinoLinkRx {
    Tcp(PosixTcpRx),
    Udp(PosixUdpRx),
    Serial(serial::ArduinoSerialRx),
}

impl ZLinkManager for ArduinoLinkManager {
    type Link<'a>
        = ArduinoLink
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
                Ok(Self::Link::Serial(serial::ArduinoSerialLink::new(path, baud)?))
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
            "tcp" => Ok(Self::Link::Tcp(PosixTcpLink::listen(address.as_str())?)),
            "udp" => {
                let _ = address;
                Err(LinkError::CouldNotListen)
            }
            "serial" => {
                let path = address.as_str();
                let baud = 115200u32;
                Ok(Self::Link::Serial(serial::ArduinoSerialLink::new(path, baud)?))
            }
            _ => Err(LinkError::CouldNotParseProtocol),
        }
    }
}