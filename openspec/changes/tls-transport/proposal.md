# Change: TLS Transport + Authentication (User/Password + Client Certificate)

## Why

Zenoh-pico supports TLS transport via `Z_FEATURE_LINK_TLS` and user/password authentication via config keys `0x43`/`0x44`. The upstream Zenoh Rust crate enables both `transport_tls` and `auth_usrpwd` by default. zenoh-nostd has zero TLS support and zero authentication — session security is entirely absent.

TLS is essential for:
- **Confidentiality**: wire encryption preventing eavesdropping on sensor data, control commands, and configuration
- **Integrity**: tamper detection via MACs in TLS record layer
- **Authentication**: server identity verification via X.509 certificates; mutual TLS (mTLS) for client identity
- **Standards compliance**: production IoT/industrial deployments require TLS; regulatory frameworks (IEC 62443, NIST 800-53) mandate transport encryption

User/password authentication is required for:
- **Access control**: binding Zenoh sessions to authenticated identities for ACL enforcement
- **Multi-tenant deployments**: distinguishing between different clients/peers on shared infrastructure
- **Interoperability with zenoh-pico/zenoh-rs**: the wire format `Auth` extension (ext ID `0x3`) already carries opaque auth payloads in `InitSyn`/`InitAck`/`OpenSyn`/`OpenAck`. User/password credentials are carried in this extension using the Zenoh wire format's existing `Auth` mechanism.

The `Auth` extension already exists in `zenoh-proto` (`exts.rs:227` — `Auth<'a> { payload: &'a [u8] }`). The establishment state machine in `zenoh-sansio` already has `auth: None` fields on all handshake messages. What is missing is: (1) populating these fields, (2) processing incoming auth payloads, (3) the TLS transport itself wrapping TCP with a TLS session, and (4) the TLS-specific config and link types.

## What Changes

### 1. zenoh-proto — TLS endpoint URI scheme

Add `tls` URI scheme to `ProtocolId`:

```rust
// In endpoint.rs
pub enum ProtocolId {
    Tcp,
    Udp,
    WebSocket,
    Serial,
    Tls,
}
```

Endpoint URI format:

```
tls://host:port? sni=<hostname>
tls://host:port#config=my-tls-profile
```

The `tls/` scheme is used by zenoh-pico and zenoh-rs (e.g., `tls/localhost:7447`). The nostd `Endpoint` parser does not use `/` as protocol separator — it uses `://` — so the mapping is: `tls/` in zenoh-pico parlance becomes `tls://` in nostd URI format.

Add `ProtocolId::Tls` variant and `TryFrom<Protocol<'_>>`:

```rust
impl TryFrom<Protocol<'_>> for ProtocolId {
    fn try_from(value: Protocol<'_>) -> Result<Self, Self::Error> {
        Ok(match value.as_str() {
            "tcp" => Self::Tcp,
            "udp" => Self::Udp,
            "ws" => Self::WebSocket,
            "serial" => Self::Serial,
            "tls" => Self::Tls,
            _ => crate::zbail!(crate::EndpointError::CouldNotParseProtocol),
        })
    }
}
```

### 2. zenoh-proto — Auth payload definitions for user/password

The upstream Zenoh wire format carries authentication in the `Auth` extension (ext ID `0x3`). For user/password authentication, the auth payload is formatted as:

```
+--------+--------+--------+--------+--------+--------+
|  locator (zstr) |  user (zstr)  | password (zstr) |
+--------+--------+--------+--------+--------+--------+
```

Each field is a Zenoh string (`zstr`): length-prefixed UTF-8. The `locator` field identifies the authenticator module (e.g., `"usrpwd"`).

Define wire-level auth types:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct UsrPwdAuth<'a> {
    pub user: &'a str,
    pub password: &'a str,
}

impl UsrPwdAuth<'_> {
    /// Wire format: three zstr fields: locator, user, password
    /// locator is always "usrpwd"
    pub fn z_body_len(&self) -> usize {
        // "usrpwd".len() prefixed + user.len() prefixed + password.len() prefixed
        let locator = "usrpwd";
        z_str_len(locator) + z_str_len(self.user) + z_str_len(self.password)
    }

    pub fn z_body_encode(&self, w: &mut impl ZWriteable) -> Result<(), CodecError> {
        z_str_encode("usrpwd", w)?;
        z_str_encode(self.user, w)?;
        z_str_encode(self.password, w)?;
        Ok(())
    }

    pub fn z_body_decode(r: &mut impl ZReadable<'a>) -> Result<Self, CodecError> {
        let locator: &str = z_str_decode(r)?;
        if locator != "usrpwd" {
            crate::zbail!(CodecError::CouldNotParseField)
        }
        let user = z_str_decode(r)?;
        let password = z_str_decode(r)?;
        Ok(Self { user, password })
    }
}
```

Helper functions for zstr encoding (since zenoh strings are length-prefixed):

```rust
fn z_str_len(s: &str) -> usize {
    let bytes = s.as_bytes();
    <usize as ZLen>::z_len(&bytes.len()) + bytes.len()
}

