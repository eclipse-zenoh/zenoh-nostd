## ADDED Requirements

### Requirement: No panic in TCP split

The system SHALL NOT panic when splitting a TCP link into TX/RX halves. The `expect()` call on mutex lock SHALL be replaced with `map_err` returning `LinkError::LinkTxFailed`.

#### Scenario: Mutex poisoned during TCP split
- **WHEN** a TCP link's internal mutex is poisoned
- **THEN** `split()` SHALL return a `LinkError` instead of panicking

### Requirement: No panic in UDP read_exact

The system SHALL NOT panic with `unimplemented!()` when `read_exact` is called on a UDP link. `read_exact` SHALL be implemented as a loop calling `read()` until the buffer is full or timeout.

#### Scenario: UDP read_exact with datagram boundaries
- **WHEN** a UDP link receives multiple datagrams
- **THEN** `read_exact` SHALL concatenate them until the buffer is filled

### Requirement: No panic in BLE split

The system SHALL NOT panic with `unimplemented!()` when splitting a BLE link. The BLE module SHALL either implement `split()` correctly or be feature-gated to prevent compilation into production builds.

#### Scenario: BLE feature gating
- **WHEN** the `bt` feature is disabled
- **THEN** the BLE module SHALL NOT be compiled
- **WHEN** the `bt` feature is enabled
- **THEN** the BLE link SHALL implement all ZLink traits without panicking

### Requirement: No panic in transport decode

The system SHALL NOT panic with `expect()` when decoding a transport frame with a missing sequence number. Missing sequence numbers SHALL return a typed error.

#### Scenario: Malformed frame without SN
- **WHEN** a transport frame is decoded without a sequence number
- **THEN** the decoder SHALL return `TransportLinkError::InvalidAttribute` instead of panicking

### Requirement: No panic in fields codec

The system SHALL NOT panic with `unreachable!()` when encoding/decoding unexpected enum variants in fields. Unexpected variants SHALL return `CodecError::UnsupportedVariant`.

#### Scenario: Unknown enum variant in codec
- **WHEN** an unknown enum variant is encountered during encoding
- **THEN** the codec SHALL return `CodecError::UnsupportedVariant` instead of panicking