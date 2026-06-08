## ADDED Requirements

### Requirement: Peer-to-peer TCP unicast transport

The system SHALL support peer-to-peer TCP unicast transport mode where two Zenoh nodes establish a bidirectional connection with mutual session negotiation, without requiring a client-server topology.

#### Scenario: Peer connect initiation

- **WHEN** a peer initiates a TCP connection to another peer using `peer_connect`
- **THEN** the transport SHALL use `WhatAmI::Peer` in the InitSyn message
- **AND** SHALL wait for an InitAck from the remote peer
- **AND** SHALL complete the session handshake (InitSyn → InitAck, OpenSyn → OpenAck)

#### Scenario: Peer listen acceptance

- **WHEN** a peer accepts an incoming TCP connection using `peer_listen`
- **THEN** the transport SHALL wait for an InitSyn with `WhatAmI::Peer`
- **AND** SHALL respond with an InitAck
- **AND** SHALL complete the session handshake

#### Scenario: Simultaneous open resolution

- **WHEN** both peers simultaneously send InitSyn as `WhatAmI::Peer`
- **THEN** the transport SHALL compare ZIDs using unsigned byte-by-byte comparison
- **AND** the peer with the lower ZID SHALL send InitAck and continue as the "listening" side
- **AND** the peer with the higher ZID SHALL drop its own InitSyn attempt and continue from the received InitAck

#### Scenario: Peer connect/listen convenience API

- **WHEN** a user calls `TransportBuilder::peer_connect()`
- **THEN** it SHALL be equivalent to `builder.with_whatami(WhatAmI::Peer).connect(...)`
- **WHEN** a user calls `TransportBuilder::peer_listen()`
- **THEN** it SHALL be equivalent to `builder.with_whatami(WhatAmI::Peer).listen(...)`

#### Scenario: Session-level peer API

- **WHEN** a user calls `session_peer_connect()` or `session_peer_listen()`
- **THEN** the Session SHALL be configured with `WhatAmI::Peer` transport
- **AND** SHALL return a `Session` ready for pub/sub/query operations

#### Scenario: Peer mode feature gate

- **WHEN** the `peer-mode` feature is disabled
- **THEN** the session-level peer API SHALL NOT be available (compile-time gated)
- **WHEN** the `peer-mode` feature is enabled
- **THEN** the session-level peer API SHALL be available