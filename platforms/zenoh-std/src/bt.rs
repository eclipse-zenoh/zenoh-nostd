// Bluetooth link stub.
//
// To enable Bluetooth support:
// 1. Add `btleplug` or platform-specific Bluetooth library to zenoh-std dependencies
// 2. Implement ZLink/ZLinkTx/ZLinkRx for the Bluetooth connection
// 3. Add "bt" protocol to StdLinkManager::connect/listen
//
// Usage: connect("bt/xx:xx:xx:xx:xx:xx")
use zenoh_nostd::platform::*;

pub struct BtStream;

impl ZLinkInfo for BtStream {
    fn mtu(&self) -> u16 { u16::MAX }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for BtStream {
    async fn write_all(&mut self, _buffer: &[u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for BtStream {
    async fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, LinkError> {
        Err(LinkError::LinkRxFailed)
    }
    async fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for BtStream {
    type Tx<'a> = Self;
    type Rx<'a> = Self;
    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) { unimplemented!() }
}
