//! Serial (UART) link for RP2040
//!
//! Full implementation requires Pico SDK's UART API via FFI:
//! - uart_init(), uart_putc_raw(), uart_getc(), uart_is_readable()
//! - Requires pico_cyw43_arch and hardware_uart
//!
//! For now, this is a stub that returns errors.

use zenoh_nostd::platform::*;

pub struct Rp2040SerialLink;

impl ZLinkInfo for Rp2040SerialLink {
    fn mtu(&self) -> u16 { 256 }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for Rp2040SerialLink {
    fn write_all(&mut self, _buffer: &[u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for Rp2040SerialLink {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for Rp2040SerialLink {
    type Tx<'a> = Rp2040SerialTx where Self: 'a;
    type Rx<'a> = Rp2040SerialRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (Rp2040SerialTx, Rp2040SerialRx)
    }
}

pub struct Rp2040SerialTx;
pub struct Rp2040SerialRx;

impl ZLinkInfo for Rp2040SerialTx {
    fn mtu(&self) -> u16 { 256 }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for Rp2040SerialTx {
    fn write_all(&mut self, _buffer: &[u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkInfo for Rp2040SerialRx {
    fn mtu(&self) -> u16 { 256 }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkRx for Rp2040SerialRx {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}