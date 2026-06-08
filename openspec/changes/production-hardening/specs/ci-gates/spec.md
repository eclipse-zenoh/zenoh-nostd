## ADDED Requirements

### Requirement: CI quality gates

The system SHALL run cross-compilation, security auditing, unsafe audit, and unused dependency checks in CI.

#### Scenario: Cross-compilation
- **WHEN** CI runs on every PR
- **THEN** SHALL cross-compile for aarch64, thumbv7em, riscv32imac, wasm32 targets
- **AND** SHALL fail if any target fails to compile

#### Scenario: Security audit
- **WHEN** CI runs on every PR
- **THEN** SHALL run `cargo deny check`
- **AND** SHALL run `cargo audit`
- **AND** SHALL fail on any advisory or license violation

#### Scenario: Unsafe audit
- **WHEN** CI runs on every PR
- **THEN** SHALL run `cargo geiger`
- **AND** SHALL flag any new unsafe blocks without SAFETY comments

#### Scenario: Unused dependency check
- **WHEN** CI runs on every PR
- **THEN** SHALL run `cargo machete`
- **AND** SHALL fail on any unused dependencies