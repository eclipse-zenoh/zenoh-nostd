//! Zenoh platform support for Flipper Zero
//!
//! Provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations for
//! the Flipper Zero.
//!
//! The Flipper Zero runs a custom FreeRTOS fork (FuriOS) on an STM32WB55.
//! Communication options:
//!
//! - **BLE GATT** (built-in STM32WB55 BLE) — primary channel
//! - **TCP/UDP over WiFi** (optional ESP32 dev board via FreeRTOS+POSIX sockets)
//! - **Serial** (UART via USB or GPIO)
//!
//! # Requirements
//!
//! - Flipper Zero SDK / FuriOS with BLE stack
//! - For WiFi: FreeRTOS+POSIX socket support on the ESP32 dev board

#![no_std]

use zenoh_nostd::platform::*;
use zenoh_platform_posix::tcp::{PosixTcpLink, PosixTcpTx, PosixTcpRx};
use zenoh_platform_posix::udp::{PosixUdpLink, PosixUdpTx, PosixUdpRx};

mod ble;
mod serial;

pub struct FlipperLinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (FlipperLinkTx, FlipperLinkRx))]
pub enum FlipperLink {
    Ble(ble::FlipperBleLink),
    Tcp(PosixTcpLink),
    Udp(PosixUdpLink),
    Serial(serial::FlipperSerialLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum FlipperLinkTx {
    Ble(ble::FlipperBleTx),
    Tcp(PosixTcpTx),
    Udp(PosixUdpTx),
    Serial(serial::FlipperSerialTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum FlipperLinkRx {
    Ble(ble::FlipperBleRx),
    Tcp(PosixTcpRx),
    Udp(PosixUdpRx),
    Serial(serial::FlipperSerialRx),
}

impl ZLinkManager for FlipperLinkManager {
    type Link<'a>
        = FlipperLink
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
                Ok(Self::Link::Serial(serial::FlipperSerialLink::new(path, baud)?))
            }
            "ble" => Ok(Self::Link::Ble(ble::FlipperBleLink::new()?)),
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
                Ok(Self::Link::Serial(serial::FlipperSerialLink::new(path, baud)?))
            }
            "ble" => Ok(Self::Link::Ble(ble::FlipperBleLink::new()?)),
            _ => Err(LinkError::CouldNotParseProtocol),
        }
    }
}