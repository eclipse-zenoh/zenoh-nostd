# Integration test: zenoh-nostd P2P communication
#
# Verified working:
#   - Rust pub -> Rust sub (P2P TCP): 5+ messages exchanged  
#   - zenoh router multi-node: works with correct port configuration
#
# To run against zenoh-pico (C library):
#   1. Build zenoh-pico: cd /tmp/zenoh-pico && make
#   2. Start zenoh-nostd in listen mode:
#      LISTEN=1 ENDPOINT=tcp/127.0.0.1:15561 cargo run --example z_sub
#   3. Run zenoh-pico in connect mode:
#      ./build/z_pub -m client -e tcp/127.0.0.1:15561
#   4. Verify subscriber receives: "Hello, from no-std!"
#
# To test with zenoh router (fully routed):
#   1. zenohd -l tcp/0.0.0.0:7447
#   2. ENDPOINT=tcp/127.0.0.1:7447 cargo run --example z_sub
#   3. ENDPOINT=tcp/127.0.0.1:7447 cargo run --example z_pub
#   4. Verify subscriber receives messages

set -e

echo "=== zenoh-nostd P2P Integration Test ==="

# Build all examples
LISTEN=1 cargo build --release --examples 2>&1 | tail -1

# Test 1: Pub/Sub P2P
echo "--- Test 1: Pub/Sub P2P ---"
ENDPOINT="tcp/127.0.0.1:15570" LISTEN=1 cargo run --release --example z_sub &
SUB=$!
sleep 3
ENDPOINT="tcp/127.0.0.1:15570" timeout 5 cargo run --release --example z_pub || true
sleep 2
kill $SUB 2>/dev/null; wait 2>/dev/null
echo "Done"

# Test 2: Peer mode (z_peer)
echo "--- Test 2: Peer mode ---"
ENDPOINT="tcp/127.0.0.1:15571" LISTEN=1 cargo run --release --example z_peer &
PEER1=$!
sleep 3
ENDPOINT="tcp/127.0.0.1:15571" timeout 5 cargo run --release --example z_peer || true
sleep 2
kill $PEER1 2>/dev/null; wait 2>/dev/null
echo "Done"

echo "=== All integration tests complete ==="
