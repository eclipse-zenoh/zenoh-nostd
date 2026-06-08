# Change: Add Query/Reply, Liveliness, and Access Control

## Why

These three features are marked as missing in the feature support matrix. Query/Reply is partially started. Liveliness and Access Control are not started. All three are core Zenoh protocol capabilities needed for upstream compatibility.

## What Changes

- Implement Query message encoding/decoding (Request, Response, ResponseFinal)
- Implement Querier API for sending queries and collecting replies
- Implement Queryable handler API for receiving and responding to queries
- Implement Liveliness token declaration and subscription
- Implement Liveliness GET for active token enumeration
- Implement Access Control configuration and rule enforcement
- Wire through session API
- Add unit tests, proptest roundtrip, fuzz targets for each
- Spec delta: protocol-features (ADDED: Query/Reply, Liveliness, ACL)

## Impact

- Affected specs: protocol-features
- Affected code: `msgs/request.rs`, `msgs/reply.rs`, `msgs/response.rs`, session API, new ACL module
- Risk: Medium — three new protocol features with compatibility implications
- Effort: Large — estimated 3-4 phases