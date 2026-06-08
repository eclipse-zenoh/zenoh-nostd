use zenoh_nostd::platform::*;

unsafe extern "C" {
    fn device_get_binding(name: *const u8) -> *const ();
    fn uart_poll_out(dev: *const (), c: u8) -> i32;
    fn uart_poll_in(dev: *const (), c: *mut u8) -> i32;
}

pub struct PlatformSerialLink {
    uart_dev: *const (),
    mtu: u16,
}

unsafe impl Send for PlatformSerialLink {}
unsafe impl Sync for PlatformSerialLink {}

impl PlatformSerialLink {
    pub fn new(path: &str, _baud_rate: u32) -> Result<Self, LinkError> {
        let mut c_name = [0u8; 32];
        let name_bytes = path.as_bytes();
        let len = name_bytes.len().min(31);
        c_name[..len].copy_from_slice(&name_bytes[..len]);
        c_name[len] = 0;

        let dev = unsafe { device_get_binding(c_name.as_ptr()) };
        if dev.is_null() {
            return Err(LinkError::CouldNotConnect);
        }

        Ok(Self { uart_dev: dev, mtu: 256 })
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

pub struct PlatformSerialTx(*const (), u16);
pub struct PlatformSerialRx(*const (), u16);

unsafe impl Send for PlatformSerialTx {}
unsafe impl Sync for PlatformSerialTx {}
unsafe impl Send for PlatformSerialRx {}
unsafe impl Sync for PlatformSerialRx {}

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
            let ret = unsafe { uart_poll_out(self.0, c) };
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
            let ret = unsafe { uart_poll_in(self.0, byte as *mut u8) };
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
                let ret = unsafe { uart_poll_in(self.0, byte as *mut u8) };
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
            let ret = unsafe { uart_poll_out(self.uart_dev, c) };
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
            let ret = unsafe { uart_poll_in(self.uart_dev, byte as *mut u8) };
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
                let ret = unsafe { uart_poll_in(self.uart_dev, byte as *mut u8) };
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
        (PlatformSerialTx(self.uart_dev, self.mtu), PlatformSerialRx(self.uart_dev, self.mtu))
    }
}