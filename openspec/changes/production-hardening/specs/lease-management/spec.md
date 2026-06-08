## ADDED Requirements

### Requirement: Session health API

The system SHALL expose the transport connection state and activity timing to the user.

#### Scenario: Query connection state
- **WHEN** the transport is open
- **THEN** `session.is_connected()` SHALL return `true`
- **WHEN** the transport has closed
- **THEN** `session.is_connected()` SHALL return `false`

#### Scenario: Timed lease management in poll()
- **WHEN** `session.poll()` is called
- **THEN** it SHALL compute elapsed time since the last poll
- **AND** SHALL use actual elapsed time for lease/keepalive timer checks

#### Scenario: Background poll for std
- **WHEN** `session.run_background(interval)` is called
- **THEN** a background thread SHALL call `poll()` at the specified interval
- **AND** SHALL stop when the session is dropped or explicitly stopped