This file documents the known interoperability status between zenoh-nostd (Rust) and zenoh-pico (C).

## Summary

| Topology | Status | Notes |
|----------|--------|-------|
| nostd ↔ nostd (P2P) | ✅ | 6 messages exchanged in direct pub/sub test |
| nostd → router → nostd | ✅ | Multi-hop routing works |
| pico → router → nostd | ✅ | 146 pico messages received by nostd subscriber |
| nostd → router → pico | ✅ | 6 nostd messages received by pico subscriber |
| nostd ↔ pico (P2P) | ❌ | Handshake fails: nostd receives InitSyn but write fails |

## P2P Cross-Protocol Issue

When nostd listens and pico connects directly (no router):

1. nostd binds TCP, accepts pico's connection
2. pico sends InitSyn
3. nostd receives InitSyn, processes it successfully
4. nostd tries to send InitAck back → LinkError(102): write fails

The TCP socket appears to close between the read and write. Potential causes:
- pico's session init closes the TCP connection after a timeout waiting for a router-like response
- nostd's async TCP socket handling has a race condition
- Both sides may expect different handshake sequences for client/peer negotiation

The version field in nostd's InitAck was fixed from `0u8` to `zenoh_proto::VERSION` (currently 9) to match the zenoh protocol specification.

## Recommended Fix Approach

The P2P cross-protocol fix requires understanding why pico closes the TCP connection before nostd can send InitAck. Steps:

1. Build pico with ZENOH_LOG=DEBUG to capture pico's transport debugging
2. Add more detailed logging to nostd's transport handshake
3. Test with a TCP proxy (e.g., socat) to observe raw byte exchange
4. Compare wire bytes between working (nostd↔nostd) and failing (nostd↔pico) cases