fn z_str_encode(s: &str, w: &mut impl ZWriteable) -> Result<(), CodecError> {
    let bytes = s.as_bytes();
    <usize as ZEncode>::z_encode(&bytes.len(), w)?;
    <&[u8] as ZEncode>::z_encode(&bytes, w)
}

fn z_str_decode<'a>(r: &mut impl ZReadable<'a>) -> Result<&'a str, CodecError> {
    let len = <usize as ZDecode>::z_decode(r)?;
    let bytes = <&[u8] as ZDecode>::z_decode(&mut r.read_slice(len)?)?;
    core::str::from_utf8(bytes).map_err(|_| CodecError::CouldNotParseField)
}
```

### 3. zenoh-proto — Auth status result types

Define authentication result that the session layer can process:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStatus {
    /// Authentication succeeded
    Accepted,
    /// Authentication rejected — close the session
    Rejected,
    /// No authentication configured — skip auth
    NotConfigured,
}
```

### 4. zenoh-sansio — Auth processing in establishment state machine

The `State::poll()` method in `transport/establishment.rs` currently hardcodes `auth: None` on every handshake message. This must become configurable.

Add an auth callback to the state machine:

```rust
pub(crate) struct AuthState {
    /// Whether auth has been verified (prevents replay)
    pub verified: bool,
}

impl State {
    pub(crate) fn poll<'a>(
        &mut self,
        input: (TransportMessage<'a>, &'a [u8]),
        auth_handler: &mut Option<impl FnMut(&[u8]) -> Result<Vec<u8>, TransportError>>,
    ) -> (Option<TransportMessage<'a>>, Option<Description>) {
        // ... when building InitAck, call auth_handler if present
        // ... when receiving InitSyn/InitAck, extract auth ext and validate
    }
}
```

The auth handler takes the peer's `Auth` payload and returns our response payload (or an error to abort). Wire format for the auth exchange:

1. **Initiator** embeds `Auth { payload: encode(UsrPwdAuth { user, password }) }` in `InitSyn`
2. **Responder** extracts `Auth` from `InitSyn`, validates credentials, embeds `Auth { payload: status_response }` in `InitAck` (or sends `Close` with auth failure)
3. **Initiator** validates auth response in `InitAck`, proceeds to `OpenSyn` (may also include `Auth` in `OpenSyn` as confirmation)
4. **Responder** validates confirmation in `OpenSyn`, responds with `OpenAck` (may include final `Auth` status)

The auth payload is opaque to the state machine — it simply passes bytes through. The `Auth` extension in InitSyn/InitAck/OpenSyn/OpenAck already exists at ext ID `0x3` in the wire format. The state machine just needs to plumb it.

### 5. zenoh-sansio — TransportBuilder auth configuration

```rust
impl<Buff> TransportBuilder<Buff> {
    pub fn with_auth(mut self, auth_payload: Vec<u8>) -> Self {
        self.auth_payload = Some(auth_payload);
        self
    }

    pub fn with_auth_handler(
        mut self,
        handler: Box<dyn FnMut(&[u8]) -> Result<Vec<u8>, TransportError>>,
    ) -> Self {
        self.auth_handler = Some(handler);
        self
    }
}
```

### 6. zenoh-nostd — Feature flags

Add to `crates/zenoh-nostd/Cargo.toml`:

```toml
[features]
transport-tls = ["zenoh-sansio/transport-tls"]

# TLS backend selection (mutually exclusive for build, select one):
tls-rustls = ["transport-tls", "dep:rustls", "dep:tokio-rustls"]
tls-embedded-tls = ["transport-tls", "dep:embedded-tls"]
tls-mbedtls = ["transport-tls", "dep:mbedtls"]

# Authentication:
auth-userpass = ["zenoh-sansio/auth-userpass"]

# Convenience:
tls-std = ["transport-tls", "tls-rustls", "std"]
tls-embassy = ["transport-tls", "tls-embedded-tls"]
```

Add to `zenoh-sansio/Cargo.toml`:

```toml
[features]
transport-tls = []
auth-userpass = []
```

