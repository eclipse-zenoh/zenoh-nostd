use zenoh_nostd::platform::*;

unsafe extern "C" {
    fn huart_lookup(name: *const u8) -> *mut ();
    fn HAL_UART_Transmit(huart: *mut (), pData: *const u8, Size: u16, Timeout: u32) -> u32;
    fn HAL_UART_Receive(huart: *mut (), pData: *mut u8, Size: u16, Timeout: u32) -> u32;
}

const HAL_OK: u32 = 0;

pub struct PlatformSerialLink {
    huart: *mut (),
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

        let huart = unsafe { huart_lookup(c_name.as_ptr()) };
        if huart.is_null() {
            return Err(LinkError::CouldNotConnect);
        }

        Ok(Self { huart, mtu: 256 })
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
        let result = unsafe { HAL_UART_Transmit(self.0, buffer.as_ptr(), buffer.len() as u16, 1000) };
        if result != HAL_OK {
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
        let len = buffer.len().min(256) as u16;
        let result = unsafe { HAL_UART_Receive(self.0, buffer.as_mut_ptr(), len, 100) };
        if result != HAL_OK {
            return Err(LinkError::LinkRxFailed);
        }
        Ok(len as usize)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let chunk_size = (buffer.len() - offset).min(64) as u16;
            let slice = &mut buffer[offset..offset + chunk_size as usize];
            let result = unsafe { HAL_UART_Receive(self.0, slice.as_mut_ptr(), chunk_size, 1000) };
            if result != HAL_OK {
                return Err(LinkError::LinkRxFailed);
            }
            offset += chunk_size as usize;
        }
        Ok(())
    }
}

impl ZLinkTx for PlatformSerialLink {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let result = unsafe { HAL_UART_Transmit(self.huart, buffer.as_ptr(), buffer.len() as u16, 1000) };
        if result != HAL_OK {
            return Err(LinkError::LinkTxFailed);
        }
        Ok(())
    }
}

impl ZLinkRx for PlatformSerialLink {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let len = buffer.len().min(256) as u16;
        let result = unsafe { HAL_UART_Receive(self.huart, buffer.as_mut_ptr(), len, 100) };
        if result != HAL_OK {
            return Err(LinkError::LinkRxFailed);
        }
        Ok(len as usize)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let chunk_size = (buffer.len() - offset).min(64) as u16;
            let slice = &mut buffer[offset..offset + chunk_size as usize];
            let result = unsafe { HAL_UART_Receive(self.huart, slice.as_mut_ptr(), chunk_size, 1000) };
            if result != HAL_OK {
                return Err(LinkError::LinkRxFailed);
            }
            offset += chunk_size as usize;
        }
        Ok(())
    }
}

impl ZLink for PlatformSerialLink {
    type Tx<'a> = PlatformSerialTx where Self: 'a;
    type Rx<'a> = PlatformSerialRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (PlatformSerialTx(self.huart, self.mtu), PlatformSerialRx(self.huart, self.mtu))
    }
}