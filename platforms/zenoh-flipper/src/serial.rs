//! Serial (UART) link for Flipper Zero
//!
//! Full implementation requires Flipper SDK's UART API via FFI:
//! - furi_hal_uart_init(), furi_hal_uart_tx(), furi_hal_uart_rx()
//!
//! For now, this is a stub that returns errors.

use zenoh_nostd::platform::*;

pub struct FlipperSerialLink;

impl ZLinkInfo for FlipperSerialLink {
    fn mtu(&self) -> u16 { 256 }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for FlipperSerialLink {
    fn write_all(&mut self, _buffer: &[u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for FlipperSerialLink {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for FlipperSerialLink {
    type Tx<'a> = FlipperSerialTx where Self: 'a;
    type Rx<'a> = FlipperSerialRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (FlipperSerialTx, FlipperSerialRx)
    }
}

pub struct FlipperSerialTx;
pub struct FlipperSerialRx;

impl ZLinkInfo for FlipperSerialTx {
    fn mtu(&self) -> u16 { 256 }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for FlipperSerialTx {
    fn write_all(&mut self, _buffer: &[u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkInfo for FlipperSerialRx {
    fn mtu(&self) -> u16 { 256 }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkRx for FlipperSerialRx {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}