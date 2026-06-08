# Protocol Compatibility

This document tracks compatibility between `zenoh-nostd` and the upstream Eclipse Zenoh Rust implementation.

---

## Compatibility Status

| Feature | Upstream | zenoh-nostd | Status |
|---------|----------|-------------|--------|
| VLE encoding | ✅ | ✅ | Compatible |
| Key Expression | ✅ | ✅ | Compatible |
| ZBuf encoding | ✅ | ✅ | Compatible |
| Transport layer | ✅ | ✅ | Compatible |
| Session handshake | ✅ | ✅ | Compatible |
| Pub/Sub | ✅ | ✅ | Compatible |
| Query/Reply | ✅ | ✅ | Fully implemented — encode, decode, roundtrip, session dispatch |
| Liveliness | ✅ | ✅ | Fully implemented — token declare/undeclare, subscriber, lease |
| Fragmentation | ✅ | ✅ | Compatible |
| Reliability | ✅ | 🔶 | Partial — in progress |
| Batching | ✅ | ✅ | Compatible |
| WebSocket transport | ✅ | 🔶 | Platform-dependent |
| Serial transport | ⬜ | ⬜ | Not in upstream |
| Access control | ✅ | ✅ | Static rule engine, FromStr parsing (alloc), integration |

---

## Compatibility Testing

### Roundtrip Tests

```text
zenoh-nostd encode → upstream decode
upstream encode → zenoh-nostd decode
zenoh-nostd state transition → expected upstream behavior
zenoh-nostd error handling → documented protocol behavior
```

### Golden Vectors

Golden vectors are stored in `tests/golden_vectors/`. Each vector captures:

- Protocol feature name
- Input bytes
- Expected decoded structure
- Expected error (if invalid)
- Source upstream version
- Encoding tool used

---

## Known Incompatibilities

| Issue | Impact | Status |
|-------|--------|--------|
| Session metadata extension | Non-critical fields may differ | Tolerated |
| Lease timing granularity | Sub-millisecond differences in embedded | Tolerated |
| Query interval semantics | Exact matching may differ | Under investigation |

---

## Version Tracking

| Element | Version |
|---------|---------|
| Zenoh protocol version | 9 |
| Wire format revision | 1 |
| zenoh-nostd crate version | 0.1.0 |
| Upstream reference | Eclipse Zenoh Rust git main |