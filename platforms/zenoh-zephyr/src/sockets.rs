#![allow(non_camel_case_types)]

pub type c_int = i32;
pub type size_t = usize;
pub type socklen_t = u32;
pub type sa_family_t = u16;

#[repr(C)]
pub struct sockaddr {
    pub sa_family: sa_family_t,
    pub sa_data: [u8; 14],
}

#[repr(C)]
pub struct sockaddr_in {
    pub sin_family: sa_family_t,
    pub sin_port: u16,
    pub sin_addr: in_addr,
    pub sin_zero: [u8; 8],
}

#[repr(C)]
pub struct in_addr {
    pub s_addr: u32,
}

#[repr(C)]
pub struct sockaddr_in6 {
    pub sin6_family: sa_family_t,
    pub sin6_port: u16,
    pub sin6_flowinfo: u32,
    pub sin6_addr: in6_addr,
    pub sin6_scope_id: u32,
}

#[repr(C)]
pub struct in6_addr {
    pub s6_addr: [u8; 16],
}

// Zephyr POSIX constants (Linux-compatible)
pub const AF_INET: c_int = 2;
pub const AF_INET6: c_int = 10;
pub const SOCK_STREAM: c_int = 1;
pub const SOCK_DGRAM: c_int = 2;
pub const SOL_SOCKET: c_int = 1;
pub const SO_REUSEADDR: c_int = 2;
pub const SO_RCVTIMEO: c_int = 20;
pub const SO_SNDTIMEO: c_int = 21;
pub const IPPROTO_TCP: c_int = 6;
pub const TCP_NODELAY: c_int = 1;

unsafe extern "C" {
    pub fn socket(domain: c_int, type_: c_int, protocol: c_int) -> c_int;
    pub fn bind(fd: c_int, addr: *const sockaddr, addrlen: socklen_t) -> c_int;
    pub fn connect(fd: c_int, addr: *const sockaddr, addrlen: socklen_t) -> c_int;
    pub fn listen(fd: c_int, backlog: c_int) -> c_int;
    pub fn accept(fd: c_int, addr: *mut sockaddr, addrlen: *mut socklen_t) -> c_int;
    pub fn send(fd: c_int, buf: *const u8, len: size_t, flags: c_int) -> isize;
    pub fn recv(fd: c_int, buf: *mut u8, len: size_t, flags: c_int) -> isize;
    pub fn close(fd: c_int) -> c_int;
    pub fn setsockopt(fd: c_int, level: c_int, optname: c_int, optval: *const u8, optlen: socklen_t) -> c_int;
    pub fn dup(fd: c_int) -> c_int;
}

pub fn to_sockaddr(addr: &str) -> Option<(sockaddr_in, sockaddr_in6, bool)> {
    let (ip_str, port_str) = addr.rsplit_once(':')?;
    let port: u16 = port_str.parse().ok()?;
    let ip_str = ip_str.trim_start_matches('[').trim_end_matches(']');

    if let Ok(v4) = ip_str.parse::<core::net::Ipv4Addr>() {
        let sin = sockaddr_in {
            sin_family: AF_INET as u16,
            sin_port: port.to_be(),
            sin_addr: in_addr { s_addr: v4.to_bits().to_be() },
            sin_zero: [0u8; 8],
        };
        return Some((sin, unsafe { core::mem::zeroed() }, true));
    }

    if let Ok(v6) = ip_str.parse::<core::net::Ipv6Addr>() {
        let sin6 = sockaddr_in6 {
            sin6_family: AF_INET6 as u16,
            sin6_port: port.to_be(),
            sin6_flowinfo: 0,
            sin6_addr: in6_addr { s6_addr: v6.octets() },
            sin6_scope_id: 0,
        };
        return Some((unsafe { core::mem::zeroed() }, sin6, false));
    }

    None
}