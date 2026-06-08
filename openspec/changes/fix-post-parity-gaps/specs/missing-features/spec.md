## ADDED Requirements

### Requirement: TLS listen path

The system SHALL support accepting TLS connections (server mode), not just initiating them.

#### Scenario: TLS accept
- **WHEN** a TLS listener accepts an incoming connection
- **THEN** the system SHALL perform a TLS handshake using `TlsAcceptor`
- **AND** SHALL return a `StdTlsLink` ready for Zenoh communication

### Requirement: WebSocket listen path

The system SHALL support accepting WebSocket connections (server mode), not just initiating them.

#### Scenario: WebSocket accept
- **WHEN** a WebSocket listener accepts an incoming connection
- **THEN** the system SHALL perform a WebSocket handshake using `tungstenite::accept`
- **AND** SHALL return a `StdWsLink` ready for Zenoh communication

### Requirement: BLE real implementation or feature gate

The BLE transport SHALL either have a real implementation (using `bluer` crate for BlueZ) OR be feature-gated behind `bt` flag to avoid dead code in production builds.

#### Scenario: BLE feature gate
- **WHEN** the `bt` feature is disabled (default)
- **THEN** the BLE module SHALL NOT be compiled
- **WHEN** the `bt` feature is enabled
- **THEN** the BLE link SHALL implement ZLink traits using BlueZ rfcomm sockets

### Requirement: STM32 serial implementation

The STM32 platform crate SHALL implement serial/UART transport using the STM32 HAL.

#### Scenario: STM32 UART link
- **WHEN** a user connects to `serial//dev/usart2` on STM32
- **THEN** the system SHALL initialize the USART peripheral
- **AND** SHALL provide ZLink read/write over UART

### Requirement: Transport integration tests

The system SHALL have end-to-end tests for transport links: TCP connect+listen roundtrip, UDP send+recv, and peer handshake.

#### Scenario: TCP transport test
- **WHEN** a TCP link connects to a listening TCP link
- **THEN** data SHALL be sent and received correctly over the transport

### Requirement: POSIX platform code deduplication

The 7 embedded POSIX platform crates SHALL share common socket/tcp/udp implementation from a single source.

#### Scenario: Shared POSIX code
- **WHEN** a bug is fixed in the shared POSIX TCP implementation
- **THEN** all 7 platforms SHALL receive the fix automatically