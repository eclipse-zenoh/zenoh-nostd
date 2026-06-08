//! Zenoh platform support for Mbed OS
//!
//! This crate provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations
//! for Mbed OS.
//!
//! NOTE: This is a stub. Full implementation requires Mbed OS networking APIs
//! via FFI (TCPSocket, UDPSocket, etc.).

#![no_std]

use zenoh_nostd::platform::*;

pub struct MbedLinkManager;

impl ZLinkManager for MbedLinkManager {
    type Link<'a>
        = MbedLink
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

pub struct MbedLink;

impl ZLinkInfo for MbedLink {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for MbedLink {
    fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for MbedLink {
    fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for MbedLink {
    type Tx<'a> = MbedLinkTx where Self: 'a;
    type Rx<'a> = MbedLinkRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (MbedLinkTx, MbedLinkRx)
    }
}

pub struct MbedLinkTx;
pub struct MbedLinkRx;

impl ZLinkInfo for MbedLinkTx {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for MbedLinkTx {
    fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkInfo for MbedLinkRx {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkRx for MbedLinkRx {
    fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}