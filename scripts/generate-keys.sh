#!/bin/bash
# LUMENYX Key Generation Script
# Generates secure wallet keys for pre-mining and storage

set -e

KEYS_DIR=${1:-"./keys"}
NUM_WALLETS=${2:-10}

echo "╔════════════════════════════════════════╗"
echo "║       LUMENYX Key Generator              ║"
echo "╚════════════════════════════════════════╝"
echo ""
echo "⚠️  SECURITY WARNING ⚠️"
echo "These keys control real funds. Store securely!"
echo ""

# Create keys directory
mkdir -p "$KEYS_DIR"
chmod 700 "$KEYS_DIR"

# Check if lumenyx binary exists
BINARY="./target/release/lumenyx-node"
if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found. Building first..."
    cargo build --release
fi

echo "Generating $NUM_WALLETS wallets..."
echo ""

for i in $(seq 1 $NUM_WALLETS); do
    echo "────────────────────────────────────────"
    echo "Wallet $i of $NUM_WALLETS"
    echo ""
    
    # Generate key
    OUTPUT=$($BINARY key generate --output-type json 2>/dev/null || echo "FALLBACK")
    
    if [ "$OUTPUT" == "FALLBACK" ]; then
        # Fallback: use subkey if available
        if command -v subkey &> /dev/null; then
            OUTPUT=$(subkey generate --output-type json)
        else
            echo "Using random generation..."
            SEED=$(openssl rand -hex 32)
            echo "{\"secretSeed\": \"0x$SEED\", \"address\": \"generated_$i\"}" > "$KEYS_DIR/wallet_$i.json"
            continue
        fi
    fi
    
    # Save to file
    echo "$OUTPUT" > "$KEYS_DIR/wallet_$i.json"
    
    # Display (partial)
    ADDRESS=$(echo "$OUTPUT" | grep -o '"address"[^,]*' | head -1 || echo "see file")
    echo "Address: $ADDRESS"
    echo "Saved to: $KEYS_DIR/wallet_$i.json"
    echo ""
done

echo "════════════════════════════════════════"
echo ""
echo "✅ Generated $NUM_WALLETS wallets in $KEYS_DIR/"
echo ""
echo "⚠️  CRITICAL SECURITY STEPS:"
echo ""
echo "1. BACKUP these files to encrypted USB (3 copies)"
echo "2. Store in separate physical locations"
echo "3. DELETE from this computer after backup"
echo "4. NEVER share or expose these files"
echo ""
echo "For pre-mining distribution (900k LUMENYX total):"
echo "  Wallet 1: ~120k LUMENYX"
echo "  Wallet 2: ~110k LUMENYX"
echo "  Wallet 3: ~100k LUMENYX"
echo "  Wallet 4:  ~95k LUMENYX"
echo "  Wallet 5:  ~90k LUMENYX"
echo "  Wallet 6:  ~85k LUMENYX"
echo "  Wallet 7:  ~80k LUMENYX"
echo "  Wallet 8:  ~75k LUMENYX"
echo "  Wallet 9:  ~70k LUMENYX"
echo "  Wallet 10: ~75k LUMENYX"
echo ""
echo "════════════════════════════════════════"
