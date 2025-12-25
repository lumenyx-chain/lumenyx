#!/bin/bash
# LUMENYX Development Setup Script
# Installs all dependencies needed to build and run LUMENYX

set -e

echo "╔════════════════════════════════════════╗"
echo "║       LUMENYX Development Setup          ║"
echo "╚════════════════════════════════════════╝"
echo ""

# Detect OS
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
else
    echo "❌ Unsupported OS: $OSTYPE"
    exit 1
fi

echo "Detected OS: $OS"
echo ""

# Install system dependencies
echo "📦 Installing system dependencies..."

if [ "$OS" == "linux" ]; then
    sudo apt update
    sudo apt install -y \
        build-essential \
        clang \
        curl \
        git \
        libssl-dev \
        llvm \
        libudev-dev \
        make \
        protobuf-compiler \
        cmake \
        pkg-config
elif [ "$OS" == "macos" ]; then
    xcode-select --install 2>/dev/null || true
    brew install openssl cmake protobuf
fi

echo "✓ System dependencies installed"
echo ""

# Install Rust
echo "🦀 Installing Rust..."
if ! command -v rustc &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
else
    echo "✓ Rust already installed: $(rustc --version)"
    rustup update stable
fi

# Add WASM target
echo "🎯 Adding WASM target..."
rustup target add wasm32-unknown-unknown
rustup component add rust-src

echo "✓ WASM target ready"
echo ""

# Verify installation
echo "════════════════════════════════════════"
echo "📋 Installation Summary:"
echo ""
echo "Rust:    $(rustc --version)"
echo "Cargo:   $(cargo --version)"
echo "Clang:   $(clang --version | head -1)"
echo ""
echo "✅ Setup complete! You can now build LUMENYX:"
echo ""
echo "  cd lumenyx"
echo "  cargo build --release"
echo ""
echo "Or use the build script:"
echo ""
echo "  ./scripts/build.sh"
echo ""
echo "════════════════════════════════════════"
