#!/bin/bash
# LUMENYX Mainnet Launch Script
# 25 December 2025, 12:00:00 UTC
# Genesis Block Creation

set -e

echo "╔════════════════════════════════════════╗"
echo "║       LUMENYX MAINNET LAUNCH             ║"
echo "║   25 December 2025, 12:00:00 UTC       ║"
echo "╚════════════════════════════════════════╝"
echo ""

# Configuration
BINARY="./target/release/lumenyx-node"
CHAIN_SPEC="./mainnet-spec.json"
SEED_NODES=""  # Add seed node addresses

# Verify timestamp
LAUNCH_TIMESTAMP=1766664000  # 25 Dec 2025 12:00 UTC
CURRENT_TIMESTAMP=$(date +%s)

if [ $CURRENT_TIMESTAMP -lt $LAUNCH_TIMESTAMP ]; then
    REMAINING=$((LAUNCH_TIMESTAMP - CURRENT_TIMESTAMP))
    HOURS=$((REMAINING / 3600))
    MINS=$(((REMAINING % 3600) / 60))
    echo "⏰ Launch in: ${HOURS}h ${MINS}m"
    echo ""
fi

echo "🚀 LAUNCH CHECKLIST:"
echo ""
echo "[ ] Genesis headline selected (from newspaper)"
echo "[ ] Chain spec updated with headline"
echo "[ ] Binaries compiled (Linux/Mac/Windows)"
echo "[ ] Seed nodes ready"
echo "[ ] GitHub repo ready to publish"
echo "[ ] BitcoinTalk post drafted"
echo "[ ] Whitepaper PDF ready"
echo "[ ] VPN + TOR active"
echo ""
read -p "All checks complete? (yes/no): " CONFIRM

if [ "$CONFIRM" != "yes" ]; then
    echo "Aborted. Complete launch checklist first."
    exit 1
fi

echo ""
echo "════════════════════════════════════════"
echo "Starting LUMENYX Mainnet..."
echo "════════════════════════════════════════"
echo ""

# Generate chain spec if not exists
if [ ! -f "$CHAIN_SPEC" ]; then
    echo "Generating chain specification..."
    $BINARY build-spec --chain mainnet > $CHAIN_SPEC
fi

# Start mainnet node
$BINARY \
    --chain $CHAIN_SPEC \
    --validator \
    --name "LUMENYX-Genesis" \
    --base-path ./mainnet-data \
    --port 30333 \
    --rpc-port 9933 \
    --ws-port 9944 \
    --rpc-cors all \
    --rpc-methods Safe \
    --prometheus-external \
    $SEED_NODES \
    2>&1 | tee mainnet.log &

NODE_PID=$!

echo ""
echo "════════════════════════════════════════"
echo "✅ LUMENYX MAINNET LAUNCHED!"
echo ""
echo "Node PID: $NODE_PID"
echo "Log: mainnet.log"
echo ""
echo "NEXT STEPS (in order):"
echo ""
echo "T+00:05 - Make GitHub repo PUBLIC"
echo "T+00:10 - Post on BitcoinTalk"
echo "T+00:15 - Verify network running"
echo "T+12:00 - Check 12h stability"
echo "T+24:00 - Begin disappearing protocol"
echo ""
echo "════════════════════════════════════════"
echo ""
echo "👻 The legend begins..."
echo ""
