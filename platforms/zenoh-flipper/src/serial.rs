use zenoh_nostd::platform::*;

unsafe extern "C" {
    fn furi_hal_uart_init(channel: u32, baud_rate: u32);
    fn furi_hal_uart_deinit(channel: u32);
    fn furi_hal_uart_tx(channel: u32, data: *const u8, len: u16) -> i32;
    fn furi_hal_uart_rx(channel: u32, data: *mut u8, len: u16, timeout_ms: u32) -> i32;
}

pub struct FlipperSerialLink {
    channel: u32,
    mtu: u16,
}

impl Drop for FlipperSerialLink {
    fn drop(&mut self) {
        unsafe { furi_hal_uart_deinit(self.channel) };
    }
}

impl FlipperSerialLink {
    pub fn new(path: &str, baud_rate: u32) -> Result<Self, LinkError> {
        let channel = match path {
            "usart" | "lp" | "LPUART" => 0u32,
            "gpio" | "uart" | "UART" => 1u32,
            _ => return Err(LinkError::CouldNotConnect),
        };

        unsafe { furi_hal_uart_init(channel, baud_rate) };

        Ok(Self { channel, mtu: 256 })
    }
}

impl ZLinkInfo for FlipperSerialLink {
    fn mtu(&self) -> u16 {
        self.mtu
    }
    fn is_streamed(&self) -> bool {
        true
    }
}

pub struct FlipperSerialTx(u32, u16);
pub struct FlipperSerialRx(u32, u16);

impl ZLinkInfo for FlipperSerialTx {
    fn mtu(&self) -> u16 {
        self.1
    }
    fn is_streamed(&self) -> bool {
        true
    }
}

impl ZLinkTx for FlipperSerialTx {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let ret = unsafe { furi_hal_uart_tx(self.0, buffer.as_ptr(), buffer.len() as u16) };
        if ret < 0 {
            return Err(LinkError::LinkTxFailed);
        }
        Ok(())
    }
}

impl ZLinkInfo for FlipperSerialRx {
    fn mtu(&self) -> u16 {
        self.1
    }
    fn is_streamed(&self) -> bool {
        true
    }
}

impl ZLinkRx for FlipperSerialRx {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let ret = unsafe { furi_hal_uart_rx(self.0, buffer.as_mut_ptr(), buffer.len() as u16, 100) };
        if ret < 0 {
            return Err(LinkError::LinkRxFailed);
        }
        Ok(ret as usize)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let remaining = (buffer.len() - offset) as u16;
            let ret = unsafe { furi_hal_uart_rx(self.0, buffer[offset..].as_mut_ptr(), remaining, 1000) };
            if ret < 0 {
                return Err(LinkError::LinkRxFailed);
            }
            if ret == 0 {
                continue;
            }
            offset += ret as usize;
        }
        Ok(())
    }
}

impl ZLinkTx for FlipperSerialLink {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let ret = unsafe { furi_hal_uart_tx(self.channel, buffer.as_ptr(), buffer.len() as u16) };
        if ret < 0 {
            return Err(LinkError::LinkTxFailed);
        }
        Ok(())
    }
}

impl ZLinkRx for FlipperSerialLink {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let ret = unsafe { furi_hal_uart_rx(self.channel, buffer.as_mut_ptr(), buffer.len() as u16, 100) };
        if ret < 0 {
            return Err(LinkError::LinkRxFailed);
        }
        Ok(ret as usize)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let remaining = (buffer.len() - offset) as u16;
            let ret = unsafe { furi_hal_uart_rx(self.channel, buffer[offset..].as_mut_ptr(), remaining, 1000) };
            if ret < 0 {
                return Err(LinkError::LinkRxFailed);
            }
            if ret == 0 {
                continue;
            }
            offset += ret as usize;
        }
        Ok(())
    }
}

impl ZLink for FlipperSerialLink {
    type Tx<'a> = FlipperSerialTx where Self: 'a;
    type Rx<'a> = FlipperSerialRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (FlipperSerialTx(self.channel, self.mtu), FlipperSerialRx(self.channel, self.mtu))
    }
}