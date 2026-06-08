use core::ffi::c_void;
use zenoh_nostd::platform::*;

unsafe extern "C" {
    fn HAL_UART_Transmit(huart: *const c_void, pData: *const u8, Size: u16, Timeout: u32) -> i32;
    fn HAL_UART_Receive(huart: *const c_void, pData: *mut u8, Size: u16, Timeout: u32) -> i32;
    static USART1: c_void;
    static USART2: c_void;
    static USART3: c_void;
}

pub struct Stm32SerialLink {
    huart: *const c_void,
    mtu: u16,
}

unsafe impl Send for Stm32SerialLink {}
unsafe impl Sync for Stm32SerialLink {}

impl Stm32SerialLink {
    pub fn new(path: &str, _baud_rate: u32) -> Result<Self, LinkError> {
        let huart = match path {
            "/dev/usart1" => unsafe { &USART1 as *const c_void },
            "/dev/usart2" => unsafe { &USART2 as *const c_void },
            "/dev/usart3" => unsafe { &USART3 as *const c_void },
            _ => return Err(LinkError::CouldNotConnect),
        };
        Ok(Self { huart, mtu: 256 })
    }
}

impl ZLinkInfo for Stm32SerialLink {
    fn mtu(&self) -> u16 { 256 }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for Stm32SerialLink {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let ret = unsafe {
            HAL_UART_Transmit(self.huart, buffer.as_ptr(), buffer.len() as u16, 1000)
        };
        if ret != 0 { Err(LinkError::LinkTxFailed) } else { Ok(()) }
    }
}

impl ZLinkRx for Stm32SerialLink {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let ret = unsafe {
            HAL_UART_Receive(self.huart, buffer.as_mut_ptr(), buffer.len() as u16, 100)
        };
        if ret != 0 { Err(LinkError::LinkRxFailed) } else { Ok(buffer.len()) }
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let ret = unsafe {
            HAL_UART_Receive(self.huart, buffer.as_mut_ptr(), buffer.len() as u16, 1000)
        };
        if ret != 0 { Err(LinkError::LinkRxFailed) } else { Ok(()) }
    }
}

impl ZLink for Stm32SerialLink {
    type Tx<'a> = Stm32SerialTx where Self: 'a;
    type Rx<'a> = Stm32SerialRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (Stm32SerialTx { huart: self.huart, mtu: self.mtu },
         Stm32SerialRx { huart: self.huart, mtu: self.mtu })
    }
}

pub struct Stm32SerialTx { huart: *const c_void, mtu: u16 }
pub struct Stm32SerialRx { huart: *const c_void, mtu: u16 }

unsafe impl Send for Stm32SerialTx {}
unsafe impl Sync for Stm32SerialTx {}
unsafe impl Send for Stm32SerialRx {}
unsafe impl Sync for Stm32SerialRx {}

impl ZLinkInfo for Stm32SerialTx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for Stm32SerialTx {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let ret = unsafe {
            HAL_UART_Transmit(self.huart, buffer.as_ptr(), buffer.len() as u16, 1000)
        };
        if ret != 0 { Err(LinkError::LinkTxFailed) } else { Ok(()) }
    }
}

impl ZLinkInfo for Stm32SerialRx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkRx for Stm32SerialRx {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let ret = unsafe {
            HAL_UART_Receive(self.huart, buffer.as_mut_ptr(), buffer.len() as u16, 100)
        };
        if ret != 0 { Err(LinkError::LinkRxFailed) } else { Ok(buffer.len()) }
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let ret = unsafe {
            HAL_UART_Receive(self.huart, buffer.as_mut_ptr(), buffer.len() as u16, 1000)
        };
        if ret != 0 { Err(LinkError::LinkRxFailed) } else { Ok(()) }
    }
}