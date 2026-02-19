#!/bin/bash
set -e

echo "=== Minichain Deployment Test Script ==="
echo

TEST_DIR="/tmp/minichain-test-$$"
MINICHAIN="cargo run --bin minichain --"

cleanup() {
    echo "Cleaning up..."
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

mkdir -p "$TEST_DIR"

echo "Step 1: Initialize blockchain"
$MINICHAIN init --authorities 1 --data-dir "$TEST_DIR/data"
echo

echo "Step 2: Create alice account"
ACCOUNT_OUTPUT=$($MINICHAIN account new --name alice --data-dir "$TEST_DIR/data")
ALICE_ADDR=$(echo "$ACCOUNT_OUTPUT" | grep "Address:" | awk '{print $2}')
echo "  Alice address: $ALICE_ADDR"
echo

echo "Step 3: Mint tokens to alice"
$MINICHAIN account mint --from authority_0 --to "$ALICE_ADDR" --amount 1000000 --data-dir "$TEST_DIR/data"
echo

echo "=== Test 1: Gas limit too low (should fail) ==="
if $MINICHAIN deploy --from alice --source ./contracts/counter.asm --gas-limit 21000 --data-dir "$TEST_DIR/data" 2>&1 | grep -q "Gas limit too low"; then
    echo "✓ Test 1 PASSED: Gas limit too low correctly rejected"
else
    echo "✗ Test 1 FAILED: Should have failed with 'Gas limit too low'"
    exit 1
fi
echo

echo "=== Test 2: Gas limit exactly required (should succeed) ==="
if $MINICHAIN deploy --from alice --source ./contracts/counter.asm --gas-limit 26600 --data-dir "$TEST_DIR/data" 2>&1 | grep -q "Contract deployment submitted"; then
    echo "✓ Test 2 PASSED: Deployment succeeded with exact gas"
else
    echo "✗ Test 2 FAILED: Should have succeeded"
    exit 1
fi
echo

echo "=== Test 3: Higher gas limit (should succeed) ==="
if $MINICHAIN deploy --from alice --source ./contracts/counter.asm --gas-limit 100000 --data-dir "$TEST_DIR/data" 2>&1 | grep -q "Contract deployment submitted"; then
    echo "✓ Test 3 PASSED: Deployment succeeded with higher gas"
else
    echo "✗ Test 3 FAILED: Should have succeeded"
    exit 1
fi
echo

echo "=== All tests passed! ==="
