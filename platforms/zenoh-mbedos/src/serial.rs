use zenoh_nostd::platform::*;

unsafe extern "C" {
    fn mbed_serial_open(name: *const u8) -> *mut ();
    fn mbed_serial_close(handle: *mut ());
    fn mbed_serial_write(handle: *mut (), data: *const u8, len: u32) -> i32;
    fn mbed_serial_read(handle: *mut (), data: *mut u8, len: u32, timeout_ms: u32) -> i32;
}

pub struct PlatformSerialLink {
    handle: *mut (),
    mtu: u16,
}

unsafe impl Send for PlatformSerialLink {}
unsafe impl Sync for PlatformSerialLink {}

impl Drop for PlatformSerialLink {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { mbed_serial_close(self.handle) };
        }
    }
}

impl PlatformSerialLink {
    pub fn new(path: &str, _baud_rate: u32) -> Result<Self, LinkError> {
        let mut c_name = [0u8; 32];
        let name_bytes = path.as_bytes();
        let len = name_bytes.len().min(31);
        c_name[..len].copy_from_slice(&name_bytes[..len]);
        c_name[len] = 0;

        let handle = unsafe { mbed_serial_open(c_name.as_ptr()) };
        if handle.is_null() {
            return Err(LinkError::CouldNotConnect);
        }

        Ok(Self { handle, mtu: 256 })
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

pub struct PlatformSerialTx(*mut (), u16);
pub struct PlatformSerialRx(*mut (), u16);

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
        let ret = unsafe { mbed_serial_write(self.0, buffer.as_ptr(), buffer.len() as u32) };
        if ret < 0 {
            return Err(LinkError::LinkTxFailed);
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
        let ret = unsafe { mbed_serial_read(self.0, buffer.as_mut_ptr(), buffer.len() as u32, 100) };
        if ret < 0 {
            return Err(LinkError::LinkRxFailed);
        }
        Ok(ret as usize)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let slice = &mut buffer[offset..];
            let ret = unsafe { mbed_serial_read(self.0, slice.as_mut_ptr(), slice.len() as u32, 1000) };
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

impl ZLinkTx for PlatformSerialLink {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let ret = unsafe { mbed_serial_write(self.handle, buffer.as_ptr(), buffer.len() as u32) };
        if ret < 0 {
            return Err(LinkError::LinkTxFailed);
        }
        Ok(())
    }
}

impl ZLinkRx for PlatformSerialLink {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let ret = unsafe { mbed_serial_read(self.handle, buffer.as_mut_ptr(), buffer.len() as u32, 100) };
        if ret < 0 {
            return Err(LinkError::LinkRxFailed);
        }
        Ok(ret as usize)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let slice = &mut buffer[offset..];
            let ret = unsafe { mbed_serial_read(self.handle, slice.as_mut_ptr(), slice.len() as u32, 1000) };
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

impl ZLink for PlatformSerialLink {
    type Tx<'a> = PlatformSerialTx where Self: 'a;
    type Rx<'a> = PlatformSerialRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (PlatformSerialTx(self.handle, self.mtu), PlatformSerialRx(self.handle, self.mtu))
    }
}