//! Zenoh platform support for Zephyr RTOS
//!
//! This crate provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations
//! for Zephyr RTOS.
//!
//! NOTE: This is a stub. Full implementation requires Zephyr's native networking
//! APIs via FFI. Zephyr uses CMake-based builds; this crate must be integrated
//! into a Zephyr application's CMakeLists.txt for native networking support.

#![no_std]

use zenoh_nostd::platform::*;

pub struct ZephyrLinkManager;

impl ZLinkManager for ZephyrLinkManager {
    type Link<'a>
        = ZephyrLink
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

pub struct ZephyrLink;

impl ZLinkInfo for ZephyrLink {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for ZephyrLink {
    fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for ZephyrLink {
    fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for ZephyrLink {
    type Tx<'a> = ZephyrLinkTx where Self: 'a;
    type Rx<'a> = ZephyrLinkRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (ZephyrLinkTx, ZephyrLinkRx)
    }
}

pub struct ZephyrLinkTx;
pub struct ZephyrLinkRx;

impl ZLinkInfo for ZephyrLinkTx {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for ZephyrLinkTx {
    fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkInfo for ZephyrLinkRx {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkRx for ZephyrLinkRx {
    fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}