// Property-based tests for platform-layer transport.
//
// To use:
//   cargo test --manifest-path platforms/zenoh-std/Cargo.toml --features proptest
//
// Examples of what could be tested here:
// - Endpoint parsing round-trips
// - MTU computation produces valid values
// - Link errors are correctly classified
