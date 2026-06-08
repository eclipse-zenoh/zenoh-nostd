//! Zenoh platform support for Raspberry Pi Pico (RP2040/RP2350)
//!
//! This crate provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations
//! for the Raspberry Pi Pico family (RP2040, RP2350).
//!
//! NOTE: This is a stub. Full implementation requires the `rp2040-hal` or
//! `rp235x-hal` networking abstractions (e.g., `embassy-net` with `cyw43`
//! WiFi driver for Pico W, or `embassy-net` with `smoltcp` for wired).

#![no_std]

use zenoh_nostd::platform::*;

pub struct Rp2040LinkManager;

impl ZLinkManager for Rp2040LinkManager {
    type Link<'a>
        = Rp2040Link
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

pub struct Rp2040Link;

impl ZLinkInfo for Rp2040Link {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for Rp2040Link {
    fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for Rp2040Link {
    fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for Rp2040Link {
    type Tx<'a> = Rp2040LinkTx where Self: 'a;
    type Rx<'a> = Rp2040LinkRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (Rp2040LinkTx, Rp2040LinkRx)
    }
}

pub struct Rp2040LinkTx;
pub struct Rp2040LinkRx;

impl ZLinkInfo for Rp2040LinkTx {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for Rp2040LinkTx {
    fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkInfo for Rp2040LinkRx {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkRx for Rp2040LinkRx {
    fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}