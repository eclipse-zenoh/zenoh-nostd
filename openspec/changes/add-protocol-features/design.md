# Design: Query/Reply, Liveliness, and Access Control

## Phase 1: Query/Reply

### Message Types

```
Request  → { key: keyexpr, params: &[u8], target: QueryTarget, timeout: Duration }
Response → { key: keyexpr, value: &[u8], encoding: Encoding }
ResponseFinal → { key: keyexpr }  (no body, signals end of replies)
```

### API Surface

```rust
// Querier
pub struct Querier<'a> { ... }
impl Querier<'a> {
    pub fn get(ke: &keyexpr) -> Self;
    pub fn timeout(self, t: Duration) -> Self;
    pub fn target(self, t: QueryTarget) -> Self;
    pub async fn call(self) -> Result<ReplyStream, SessionError>;
}

// Queryable
pub struct Queryable<'a> { ... }
impl Queryable<'a> {
    pub fn reply(&self, response: Response) -> Result<(), SessionError>;
    pub fn reply_final(&self, response: Response) -> Result<(), SessionError>;
}
```

## Phase 2: Liveleliness

### Token Lifecycle

```
declare token → periodic refresh → undeclare on drop
subscribe   → receive TokenAlive/TokenDead callbacks
get         → enumerate active tokens matching keyexpr
```

## Phase 3: Access Control

### Rule Engine

```rust
enum AclRule {
    Allow { operation: Op, keyexpr: Pattern },
    Deny { operation: Op, keyexpr: Pattern },
}

struct AccessControl {
    rules: Vec<AclRule>,
}

impl AccessControl {
    fn check(&self, op: Op, ke: &keyexpr) -> Result<(), SessionError>;
}
```

## Dependencies

| Phase | Depends On | Risk |
|-------|-----------|------|
| 1: Query/Reply | Declare/Undeclare messages | Low |
| 2: Liveliness | Query/Reply API | Medium |
| 3: Access Control | Session state machine | Low |