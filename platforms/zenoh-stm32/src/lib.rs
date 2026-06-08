//! Zenoh platform support for STM32 (LwIP / HAL Ethernet)
//!
//! Provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations via
//! POSIX socket FFI (LwIP on STM32).
//!
//! # Requirements
//!
//! - STM32 HAL with Ethernet enabled (LwIP stack)
//! - POSIX socket API via LwIP
//! - For serial: STM32 HAL UART peripheral configured

#![no_std]

use zenoh_nostd::platform::*;

mod serial;
mod sockets;
mod tcp;
mod udp;

pub struct Stm32LinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (Stm32LinkTx, Stm32LinkRx))]
pub enum Stm32Link {
    Tcp(tcp::PlatformTcpLink),
    Udp(udp::PlatformUdpLink),
    Serial(serial::PlatformSerialLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum Stm32LinkTx {
    Tcp(tcp::PlatformTcpTx),
    Udp(udp::PlatformUdpTx),
    Serial(serial::PlatformSerialTx),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum Stm32LinkRx {
    Tcp(tcp::PlatformTcpRx),
    Udp(udp::PlatformUdpRx),
    Serial(serial::PlatformSerialRx),
}

impl ZLinkManager for Stm32LinkManager {
    type Link<'a>
        = Stm32Link
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
            "serial" => Ok(Self::Link::Serial(serial::PlatformSerialLink)),
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
            "serial" => Ok(Self::Link::Serial(serial::PlatformSerialLink)),
            _ => zenoh::zbail!(LinkError::CouldNotParseProtocol),
        }
    }
}