### 7. zenoh-nostd — TLS config types

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct TlsConfig {
    // Server authentication:
    pub root_ca_certificate: Option<CertificateRef>,
    pub verify_name: bool,  // CN/SAN hostname verification (default: true)

    // Server-side (listen):
    pub listen_private_key: Option<CertificateRef>,
    pub listen_certificate: Option<CertificateRef>,

    // Client-side (connect):
    pub connect_private_key: Option<CertificateRef>,
    pub connect_certificate: Option<CertificateRef>,

    // Mutual TLS:
    pub enable_mtls: bool,

    // Session management:
    pub sni: Option<heapless::String<256>>,  // SNI hostname
    pub close_on_expiration: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CertificateRef {
    /// Path to PEM/DER file (requires `std` feature)
    Path(heapless::String<256>),
    /// Inline PEM bytes
    Pem(heapless::Vec<u8, 16384>),
    /// Inline DER bytes
    Der(heapless::Vec<u8, 16384>),
}
```

### 8. zenoh-nostd — TlsLink type

Create `crates/zenoh-nostd/src/io/link/tls.rs`:

```rust
#[cfg(feature = "transport-tls")]
pub struct TlsLink<TlsStream: embedded_io_async::Read + embedded_io_async::Write> {
    inner: EmbeddedIOLink<TlsStream, TlsStream>,
    config: TlsConfig,
    negotiated_cipher_suite: Option<CipherSuiteInfo>,
}

#[cfg(feature = "transport-tls")]
impl<TlsStream: embedded_io_async::Read + embedded_io_async::Write> ZLink
    for TlsLink<TlsStream>
{
    type Tx<'link> = EmbeddedIOLinkTx<'link, TlsStream>
    where Self: 'link;
    type Rx<'link> = EmbeddedIOLinkRx<'link, TlsStream>
    where Self: 'link;
    // ... delegate to inner
}
```

TLS is a stream-oriented transport (`is_streamed = true`), so the read path uses `read_exact`. The `EmbeddedIOLink` wrapper handles framing.

The TLS stream is the result of a TLS handshake over a TCP stream. The specific TLS implementation depends on the backend:

- **rustls (std)**: wraps `tokio::net::TcpStream` with `tokio-rustls::TlsConnector`/`TlsAcceptor`
- **embedded-tls (no_std)**: wraps an `embedded-io` Read/Write with `embedded_tls::TlsConfig`
- **mbedtls (no_std)**: wraps an `embedded-io` Read/Write with `mbedtls` SSL context

The `TlsLink` itself is backend-agnostic — it works with any type implementing `embedded_io_async::Read + Write`. The TLS handshake happens before the `TlsLink` is constructed.

### 9. zenoh-nostd — TlsLinkManager and TlsHal trait

```rust
#[cfg(feature = "transport-tls")]
pub trait TlsHal {
    type TlsStream: embedded_io_async::Read + embedded_io_async::Write;
    type Error: Display;

    /// Connect to a TLS server
    async fn connect(
        endpoint: &Endpoint<'_>,
        config: &TlsConfig,
    ) -> Result<Self::TlsStream, Self::Error>;

    /// Accept a TLS connection from a client
    async fn accept(
        endpoint: &Endpoint<'_>,
        config: &TlsConfig,
    ) -> Result<Self::TlsStream, Self::Error>;
}

#[cfg(feature = "transport-tls")]
pub struct TlsLinkManager<Hal: TlsHal> {
    hal: Hal,
    config: TlsConfig,
}

#[cfg(feature = "transport-tls")]
impl<Hal: TlsHal> ZLinkManager for TlsLinkManager<Hal> {
    type Link<'a> = TlsLink<Hal::TlsStream>;

    async fn connect(&self, endpoint: Endpoint<'_>) -> Result<Self::Link<'_>, LinkError> {
        let stream = Hal::connect(&endpoint, &self.config).await?;
        let mtu = 65535; // TLS record layer max is 16384, but Zenoh batch is independent
        Ok(TlsLink {
            inner: EmbeddedIOLink::new(stream, stream, mtu, true),
            config: self.config.clone(),
            negotiated_cipher_suite: None,
        })
    }

    async fn listen(&self, endpoint: Endpoint<'_>) -> Result<Self::Link<'_>, LinkError> {
        let stream = Hal::accept(&endpoint, &self.config).await?;
        let mtu = 65535;
        Ok(TlsLink {
            inner: EmbeddedIOLink::new(stream, stream, mtu, true),
            config: self.config.clone(),
            negotiated_cipher_suite: None,
        })
    }
}
```

### 10. zenoh-nostd — std TLS HAL implementation (rustls + tokio)

```rust
#[cfg(feature = "tls-rustls")]
#[cfg(feature = "std")]
mod tls_hal_std;

#[cfg(feature = "tls-rustls")]
impl TlsHal for StdTlsHal {
    type TlsStream = tokio_rustls::TlsStream<tokio::net::TcpStream>;
    type Error = Box<dyn core::error::Error>;

    async fn connect(endpoint: &Endpoint<'_>, config: &TlsConfig) -> Result<Self::TlsStream, Self::Error> {
        let addr: SocketAddr = endpoint.address().try_into()?;

        // Build rustls client config
        let mut root_store = rustls::RootCertStore::empty();
        match &config.root_ca_certificate {
            Some(CertificateRef::Path(path)) => {
                let certs = rustls_pemfile::certs(&mut std::io::BufReader::new(
                    std::fs::File::open(path.as_str())?
                ))?;
                root_store.add_parsable_certificates(&certs);
            }
            Some(CertificateRef::Pem(pem)) => {
                let certs = rustls_pemfile::certs(&mut &pem[..])?;
                root_store.add_parsable_certificates(&certs);
            }
            Some(CertificateRef::Der(der)) => {
                root_store.add_parsable_certificates(&[der]);
            }
            None => {
                // Use webpki-roots as default CA bundle
                root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
                    rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                        ta.subject,
                        ta.spki,
                        ta.name_constraints,
                    )
                }));
            }
        }

        let mut builder = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store);

        // mTLS: provide client certificate
        if config.enable_mtls {
            let certs = load_certs(&config.connect_certificate)?;
            let key = load_private_key(&config.connect_private_key)?;
            builder = builder.with_client_auth_cert(certs, key)?;
        } else {
            builder = builder.with_no_client_auth();
        }

        let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(builder));
        let tcp = tokio::net::TcpStream::connect(addr).await?;

        let sni = config.sni.as_ref()
            .map(|s| s.as_str())
            .unwrap_or("localhost");

        let stream = connector.connect(
            rustls::ServerName::try_from(sni)?,
            tcp,
        ).await?;

        Ok(stream)
    }

    async fn accept(endpoint: &Endpoint<'_>, config: &TlsConfig) -> Result<Self::TlsStream, Self::Error> {
        let addr: SocketAddr = endpoint.address().try_into()?;

        // Build rustls server config
        let certs = load_certs(&config.listen_certificate)?;
        let key = load_private_key(&config.listen_private_key)?;

        let mut builder = rustls::ServerConfig::builder()
            .with_safe_defaults();

        if config.enable_mtls {
            let mut root_store = rustls::RootCertStore::empty();
            if let Some(ca) = &config.root_ca_certificate {
                let ca_certs = match ca {
                    CertificateRef::Path(p) => rustls_pemfile::certs(&mut std::io::BufReader::new(
                        std::fs::File::open(p.as_str())?
                    ))?,
                    CertificateRef::Pem(p) => rustls_pemfile::certs(&mut &p[..])?,
                    CertificateRef::Der(d) => vec![d.clone().into()],
                };
                root_store.add_parsable_certificates(&ca_certs);
            }
            builder = builder.with_client_cert_verifier(
                std::sync::Arc::new(rustls::server::AllowAnyAuthenticatedClient::new(root_store))
            );
        } else {
            builder = builder.with_no_client_auth();
        }

        let server_config = std::sync::Arc::new(
            builder.with_single_cert(certs, key)?
        );

        let tcp = tokio::net::TcpListener::bind(addr).await?;
        let (tcp_stream, _) = tcp.accept().await?;

        let acceptor = tokio_rustls::TlsAcceptor::from(server_config);
        let stream = acceptor.accept(tcp_stream).await?;

        Ok(stream)
    }
}
```

Helper functions for certificate loading:

```rust
#[cfg(feature = "tls-rustls")]
fn load_certs(cert: &Option<CertificateRef>) -> Result<Vec<rustls::Certificate>, BoxError> {
    match cert {
        Some(CertificateRef::Path(p)) => {
            let f = std::fs::File::open(p.as_str())?;
            Ok(rustls_pemfile::certs(&mut std::io::BufReader::new(f))?
                .into_iter()
                .map(rustls::Certificate)
                .collect())
        }
        Some(CertificateRef::Pem(p)) => {
            Ok(rustls_pemfile::certs(&mut &p[..])?
                .into_iter()
                .map(rustls::Certificate)
                .collect())
        }
        Some(CertificateRef::Der(d)) => {
            Ok(vec![rustls::Certificate(d.to_vec())])
        }
        None => Err("No certificate provided".into()),
    }
}

