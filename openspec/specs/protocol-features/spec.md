# Protocol Features Specification

## Overview

`zenoh-nostd` is a `no_std` implementation of the Zenoh protocol. All zenoh-pico parity features have been implemented, including Query/Reply, Liveliness, Access Control, peer-to-peer handshake, multicast declarations, authentication, reliability (NACK-based), and session checks. The implementation supports transports over POSIX sockets (TCP/UDP), Serial/UART (USB CDC), BLE (BlueZ), TLS, and raw Ethernet.

---

## ADDED Requirements

### Requirement: Query/Reply support

The system SHALL support the Query/Reply protocol pattern.

#### Scenario: Query message encoding

GIVEN a query request with key expression, parameters, and target
WHEN encoding the request
THEN the encoder SHALL produce a valid `Request` message per the Zenoh wire format
AND SHALL correctly encode query parameters, target kind, and timeout
AND SHALL return `Err` if the buffer is too small

#### Scenario: Query message decoding

GIVEN a valid `Request` message byte sequence
WHEN decoding
THEN the decoder SHALL produce the correct query fields (keyexpr, parameters, target, timeout)
AND SHALL reject truncated or malformed query messages with `CodecError`

#### Scenario: Reply message flow

GIVEN a query request
WHEN a matching reply is received
THEN the system SHALL produce a `Response` or `ResponseFinal` message
AND SHALL correctly associate the reply with the pending query
AND SHALL signal query completion on `ResponseFinal`

#### Scenario: Queryable handler API

GIVEN a registered queryable
WHEN a matching query arrives
THEN the handler SHALL receive the query key expression and parameters
AND SHALL be able to send zero or more replies followed by a final response
AND SHALL support both sync and async handler patterns

#### Scenario: Querier API

GIVEN a querier (query builder)
WHEN sending a query
THEN the querier SHALL allow specifying key expression, parameters, target, and timeout
AND SHALL collect replies until `ResponseFinal` or timeout
AND SHALL support both one-shot and streaming query patterns

### Requirement: Liveliness API

The system SHALL support the Zenoh Liveliness protocol for token-based presence detection.

#### Scenario: Liveliness token declaration

GIVEN a liveliness token with a key expression
WHEN declaring the token
THEN the system SHALL send a `Declare` message with the liveliness flag
AND SHALL periodically refresh the token within the lease period
AND SHALL send an `Undeclare` message when the token is dropped

#### Scenario: Liveliness subscriber

GIVEN a subscriber on a liveliness key expression
WHEN a liveliness token appears or disappears
THEN the subscriber SHALL receive a notification with the token's key expression
AND SHALL receive a `TokenAlive` signal on token appearance
AND SHALL receive a `TokenDead` signal on token disappearance or timeout

#### Scenario: Liveliness GET

GIVEN a GET request on a liveliness key expression
WHEN liveliness tokens exist matching that key expression
THEN the GET SHALL return the key expressions of all matching active tokens
AND SHALL NOT return tokens that have expired or been undeclared

### Requirement: Access Control (ACL)

The system SHALL support access control lists for restricting Zenoh operations.

#### Scenario: ACL rule matching

GIVEN an ACL with allow/deny rules
WHEN an operation (pub, sub, query, reply) is attempted on a key expression
THEN the system SHALL check the ACL rules in order
AND SHALL allow the operation if a matching allow rule is found before any deny rule
AND SHALL deny the operation if a matching deny rule is found before any allow rule
AND SHALL default to deny if no rules match

#### Scenario: ACL rule types

GIVEN the ACL configuration
WHEN defining rules
THEN the system SHALL support `Allow` and `Deny` rule types
AND SHALL support rules for each operation type (pub, sub, query, reply)
AND SHALL support key expression patterns with wildcards in rules
AND SHALL support per-rule priority ordering

#### Scenario: ACL configuration

GIVEN an ACL configuration at session open
WHEN establishing a session
THEN the system SHALL load and validate the ACL rules
AND SHALL reject invalid rule configurations with a typed error
AND SHALL apply ACL enforcement to all subsequent operations

### Requirement: Peer-to-peer handshake

The system SHALL support peer-to-peer session establishment.

#### Scenario: Peer handshake Init/Ack

GIVEN two peers configured for peer-to-peer
WHEN establishing a session
THEN the initiator SHALL send a `Init` message with peer-specific fields
AND the responder SHALL reply with a `InitAck`
AND both sides SHALL negotiate capabilities and version
AND the resulting session SHALL support direct pub/sub and query/reply

### Requirement: Serial/UART transport (USB CDC)

The system SHALL support transport over serial links (including USB CDC ACM).

#### Scenario: Serial transport send/receive

