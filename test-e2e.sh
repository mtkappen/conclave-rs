#!/bin/bash
# End-to-end test for Conclave P2P campaign sync
# Run this script to test the full workflow between two CLI instances

set -e

echo "=== Conclave E2E Test ==="
echo ""

# Clean up any previous test data
rm -rf /tmp/conclave-test-1 /tmp/conclave-test-2

# Create test directories
mkdir -p /tmp/conclave-test-1 /tmp/conclave-test-2

echo "Step 1: Initialize identities for both peers..."
export XDG_DATA_HOME=/tmp/conclave-test-1
./target/debug/conclave init --name "DM Player" > /dev/null
echo "✓ Peer 1 identity created"

export XDG_DATA_HOME=/tmp/conclave-test-2
./target/debug/conclave init --name "Player One" > /dev/null
echo "✓ Peer 2 identity created"

echo ""
echo "Step 2: Create campaign on Peer 1..."
export XDG_DATA_HOME=/tmp/conclave-test-1
CAMPAIGN_ID=$(./target/debug/conclave new-campaign "Test Campaign" | grep -oP '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
echo "✓ Campaign created: $CAMPAIGN_ID"

echo ""
echo "Step 3: Start Peer 1 listener..."
export XDG_DATA_HOME=/tmp/conclave-test-1
./target/debug/conclave --port 7777 listen &
PEER1_PID=$!
sleep 2
echo "✓ Peer 1 listening on port 7777"

echo ""
echo "Step 4: Join campaign from Peer 2..."
export XDG_DATA_HOME=/tmp/conclave-test-2
./target/debug/conclave join-campaign $CAMPAIGN_ID 127.0.0.1:7777
echo "✓ Peer 2 joined campaign"

echo ""
echo "Step 5: Send chat message from Peer 2..."
export XDG_DATA_HOME=/tmp/conclave-test-2
./target/debug/conclave send-chat $CAMPAIGN_ID "Hello from Peer 2!"
echo "✓ Chat message sent"

sleep 1

echo ""
echo "Step 6: Check members on both peers..."
export XDG_DATA_HOME=/tmp/conclave-test-1
echo "Peer 1 members:"
./target/debug/conclave members $CAMPAIGN_ID

export XDG_DATA_HOME=/tmp/conclave-test-2
echo "Peer 2 members:"
./target/debug/conclave members $CAMPAIGN_ID

echo ""
echo "Step 7: Roll dice from Peer 2..."
export XDG_DATA_HOME=/tmp/conclave-test-2
./target/debug/conclave roll 1d20+5 --campaign $CAMPAIGN_ID
echo "✓ Dice rolled"

echo ""
echo "Step 8: Query remote peer via RPC..."
export XDG_DATA_HOME=/tmp/conclave-test-1
echo "Querying Peer 2 for members:"
./target/debug/conclave rpc-members $CAMPAIGN_ID 127.0.0.1:7778

echo ""
echo "Step 9: Cleanup..."
kill $PEER1_PID 2>/dev/null || true
rm -rf /tmp/conclave-test-1 /tmp/conclave-test-2
echo "✓ Test directories cleaned up"

echo ""
echo "=== E2E Test Complete ==="
echo "All steps passed successfully!"
