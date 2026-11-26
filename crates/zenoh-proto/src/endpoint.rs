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
/// Known protocol names; recognition here does not imply adapter support.
pub enum ProtocolId {
    /// TCP byte-stream protocol (`tcp`).
    Tcp,
    /// UDP datagram protocol (`udp`).
    Udp,
    /// WebSocket message protocol (`ws`).
    WebSocket,
    /// Serial protocol name (`serial`); availability depends on the adapter.
    Serial,
}

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
/// Borrowed protocol prefix of an endpoint, such as `tcp` or `ws`.
pub struct Protocol<'a>(&'a str);

impl<'a> Protocol<'a> {
    /// Borrow the underlying UTF-8 text without copying or allocating.
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
/// Borrowed endpoint address. Socket conversion accepts numeric IP addresses and ports, not DNS names.
pub struct Address<'a>(&'a str);

impl<'a> Address<'a> {
    /// Borrow the underlying UTF-8 text without copying or allocating.
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
/// Borrowed `protocol/address` locator, parsed with `Endpoint::try_from`.
///
/// Parsing checks the separator and rejects metadata (`?`) and configuration (`#`).
/// It does not validate the protocol or resolve the address; the adapter does that.
/// Keep the input string alive for the endpoint's lifetime.
///
/// ```
/// use zenoh_proto::Endpoint;
/// let endpoint = Endpoint::try_from("tcp/192.0.2.1:7447")?;
/// assert_eq!(endpoint.protocol().as_str(), "tcp");
/// assert_eq!(endpoint.address().as_str(), "192.0.2.1:7447");
/// assert!(Endpoint::try_from("tcp/192.0.2.1:7447#foo=bar").is_err());
/// # Ok::<(), zenoh_proto::EndpointError>(())
/// ```
pub struct Endpoint<'a> {
    pub(super) inner: &'a str,
}

impl Endpoint<'_> {
    /// Borrow the protocol prefix; this accessor does not check adapter support.
    pub fn protocol(&self) -> Protocol<'_> {
        Protocol(protocol(self.inner))
    }

    /// Borrow the address portion, excluding the protocol separator.
    pub fn address(&self) -> Address<'_> {
        Address(address(self.inner))
    }
}

impl fmt::Display for Endpoint<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.inner)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Endpoint<'_> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "{}", self.inner)
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
                crate::zbail!(crate::EndpointError::MetadataNotSupported)
            }

            (None, Some(cidx)) if cidx > pidx && !s[cidx + 1..].is_empty() => {
                crate::zbail!(crate::EndpointError::ConfigNotSupported)
            }

            (Some(midx), Some(cidx))
                if midx > pidx
                    && cidx > midx
                    && !s[midx + 1..cidx].is_empty()
                    && !s[cidx + 1..].is_empty() =>
            {
                crate::zbail!(crate::EndpointError::MetadataNotSupported)
            }
            _ => Err(crate::EndpointError::MetadataNotSupported),
        }
    }
}