GIVEN a serial link (UART or USB CDC)
WHEN sending and receiving Zenoh messages
THEN the transport SHALL use a framed serial protocol
AND SHALL handle flow control via the underlying serial driver
AND SHALL support both synchronous and poll-driven modes

#### Scenario: Serial transport framing

GIVEN a serial transport link
WHEN encoding a batch for serial
THEN the encoder SHALL add frame delimiters for message boundary detection
AND SHALL escape byte sequences that could interfere with framing

### Requirement: BLE transport

The system SHALL support transport over Bluetooth Low Energy (BLE) via BlueZ on Linux.

#### Scenario: BLE transport send/receive

GIVEN a BLE connection via BlueZ
WHEN sending and receiving Zenoh messages
THEN the transport SHALL use GATT notifications for data transfer
AND SHALL handle MTU negotiation and segmentation

### Requirement: TLS transport

The system SHALL support transport over TLS for encrypted communication.

#### Scenario: TLS transport send/receive

GIVEN a TLS-encrypted connection
WHEN establishing a transport session
THEN the transport SHALL perform TLS handshake before Zenoh handshake
AND SHALL encrypt all subsequent Zenoh messages
AND SHALL support both client and server certificate authentication

### Requirement: Raw Ethernet transport

The system SHALL support transport directly over raw Ethernet frames.

#### Scenario: Raw Ethernet framing

GIVEN a raw Ethernet link
WHEN sending a Zenoh batch
THEN the transport SHALL encapsulate the batch in Ethernet II frames
AND SHALL use a configurable EtherType for Zenoh traffic
AND SHALL handle ARP resolution for destination MAC

#### Scenario: Raw Ethernet receive

GIVEN a raw Ethernet socket in promiscuous or bound mode
WHEN receiving Ethernet frames
THEN the transport SHALL filter frames by EtherType
AND SHALL reassemble the Zenoh batch from frame payloads

### Requirement: Auto-reconnect

The system SHALL support automatic reconnection on transport failure.

#### Scenario: Reconnect on disconnect

GIVEN an established session
WHEN the transport connection is lost
THEN the system SHALL attempt to reconnect with configurable retry delay and max attempts
AND SHALL re-establish the session handshake on successful reconnection
AND SHALL notify the application of reconnection events

#### Scenario: Reconnect bounds

GIVEN an auto-reconnect configuration
WHEN the transport repeatedly fails
THEN the system SHALL respect the max retry count
AND SHALL enter a failed state when retries are exhausted
AND SHALL use exponential backoff between attempts

### Requirement: Session check

The system SHALL support periodic session health checks.

#### Scenario: Session check keep-alive

GIVEN an open session
WHEN no messages have been exchanged within the check interval
THEN the system SHALL send a session check (ping) message
AND SHALL expect a session check reply (pong) within a timeout
AND SHALL consider the session dead if no reply is received

### Requirement: Multicast declarations

The system SHALL support multicast-style declarations for peer-to-peer topologies.

#### Scenario: Multicast declare/undeclare

GIVEN a peer-to-peer network
WHEN a subscriber or queryable is declared
THEN the declaration SHALL be multicast to all connected peers
AND SHALL include the key expression and declaration kind
AND undeclaration SHALL similarly be multicast

### Requirement: User/password authentication

The system SHALL support username/password-based authentication.

#### Scenario: Username/password auth during handshake

GIVEN a session configured with user/password authentication
WHEN establishing the session
THEN the client SHALL include the user/password credential in the handshake
AND the server SHALL validate the credential against its configuration
AND SHALL reject the session if authentication fails
AND SHALL proceed with the session if authentication succeeds

### Requirement: TLS client certificate authentication

The system SHALL support TLS client certificate authentication.

#### Scenario: TLS client certificate auth

GIVEN a TLS session configured for mutual TLS
WHEN establishing the TLS connection
THEN the server SHALL request a client certificate
AND the client SHALL present its certificate
AND the server SHALL validate the certificate chain
AND SHALL reject the connection if validation fails

### Requirement: NACK reliability

The system SHALL support NACK-based reliability for lossy transports.

#### Scenario: NACK generation

GIVEN a reliable transport session
WHEN a gap in sequence numbers is detected
THEN the receiver SHALL send a NACK message identifying the missing sequence numbers
AND SHALL do so within a configurable timeout

#### Scenario: NACK response

GIVEN a received NACK message
WHEN the sender processes it
THEN the sender SHALL retransmit the identified messages from its send buffer
AND SHALL maintain a bounded history of sent messages for retransmission

---

## REMOVED Requirements

The following requirements are out of scope for the current specification:

- Plugin system support (not applicable for `no_std`)
- EVAL (remote procedure evaluation — complex, deferred)
- Mesh routing (complex, defer to future)