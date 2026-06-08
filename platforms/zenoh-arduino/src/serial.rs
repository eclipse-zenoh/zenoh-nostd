//! Serial (UART) link for Arduino framework
//!
//! Full implementation requires Arduino's HardwareSerial API via FFI:
//! - Serial.begin(), Serial.write(), Serial.read(), Serial.available()
//! - Different boards use different UART peripherals
//!
//! For now, this is a stub that returns errors.

use zenoh_nostd::platform::*;

pub struct ArduinoSerialLink;

impl ZLinkInfo for ArduinoSerialLink {
    fn mtu(&self) -> u16 { 256 }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for ArduinoSerialLink {
    fn write_all(&mut self, _buffer: &[u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for ArduinoSerialLink {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for ArduinoSerialLink {
    type Tx<'a> = ArduinoSerialTx where Self: 'a;
    type Rx<'a> = ArduinoSerialRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (ArduinoSerialTx, ArduinoSerialRx)
    }
}

pub struct ArduinoSerialTx;
pub struct ArduinoSerialRx;

impl ZLinkInfo for ArduinoSerialTx {
    fn mtu(&self) -> u16 { 256 }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for ArduinoSerialTx {
    fn write_all(&mut self, _buffer: &[u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkInfo for ArduinoSerialRx {
    fn mtu(&self) -> u16 { 256 }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkRx for ArduinoSerialRx {
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}