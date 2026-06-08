//! Serial (UART) link for Zephyr RTOS
//!
//! Full implementation requires Zephyr's UART API via FFI:
//! - `uart_irq_rx_enable`, `uart_irq_tx_enable`, `uart_fifo_read`, `uart_fifo_fill`
//! - Requires `CONFIG_SERIAL` and `CONFIG_UART_INTERRUPT_DRIVEN` Kconfig options.
//!
//! For now, this is a stub that returns errors.

use zenoh_nostd::platform::*;

pub struct PlatformSerialLink;

impl ZLinkInfo for PlatformSerialLink {
    fn mtu(&self) -> u16 { 256 }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for PlatformSerialLink {
    fn write_all(&mut self, _buffer: &[u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for PlatformSerialLink {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for PlatformSerialLink {
    type Tx<'a> = PlatformSerialTx where Self: 'a;
    type Rx<'a> = PlatformSerialRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (PlatformSerialTx, PlatformSerialRx)
    }
}

pub struct PlatformSerialTx;
pub struct PlatformSerialRx;

impl ZLinkInfo for PlatformSerialTx {
    fn mtu(&self) -> u16 { 256 }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for PlatformSerialTx {
    fn write_all(&mut self, _buffer: &[u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkInfo for PlatformSerialRx {
    fn mtu(&self) -> u16 { 256 }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkRx for PlatformSerialRx {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}