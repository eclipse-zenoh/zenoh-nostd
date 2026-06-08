## ADDED Requirements

### Requirement: Auth verification callback

The system SHALL allow server-side verification of authentication credentials during the session handshake.

#### Scenario: Valid credentials accepted
- **WHEN** a client connects with valid username/password
- **THEN** the server SHALL accept the connection and complete the handshake

#### Scenario: Invalid credentials rejected
- **WHEN** a client connects with invalid username/password
- **THEN** the server SHALL close the connection with `AuthRejected` reason

#### Scenario: No verifier configured
- **WHEN** no auth verifier is configured on the server
- **THEN** any auth credentials SHALL be accepted (pass-through, backward compatible)