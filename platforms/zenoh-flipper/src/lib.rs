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

mod ble;
mod serial;
mod sockets;
mod tcp;
mod udp;

pub struct FlipperLinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (FlipperLinkTx, FlipperLinkRx))]
pub enum FlipperLink {
    Ble(ble::FlipperBleLink),
    Tcp(tcp::FlipperTcpLink),
    Udp(udp::FlipperUdpLink),
    Serial(serial::FlipperSerialLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum FlipperLinkTx {
    Ble(ble::FlipperBleTx),
    Tcp(tcp::FlipperTcpTx),
    Udp(udp::FlipperUdpTx),
    Serial(serial::FlipperSerialTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum FlipperLinkRx {
    Ble(ble::FlipperBleRx),
    Tcp(tcp::FlipperTcpRx),
    Udp(udp::FlipperUdpRx),
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
            "tcp" => Ok(Self::Link::Tcp(tcp::FlipperTcpLink::connect(address.as_str())?)),
            "udp" => Ok(Self::Link::Udp(udp::FlipperUdpLink::connect(address.as_str())?)),
            "serial" => Ok(Self::Link::Serial(serial::FlipperSerialLink)),
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
            "tcp" => Ok(Self::Link::Tcp(tcp::FlipperTcpLink::listen(address.as_str())?)),
            "udp" => {
                let _ = address;
                Err(LinkError::CouldNotListen)
            }
            "serial" => Ok(Self::Link::Serial(serial::FlipperSerialLink)),
            "ble" => Ok(Self::Link::Ble(ble::FlipperBleLink::new()?)),
            _ => Err(LinkError::CouldNotParseProtocol),
        }
    }
}