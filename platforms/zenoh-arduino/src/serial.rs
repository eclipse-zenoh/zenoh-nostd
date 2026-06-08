use zenoh_nostd::platform::*;

unsafe extern "C" {
    fn arduino_serial_begin(uart_id: u32, baud: u32) -> u32;
    fn arduino_serial_write(uart_id: u32, data: *const u8, len: u32) -> i32;
    fn arduino_serial_available(uart_id: u32) -> i32;
    fn arduino_serial_read_bytes(uart_id: u32, data: *mut u8, len: u32) -> i32;
}

pub struct ArduinoSerialLink {
    uart_id: u32,
    mtu: u16,
}

impl ArduinoSerialLink {
    pub fn new(path: &str, baud_rate: u32) -> Result<Self, LinkError> {
        let uart_id = if path.ends_with('0') || path.contains("Serial") {
            0u32
        } else if path.ends_with('1') {
            1u32
        } else if path.ends_with('2') {
            2u32
        } else {
            return Err(LinkError::CouldNotConnect);
        };

        let result = unsafe { arduino_serial_begin(uart_id, baud_rate) };
        if result == 0 {
            return Err(LinkError::CouldNotConnect);
        }

        Ok(Self { uart_id, mtu: 256 })
    }
}

impl ZLinkInfo for ArduinoSerialLink {
    fn mtu(&self) -> u16 {
        self.mtu
    }
    fn is_streamed(&self) -> bool {
        true
    }
}

pub struct ArduinoSerialTx(u32, u16);
pub struct ArduinoSerialRx(u32, u16);

impl ZLinkInfo for ArduinoSerialTx {
    fn mtu(&self) -> u16 {
        self.1
    }
    fn is_streamed(&self) -> bool {
        true
    }
}

impl ZLinkTx for ArduinoSerialTx {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let ret = unsafe { arduino_serial_write(self.0, buffer.as_ptr(), buffer.len() as u32) };
        if ret < 0 {
            return Err(LinkError::LinkTxFailed);
        }
        Ok(())
    }
}

impl ZLinkInfo for ArduinoSerialRx {
    fn mtu(&self) -> u16 {
        self.1
    }
    fn is_streamed(&self) -> bool {
        true
    }
}

impl ZLinkRx for ArduinoSerialRx {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let avail = unsafe { arduino_serial_available(self.0) };
        if avail <= 0 {
            return Err(LinkError::LinkRxFailed);
        }
        let to_read = (avail as usize).min(buffer.len());
        let ret = unsafe { arduino_serial_read_bytes(self.0, buffer.as_mut_ptr(), to_read as u32) };
        if ret < 0 {
            return Err(LinkError::LinkRxFailed);
        }
        Ok(ret as usize)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let remaining = buffer.len() - offset;
            let ret = unsafe { arduino_serial_read_bytes(self.0, buffer[offset..].as_mut_ptr(), remaining as u32) };
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

impl ZLinkTx for ArduinoSerialLink {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let ret = unsafe { arduino_serial_write(self.uart_id, buffer.as_ptr(), buffer.len() as u32) };
        if ret < 0 {
            return Err(LinkError::LinkTxFailed);
        }
        Ok(())
    }
}

impl ZLinkRx for ArduinoSerialLink {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let avail = unsafe { arduino_serial_available(self.uart_id) };
        if avail <= 0 {
            return Err(LinkError::LinkRxFailed);
        }
        let to_read = (avail as usize).min(buffer.len());
        let ret = unsafe { arduino_serial_read_bytes(self.uart_id, buffer.as_mut_ptr(), to_read as u32) };
        if ret < 0 {
            return Err(LinkError::LinkRxFailed);
        }
        Ok(ret as usize)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let remaining = buffer.len() - offset;
            let ret = unsafe { arduino_serial_read_bytes(self.uart_id, buffer[offset..].as_mut_ptr(), remaining as u32) };
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

impl ZLink for ArduinoSerialLink {
    type Tx<'a> = ArduinoSerialTx where Self: 'a;
    type Rx<'a> = ArduinoSerialRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (ArduinoSerialTx(self.uart_id, self.mtu), ArduinoSerialRx(self.uart_id, self.mtu))
    }
}