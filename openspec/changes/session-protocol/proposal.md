# Change: Session Check + Multicast Declarations Protocol Features

## Why

zenoh-pico supports two protocol features that zenoh-nostd lacks:

1. **`Z_FEATURE_SESSION_CHECK`** (v1.1.0+): Before sending data or a query, the session validates whether a subscriber/publisher/queryable actually exists for the target key expression. This prevents sending messages into the void and enables early error detection.

2. **`Z_FEATURE_MULTICAST_DECLARATIONS`** (v1.0.0+): In multicast transport modes (e.g., raw Ethernet, UDP multicast), peers exchange `Declare` messages for publishers and subscribers. Each peer maintains a local view of remote declarations, enabling interest-based filtering — only forward messages to key expressions that have at least one interested peer.

Without these, nostd sessions are "blind":
- Publishers publish regardless of whether anyone is listening (wasteful on constrained networks)
- Queriers send queries that may never be answered (no validation that a matching queryable exists)
- Multicast transports have no filtering — every message goes to every peer, wasting bandwidth and CPU

## What Changes

### 1. Feature flags

Add to `zenoh-sansio` and `zenoh-nostd`:

```toml
# zenoh-sansio/Cargo.toml
[features]
session-check = []
multicast-declarations = []
```

These are independent feature flags. `multicast-declarations` depends on session-check for validation, but session-check can operate standalone (for unicast transports where remote state is inferred from the transport link).

### 2. zenoh-proto — Declare message types

The Zenoh protocol already defines `Declare` messages in the wire format. Verify and extend the codec support in `zenoh-proto`:

```rust
// Existing declare subtypes (verify roundtrip):
pub enum Declare {
    Publisher { key_expr: KeyExpr<'_> },
    Subscriber { key_expr: KeyExpr<'_> },
    Queryable { key_expr: KeyExpr<'_> },
    // For undeclare:
    UndeclarePublisher { key_expr: KeyExpr<'_> },
    UndeclareSubscriber { key_expr: KeyExpr<'_> },
    UndeclareQueryable { key_expr: KeyExpr<'_> },
}
```

Add:
- Property tests verifying encode/decode roundtrip for all `Declare` subtypes
- Golden vectors for each `Declare` variant (with and without key expression suffix)
- Fuzz target for malformed `Declare` messages
- Wire-format compatibility test: encode nostd Declare → decode with upstream Zenoh, and vice versa

### 3. zenoh-sansio — Session Check state

Add session-local tracking of active declarations:

```rust
#[cfg(feature = "session-check")]
#[derive(Clone, Debug)]
pub struct SessionDeclarations {
    local_publishers: heapless::Vec<KeyExpr<'static>, MAX_DECLARATIONS>,
    local_subscribers: heapless::Vec<KeyExpr<'static>, MAX_DECLARATIONS>,
    local_queryables: heapless::Vec<KeyExpr<'static>, MAX_DECLARATIONS>,
}

#[cfg(feature = "session-check")]
impl SessionDeclarations {
    pub fn has_subscriber(&self, key_expr: &KeyExpr<'_>) -> bool {
        self.local_subscribers.iter()
            .any(|k| k.intersects(key_expr))
    }

    pub fn has_publisher(&self, key_expr: &KeyExpr<'_>) -> bool {
        self.local_publishers.iter()
            .any(|k| k.intersects(key_expr))
    }

    pub fn has_queryable(&self, key_expr: &KeyExpr<'_>) -> bool {
        self.local_queryables.iter()
            .any(|k| k.intersects(key_expr))
    }
}
```

Key design:
- `MAX_DECLARATIONS` is a compile-time constant (configurable via feature or const generic). Default: 64.
- Key expression matching uses `intersects()` (subset/superset/overlap checking), not exact equality — this matches Zenoh's key expression semantics
- The declaration list is bounded (`heapless::Vec` with compile-time capacity) — never allocates

### 4. zenoh-sansio — Session Check validation

Add `SessionCheck` validation to the `zenoh-sansio` session state machine:

```rust
#[cfg(feature = "session-check")]
impl SessionCheck {
    /// Validate that at least one subscriber exists for the given key expression.
    /// Returns Ok(()) if a matching subscriber is registered, or
    /// Err(SessionError::NoMatchingSubscriber) otherwise.
    pub fn validate_publish(&self, key_expr: &KeyExpr<'_>) -> Result<(), SessionError> {
        if self.declarations.has_subscriber(key_expr) {
            Ok(())
        } else {
            Err(SessionError::NoMatchingSubscriber)
        }
    }

    /// Validate that at least one queryable exists for the given key expression.
    pub fn validate_query(&self, key_expr: &KeyExpr<'_>) -> Result<(), SessionError> {
        if self.declarations.has_queryable(key_expr) {
            Ok(())
        } else {
            Err(SessionError::NoMatchingQueryable)
        }
    }

    /// Validate that at least one publisher exists for the given key expression.
    pub fn validate_subscribe(&self, key_expr: &KeyExpr<'_>) -> Result<(), SessionError> {
        if self.declarations.has_publisher(key_expr) {
            Ok(())
        } else {
            Err(SessionError::NoMatchingPublisher)
        }
    }
}
```

