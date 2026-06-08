use zenoh_nostd::platform::*;

unsafe extern "C" {
    fn uart0_get_instance() -> *mut ();
    fn uart1_get_instance() -> *mut ();
    fn uart_init(uart: *mut (), baud_rate: u32) -> i32;
    fn uart_set_format(uart: *mut (), data_bits: u32, stop_bits: u32, parity: u32);
    fn uart_putc_raw(uart: *mut (), c: u8);
    fn uart_is_readable(uart: *mut ()) -> i32;
    fn uart_getc(uart: *mut ()) -> i32;
}

pub struct Rp2040SerialLink {
    uart: *mut (),
    mtu: u16,
}

unsafe impl Send for Rp2040SerialLink {}
unsafe impl Sync for Rp2040SerialLink {}

impl Rp2040SerialLink {
    pub fn new(path: &str, baud_rate: u32) -> Result<Self, LinkError> {
        let uart = match path {
            "uart0" | "/dev/ttyS0" => unsafe { uart0_get_instance() },
            "uart1" | "/dev/ttyS1" => unsafe { uart1_get_instance() },
            _ => return Err(LinkError::CouldNotConnect),
        };

        if uart.is_null() {
            return Err(LinkError::CouldNotConnect);
        }

        let result = unsafe { uart_init(uart, baud_rate) };
        if result < 0 {
            return Err(LinkError::CouldNotConnect);
        }

        unsafe { uart_set_format(uart, 8, 1, 0) };

        Ok(Self { uart, mtu: 256 })
    }
}

impl ZLinkInfo for Rp2040SerialLink {
    fn mtu(&self) -> u16 {
        self.mtu
    }
    fn is_streamed(&self) -> bool {
        true
    }
}

pub struct Rp2040SerialTx(*mut (), u16);
pub struct Rp2040SerialRx(*mut (), u16);

unsafe impl Send for Rp2040SerialTx {}
unsafe impl Sync for Rp2040SerialTx {}
unsafe impl Send for Rp2040SerialRx {}
unsafe impl Sync for Rp2040SerialRx {}

impl ZLinkInfo for Rp2040SerialTx {
    fn mtu(&self) -> u16 {
        self.1
    }
    fn is_streamed(&self) -> bool {
        true
    }
}

impl ZLinkTx for Rp2040SerialTx {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        for &c in buffer {
            unsafe { uart_putc_raw(self.0, c) };
        }
        Ok(())
    }
}

impl ZLinkInfo for Rp2040SerialRx {
    fn mtu(&self) -> u16 {
        self.1
    }
    fn is_streamed(&self) -> bool {
        true
    }
}

impl ZLinkRx for Rp2040SerialRx {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let mut count = 0usize;
        for byte in buffer.iter_mut() {
            if unsafe { uart_is_readable(self.0) } == 0 {
                if count == 0 {
                    return Err(LinkError::LinkRxFailed);
                }
                break;
            }
            let c = unsafe { uart_getc(self.0) };
            if c < 0 {
                return Err(LinkError::LinkRxFailed);
            }
            *byte = c as u8;
            count += 1;
        }
        Ok(count)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        for byte in buffer.iter_mut() {
            loop {
                if unsafe { uart_is_readable(self.0) } == 0 {
                    continue;
                }
                let c = unsafe { uart_getc(self.0) };
                if c >= 0 {
                    *byte = c as u8;
                    break;
                }
                if c < -1 {
                    return Err(LinkError::LinkRxFailed);
                }
            }
        }
        Ok(())
    }
}

impl ZLinkTx for Rp2040SerialLink {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        for &c in buffer {
            unsafe { uart_putc_raw(self.uart, c) };
        }
        Ok(())
    }
}

impl ZLinkRx for Rp2040SerialLink {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let mut count = 0usize;
        for byte in buffer.iter_mut() {
            if unsafe { uart_is_readable(self.uart) } == 0 {
                if count == 0 {
                    return Err(LinkError::LinkRxFailed);
                }
                break;
            }
            let c = unsafe { uart_getc(self.uart) };
            if c < 0 {
                return Err(LinkError::LinkRxFailed);
            }
            *byte = c as u8;
            count += 1;
        }
        Ok(count)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        for byte in buffer.iter_mut() {
            loop {
                if unsafe { uart_is_readable(self.uart) } == 0 {
                    continue;
                }
                let c = unsafe { uart_getc(self.uart) };
                if c >= 0 {
                    *byte = c as u8;
                    break;
                }
                if c < -1 {
                    return Err(LinkError::LinkRxFailed);
                }
            }
        }
        Ok(())
    }
}

impl ZLink for Rp2040SerialLink {
    type Tx<'a> = Rp2040SerialTx where Self: 'a;
    type Rx<'a> = Rp2040SerialRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (Rp2040SerialTx(self.uart, self.mtu), Rp2040SerialRx(self.uart, self.mtu))
    }
}