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

mod serial;
mod sockets;
mod tcp;
mod udp;

pub struct ArduinoLinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (ArduinoLinkTx, ArduinoLinkRx))]
pub enum ArduinoLink {
    Tcp(tcp::ArduinoTcpLink),
    Udp(udp::ArduinoUdpLink),
    Serial(serial::ArduinoSerialLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum ArduinoLinkTx {
    Tcp(tcp::ArduinoTcpTx),
    Udp(udp::ArduinoUdpTx),
    Serial(serial::ArduinoSerialTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum ArduinoLinkRx {
    Tcp(tcp::ArduinoTcpRx),
    Udp(udp::ArduinoUdpRx),
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
            "tcp" => Ok(Self::Link::Tcp(tcp::ArduinoTcpLink::connect(address.as_str())?)),
            "udp" => Ok(Self::Link::Udp(udp::ArduinoUdpLink::connect(address.as_str())?)),
            "serial" => Ok(Self::Link::Serial(serial::ArduinoSerialLink)),
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
            "tcp" => Ok(Self::Link::Tcp(tcp::ArduinoTcpLink::listen(address.as_str())?)),
            "udp" => {
                let _ = address;
                Err(LinkError::CouldNotListen)
            }
            "serial" => Ok(Self::Link::Serial(serial::ArduinoSerialLink)),
            _ => Err(LinkError::CouldNotParseProtocol),
        }
    }
}