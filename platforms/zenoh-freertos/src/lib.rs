//! Zenoh platform support for FreeRTOS+TCP
//!
//! This crate provides ZLinkManager, ZLink, ZLinkTx, ZLinkRx implementations
//! for FreeRTOS+TCP.
//!
//! NOTE: This is a stub. Full implementation requires FreeRTOS+TCP socket APIs
//! via FFI (FreeRTOS_send, FreeRTOS_recv, etc.).

#![no_std]

use zenoh_nostd::platform::*;

pub struct FreeRtosLinkManager;

impl ZLinkManager for FreeRtosLinkManager {
    type Link<'a>
        = FreeRtosLink
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

pub struct FreeRtosLink;

impl ZLinkInfo for FreeRtosLink {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for FreeRtosLink {
    fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for FreeRtosLink {
    fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for FreeRtosLink {
    type Tx<'a> = FreeRtosLinkTx where Self: 'a;
    type Rx<'a> = FreeRtosLinkRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (FreeRtosLinkTx, FreeRtosLinkRx)
    }
}

pub struct FreeRtosLinkTx;
pub struct FreeRtosLinkRx;

impl ZLinkInfo for FreeRtosLinkTx {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for FreeRtosLinkTx {
    fn write_all(&mut self, _buffer: &[u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkInfo for FreeRtosLinkRx {
    fn mtu(&self) -> u16 { 0 }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkRx for FreeRtosLinkRx {
    fn read(&mut self, _buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}