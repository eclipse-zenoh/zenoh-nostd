#![allow(non_camel_case_types)]

use core::net::Ipv4Addr;

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

#[cfg(not(feature = "freertos"))]
unsafe extern "C" {
    #[link_name = "socket"]
    fn socket_impl(domain: c_int, type_: c_int, protocol: c_int) -> c_int;
    #[link_name = "bind"]
    fn bind_impl(fd: c_int, addr: *const sockaddr, addrlen: socklen_t) -> c_int;
    #[link_name = "connect"]
    fn connect_impl(fd: c_int, addr: *const sockaddr, addrlen: socklen_t) -> c_int;
    #[link_name = "listen"]
    fn listen_impl(fd: c_int, backlog: c_int) -> c_int;
    #[link_name = "accept"]
    fn accept_impl(fd: c_int, addr: *mut sockaddr, addrlen: *mut socklen_t) -> c_int;
    #[link_name = "send"]
    fn send_impl(fd: c_int, buf: *const u8, len: size_t, flags: c_int) -> isize;
    #[link_name = "recv"]
    fn recv_impl(fd: c_int, buf: *mut u8, len: size_t, flags: c_int) -> isize;
    #[link_name = "close"]
    fn close_impl(fd: c_int) -> c_int;
    #[link_name = "setsockopt"]
    fn setsockopt_impl(fd: c_int, level: c_int, optname: c_int, optval: *const u8, optlen: socklen_t) -> c_int;
    #[link_name = "dup"]
    fn dup_impl(fd: c_int) -> c_int;
}

#[cfg(feature = "freertos")]
unsafe extern "C" {
    #[link_name = "FreeRTOS_socket"]
    fn socket_impl(domain: c_int, type_: c_int, protocol: c_int) -> c_int;
    #[link_name = "FreeRTOS_bind"]
    fn bind_impl(fd: c_int, addr: *const sockaddr, addrlen: socklen_t) -> c_int;
    #[link_name = "FreeRTOS_connect"]
    fn connect_impl(fd: c_int, addr: *const sockaddr, addrlen: socklen_t) -> c_int;
    #[link_name = "FreeRTOS_listen"]
    fn listen_impl(fd: c_int, backlog: c_int) -> c_int;
    #[link_name = "FreeRTOS_accept"]
    fn accept_impl(fd: c_int, addr: *mut sockaddr, addrlen: *mut socklen_t) -> c_int;
    #[link_name = "FreeRTOS_send"]
    fn send_impl(fd: c_int, buf: *const u8, len: size_t, flags: c_int) -> isize;
    #[link_name = "FreeRTOS_recv"]
    fn recv_impl(fd: c_int, buf: *mut u8, len: size_t, flags: c_int) -> isize;
    #[link_name = "FreeRTOS_closesocket"]
    fn close_impl(fd: c_int) -> c_int;
    #[link_name = "FreeRTOS_setsockopt"]
    fn setsockopt_impl(fd: c_int, level: c_int, optname: c_int, optval: *const u8, optlen: socklen_t) -> c_int;
}

#[inline]
pub unsafe fn socket(domain: c_int, type_: c_int, protocol: c_int) -> c_int {
    unsafe { socket_impl(domain, type_, protocol) }
}

#[inline]
pub unsafe fn bind(fd: c_int, addr: *const sockaddr, addrlen: socklen_t) -> c_int {
    unsafe { bind_impl(fd, addr, addrlen) }
}

#[inline]
pub unsafe fn connect(fd: c_int, addr: *const sockaddr, addrlen: socklen_t) -> c_int {
    unsafe { connect_impl(fd, addr, addrlen) }
}

#[inline]
pub unsafe fn listen(fd: c_int, backlog: c_int) -> c_int {
    unsafe { listen_impl(fd, backlog) }
}

#[inline]
pub unsafe fn accept(fd: c_int, addr: *mut sockaddr, addrlen: *mut socklen_t) -> c_int {
    unsafe { accept_impl(fd, addr, addrlen) }
}

#[inline]
pub unsafe fn send(fd: c_int, buf: *const u8, len: size_t, flags: c_int) -> isize {
    unsafe { send_impl(fd, buf, len, flags) }
}

#[inline]
pub unsafe fn recv(fd: c_int, buf: *mut u8, len: size_t, flags: c_int) -> isize {
    unsafe { recv_impl(fd, buf, len, flags) }
}

#[inline]
pub unsafe fn close(fd: c_int) -> c_int {
    unsafe { close_impl(fd) }
}

#[inline]
pub unsafe fn setsockopt(fd: c_int, level: c_int, optname: c_int, optval: *const u8, optlen: socklen_t) -> c_int {
    unsafe { setsockopt_impl(fd, level, optname, optval, optlen) }
}

#[cfg(not(feature = "freertos"))]
#[inline]
pub unsafe fn dup(fd: c_int) -> c_int {
    unsafe { dup_impl(fd) }
}

pub fn to_sockaddr(addr: &str) -> Option<(sockaddr_in, sockaddr_in6, bool)> {
    let (ip_str, port_str) = addr.rsplit_once(':')?;
    let port: u16 = port_str.parse().ok()?;
    let ip_str = ip_str.trim_start_matches('[').trim_end_matches(']');

    if let Ok(v4) = ip_str.parse::<Ipv4Addr>() {
        let sin = sockaddr_in {
            sin_family: AF_INET as u16,
            sin_port: port.to_be(),
            sin_addr: in_addr { s_addr: u32::from(v4).to_be() },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_sockaddr_ipv4() {
        let result = to_sockaddr("192.168.1.1:7447");
        assert!(result.is_some());
        let (sin4, _sin6, is_v4) = result.unwrap();
        assert!(is_v4);
        assert_eq!(sin4.sin_family, AF_INET as u16);
        assert_eq!(sin4.sin_port, 7447u16.to_be());
        let expected_ip = u32::from(Ipv4Addr::new(192, 168, 1, 1)).to_be();
        assert_eq!(sin4.sin_addr.s_addr, expected_ip);
    }

    #[test]
    fn test_to_sockaddr_ipv6() {
        let result = to_sockaddr("[::1]:7447");
        assert!(result.is_some());
        let (_sin4, sin6, is_v4) = result.unwrap();
        assert!(!is_v4);
        assert_eq!(sin6.sin6_family, AF_INET6 as u16);
        assert_eq!(sin6.sin6_port, 7447u16.to_be());
        assert_eq!(sin6.sin6_addr.s6_addr, core::net::Ipv6Addr::LOCALHOST.octets());
    }

    #[test]
    fn test_to_sockaddr_invalid_no_port() {
        assert!(to_sockaddr("192.168.1.1").is_none());
    }

    #[test]
    fn test_to_sockaddr_invalid_port() {
        assert!(to_sockaddr("192.168.1.1:notaport").is_none());
    }

    #[test]
    fn test_to_sockaddr_empty() {
        assert!(to_sockaddr("").is_none());
    }
}