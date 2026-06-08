use core::{convert::TryFrom, fmt, net::SocketAddr, str::FromStr};

const PROTO_SEPARATOR: char = '/';
const METADATA_SEPARATOR: char = '?';
const CONFIG_SEPARATOR: char = '#';

fn protocol(s: &str) -> &str {
    let pdix = s.find(PROTO_SEPARATOR).unwrap_or(s.len());
    &s[..pdix]
}

fn address(s: &str) -> &str {
    let pdix = s.find(PROTO_SEPARATOR).unwrap_or(s.len());
    let midx = s.find(METADATA_SEPARATOR).unwrap_or(s.len());
    let cidx = s.find(CONFIG_SEPARATOR).unwrap_or(s.len());
    &s[pdix + 1..midx.min(cidx)]
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum ProtocolId {
    Tcp,
    Udp,
    WebSocket,
    Serial,
}

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Protocol<'a>(&'a str);

impl<'a> Protocol<'a> {
    pub fn as_str(&self) -> &'_ str {
        self.0
    }
}

impl AsRef<str> for Protocol<'_> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Protocol<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for Protocol<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl TryFrom<Protocol<'_>> for ProtocolId {
    type Error = crate::EndpointError;

    fn try_from(value: Protocol<'_>) -> Result<Self, Self::Error> {
        Ok(match value.as_str() {
            "tcp" => Self::Tcp,
            "udp" => Self::Udp,
            "ws" => Self::WebSocket,
            "serial" => Self::Serial,
            _ => crate::zbail!(crate::EndpointError::CouldNotParseProtocol),
        })
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Address<'a>(&'a str);

impl<'a> Address<'a> {
    pub fn as_str(&self) -> &'_ str {
        self.0
    }
}

impl AsRef<str> for Address<'_> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Address<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for Address<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl<'a> From<&'a str> for Address<'a> {
    fn from(value: &'a str) -> Self {
        Address(value)
    }
}

impl TryFrom<Address<'_>> for SocketAddr {
    type Error = crate::EndpointError;

    fn try_from(value: Address<'_>) -> Result<Self, Self::Error> {
        SocketAddr::from_str(value.as_str()).map_err(|_| crate::EndpointError::CouldNotParseAddress)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Endpoint<'a> {
    pub(super) inner: &'a str,
}

impl Endpoint<'_> {
    pub fn protocol(&self) -> Protocol<'_> {
        Protocol(protocol(self.inner))
    }

    pub fn address(&self) -> Address<'_> {
        Address(address(self.inner))
    }

    pub fn address_with_metadata(&self) -> &str {
        let pdix = self
            .inner
            .find(PROTO_SEPARATOR)
            .unwrap_or(self.inner.len());
        &self.inner[pdix + 1..]
    }
}

impl fmt::Display for Endpoint<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.inner)
    }
}

impl fmt::Debug for Endpoint<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl<'a> TryFrom<&'a str> for Endpoint<'a> {
    type Error = crate::EndpointError;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        let pidx = s
            .find(PROTO_SEPARATOR)
            .and_then(|i| (!s[..i].is_empty() && !s[i + 1..].is_empty()).then_some(i))
            .ok_or(crate::EndpointError::NoProtocolSeparator)?;

        match (s.find(METADATA_SEPARATOR), s.find(CONFIG_SEPARATOR)) {
            (None, None) => Ok(Endpoint { inner: s }),

            (Some(midx), None) if midx > pidx && !s[midx + 1..].is_empty() => {
                Ok(Endpoint { inner: s })
            }

            (None, Some(cidx)) if cidx > pidx && !s[cidx + 1..].is_empty() => {
                crate::zbail!(crate::EndpointError::ConfigNotSupported)
            }

            (Some(_), Some(_)) => {
                crate::zbail!(crate::EndpointError::ConfigNotSupported)
            }

            _ => Err(crate::EndpointError::MetadataNotSupported),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::convert::TryFrom;
    use crate::Endpoint;

    #[test]
    fn endpoint_tcp() {
        let ep = Endpoint::try_from("tcp/127.0.0.1:7447").unwrap();
        assert_eq!(ep.protocol().as_str(), "tcp");
        assert_eq!(ep.address().as_str(), "127.0.0.1:7447");
    }

    #[test]
    fn endpoint_udp() {
        let ep = Endpoint::try_from("udp/192.168.1.1:7447").unwrap();
        assert_eq!(ep.protocol().as_str(), "udp");
        assert_eq!(ep.address().as_str(), "192.168.1.1:7447");
    }

    #[test]
    fn endpoint_serial() {
        let ep = Endpoint::try_from("serial//dev/ttyUSB0").unwrap();
        assert_eq!(ep.protocol().as_str(), "serial");
        assert_eq!(ep.address().as_str(), "/dev/ttyUSB0");
    }

    #[test]
    fn endpoint_no_protocol_separator_is_err() {
        assert!(Endpoint::try_from("tcp").is_err());
    }

    #[test]
    fn endpoint_empty_address_is_err() {
        assert!(Endpoint::try_from("tcp/").is_err());
    }

    #[test]
    fn endpoint_metadata_allowed() {
        let ep = Endpoint::try_from("serial//dev/ttyUSB0?baud=9600&parity=even").unwrap();
        assert_eq!(ep.protocol().as_str(), "serial");
        assert_eq!(ep.address().as_str(), "/dev/ttyUSB0");
    }

    #[test]
    fn endpoint_metadata_allowed_tcp() {
        let ep = Endpoint::try_from("tcp/127.0.0.1:7447?mtu=1500").unwrap();
        assert_eq!(ep.protocol().as_str(), "tcp");
        assert_eq!(ep.address().as_str(), "127.0.0.1:7447");
    }

    #[test]
    fn endpoint_config_not_supported() {
        assert!(Endpoint::try_from("tcp/127.0.0.1:7447#config").is_err());
    }

    #[test]
    fn endpoint_display() {
        let ep = Endpoint::try_from("tcp/127.0.0.1:7447").unwrap();
        assert_eq!(format!("{}", ep), "tcp/127.0.0.1:7447");
    }

    #[test]
    fn endpoint_protocol_address_accessors() {
        let ep = Endpoint::try_from("ws/example.com:8080/path").unwrap();
        assert_eq!(ep.protocol().as_str(), "ws");
        assert_eq!(ep.address().as_str(), "example.com:8080/path");
    }

    #[test]
    fn endpoint_address_with_metadata() {
        let ep = Endpoint::try_from("serial//dev/ttyUSB0?baud=9600&parity=even").unwrap();
        assert_eq!(ep.address_with_metadata(), "/dev/ttyUSB0?baud=9600&parity=even");
    }

    #[test]
    fn endpoint_address_with_metadata_no_query() {
        let ep = Endpoint::try_from("serial//dev/ttyUSB0").unwrap();
        assert_eq!(ep.address_with_metadata(), "/dev/ttyUSB0");
    }
}