#[cfg(feature = "tls-rustls")]
fn load_private_key(key: &Option<CertificateRef>) -> Result<rustls::PrivateKey, BoxError> {
    match key {
        Some(CertificateRef::Path(p)) => {
            let f = std::fs::File::open(p.as_str())?;
            let mut keys = rustls_pemfile::pkcs8_private_keys(&mut std::io::BufReader::new(f))?;
            keys.pop().map(rustls::PrivateKey).ok_or_else(|| "No private key found".into())
        }
        Some(CertificateRef::Pem(p)) => {
            let mut keys = rustls_pemfile::pkcs8_private_keys(&mut &p[..])?;
            keys.pop().map(rustls::PrivateKey).ok_or_else(|| "No private key found".into())
        }
        Some(CertificateRef::Der(d)) => {
            Ok(rustls::PrivateKey(d.to_vec()))
        }
        None => Err("No private key provided".into()),
    }
}
```

### 11. zenoh-nostd — Embedded TLS HAL (embedded-tls)

For `no_std` targets, use the `embedded-tls` crate:

```rust
#[cfg(feature = "tls-embedded-tls")]
pub struct EmbeddedTlsHal<Tcp: embedded_io_async::Read + embedded_io_async::Write> {
    _phantom: PhantomData<Tcp>,
}

