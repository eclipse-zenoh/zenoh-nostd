use zenoh_nostd::platform::*;
use crate::sockets;

pub struct PlatformUdpLink {
    fd: i32,
    mtu: u16,
}

impl PlatformUdpLink {
    pub fn connect(endpoint: &str) -> Result<Self, LinkError> {
        let (sin4, sin6, is_v4) = sockets::to_sockaddr(endpoint).ok_or(LinkError::CouldNotParseProtocol)?;

        let domain = if is_v4 { sockets::AF_INET } else { sockets::AF_INET6 };
        let fd = unsafe { sockets::FreeRTOS_socket(domain, sockets::SOCK_DGRAM, 0) };
        if fd < 0 {
            return Err(LinkError::CouldNotConnect);
        }

        let result = if is_v4 {
            unsafe {
                sockets::FreeRTOS_connect(
                    fd,
                    &sin4 as *const _ as *const sockets::sockaddr,
                    core::mem::size_of::<sockets::sockaddr_in>() as u32,
                )
            }
        } else {
            unsafe {
                sockets::FreeRTOS_connect(
                    fd,
                    &sin6 as *const _ as *const sockets::sockaddr,
                    core::mem::size_of::<sockets::sockaddr_in6>() as u32,
                )
            }
        };

        if result < 0 {
            unsafe { sockets::FreeRTOS_closesocket(fd) };
            return Err(LinkError::CouldNotConnect);
        }

        let timeout_ms = 10u32;
        unsafe {
            sockets::FreeRTOS_setsockopt(
                fd,
                sockets::SOL_SOCKET,
                sockets::SO_RCVTIMEO,
                &timeout_ms as *const _ as *const u8,
                core::mem::size_of::<u32>() as u32,
            );
        }

        Ok(Self { fd, mtu: 8192 })
    }
}

impl Drop for PlatformUdpLink {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe { sockets::FreeRTOS_closesocket(self.fd) };
        }
    }
}

impl ZLinkInfo for PlatformUdpLink {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for PlatformUdpLink {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let n = unsafe { sockets::FreeRTOS_send(self.fd, buffer.as_ptr(), buffer.len(), 0) };
        if n < 0 { Err(LinkError::LinkTxFailed) } else { Ok(()) }
    }
}

impl ZLinkRx for PlatformUdpLink {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let n = unsafe { sockets::FreeRTOS_recv(self.fd, buffer.as_mut_ptr(), buffer.len(), 0) };
        if n < 0 { Err(LinkError::LinkRxFailed) } else { Ok(n as usize) }
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}

impl ZLink for PlatformUdpLink {
    type Tx<'a> = PlatformUdpTx where Self: 'a;
    type Rx<'a> = PlatformUdpRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        let fd = self.fd;
        self.fd = -1;
        (
            PlatformUdpTx { fd, mtu: self.mtu },
            PlatformUdpRx { fd, mtu: self.mtu },
        )
    }
}

pub struct PlatformUdpTx { fd: i32, mtu: u16 }
pub struct PlatformUdpRx { fd: i32, mtu: u16 }

impl Drop for PlatformUdpTx { fn drop(&mut self) {} }
impl Drop for PlatformUdpRx { fn drop(&mut self) { if self.fd >= 0 { unsafe { sockets::FreeRTOS_closesocket(self.fd) }; } } }

impl ZLinkInfo for PlatformUdpTx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkTx for PlatformUdpTx {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let n = unsafe { sockets::FreeRTOS_send(self.fd, buffer.as_ptr(), buffer.len(), 0) };
        if n < 0 { Err(LinkError::LinkTxFailed) } else { Ok(()) }
    }
}

impl ZLinkInfo for PlatformUdpRx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { false }
}

impl ZLinkRx for PlatformUdpRx {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let n = unsafe { sockets::FreeRTOS_recv(self.fd, buffer.as_mut_ptr(), buffer.len(), 0) };
        if n < 0 { Err(LinkError::LinkRxFailed) } else { Ok(n as usize) }
    }

    fn read_exact(&mut self, _buffer: &mut [u8]) -> Result<(), LinkError> {
        Err(LinkError::LinkRxFailed)
    }
}