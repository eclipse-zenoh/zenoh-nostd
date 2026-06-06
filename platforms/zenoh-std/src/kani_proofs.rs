// Kani proofs for platform-layer transport (TCP, UDP, WebSocket).
//
// To use:
//   1. Install Kani: cargo install --locked kani-verifier && cargo kani setup
//   2. Run: cargo kani -p zenoh-std --features kani

// Examples of what could be verified here:
// - TCP link MTU computation correctness
// - URI parsing/reconstruction for WebSocket connections
// - Socket address conversion (Endpoint -> SocketAddr)
// - Link error handling for each transport type