Validation is called before sending data/queries/subscriptions:

```rust
#[cfg(feature = "session-check")]
impl Session {
    pub fn put(&mut self, key_expr: &KeyExpr<'_>, value: &[u8]) -> Result<(), SessionError> {
        self.session_check.validate_publish(key_expr)?;
        // Proceed with publish...
    }

    pub fn get(&mut self, key_expr: &KeyExpr<'_>) -> Result<(), SessionError> {
        self.session_check.validate_query(key_expr)?;
        // Proceed with query...
    }
}
```

### 5. zenoh-sansio — Multicast Declarations state

Add tracking for remote peer declarations received over multicast transports:

```rust
#[cfg(feature = "multicast-declarations")]
pub struct PeerDeclarations {
    /// Map from peer ZID to their declared entities.
    declared: heapless::Vec<(ZId, Declarations), MAX_PEERS>,
}

#[cfg(feature = "multicast-declarations")]
impl PeerDeclarations {
    pub fn on_declare(&mut self, peer: ZId, declare: Declare) -> Result<(), SessionError> {
        // Add or update the peer's declarations
        // ...
    }

    pub fn on_undeclare(&mut self, peer: ZId, declare: Declare) -> Result<(), SessionError> {
        // Remove the peer's declaration
        // ...
    }

    /// Check if any known peer has declared interest in the given key expression.
    pub fn has_interest(&self, key_expr: &KeyExpr<'_>, kind: DeclareKind) -> bool {
        self.declared.iter().any(|(_, d)| d.has_interest(key_expr, kind))
    }
}
```

### 6. zenoh-sansio — Declare message flow in multicast transport

When `multicast-declarations` is enabled and the transport is multicast (Ethernet, UDP multicast):

**On declare (local → remote):**

```rust
// When a local subscriber is declared:
session.send_declare(Declare::Subscriber { key_expr: &key_expr })?;
// All multicast peers receive this and track it

// When a local publisher is declared:
session.send_declare(Declare::Publisher { key_expr: &key_expr })?;
```

**On receive (remote → local):**

```rust
// In the transport receive loop:
match message {
    ZenohMessage::Declare(declare) => {
        #[cfg(feature = "multicast-declarations")]
        peer_declarations.on_declare(peer_zid, declare)?;
    }
    // ...
}
```

**Interest-based filtering:**

```rust
#[cfg(feature = "multicast-declarations")]
fn should_forward(&self, message: &ZenohMessage, peer: ZId) -> bool {
    match message {
        ZenohMessage::Put { key_expr, .. } => {
            // Only forward Put to peers that have a matching subscriber
            self.peer_declarations.has_interest(key_expr, DeclareKind::Subscriber)
        }
        // ...
    }
}
```

### 7. zenoh-nostd — Session API integration

Wire session check into the `Session` public API:

```rust
#[cfg(feature = "session-check")]
impl Session {
    /// Attempt to publish, returning error if no subscriber matches.
    pub fn put_checked(&mut self, key_expr: &KeyExpr<'_>, value: &[u8]) -> Result<(), SessionError> {
        self.session_check.validate_publish(key_expr)?;
        self.put(key_expr, value)
    }

    /// Attempt to query, returning error if no queryable matches.
    pub fn get_checked(&mut self, key_expr: &KeyExpr<'_>) -> Result<(), SessionError> {
        self.session_check.validate_query(key_expr)?;
        self.get(key_expr)
    }
}
```

These are separate `_checked` methods rather than modifying the existing `put`/`get` — the check is optional behavior. Users who don't enable `session-check` get the existing unchecked path.

For multicast declarations, wire the declare send/receive into the multicast transport loop:

```rust
#[cfg(feature = "multicast-declarations")]
impl Session {
    /// Called by the transport layer when a Declare message arrives from a peer.
    pub fn handle_declare(&mut self, peer: ZId, declare: Declare) -> Result<(), SessionError> {
        self.peer_declarations.on_declare(peer, declare)?;
        // Notify the session check to update local view
        // ...
        Ok(())
    }
}
```

### 8. Tests

