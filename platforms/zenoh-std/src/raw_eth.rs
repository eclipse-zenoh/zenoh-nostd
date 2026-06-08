use std::sync::{Arc, Mutex};
use zenoh_nostd::platform::*;

#[cfg(target_os = "linux")]
mod raw {
    use std::mem;
    use std::os::fd::RawFd;

    use zenoh_nostd::platform::LinkError;

    /// Zenoh EtherType: 0x6C69 ("li" in ASCII)
    const ZENOH_ETHERTYPE: u16 = 0x6C69;

    pub struct RawEthSocket {
        fd: RawFd,
    }

    impl RawEthSocket {
        pub fn open(interface: &str) -> Result<Self, LinkError> {
            use std::ffi::CString;

            let fd = unsafe {
                let fd = libc::socket(
                    libc::AF_PACKET,
                    libc::SOCK_RAW,
                    (ZENOH_ETHERTYPE as u16).to_be() as i32,
                );
                if fd < 0 {
                    return Err(LinkError::CouldNotConnect);
                }
                fd
            };

            let ifname = CString::new(interface).map_err(|_| LinkError::CouldNotConnect)?;
            let ifindex = unsafe { libc::if_nametoindex(ifname.as_ptr()) };
            if ifindex == 0 {
                unsafe {
                    libc::close(fd);
                }
                return Err(LinkError::CouldNotConnect);
            }

            let mut addr: libc::sockaddr_ll = unsafe { mem::zeroed() };
            addr.sll_family = libc::AF_PACKET as u16;
            addr.sll_ifindex = ifindex as i32;
            addr.sll_protocol = (ZENOH_ETHERTYPE as u16).to_be();

            let bind_result = unsafe {
                libc::bind(
                    fd,
                    &addr as *const _ as *const libc::sockaddr,
                    mem::size_of::<libc::sockaddr_ll>() as u32,
                )
            };
            if bind_result < 0 {
                unsafe {
                    libc::close(fd);
                }
                return Err(LinkError::CouldNotConnect);
            }

            Ok(Self { fd })
        }

        pub fn send(&self, buf: &[u8]) -> Result<(), LinkError> {
            let ret = unsafe { libc::send(self.fd, buf.as_ptr() as *const _, buf.len(), 0) };
            if ret < 0 {
                Err(LinkError::LinkTxFailed)
            } else {
                Ok(())
            }
        }

        pub fn recv(&self, buf: &mut [u8]) -> Result<usize, LinkError> {
            let ret = unsafe { libc::recv(self.fd, buf.as_mut_ptr() as *mut _, buf.len(), 0) };
            if ret < 0 {
                Err(LinkError::LinkRxFailed)
            } else {
                Ok(ret as usize)
            }
        }
    }

    impl Drop for RawEthSocket {
        fn drop(&mut self) {
            unsafe {
                libc::close(self.fd);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod raw {
    use zenoh_nostd::platform::LinkError;

    pub struct RawEthSocket;

    impl RawEthSocket {
        pub fn open(_interface: &str) -> Result<Self, LinkError> {
            Err(LinkError::CouldNotConnect)
        }

        pub fn send(&self, _buf: &[u8]) -> Result<(), LinkError> {
            Err(LinkError::LinkTxFailed)
        }

        pub fn recv(&self, _buf: &mut [u8]) -> Result<usize, LinkError> {
            Err(LinkError::LinkRxFailed)
        }
    }
}

pub struct StdRawEthLink {
    socket: Arc<Mutex<raw::RawEthSocket>>,
    mtu: u16,
}

impl StdRawEthLink {
    pub fn new(interface: &str) -> Result<Self, LinkError> {
        let socket = raw::RawEthSocket::open(interface)?;
        Ok(Self {
            socket: Arc::new(Mutex::new(socket)),
            mtu: 1500,
        })
    }
}

pub struct RawEthTx {
    socket: Arc<Mutex<raw::RawEthSocket>>,
    mtu: u16,
}

pub struct RawEthRx {
    socket: Arc<Mutex<raw::RawEthSocket>>,
    mtu: u16,
}

impl ZLinkInfo for StdRawEthLink {
    fn mtu(&self) -> u16 {
        self.mtu
    }
    fn is_streamed(&self) -> bool {
        false
    }
}

impl ZLinkInfo for RawEthTx {
    fn mtu(&self) -> u16 {
        self.mtu
    }
    fn is_streamed(&self) -> bool {
        false
    }
}

impl ZLinkInfo for RawEthRx {
    fn mtu(&self) -> u16 {
        self.mtu
    }
    fn is_streamed(&self) -> bool {
        false
    }
}

impl ZLinkTx for StdRawEthLink {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        let socket = self.socket.lock().map_err(|_| LinkError::LinkTxFailed)?;
        socket.send(buffer)
    }
}

impl ZLinkTx for RawEthTx {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        let socket = self.socket.lock().map_err(|_| LinkError::LinkTxFailed)?;
        socket.send(buffer)
    }
}

impl ZLinkRx for StdRawEthLink {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let socket = self.socket.lock().map_err(|_| LinkError::LinkRxFailed)?;
        socket.recv(buffer)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let n = self.read(&mut buffer[offset..])?;
            if n == 0 {
                return Err(LinkError::LinkRxFailed);
            }
            offset += n;
        }
        Ok(())
    }
}

impl ZLinkRx for RawEthRx {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        let socket = self.socket.lock().map_err(|_| LinkError::LinkRxFailed)?;
        socket.recv(buffer)
    }

    fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        let mut offset = 0;
        while offset < buffer.len() {
            let n = self.read(&mut buffer[offset..])?;
            if n == 0 {
                return Err(LinkError::LinkRxFailed);
            }
            offset += n;
        }
        Ok(())
    }
}

impl ZLink for StdRawEthLink {
    type Tx<'a> = RawEthTx
    where
        Self: 'a;

    type Rx<'a> = RawEthRx
    where
        Self: 'a;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (
            RawEthTx {
                socket: self.socket.clone(),
                mtu: self.mtu,
            },
            RawEthRx {
                socket: self.socket.clone(),
                mtu: self.mtu,
            },
        )
    }
}