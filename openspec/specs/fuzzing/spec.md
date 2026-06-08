# Fuzzing Specification

## Overview

Fuzz targets exercise protocol decoders with arbitrary byte sequences to find panics, hangs, or memory safety violations. Currently 4 active targets (`transport_decode`, `establishment`, `establishment_client`, `keyexpr`).

---

## ADDED Requirements

### Requirement: decode_frame fuzz target

The system SHALL have a fuzz target for frame-level decoding.

#### Scenario: Frame decoder fuzz

GIVEN an arbitrary byte sequence up to 4096 bytes
WHEN fed to the frame decoder
THEN the decoder SHALL NOT panic
AND SHALL NOT hang (return within bounded time)
AND SHALL return either `Ok(Frame)` or `Err(TransportError)`
AND SHALL NOT produce a valid frame from clearly invalid input

#### Scenario: Frame boundary conditions

GIVEN truncated frames (first N bytes of a valid frame for N < frame length)
WHEN fed to the frame decoder
THEN the decoder SHALL return `Err` (not panic)
AND SHALL NOT interpret incomplete data as a valid frame

### Requirement: decode_message fuzz target

The system SHALL have a fuzz target for message-level decoding.

#### Scenario: Message decoder fuzz

GIVEN an arbitrary byte sequence up to 2048 bytes
WHEN fed to the message decoder (after transport framing)
THEN the decoder SHALL NOT panic
AND SHALL return either `Ok(Message)` or `Err(CodecError)`
AND SHALL NOT produce a valid message from clearly invalid input

#### Scenario: Message type coverage

GIVEN the set of all supported message types
WHEN fuzzing the message decoder
THEN each message type SHALL be reachable from valid fuzz inputs in the initial corpus
AND fuzz inputs SHALL cover InitSyn, InitAck, OpenSyn, OpenAck, Close, Data, Declare, Undeclare

### Requirement: Fuzz target lifecycle

The system SHALL maintain fuzz targets as living artifacts.

#### Scenario: Fuzz compilation

GIVEN a CI run
WHEN checking fuzz targets
THEN `cargo fuzz build` SHALL succeed for all registered targets
AND `fuzz_target_1.rs` SHALL NOT exist as a dead stub

#### Scenario: Fuzz regression

GIVEN a code change affecting decoder logic
WHEN rebuilding fuzz targets
THEN all existing fuzz targets SHALL compile without errors
AND any new decode paths SHALL be covered by either an existing or new fuzz target

---

## MODIFIED Requirements

### Requirement: Fuzz target count

The system SHALL have fuzz targets for all protocol decoders exposed to untrusted input.

#### Scenario: Target inventory

GIVEN the current fuzz target count of 4
WHEN auditing decoder exposure
THEN `transport_decode` SHALL cover batch-level prefixed and raw decode
AND `establishment` SHALL cover server-side session handshake
AND `establishment_client` SHALL cover client-side session handshake
AND `keyexpr` SHALL cover key expression parsing
AND `decode_frame` SHALL be added (planned)
AND `decode_message` SHALL be added (planned)