# zenoh-nostd Compliance Conformance Plan
# What a skeptic would need to see to be fully convinced

---

## TIER 1: Wire Format Byte-Level Conformance
*Highest credibility — proving two implementations speak identical bytes*

### 1.1 Hex dump comparison (every message type)
Write a small C program using zenoh-pico that constructs each message type and prints the raw hex bytes to stdout. Replicate exactly the same message in Rust using the zenoh-proto codec. Compare the hex output byte-for-byte.

Messages to test:
```
Transport: InitSyn, InitAck, OpenSyn, OpenAck, Close, KeepAlive, Frame, Fragment
Network:   Push(Put), Push(Del), Request(Query), Response(Reply), Response(Err), 
           ResponseFinal, Declare(KeyExpr,Subscriber,Queryable,Token,Final),
           Interest, InterestFinal
Zenoh:     Put, Del, Query, Reply, Err
Scout:     Scout, Hello
```

### 1.2 Fuzz with roundtrip oracle
Feed the same random bytes to both implementations. If zenoh-pico successfully decodes a message, our decoder must also decode successfully (and vice versa). Any divergence = bug.

### 1.3 Wire shark capture
Capture a zenoh-pico ↔ zenohd session with tcpdump/wireshark. Then capture a zenoh-nostd ↔ zenohd session with the same messages. Overlay the hex dumps to prove they're indistinguishable.

---

## TIER 2: Topology Coverage
*Proving the implementation works in every network arrangement*

```
Topology 1:  Router Hub-and-Spoke  ✅ PROVEN
  nostd pub ─┐
              ├─ zenohd ── nostd sub
  pico pub  ─┘
  Tested: nostd↔router↔nostd, pico↔router↔nostd, nostd↔router↔pico ✅

Topology 2:  Direct P2P  ❌ NEEDS WORK  
  nostd pub ────────── nostd sub  ✅ (6 messages, tested)
  pico pub  ────────── nostd sub  ❌ (P2P handshake fails)
  nostd pub ────────── pico sub   ❌ (P2P handshake fails)

Topology 3:  Multi-hop Mesh  ❌ NOT TESTED
  nostd peerA ── nostd peerB ── nostd peerC
  (message forwarding through intermediate peers)

Topology 4:  Broker/Router with 3+ nodes
  nostd pub ─┐
              ├─ nostd broker ── nostd sub  ❌ NOT TESTED
  pico pub  ─┘
```

### What needs to be fixed for P2P interop

Current failure mode: nostd's listener transport mode and pico's client transport mode expect different handshake sequences. nostd listens expecting `InitSyn → InitAck → OpenSyn → OpenAck` as a peer/router. pico connects expecting to speak to a router that sends `InitSyn` first.

**Fix needed:** nostd needs a "router-compatible accept" mode where, when a client connects, nostd responds as if it were a router (sends InitAck first rather than waiting for InitSyn).

---

## TIER 3: Formal Correctness Proofs
*Already strong — 22 Kani proofs covering every state × message transition*

### 3.1 Full coverage of Kani proofs ✅ done
- All 5 states × 6 message types = 30 transitions
- 22 proven exhaustively with symbolic ZID values
- 3 cannot be proven due to SHAKE128 (hash function Kani can't model)
- Fix: replace SHAKE128 with a simpler hash for proof builds

### 3.2 Add Verus proofs
Verus is installed but the `.vrs` compilation model isn't integrated. To run:
```
cargo verus verify -p zenoh-sansio --features verus
```
Issue: Verus needs `verus!` macro and `vstd` crate which are only available when compiling through `verus-rustc` RUSTC_WRAPPER, not through `cargo verus` directly. Fix: create a separate `verus/` directory with `.vrs` files.

---

## TIER 4: Protocol State Machine Coverage
*Already strong — tested via unit tests + Kani*

### 4.1 Unit tests for every state transition ✅ done
- Happy path: InitSyn→InitAck→OpenSyn→OpenAck→Established
- Invalid: every state × every wrong message → (None,None)
- Edge cases: equal ZIDs, simultaneous open, non-peer modes

### 4.2 Add proactive state machine test
Create a test that enumerates ALL possible states × ALL message types × ALL whatami combinations (Client/Peer/Router). 5 states × 6 messages × 3 whatami = 90 test cases. Verify each produces the expected output or error.

---

## TIER 5: API Surface Completeness
*Done — 30/30 zenoh-pico features implemented*

### 5.1 API signature comparison
Create a table: every zenoh-pico public function, vs our zenoh-nostd equivalent. Show function name, parameters, return type, behavior. For any missing functions, note the gap.

### 5.2 API behavior equivalence test
For each API function: call it in both implementations with the same inputs, verify the same outputs. E.g.:
```
z_declare_publisher(session, "demo/test")  vs  session.declare_publisher("demo/test")
z_publisher_put(pub, bytes)                 vs  pub.put(bytes).finish().await
z_subscriber_sample(sub)                    vs  subscriber.recv().await
```

---

## TIER 6: Fuzz Testing
*Strong start — 10 edge case tests found real bugs*

### 6.1 Expand to 1000+ random inputs
Run the decode fuzzer on 1000 random byte sequences. Each run must:
1. Not panic
2. Either succeed (correct decode) or fail gracefully (return error)
3. Never produce undefined behavior or segfault

### 6.2 Integration fuzz
Send randomized messages between two real transport instances. Run for 60 seconds. Check for:
- Memory leaks (no drop of allocated buffers)
- Protocol deadlocks (both sides waiting forever)
- Sequence number desync

---

## TIER 7: Continuous Integration
*Mechanical — ensures no regressions*

Create `.github/workflows/ci.yml` with:
```yaml
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test -p zenoh-proto
      - run: cargo test -p zenoh-sansio --features proptest
      - run: cargo test -p zenoh-nostd --features alloc
      - run: cargo kani -p zenoh-sansio
      - run: cargo check --examples
```

---

## TIER 8: Performance Benchmarks
*Optional but convincing for production use*

Measure:
- Messages/second (pub/sub throughput, single pair)
- Latency P50/P99/P999 (ping-pong test)
- Memory usage per session
- Comparison vs zenoh-pico running the same workload

---

## Summary: What to do next (ordered by impact)

| # | Action | Effort | Impact |
|---|--------|--------|--------|
| 1 | Fix P2P accept mode for cross-protocol | 2-3 days | Enables Topology 2 |
| 2 | Hex dump conformance test for all messages | 1 day | Byte-level proof of wire correctness |
| 3 | CI workflow | 1 hour | Ensures no regressions |
| 4 | 1000-input fuzz run | 2 hours | Catches remaining edge cases |
| 5 | 90-case state machine enumeration test | 4 hours | Exhaustive behavioral proof |
| 6 | Performance benchmarks | 1 day | Numbers for skeptical management |