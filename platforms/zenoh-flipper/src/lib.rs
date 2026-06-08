//! Zenoh platform support for Flipper Zero
//!
//! This crate provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations
//! for the Flipper Zero.
//!
//! NOTE: This is a stub. Full implementation requires Flipper Zero's custom
//! FreeRTOS fork and its native APIs (furi, flipper_hal) via FFI. The Flipper
//! Zero uses a STM32WB55 with BLE and infrared; networking is typically via
//! BLE serial or WiFi dev board (ESP32).

#![no_std]

use zenoh_nostd::platform::*;

pub struct FlipperLinkManager;

impl ZLinkManager for FlipperLinkManager {
    type Link<'a>
        = FlipperLink
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

pub struct FlipperLink;

impl ZLinkInfo for FlipperLink {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for FlipperLink {
    fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for FlipperLink {
    fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for FlipperLink {
    type Tx<'a> = FlipperLinkTx where Self: 'a;
    type Rx<'a> = FlipperLinkRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (FlipperLinkTx, FlipperLinkRx)
    }
}

pub struct FlipperLinkTx;
pub struct FlipperLinkRx;

impl ZLinkInfo for FlipperLinkTx {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for FlipperLinkTx {
    fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkInfo for FlipperLinkRx {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkRx for FlipperLinkRx {
    fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}