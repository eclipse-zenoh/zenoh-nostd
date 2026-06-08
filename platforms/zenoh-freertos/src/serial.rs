use zenoh_nostd::platform::*;

unsafe extern "C" {
    fn uart_init(uart_id: u32, baud_rate: u32) -> u32;
    fn uart_write_byte(uart_id: u32, c: u8) -> i32;
    fn uart_read_byte(uart_id: u32, c: *mut u8, timeout_ms: u32) -> i32;
    fn uart_deinit(uart_id: u32);
}

pub struct PlatformSerialLink {
    uart_id: u32,
    mtu: u16,
}

impl PlatformSerialLink {
    pub fn new(path: &str, baud_rate: u32) -> Result<Self, LinkError> {
        let uart_id = path.parse::<u32>().map_err(|_| LinkError::CouldNotConnect)?;
        let result = unsafe { uart_init(uart_id, baud_rate) };
        if result == 0 {
            return Err(LinkError::CouldNotConnect);
        }
        Ok(Self { uart_id, mtu: 256 })
    }
}

impl ZLinkInfo for PlatformSerialLink {
    fn mtu(&self) -> u16 {
        self.mtu
    }
    fn is_streamed(&self) -> bool {
        true
    }
}

pub struct PlatformSerialTx(u32, u16);
pub struct PlatformSerialRx(u32, u16);

impl ZLinkInfo for PlatformSerialTx {
    fn mtu(&self) -> u16 {
        self.1
    }
    fn is_streamed(&self) -> bool {
        true
    }
}

impl ZLinkTx for PlatformSerialTx {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        for &c in buffer {
            let ret = unsafe { uart_write_byte(self.0, c) };
            if ret < 0 {
                return Err(LinkError::LinkTxFailed);
            }
        }
        Ok(())
    }
}

impl ZLinkInfo for PlatformSerialRx {
    fn mtu(&self) -> u16 {
        self.1
    }
    fn is_streamed(&self) -> bool {
        true
    }
}

impl ZLinkRx for PlatformSerialRx {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let mut count = 0usize;
        for byte in buffer.iter_mut() {
            let ret = unsafe { uart_read_byte(self.0, byte as *mut u8, 10) };
            if ret < 0 {
                if count == 0 {
                    return Err(LinkError::LinkRxFailed);
                }
                break;
            }
            count += 1;
        }
        Ok(count)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        for byte in buffer.iter_mut() {
            loop {
                let ret = unsafe { uart_read_byte(self.0, byte as *mut u8, 1000) };
                if ret >= 0 {
                    break;
                }
                if ret < -1 {
                    return Err(LinkError::LinkRxFailed);
                }
            }
        }
        Ok(())
    }
}

impl ZLinkTx for PlatformSerialLink {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        for &c in buffer {
            let ret = unsafe { uart_write_byte(self.uart_id, c) };
            if ret < 0 {
                return Err(LinkError::LinkTxFailed);
            }
        }
        Ok(())
    }
}

impl ZLinkRx for PlatformSerialLink {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let mut count = 0usize;
        for byte in buffer.iter_mut() {
            let ret = unsafe { uart_read_byte(self.uart_id, byte as *mut u8, 10) };
            if ret < 0 {
                if count == 0 {
                    return Err(LinkError::LinkRxFailed);
                }
                break;
            }
            count += 1;
        }
        Ok(count)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        for byte in buffer.iter_mut() {
            loop {
                let ret = unsafe { uart_read_byte(self.uart_id, byte as *mut u8, 1000) };
                if ret >= 0 {
                    break;
                }
                if ret < -1 {
                    return Err(LinkError::LinkRxFailed);
                }
            }
        }
        Ok(())
    }
}

impl ZLink for PlatformSerialLink {
    type Tx<'a> = PlatformSerialTx where Self: 'a;
    type Rx<'a> = PlatformSerialRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (PlatformSerialTx(self.uart_id, self.mtu), PlatformSerialRx(self.uart_id, self.mtu))
    }
}