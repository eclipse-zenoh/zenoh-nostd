# Formal Verification Specification

## Overview

Formal verification for `zenoh-nostd` uses three tools targeting different assurance levels: Kani (bounded model checking — active, 69 proofs), Creusot (deductive verification — not started), and Verus (full program verification — not started).

---

## ADDED Requirements

### Requirement: Creusot contracts for pure functions

The system SHALL annotate pure functions with Creusot contracts (preconditions, postconditions, invariants).

#### Scenario: Length calculation contracts

GIVEN a type implementing `ZLen`
WHEN `z_len()` is called
THEN Creusot SHALL verify the returned length is ≥ the actual encoded byte count
AND SHALL verify the returned length is ≤ a known maximum for the type

#### Scenario: Bounds check contracts

GIVEN a buffer read/write operation
WHEN the operation is called
THEN Creusot SHALL verify that index accesses stay within the declared buffer capacity
AND SHALL verify that cursor advancement never exceeds the buffer size

#### Scenario: SN arithmetic contracts

GIVEN a sequence number increment
WHEN SN wrapping occurs at u32::MAX
THEN Creusot SHALL verify the epoch counter increments exactly once
AND SHALL verify the combined (epoch, sn) key is monotonic

### Requirement: Verus proofs for protocol kernels

The system SHALL verify critical protocol kernels with Verus.

#### Scenario: Session state machine

GIVEN a session state machine with states `{Closed, Opening, Open, Degraded, Closing, Failed}`
WHEN any valid event is processed
THEN Verus SHALL verify the resulting state is a valid next state per the transition table
AND SHALL verify invalid transitions return a typed error
AND SHALL verify the machine never enters an unreachable state

#### Scenario: Packet length invariants

GIVEN any encoded protocol message
WHEN measuring its total encoded length
THEN Verus SHALL verify the total length does not exceed the declared maximum for that message type
AND SHALL verify the individual field offsets do not overlap

#### Scenario: Parser cursor progress

GIVEN a decoder processing a buffer
WHEN any decode step completes
THEN Verus SHALL verify the cursor has advanced by at least 1 byte
AND SHALL verify the cursor never exceeds the buffer length

### Requirement: Proof completion targets

The system SHALL achieve the following proof completion milestones:

#### Scenario: Proof counts

GIVEN the current proof inventory
WHEN tracking completion
THEN Kani SHALL have ≥100 proofs (currently 69)
AND Creusot SHALL have ≥20 contracted functions (currently 0)
AND Verus SHALL have ≥5 verified kernels (currently 0)

#### Scenario: CI integration

GIVEN a CI pipeline run
WHEN formal verification tools are available
THEN `cargo kani -p zenoh-sansio` SHALL run and pass
AND `cargo creusot -p zenoh-sansio --features creusot` SHALL run and pass
AND `cargo verus -p zenoh-sansio --features verus` SHALL run and pass

---

## MODIFIED Requirements

### Requirement: Kani proof maintenance

The system SHALL maintain existing Kani proofs.

#### Scenario: Kani regression

GIVEN a code change affecting verified modules
WHEN CI runs `cargo kani -p zenoh-sansio`
THEN all 69 existing Kani proofs SHALL continue to pass
AND any new proofs SHALL be added for modified or added logic