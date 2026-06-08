//! Zenoh platform support for OpenCR board
//!
//! This crate provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations
//! for the OpenCR board (STM32F7-based, used with ROS/ROS2).
//!
//! NOTE: This is a stub. Full implementation requires OpenCR's networking
//! capabilities via its STM32 HAL bindings (UART, USB CDC, or Ethernet).

#![no_std]

use zenoh_nostd::platform::*;

pub struct OpencrLinkManager;

impl ZLinkManager for OpencrLinkManager {
    type Link<'a>
        = OpencrLink
    where
        Self: 'a;

    fn connect(
        &self,
        _endpoint: Endpoint<'_>,
    ) -> core::result::Result<Self::Link<'_>, LinkError> {
        Err(LinkError::CouldNotConnect)
    }

    fn listen(
        &self,
        _endpoint: Endpoint<'_>,
    ) -> core::result::Result<Self::Link<'_>, LinkError> {
        Err(LinkError::CouldNotConnect)
    }
}

pub struct OpencrLink;

impl ZLinkInfo for OpencrLink {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for OpencrLink {
    fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for OpencrLink {
    fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for OpencrLink {
    type Tx<'a> = OpencrLinkTx where Self: 'a;
    type Rx<'a> = OpencrLinkRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (OpencrLinkTx, OpencrLinkRx)
    }
}

pub struct OpencrLinkTx;
pub struct OpencrLinkRx;

impl ZLinkInfo for OpencrLinkTx {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for OpencrLinkTx {
    fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkInfo for OpencrLinkRx {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkRx for OpencrLinkRx {
    fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}