#[cfg(feature = "tls-embedded-tls")]
impl<Tcp: embedded_io_async::Read + embedded_io_async::Write> TlsHal
    for EmbeddedTlsHal<Tcp>
{
    type TlsStream = embedded_tls::TlsStream<Tcp, embedded_tls::Aes128GcmSha256>;
    type Error = embedded_tls::TlsError;

    async fn connect(endpoint: &Endpoint<'_>, config: &TlsConfig) -> Result<Self::TlsStream, Self::Error> {
        // 1. Open TCP connection (platform-specific, passed in as Tcp)
        // 2. Create TLS client config from CA cert in config.root_ca_certificate
        // 3. Perform TLS handshake
        // 4. Return TlsStream
        todo!()
    }

    async fn accept(endpoint: &Endpoint<'_>, config: &TlsConfig) -> Result<Self::TlsStream, Self::Error> {
        // 1. Accept TCP connection
        // 2. Create TLS server config from listen_certificate + listen_private_key
        // 3. Perform TLS handshake (optionally with mTLS client cert request)
        // 4. Return TlsStream
        todo!()
    }
}
```

The `embedded-tls` crate supports:
- TLS 1.3 client and server
- AES-128-GCM-SHA256, AES-256-GCM-SHA384, CHACHA20-POLY1305-SHA256 cipher suites
- X.509 certificate verification (limited, but sufficient for CA-pinned deployments)
- No heap allocation (all buffers are fixed-size)
- Compatible with `embedded-io` and `embedded-io-async`

For mbedTLS, the implementation follows the same pattern but uses mbedtls-sys or rust-mbedtls bindings.

### 12. zenoh-nostd — TransportLinkManager TLS entry points

Add to `TransportLinkManager`:

```rust
impl<LinkManager> TransportLinkManager<LinkManager> {
    pub async fn tls_connect<Buff>(
        &self,
        endpoint: Endpoint<'_>,
        tls_config: &TlsConfig,
        auth: Option<UsrPwdAuth<'_>>,
        buff: Buff,
    ) -> Result<TransportLink<TlsLink<Hal::TlsStream>, Buff>, TransportLinkError>
    where
        LinkManager: TlsHal,
        Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
    {
        let mut link = TlsLinkManager::new(LinkManager::new(), tls_config.clone())
            .connect(endpoint).await?;

        let connect = async || {
            let streamed = link.is_streamed();
            let mut builder = Transport::builder(buff)
                .with_zid(self.zid)
                .with_whatami(self.whatami)
                .with_lease(self.lease)
                .with_resolution(self.resolution);

            // Wire up auth payload if configured
            if let Some(auth) = auth {
                // Encode auth into Auth extension format
                let auth_bytes = encode_usrpwd_auth(&auth);
                builder = builder.with_auth(auth_bytes);
            }

            builder
                .connect_async(
                    &mut link,
                    async |link, bytes| {
                        if link.is_streamed() {
                            link.read_exact(bytes).await.map(|_| bytes.len())
                        } else {
                            link.read(bytes).await
                        }
                    },
                    async |link, bytes| link.write_all(bytes).await,
                )
                .with_prefixed(streamed)
                .finish_async()
                .await
        };

        let transport = with_timeout(self.open_timeout.try_into().unwrap(), connect())
            .await
            .map_err(|_| TransportLinkError::OpenTimeout)?
            .map_err(|e| e.flatten_map::<TransportLinkError>())?;

        Ok(TransportLink::new(link, transport))
    }
}
```

### 13. zenoh-nostd — User/password auth encoding/decoding

```rust
#[cfg(feature = "auth-userpass")]
pub fn encode_usrpwd_auth(auth: &UsrPwdAuth<'_>) -> heapless::Vec<u8, 512> {
    let mut buf = heapless::Vec::new();
    let mut writer = ZenohWriter::new(&mut buf);
    UsrPwdAuth {
        user: auth.user,
        password: auth.password,
    }.z_body_encode(&mut writer).ok();
    buf
}

