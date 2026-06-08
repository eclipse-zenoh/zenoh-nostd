## Context

The zenoh-pico parity audit identified 19 gaps across three categories: production panic paths, incorrect behavior, and missing features. All are in code that was written during the rapid parity implementation phase and needs hardening for production use.

## Decisions

### Priority: Panic paths first, behavior second, features third

The 5 panic paths are the most urgent — they will crash a device in the field. The 8 behavior issues cause incorrect protocol operation. The 6 missing features affect functionality.

### Approach: Batch fixes by category

Fix all 5 panic paths together (they're mechanical), then all 8 behavior issues, then the 6 missing features. This minimizes context switching.

### Code deduplication: New `zenoh-platform-posix` crate

The 7 POSIX-based embedded platforms share ~90% of their socket/TCP/UDP code. A shared crate eliminates duplication and ensures consistent fixes across all platforms.

## Migration Plan

None — bug fixes only, no breaking changes to public API.

## Open Questions

- BLE: use `bluer` crate (sync D-Bus) or use `bt-rfcomm` directly?
- TLS Acceptor: requires a server identity (cert+key). Where does the user provide this?
- Do we want feature-gated stubs or real implementations for BLE and STM32 serial?