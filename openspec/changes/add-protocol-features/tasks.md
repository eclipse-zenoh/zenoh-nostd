# Tasks: Add Protocol Features

## Phase 1: Query/Reply

- [x] 1.1 Implement Request message encode/decode in `msgs/request.rs` (exists, verify)
- [x] 1.2 Implement Response message encode/decode in `msgs/response.rs`
- [x] 1.3 Implement ResponseFinal message encode/decode
- [x] 1.4 Add Querier API (`api/session/querier.rs` — partially exists, complete)
- [x] 1.5 Add Queryable handler API
- [x] 1.6 Wire query flow through session state machine
- [x] 1.7 Add unit tests for all query/reply message types
- [x] 1.8 Add proptest roundtrip for Request/Response/ResponseFinal
- [x] 1.9 Add query/reply fuzz target — Request already in decode_message.rs, Response + ResponseFinal added

## Phase 2: Liveliness

- [x] 2.1 Implement Liveliness token declaration (Declare with liveliness flag)
- [x] 2.2 Implement Liveliness subscriber (receive TokenAlive/TokenDead)
- [x] 2.3 Implement Liveliness GET (enumerate active tokens)
- [x] 2.4 Add token refresh/lease management
- [x] 2.5 Add liveliness unit tests
- [x] 2.6 Add liveliness proptest

## Phase 3: Access Control

- [x] 3.1 Implement AclRule and AccessControl structs
- [x] 3.2 Implement rule parsing from config
- [x] 3.3 Implement permission check at session operations
- [x] 3.4 Add ACL unit tests
- [x] 3.5 Add ACL integration test

## Verification

- [x] 4.1 Update feature support matrix with new status
- [x] 4.2 Update protocol_compatibility.md
- [x] 4.3 Add golden vectors for query/reply messages