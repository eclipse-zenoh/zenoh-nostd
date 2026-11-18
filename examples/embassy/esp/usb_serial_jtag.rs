//! Polling USB RX keeps incoming terminal bytes independent of TX interrupts.
use esp_hal::{Async, usb_serial_jtag::UsbSerialJtagRx};
use zenoh_examples_common::cli::ConsoleRead;

pub struct UsbRead(pub UsbSerialJtagRx<'static, Async>);
impl ConsoleRead for UsbRead {
    async fn read(&mut self, bytes: &mut [u8]) -> Result<usize, ()> {
        if bytes.is_empty() {
            return Ok(0);
        }
        loop {
            let n = self.0.drain_rx_fifo(bytes);
            if n != 0 {
                return Ok(n);
            }
            embassy_time::Timer::after_millis(1).await;
        }
    }
}
