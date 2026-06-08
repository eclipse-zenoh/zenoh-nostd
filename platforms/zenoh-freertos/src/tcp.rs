use zenoh_nostd::platform::*;
use crate::sockets;

pub struct PlatformTcpLink {
    fd: i32,
    mtu: u16,
}

impl PlatformTcpLink {
    pub fn listen(endpoint: &str) -> Result<Self, LinkError> {
        let (sin4, sin6, is_v4) = sockets::to_sockaddr(endpoint).ok_or(LinkError::CouldNotParseProtocol)?;

        let domain = if is_v4 { sockets::AF_INET } else { sockets::AF_INET6 };
        let fd = unsafe { sockets::FreeRTOS_socket(domain, sockets::SOCK_STREAM, 0) };
        if fd < 0 {
            return Err(LinkError::CouldNotListen);
        }

        let optval = 1i32;
        unsafe {
            sockets::FreeRTOS_setsockopt(
                fd,
                sockets::SOL_SOCKET,
                sockets::SO_REUSEADDR,
                &optval as *const _ as *const u8,
                core::mem::size_of::<i32>() as u32,
            );
        }

        let bind_result = if is_v4 {
            unsafe {
                sockets::FreeRTOS_bind(
                    fd,
                    &sin4 as *const _ as *const sockets::sockaddr,
                    core::mem::size_of::<sockets::sockaddr_in>() as u32,
                )
            }
        } else {
            unsafe {
                sockets::FreeRTOS_bind(
                    fd,
                    &sin6 as *const _ as *const sockets::sockaddr,
                    core::mem::size_of::<sockets::sockaddr_in6>() as u32,
                )
            }
        };
        if bind_result < 0 {
            unsafe { sockets::FreeRTOS_closesocket(fd) };
            return Err(LinkError::CouldNotListen);
        }

        let listen_result = unsafe { sockets::FreeRTOS_listen(fd, 1) };
        if listen_result < 0 {
            unsafe { sockets::FreeRTOS_closesocket(fd) };
            return Err(LinkError::CouldNotListen);
        }

        let mut client_addr: sockets::sockaddr = unsafe { core::mem::zeroed() };
        let mut addrlen = core::mem::size_of::<sockets::sockaddr>() as u32;
        let client_fd = unsafe { sockets::FreeRTOS_accept(fd, &mut client_addr as *mut _, &mut addrlen as *mut _) };
        if client_fd < 0 {
            unsafe { sockets::FreeRTOS_closesocket(fd) };
            return Err(LinkError::CouldNotListen);
        }

        unsafe { sockets::FreeRTOS_closesocket(fd) };

        Ok(Self { fd: client_fd, mtu: u16::MAX })
    }

    pub fn connect(endpoint: &str) -> Result<Self, LinkError> {
        let (sin4, sin6, is_v4) = sockets::to_sockaddr(endpoint).ok_or(LinkError::CouldNotParseProtocol)?;

        let domain = if is_v4 { sockets::AF_INET } else { sockets::AF_INET6 };
        let fd = unsafe { sockets::FreeRTOS_socket(domain, sockets::SOCK_STREAM, 0) };
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

        let nodelay: u32 = 1;
        unsafe {
            sockets::FreeRTOS_setsockopt(
                fd,
                sockets::IPPROTO_TCP,
                sockets::TCP_NODELAY,
                &nodelay as *const _ as *const u8,
                core::mem::size_of::<u32>() as u32,
            );
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

        Ok(Self { fd, mtu: u16::MAX })
    }
}

impl Drop for PlatformTcpLink {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe { sockets::FreeRTOS_closesocket(self.fd) };
        }
    }
}

impl ZLinkInfo for PlatformTcpLink {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for PlatformTcpLink {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let n = unsafe { sockets::FreeRTOS_send(self.fd, buffer[offset..].as_ptr(), buffer.len() - offset, 0) };
            if n < 0 { return Err(LinkError::LinkTxFailed); }
            offset += n as usize;
        }
        Ok(())
    }
}

impl ZLinkRx for PlatformTcpLink {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let n = unsafe { sockets::FreeRTOS_recv(self.fd, buffer.as_mut_ptr(), buffer.len(), 0) };
        if n < 0 { Err(LinkError::LinkRxFailed) } else { Ok(n as usize) }
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let n = unsafe { sockets::FreeRTOS_recv(self.fd, buffer[offset..].as_mut_ptr(), buffer.len() - offset, 0) };
            if n <= 0 { return Err(LinkError::LinkRxFailed); }
            offset += n as usize;
        }
        Ok(())
    }
}

impl ZLink for PlatformTcpLink {
    type Tx<'a> = PlatformTcpTx where Self: 'a;
    type Rx<'a> = PlatformTcpRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        let fd = self.fd;
        self.fd = -1;
        (
            PlatformTcpTx { fd, mtu: self.mtu },
            PlatformTcpRx { fd, mtu: self.mtu },
        )
    }
}

pub struct PlatformTcpTx { fd: i32, mtu: u16 }
pub struct PlatformTcpRx { fd: i32, mtu: u16 }

impl Drop for PlatformTcpTx { fn drop(&mut self) {} }
impl Drop for PlatformTcpRx { fn drop(&mut self) { if self.fd >= 0 { unsafe { sockets::FreeRTOS_closesocket(self.fd) }; } } }

impl ZLinkInfo for PlatformTcpTx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for PlatformTcpTx {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let n = unsafe { sockets::FreeRTOS_send(self.fd, buffer[offset..].as_ptr(), buffer.len() - offset, 0) };
            if n < 0 { return Err(LinkError::LinkTxFailed); }
            offset += n as usize;
        }
        Ok(())
    }
}

impl ZLinkInfo for PlatformTcpRx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkRx for PlatformTcpRx {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let n = unsafe { sockets::FreeRTOS_recv(self.fd, buffer.as_mut_ptr(), buffer.len(), 0) };
        if n < 0 { Err(LinkError::LinkRxFailed) } else { Ok(n as usize) }
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let n = unsafe { sockets::FreeRTOS_recv(self.fd, buffer[offset..].as_mut_ptr(), buffer.len() - offset, 0) };
            if n <= 0 { return Err(LinkError::LinkRxFailed); }
            offset += n as usize;
        }
        Ok(())
    }
}