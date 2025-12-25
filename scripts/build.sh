#!/bin/bash
# LUMENYX Build Script
# Usage: ./scripts/build.sh [release|debug]

set -e

MODE=${1:-release}
PROJECT_ROOT=$(dirname $(dirname $(realpath $0)))

cd "$PROJECT_ROOT"

echo "╔════════════════════════════════════════╗"
echo "║         LUMENYX Build Script             ║"
echo "╚════════════════════════════════════════╝"
echo ""

# Check Rust installation
if ! command -v rustc &> /dev/null; then
    echo "❌ Rust not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
fi

echo "✓ Rust version: $(rustc --version)"

# Check WASM target
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo "Installing WASM target..."
    rustup target add wasm32-unknown-unknown
fi

echo "✓ WASM target installed"
echo ""

# Build
if [ "$MODE" == "release" ]; then
    echo "🔨 Building in RELEASE mode..."
    cargo build --release
    BINARY="target/release/lumenyx-node"
else
    echo "🔨 Building in DEBUG mode..."
    cargo build
    BINARY="target/debug/lumenyx-node"
fi

echo ""
echo "════════════════════════════════════════"
echo "✅ Build complete!"
echo ""
echo "Binary: $BINARY"
echo ""
echo "Run with:"
echo "  $BINARY --dev           # Development mode"
echo "  $BINARY --chain mainnet # Mainnet"
echo "════════════════════════════════════════"
