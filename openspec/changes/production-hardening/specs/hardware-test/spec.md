## ADDED Requirements

### Requirement: Real device and emulator testing

The system SHALL be tested on actual or emulated target hardware for the primary platforms.

#### Scenario: std TCP against zenohd
- **WHEN** a nostd TCP session connects to an upstream zenohd router
- **THEN** pub/sub messages SHALL be delivered correctly in both directions

#### Scenario: STM32 QEMU codec tests
- **WHEN** running the STM32 example in QEMU
- **THEN** the codec roundtrip tests SHALL pass

#### Scenario: Serial loopback
- **WHEN** two nostd instances communicate over a virtual serial port pair
- **THEN** messages SHALL be delivered correctly