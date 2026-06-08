use zenoh_nostd::platform::*;
use crate::sockets;

pub struct PosixTcpLink {
    fd: i32,
    mtu: u16,
}

impl PosixTcpLink {
    pub fn listen(endpoint: &str) -> Result<Self, LinkError> {
        let (sin4, sin6, is_v4) = sockets::to_sockaddr(endpoint).ok_or(LinkError::CouldNotParseProtocol)?;

        let domain = if is_v4 { sockets::AF_INET } else { sockets::AF_INET6 };
        let fd = unsafe { sockets::socket(domain, sockets::SOCK_STREAM, 0) };
        if fd < 0 {
            return Err(LinkError::CouldNotListen);
        }

        let optval = 1i32;
        unsafe {
            sockets::setsockopt(
                fd,
                sockets::SOL_SOCKET,
                sockets::SO_REUSEADDR,
                &optval as *const _ as *const u8,
                core::mem::size_of::<i32>() as u32,
            );
        }

        let bind_result = if is_v4 {
            unsafe {
                sockets::bind(
                    fd,
                    &sin4 as *const _ as *const sockets::sockaddr,
                    core::mem::size_of::<sockets::sockaddr_in>() as u32,
                )
            }
        } else {
            unsafe {
                sockets::bind(
                    fd,
                    &sin6 as *const _ as *const sockets::sockaddr,
                    core::mem::size_of::<sockets::sockaddr_in6>() as u32,
                )
            }
        };
        if bind_result < 0 {
            unsafe { sockets::close(fd) };
            return Err(LinkError::CouldNotListen);
        }

        let listen_result = unsafe { sockets::listen(fd, 1) };
        if listen_result < 0 {
            unsafe { sockets::close(fd) };
            return Err(LinkError::CouldNotListen);
        }

        let mut client_addr: sockets::sockaddr = unsafe { core::mem::zeroed() };
        let mut addrlen = core::mem::size_of::<sockets::sockaddr>() as u32;
        let client_fd = unsafe { sockets::accept(fd, &mut client_addr as *mut _, &mut addrlen as *mut _) };
        if client_fd < 0 {
            unsafe { sockets::close(fd) };
            return Err(LinkError::CouldNotListen);
        }

        unsafe { sockets::close(fd) };

        Ok(Self { fd: client_fd, mtu: u16::MAX })
    }

    pub fn connect(endpoint: &str) -> Result<Self, LinkError> {
        let (sin4, sin6, is_v4) = sockets::to_sockaddr(endpoint).ok_or(LinkError::CouldNotParseProtocol)?;

        let domain = if is_v4 { sockets::AF_INET } else { sockets::AF_INET6 };
        let fd = unsafe { sockets::socket(domain, sockets::SOCK_STREAM, 0) };
        if fd < 0 {
            return Err(LinkError::CouldNotConnect);
        }

        let result = if is_v4 {
            unsafe {
                sockets::connect(
                    fd,
                    &sin4 as *const _ as *const sockets::sockaddr,
                    core::mem::size_of::<sockets::sockaddr_in>() as u32,
                )
            }
        } else {
            unsafe {
                sockets::connect(
                    fd,
                    &sin6 as *const _ as *const sockets::sockaddr,
                    core::mem::size_of::<sockets::sockaddr_in6>() as u32,
                )
            }
        };

        if result < 0 {
            unsafe { sockets::close(fd) };
            return Err(LinkError::CouldNotConnect);
        }

        let nodelay: u32 = 1;
        unsafe {
            sockets::setsockopt(
                fd,
                sockets::IPPROTO_TCP,
                sockets::TCP_NODELAY,
                &nodelay as *const _ as *const u8,
                core::mem::size_of::<u32>() as u32,
            );
        }

        let timeout_ms = 10u32;
        unsafe {
            sockets::setsockopt(
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

impl Drop for PosixTcpLink {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe { sockets::close(self.fd) };
        }
    }
}

impl ZLinkInfo for PosixTcpLink {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for PosixTcpLink {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let n = unsafe { sockets::send(self.fd, buffer[offset..].as_ptr(), buffer.len() - offset, 0) };
            if n < 0 { return Err(LinkError::LinkTxFailed); }
            offset += n as usize;
        }
        Ok(())
    }
}

impl ZLinkRx for PosixTcpLink {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let n = unsafe { sockets::recv(self.fd, buffer.as_mut_ptr(), buffer.len(), 0) };
        if n < 0 { Err(LinkError::LinkRxFailed) } else { Ok(n as usize) }
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let n = unsafe { sockets::recv(self.fd, buffer[offset..].as_mut_ptr(), buffer.len() - offset, 0) };
            if n <= 0 { return Err(LinkError::LinkRxFailed); }
            offset += n as usize;
        }
        Ok(())
    }
}

impl ZLink for PosixTcpLink {
    type Tx<'a> = PosixTcpTx where Self: 'a;
    type Rx<'a> = PosixTcpRx where Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        let fd = self.fd;
        self.fd = -1;
        (
            PosixTcpTx { fd, mtu: self.mtu },
            PosixTcpRx { fd, mtu: self.mtu },
        )
    }
}

pub struct PosixTcpTx { fd: i32, mtu: u16 }
pub struct PosixTcpRx { fd: i32, mtu: u16 }

impl Drop for PosixTcpTx { fn drop(&mut self) {} }
impl Drop for PosixTcpRx { fn drop(&mut self) { if self.fd >= 0 { unsafe { sockets::close(self.fd) }; } } }

impl ZLinkInfo for PosixTcpTx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkTx for PosixTcpTx {
    fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let n = unsafe { sockets::send(self.fd, buffer[offset..].as_ptr(), buffer.len() - offset, 0) };
            if n < 0 { return Err(LinkError::LinkTxFailed); }
            offset += n as usize;
        }
        Ok(())
    }
}

impl ZLinkInfo for PosixTcpRx {
    fn mtu(&self) -> u16 { self.mtu }
    fn is_streamed(&self) -> bool { true }
}

impl ZLinkRx for PosixTcpRx {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        let n = unsafe { sockets::recv(self.fd, buffer.as_mut_ptr(), buffer.len(), 0) };
        if n < 0 { Err(LinkError::LinkRxFailed) } else { Ok(n as usize) }
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let n = unsafe { sockets::recv(self.fd, buffer[offset..].as_mut_ptr(), buffer.len() - offset, 0) };
            if n <= 0 { return Err(LinkError::LinkRxFailed); }
            offset += n as usize;
        }
        Ok(())
    }
}