#[cfg(feature = "auth-userpass")]
pub fn decode_usrpwd_auth(payload: &[u8]) -> Result<UsrPwdAuth<'_>, CodecError> {
    let mut reader = ZenohReader::new(payload);
    UsrPwdAuth::z_body_decode(&mut reader)
}
```

### 14. zenoh-nostd — Session API integration

Add session-level convenience for TLS + auth:

```rust
#[cfg(feature = "transport-tls")]
pub async fn session_tls_connect<'res, Config>(
    config: Config,
    endpoint: Endpoint<'_>,
    tls_config: TlsConfig,
    auth: Option<UsrPwdAuth<'_>>,
    buff: impl AsMut<[u8]> + AsRef<[u8]> + Clone + 'res,
) -> Result<Session<'res, Config>, TransportLinkError>
where
    Config: SessionConfig,
{
    // Build TransportLinkManager, call tls_connect, wrap in Session
}
```

### 15. Config key mapping

Map zenoh-pico config keys to nostd config fields for compatibility:

| zenoh-pico Key | nostd Config Field | Type |
|---|---|---|
| `0x43` (Z_CONFIG_USER_KEY) | `auth.user` | `heapless::String<64>` |
| `0x44` (Z_CONFIG_PASSWORD_KEY) | `auth.password` | `heapless::String<256>` |
| `0x4B` (TLS_ROOT_CA_CERTIFICATE) | `tls.root_ca_certificate` | `Option<CertificateRef>` |
| `0x4C` (TLS_ROOT_CA_CERTIFICATE_BASE64) | `tls.root_ca_certificate` | `Option<CertificateRef>` |
| `0x4D` (TLS_LISTEN_PRIVATE_KEY) | `tls.listen_private_key` | `Option<CertificateRef>` |
| `0x4E` (TLS_LISTEN_PRIVATE_KEY_BASE64) | `tls.listen_private_key` | `Option<CertificateRef>` |
| `0x4F` (TLS_LISTEN_CERTIFICATE) | `tls.listen_certificate` | `Option<CertificateRef>` |
| `0x50` (TLS_LISTEN_CERTIFICATE_BASE64) | `tls.listen_certificate` | `Option<CertificateRef>` |
| `0x51` (TLS_ENABLE_MTLS) | `tls.enable_mtls` | `bool` |
| `0x52` (TLS_CONNECT_PRIVATE_KEY) | `tls.connect_private_key` | `Option<CertificateRef>` |
| `0x53` (TLS_CONNECT_PRIVATE_KEY_BASE64) | `tls.connect_private_key` | `Option<CertificateRef>` |
| `0x54` (TLS_CONNECT_CERTIFICATE) | `tls.connect_certificate` | `Option<CertificateRef>` |
| `0x55` (TLS_CONNECT_CERTIFICATE_BASE64) | `tls.connect_certificate` | `Option<CertificateRef>` |
| `0x56` (TLS_VERIFY_NAME_ON_CONNECT) | `tls.verify_name` | `bool` |

### 16. TLS-specific concerns

**Session resumption**: TLS session tickets allow resumption without full handshake. The rustls backend supports this via `rustls::ClientConfig::resumption`. For embedded targets where memory is constrained, session resumption may be disabled.

**Cipher suite negotiation**: The TLS backend selects the strongest mutually-supported cipher suite. For rustls, safe defaults are used (`with_safe_defaults()`). For embedded-tls, the cipher suite is a generic parameter (e.g., `Aes128GcmSha256`). The TlsLink should expose the negotiated cipher suite for diagnostics.

**SNI (Server Name Indication)**: The client must send the SNI extension to allow the server to present the correct certificate. Configurable via `TlsConfig.sni`. Defaults to the host portion of the endpoint address.

**Certificate verification modes**:
- **Full verification** (default): verify peer cert against root CA, verify hostname matches CN/SAN
- **Skip hostname verification** (`verify_name = false`): verify certificate chain but skip CN/SAN check (useful for IP-based connections)
- **Skip all verification** (not implemented, security risk — documented as unsupported)

**TLS record layer MTU**: The TLS record layer has a maximum of 16384 bytes. However, Zenoh's batch size is independent of TLS record size — a single Zenoh batch may span multiple TLS records. The `TlsLink` MTU should be set to the TCP MSS minus TLS overhead, typically 1460 - (TLS header + MAC + padding) ≈ 1400 bytes for maximum efficiency. However, for simplicity, MTU can be set to 65535 and fragmentation handled at the TCP/TLS layer.

**Close notify**: TLS normally requires a `close_notify` alert before TCP FIN. The transport should handle graceful shutdown by sending `close_notify` before closing the TCP socket. For abrupt disconnections (e.g., timeout), the TLS session is discarded.

### 17. Tests

| Test | Scope | Method |
|---|---|---|
| TLS endpoint URI parsing | `zenoh-proto` | Unit test + golden vectors |
| `UsrPwdAuth` encode/decode roundtrip | `zenoh-proto` | Unit test |
| Auth payload in InitSyn/InitAck roundtrip | `zenoh-proto` | Encode InitSyn with auth, decode, verify |
| `TlsLink` read/write | `zenoh-nostd` | Mock TLS stream (pre-negotiated), test Zenoh handshake over it |
| `TlsHal` std rustls connect | `zenoh-nostd` | Integration test with `rcgen`-generated self-signed certs, local TCP pair |
| `TlsHal` std rustls accept | `zenoh-nostd` | Integration test, server+client on loopback |
| Full TLS handshake + Zenoh session | `zenoh-nostd` | End-to-end: TLS + InitSyn/InitAck/OpenSyn/OpenAck over TLS link |
| mTLS client auth | `zenoh-nostd` | Test with client certificate required and verified |
| User/password auth via Auth extension | `zenoh-nostd` | UsrPwdAuth payload in InitSyn, verified on responder side |
| User/password auth rejection | `zenoh-nostd` | Invalid credentials → auth failure → close |
| Wrong password rejected | `zenoh-nostd` | Verify session does not open with bad password |
| Certificate verification failure | `zenoh-nostd` | Connect with wrong CA → TLS handshake failure |
| SNI verification | `zenoh-nostd` | Connect with wrong hostname → verify_name=true rejects |

### 18. Fuzzing

Add fuzz target for TLS endpoint parsing and auth payload:

```rust
// fuzz_targets/tls_endpoint.rs
fuzz_tls_endpoint!();  // Fuzz URI parsing with TLS params: host, port, SNI

