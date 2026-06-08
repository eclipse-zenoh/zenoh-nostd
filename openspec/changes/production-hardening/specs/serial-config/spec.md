## ADDED Requirements

### Requirement: Configurable serial port parameters

The system SHALL parse baud rate, parity, data bits, and stop bits from serial endpoint URIs and apply them to the serial port.

#### Scenario: Full serial configuration from URI
- **WHEN** connecting to `serial:///dev/ttyUSB0?baud=115200&parity=even&bits=7&stop=2`
- **THEN** the serial port SHALL be configured with 115200 baud, even parity, 7 data bits, 2 stop bits

#### Scenario: Default serial configuration
- **WHEN** connecting to `serial:///dev/ttyUSB0` without query parameters
- **THEN** the serial port SHALL use defaults (115200, 8N1)

#### Scenario: Partial serial configuration
- **WHEN** connecting to `serial:///dev/ttyS0?baud=9600`
- **THEN** the serial port SHALL use 9600 baud with defaults for parity, data bits, stop bits