| Test | Scope | Method |
|---|---|---|
| Declare message encode/decode roundtrip | `zenoh-proto` | Unit test + golden vectors for all 6 Declare subtypes |
| Declare message with upstream compatibility | `zenoh-proto` | Encode nostd → decode with upstream zenoh crate |
| Session check: validate publish with subscriber | `zenoh-sansio` | Register subscriber, publish matching keyexpr, expect Ok |
| Session check: validate publish without subscriber | `zenoh-sansio` | Publish without subscriber, expect `NoMatchingSubscriber` error |
| Session check: validate query with queryable | `zenoh-sansio` | Register queryable, query matching keyexpr, expect Ok |
| Session check: validate query without queryable | `zenoh-sansio` | Query without queryable, expect `NoMatchingQueryable` error |
| Session check: key expression intersection | `zenoh-sansio` | Register `a/b/*`, publish to `a/b/c`, expect Ok; publish to `x/y`, expect error |
| Session check: bounded declaration list | `zenoh-sansio` | Add MAX_DECLARATIONS+1 declarations, expect capacity error |
| Peer declarations: track and lookup | `zenoh-sansio` | Multiple peers declare publishers/subscribers, verify interest lookup |
| Peer declarations: undeclare | `zenoh-sansio` | Peer declares then undeclares, verify interest removed |
| Peer declarations: max peers | `zenoh-sansio` | Add MAX_PEERS+1 peers, expect capacity error |
| Multicast declare message flow | `zenoh-nostd` | Create 2 sessions on multicast transport, declare on one, verify other receives declare |
| Multicast interest-based filtering | `zenoh-nostd` | 3 peers on multicast: A declares subscriber for X, B publishes X, C publishes Y. Verify B's Put reaches A but C's Put does not |
| Session check + multicast combined | `zenoh-nostd` | Enable both features, verify session check uses multicast-declared state |

### 9. Fuzzing

Add fuzz target for `Declare` message parsing:
```
fuzz_targets/declare_message.rs
```

### 10. Property tests

```rust
proptest! {
    #[test]
    fn declare_roundtrip(declare: Declare) {
        let encoded = declare.encode();
        let decoded = Declare::decode(&encoded).unwrap();
        assert_eq!(declare, decoded);
    }

    #[test]
    fn malformed_declare_rejected(bytes: Vec<u8>) {
        // Parser never panics on arbitrary bytes
        let _ = Declare::decode(&bytes);
    }

    #[test]
    fn declaration_bounds(declarations: Vec<(KeyExpr, DeclareKind)>) {
        // Adding more than MAX_DECLARATIONS returns capacity error
        let mut state = SessionDeclarations::new();
        for (i, (ke, kind)) in declarations.iter().enumerate() {
            let result = state.add(ke, *kind);
            if i < MAX_DECLARATIONS {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
            }
        }
    }
}
```

### 11. Kani harnesses

Add bounded verification for:
- `SessionDeclarations::add()`: no overflow on bounded vector, capacity invariant holds
- `PeerDeclarations::on_declare()`: no panic, correct peer tracking up to MAX_PEERS
- `SessionCheck::validate_publish()`: terminating intersection check for bounded key expression
- `Declare` parser: no panic for any input up to maximum message length

## Capabilities

### New

- `session-check` — Validate publisher/subscriber/queryable existence before sending data, queries, or subscriptions; returns typed errors when no matching entity exists
- `multicast-declarations` — Exchange `Declare`/`Undeclare` messages for publishers, subscribers, and queryables in multicast transport sessions; peer tracking with bounded declaration storage; interest-based filtering to avoid forwarding messages to uninterested peers

### Modified

- `zenoh-proto`: `Declare` message codec verification, property tests, golden vectors, fuzz target
- `zenoh-sansio`: `SessionDeclarations`, `SessionCheck`, `PeerDeclarations` types (gated); state machine integration for declare message handling and interest-based filtering
- `zenoh-nostd`: `put_checked()`/`get_checked()` API methods; declare message wiring in multicast transport loop

### Unchanged

- Unicast transport paths — session-check for unicast uses local declarations only (no multicast declare exchange needed; the transport link implies peer existence)
- Existing `put()`/`get()` methods continue to work without session check
- Session establishment handshake — no changes to `InitSyn`/`InitAck`/`OpenSyn`/`OpenAck`

## Impact

| Crate | Change | Risk |
|---|---|---|
| `zenoh-proto` | `Declare` message codec verification, golden vectors, fuzz targets | Low — additive, no existing code modified |
| `zenoh-sansio` | Three new state types (gated), declare message handling, interest-based filtering | Low — additive, gated behind feature flags |
| `zenoh-nostd` | API `_checked` methods, declare message wiring in multicast transport | Low — additive optional methods |
| `openspec/protocol-features` | Add session-check and multicast-declarations to feature matrix | None |

## Known Gaps

- Session check uses `intersects()` for key expression matching — this is the Zenoh protocol's intended behavior but means validation is conservative (returns Ok if any overlap exists, even for partial overlap that may not satisfy the sender's intent)
- Multicast declarations are scoped to a single multicast transport — if a session has multiple transports (e.g., TCP + multicast), declarations are not propagated across transports. This matches zenoh-pico behavior
- Interest-based filtering uses exact key expression intersection — no support for advanced filtering (e.g., content-based, time-based). This is deferred
- `MAX_DECLARATIONS` and `MAX_PEERS` are compile-time constants — runtime configurable sizes would require alloc or const generics with expressions (RFC 2000). Deferred until const generics are more expressive
- Session check is a best-effort validation, not a guarantee — a subscriber could undeclare between the check and the publish. This matches upstream Zenoh behavior (session check is advisory, not transactional)
- No `Declare` message retransmission on transport loss — if multicast declarations are lost, peers may have stale views. A periodic re-declaration mechanism is deferred to future work