// fuzz_targets/auth_payload.rs
fuzz_auth_payload!();  // Fuzz UsrPwdAuth decode with arbitrary bytes
```

### 19. Property tests

```rust
proptest! {
    #[test]
    fn usrpwd_auth_roundtrip(user in "\\PC{1,64}", password in "\\PC{1,256}") {
        let auth = UsrPwdAuth { user: &user, password: &password };
        let encoded = encode_usrpwd_auth(&auth);
        let decoded = decode_usrpwd_auth(&encoded).unwrap();
        prop_assert_eq!(auth.user, decoded.user);
        prop_assert_eq!(auth.password, decoded.password);
    }

    #[test]
    fn tls_config_roundtrip(config: TlsConfig) {
        // Verify config serialization across feature boundaries
    }
}
```

### 20. Kani harnesses

Add bounded verification for:
- `UsrPwdAuth` encode/decode: no panic on any input, bounded buffer writes
- Auth payload parsing: no integer overflow in length fields
- CertificateRef path handling: no buffer overrun on long paths
- TlsConfig validation: boolean flags produce valid state combinations

### 21. Golden vectors

Store at `tests/golden_vectors/tls/`:

| Name | Description |
|---|---|
| `init_syn_with_auth.bin` | InitSyn with UsrPwdAuth extension |
| `init_ack_with_auth.bin` | InitAck with auth response |
| `open_syn_with_auth.bin` | OpenSyn carrying auth confirmation |
| `tls_endpoint_uri.json` | Endpoint URI parsing vectors |

Compatibility vectors:
```
zenoh-nostd encode → upstream Zenoh Rust decode (verify Auth ext handled)
upstream Zenoh Rust encode → zenoh-nostd decode (verify Auth ext parsed)
```

## Capabilities

### New

- `tls-transport` — TLS transport wrapping TCP with TLS 1.3/1.2 session, supporting `tls://` endpoint URI scheme
- `user-pass-auth` — User/password authentication via Zenoh wire format `Auth` extension (ext ID 0x3) in InitSyn/InitAck/OpenSyn/OpenAck
- `tls-auth` — TLS certificate-based peer authentication, including mTLS with client certificate validation
- `auth-payload` — Opaque Auth extension payload plumbing through the state machine

### Modified

- `zenoh-proto`: `ProtocolId` gains `Tls` variant; new `UsrPwdAuth` type and auth wire encoding; zstr helpers
- `zenoh-sansio`: State machine gains auth handler plumbing; `TransportBuilder` gains `with_auth()` and `with_auth_handler()`
- `zenoh-nostd`: New `TlsLink`, `TlsLinkManager`, `TlsHal` trait; `TlsConfig` type; auth encoding/decoding; feature flags
- `zenoh-sansio` establishment: Auth payload wired through InitSyn/InitAck/OpenSyn/OpenAck production and consumption

### Unchanged

- `EmbeddedIOLink` — reused as `TlsLink` inner type (TLS wraps a byte stream, just like TCP)
- Session state machine — TLS transport yields the same `TransportLink<_, Buff>` as TCP/serial
- Transport codec — `TransportTx`, `TransportRx`, `Transport` in `zenoh-sansio` unchanged
- Driver, keepalive, lease management — identical to TCP
- `Auth` extension wire format (ext ID `0x3`) — already exists in zenoh-proto, unchanged

## Impact

