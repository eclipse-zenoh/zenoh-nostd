## ADDED Requirements

### Requirement: Serial marked as streamed

The system SHALL report `is_streamed() = true` for serial UART links. Serial is a byte-stream protocol, not datagram.

#### Scenario: Serial stream semantics
- **WHEN** a serial link's `is_streamed()` is queried
- **THEN** it SHALL return `true`

### Requirement: USB CDC marked as streamed

The system SHALL report `is_streamed() = true` for USB CDC ACM links. USB CDC is a byte-stream protocol.

#### Scenario: USB CDC stream semantics
- **WHEN** a USB CDC link's `is_streamed()` is queried
- **THEN** it SHALL return `true`

### Requirement: Multicast transport registered

The system SHALL register the multicast transport module in `StdLinkManager` so it can be used via the `"udp-mcast"` protocol.

#### Scenario: Multicast endpoint parsing
- **WHEN** a user connects with `udp-mcast/239.0.0.1:7447`
- **THEN** the system SHALL route to the multicast link implementation

### Requirement: Auto-reconnect wired into session

The system SHALL wire `AutoReconnectConfig` into the session's run loop so that retryable errors trigger reconnection with exponential backoff.

#### Scenario: Session reconnects on transport error
- **WHEN** a session is configured with `AutoReconnectConfig`
- **AND** a retryable transport error occurs
- **THEN** the session SHALL attempt reconnection with exponential backoff
- **AND** SHALL respect `max_retries` and `max_delay_ms` configuration

### Requirement: Feature flags properly gated

The `multicast-declarations` feature flag SHALL be used in `#[cfg()]` attributes, not just defined in Cargo.toml.

#### Scenario: Feature flag controls compilation
- **WHEN** `multicast-declarations` is disabled
- **THEN** DeclarePublisher/UndeclarePublisher code SHALL be compiled out

### Requirement: IPv6 addresses parsed with brackets

All POSIX-based platform crates SHALL accept IPv6 addresses with brackets (e.g., `[::1]:7447`) in `to_sockaddr()`.

#### Scenario: Bracketed IPv6 parsing
- **WHEN** a user connects to `[::1]:7447`
- **THEN** the address parser SHALL strip brackets and parse the IPv6 address

### Requirement: TCP_NODELAY set on embedded platforms

All embedded TCP link implementations SHALL set `TCP_NODELAY` on connected sockets.

#### Scenario: Nodelay on embedded TCP
- **WHEN** an embedded platform establishes a TCP connection
- **THEN** the system SHALL set `TCP_NODELAY` to disable Nagle's algorithm