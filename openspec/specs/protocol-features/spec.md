# Protocol Features Specification

## Overview

`zenoh-nostd` is a `no_std` implementation of the Zenoh protocol. Several features from the upstream Eclipse Zenoh implementation are not yet implemented: Query/Reply (in progress), Liveliness (not started), and Access Control (not started).

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

---

## REMOVED Requirements

The following requirements are out of scope for the current specification:

- Plugin system support (not applicable for `no_std`)
- EVAL (remote procedure evaluation — complex, deferred)
- Peer-to-peer handshake (simplified, defer to future)
- Mesh routing (complex, defer to future)