| Crate | Change | Risk |
|---|---|---|
| `zenoh-proto` | `ProtocolId::Tls` variant, `UsrPwdAuth` type, zstr helpers, golden vectors | Low — additive, no existing code modified |
| `zenoh-sansio` | Auth handler plumbing in state machine, `TransportBuilder` auth methods, new feature flags | Medium — state machine gains auth callback, must not break existing non-auth paths |
| `zenoh-nostd` | `TlsLink`, `TlsLinkManager`, `TlsHal` trait, `TlsConfig` type, auth encoding/decoding, rustls std HAL, embedded-tls HAL, feature flags | Medium-High — new crypto dependencies, platform-specific TLS HAL bindings |
| `zenoh-nostd` (std) | `tls-rustls` backend via `rustls` + `tokio-rustls` + `webpki-roots` | Medium — `rustls` is pure Rust, no C dependencies |
| `zenoh-nostd` (embedded) | `tls-embedded-tls` or `tls-mbedtls` backend | Medium — embedded-tls is pure Rust but limited cipher support; mbedtls requires C toolchain |
| `docs/` | Updated compatibility matrix, TLS configuration guide, auth guide | None |

## Dependencies

### std (tls-rustls)
```toml
[dependencies]
rustls = { version = "0.23", optional = true, default-features = false }
tokio-rustls = { version = "0.26", optional = true, default-features = false }
rustls-pemfile = { version = "2", optional = true }
rustls-webpki = { version = "0.102", optional = true }
webpki-roots = { version = "1", optional = true }
tokio = { version = "1", optional = true, features = ["net", "io-util"] }
```

### no_std (tls-embedded-tls)
```toml
[dependencies]
embedded-tls = { version = "0.2", optional = true, default-features = false, features = ["async"] }
embedded-io-async = { version = "0.6", optional = true }
```

## Phasing

| Phase | Scope | Deliverable |
|---|---|---|
| 1 | `zenoh-proto` TLS URI + auth types | `ProtocolId::Tls`, `UsrPwdAuth`, zstr helpers, golden vectors, fuzz target, property tests |
| 2 | `zenoh-nostd` `TlsConfig` + `TlsHal` trait | Config type, HAL trait, core abstraction with mock for testing |
| 3 | `zenoh-nostd` `TlsLink` + `TlsLinkManager` | Link wrapping `EmbeddedIOLink`, manager delegating to HAL, unit tests with mock TLS stream |
| 4 | `zenoh-sansio` auth plumbing | Auth callback in state machine, `TransportBuilder` auth methods, auth passthrough in handshake |
| 5 | Std TLS HAL (rustls) | `rustls` + `tokio-rustls` implementation, RustlsHal, integration test with self-signed certs |
| 6 | User/password auth flow | Full auth exchange: InitSyn with UsrPwdAuth → InitAck with verification → session open or reject |
| 7 | mTLS support | Client certificate loading, server-side certificate request/verification, `enable_mtls` config |
| 8 | Embedded TLS HAL (embedded-tls) | `embedded-tls` backend for no_std targets, cipher suite selection, minimal cert verification |
| 9 | Session API integration | `session_tls_connect` entry point, config builder pattern, auth config wiring |
| 10 | Verification | Kani harnesses for auth encode/decode, fuzz targets for TLS URI + auth payload, integration test suite, golden vector compatibility with upstream zenoh-rs |
| 11 | Documentation | TLS configuration guide, auth guide, compatibility matrix update, embedded runtime checklist |

## Known Gaps

- `embedded-tls` crate has limited X.509 certificate chain verification (supports root CA pinning but not full chain validation). For production embedded deployments requiring full PKI, mbedTLS or a custom verification callback is recommended.
- `embedded-tls` only supports TLS 1.3. TLS 1.2 fallback is not available on this backend. The rustls backend supports both.
- Session resumption (TLS session tickets) is not implemented in Phase 1. The initial implementation performs a full handshake on every connection. Session resumption can be added in a follow-up phase.
- OCSP stapling and CRL checking are not implemented. Certificate revocation is not detected at the TLS layer. Applications requiring revocation checking should use short-lived certificates with frequent rotation.
- The `Auth` extension payload is opaque bytes — the state machine does not interpret auth data. This means auth failures result in transport-level `Close` messages. There is no application-level error reporting for auth failures beyond "session closed."
- The rustls `with_safe_defaults()` cipher suite selection may not be suitable for all embedded deployments. If hardware crypto acceleration is available, the cipher suite should be configured to prefer hardware-backed algorithms.
- mbedTLS backend is not implemented in the initial phases due to C toolchain requirements and binding complexity. It can be added as a community contribution or follow-up phase.
- On targets without `std` file I/O, certificate loading must use inline PEM/DER bytes. The `CertificateRef::Path` variant is only available with the `std` feature.
- Hostname verification on no_std targets: `embedded-tls` has limited support for SAN matching. The `verify_name` flag controls whether hostname verification is attempted or skipped.