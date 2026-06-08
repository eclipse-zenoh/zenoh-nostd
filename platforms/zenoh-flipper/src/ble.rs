//! BLE (Bluetooth Low Energy) link for Flipper Zero
//!
//! The Flipper Zero uses an STM32WB55 with BLE capability. The primary
//! communication for Zenoh on Flipper Zero is expected to be BLE GATT.
//!
//! Flipper SDK provides:
//! - `ble_app_init()` — initialize BLE stack
//! - `ble_app_send()` — send data over BLE
//! - `ble_app_get_data()` — receive data from BLE
//!
//! These are C functions accessed via FFI. For now, this is a stub.

#![allow(non_camel_case_types)]

use zenoh_nostd::platform::*;

pub struct FlipperBleLink {
    mtu: u16,
}

impl FlipperBleLink {
    pub fn new() -> Result<Self, LinkError> {
        // TODO: Call ble_app_init() via FFI
        // Requires Flipper SDK headers and linking
        Ok(Self { mtu: 128 })
    }
}

impl ZLinkInfo for FlipperBleLink {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for FlipperBleLink {
    fn write_all(&mut self, _buffer: &[u8]) -> Result<(), LinkError> {
        // TODO: Implement ble_app_send(buffer, len)
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for FlipperBleLink {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, LinkError> {
        // TODO: Implement ble_app_get_data(buffer, len)
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for FlipperBleLink {
    type Tx<'a> = FlipperBleTx where Self: 'a;
    type Rx<'a> = FlipperBleRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        let mtu = self.mtu;
        (FlipperBleTx { mtu }, FlipperBleRx { mtu })
    }
}

pub struct FlipperBleTx { mtu: u16 }
pub struct FlipperBleRx { mtu: u16 }

impl ZLinkInfo for FlipperBleTx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for FlipperBleTx {
    fn write_all(&mut self, _buffer: &[u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkInfo for FlipperBleRx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkRx for FlipperBleRx {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}