#!/bin/bash
# LUMENYX Pre-Mining Script
# For founder mining: ~12 days before mainnet launch
# Target: 840,000 LUMENYX across 10 wallets

set -e

echo "╔════════════════════════════════════════╗"
echo "║       LUMENYX Pre-Mining Script          ║"
echo "║     27-29 December 2025                ║"
echo "╚════════════════════════════════════════╝"
echo ""

# Configuration
BINARY="./target/release/lumenyx-node"
CHAIN="dev"  # Change to your private chain for pre-mining
KEYS_DIR="./keys"

# Check binary
if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found at $BINARY"
    echo "Run: cargo build --release"
    exit 1
fi

# Check keys
if [ ! -d "$KEYS_DIR" ]; then
    echo "❌ Keys directory not found at $KEYS_DIR"
    echo "Run: ./scripts/generate-keys.sh"
    exit 1
fi

echo "⚠️  PRE-MINING CHECKLIST:"
echo ""
echo "[ ] VPN active (Mullvad/ProtonVPN)"
echo "[ ] TOR running for extra privacy"
echo "[ ] Keys backed up to encrypted USB (3 copies)"
echo "[ ] Computer disconnected from personal accounts"
echo "[ ] No personal identifiers in git config"
echo ""
read -p "Have you completed all checks? (yes/no): " CONFIRM

if [ "$CONFIRM" != "yes" ]; then
    echo "Aborted. Complete security checklist first."
    exit 1
fi

echo ""
echo "Starting pre-mining node..."
echo "Target: 840,000 LUMENYX (~12 days)"
echo ""

# Start mining node
$BINARY \
    --chain $CHAIN \
    --validator \
    --name "PreMiningNode" \
    --base-path ./pre-mining-data \
    --rpc-cors all \
    --rpc-methods Unsafe \
    --unsafe-rpc-external \
    2>&1 | tee pre-mining.log &

NODE_PID=$!
echo "Node started with PID: $NODE_PID"

echo ""
echo "════════════════════════════════════════"
echo ""
echo "📊 Mining Progress Monitor"
echo ""
echo "Watch with: tail -f pre-mining.log"
echo ""
echo "Expected timeline:"
echo "  Duration: ~12 days before mainnet"
echo "  Blocks: ~350,000 (at 3 sec/block)"
echo "  Reward: 2.4 LUMENYX/block × 350,000 = 840,000 LUMENYX"
echo ""
echo "Stop with: kill $NODE_PID"
echo ""
echo "════════════════════════════════════════"
