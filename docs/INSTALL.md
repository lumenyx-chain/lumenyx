# LUMENYX Installation Guide

## System Requirements

### Minimum
- **CPU**: 4 cores
- **RAM**: 8 GB
- **Storage**: 100 GB SSD
- **Network**: 10 Mbps

### Recommended
- **CPU**: 8+ cores
- **RAM**: 16 GB
- **Storage**: 500 GB NVMe SSD
- **Network**: 100 Mbps

## Quick Install (Ubuntu 22.04/24.04)

```bash
# One-line install
curl -sSf https://raw.githubusercontent.com/lumenyx-chain/lumenyx/main/scripts/setup.sh | bash
```

## Manual Installation

### 1. Install System Dependencies

#### Ubuntu/Debian
```bash
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
```

#### Fedora
```bash
sudo dnf install -y \
  clang \
  curl \
  git \
  openssl-devel \
  make \
  protobuf-compiler \
  cmake
```

#### macOS
```bash
xcode-select --install
brew install openssl cmake protobuf
```

### 2. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Verify installation
rustc --version  # Should be 1.75.0 or later
```

### 3. Add WASM Target

```bash
rustup target add wasm32-unknown-unknown
rustup component add rust-src
```

### 4. Clone and Build

```bash
# Clone repository
git clone https://github.com/lumenyx-chain/lumenyx.git
cd lumenyx

# Build release binary
cargo build --release

# Verify build
./target/release/lumenyx-node --version
```

Build takes 10-30 minutes depending on your hardware.

## Running LUMENYX

### Development Mode (Testing)

```bash
./target/release/lumenyx-node --dev
```

This starts a single-node development chain with:
- Pre-funded test accounts (Alice, Bob, etc.)
- Fast block times
- No persistence between restarts

### Mainnet Sync

```bash
./target/release/lumenyx-node --chain mainnet
```

### Common Options

```bash
./target/release/lumenyx-node \
  --chain mainnet \
  --name "MyLumenyxNode" \
  --base-path /data/lumenyx \
  --rpc-port 9933 \
  --ws-port 9944 \
  --port 30333 \
  --rpc-cors all \
  --rpc-methods Safe \
  --rpc-external \
  --ws-external \
  --prometheus-external
```

### Run as System Service

```bash
# Create systemd service
sudo tee /etc/systemd/system/lumenyx.service > /dev/null <<EOF
[Unit]
Description=LUMENYX Node
After=network.target

[Service]
Type=simple
User=lumenyx
ExecStart=/usr/local/bin/lumenyx-node --chain mainnet --name "MyNode"
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl enable lumenyx
sudo systemctl start lumenyx

# Check status
sudo systemctl status lumenyx
journalctl -u lumenyx -f
```

## Ports

| Port | Protocol | Description |
|------|----------|-------------|
| 30333 | TCP | P2P networking |
| 9933 | TCP | HTTP RPC |
| 9944 | TCP | WebSocket RPC |
| 9615 | TCP | Prometheus metrics |

## Firewall Configuration

```bash
# UFW (Ubuntu)
sudo ufw allow 30333/tcp  # P2P
sudo ufw allow 9933/tcp   # RPC (optional, for APIs)
sudo ufw allow 9944/tcp   # WebSocket (optional)
```

## Troubleshooting

### Build Fails

1. **Ensure Rust is up to date**:
   ```bash
   rustup update stable
   ```

2. **Clear cargo cache**:
   ```bash
   cargo clean
   cargo build --release
   ```

3. **Check WASM target**:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

### Node Won't Sync

1. **Check ports are open**:
   ```bash
   sudo netstat -tulpn | grep 30333
   ```

2. **Delete chain data and restart**:
   ```bash
   ./target/release/lumenyx-node purge-chain --chain mainnet -y
   ./target/release/lumenyx-node --chain mainnet
   ```

### Out of Memory

Increase swap or reduce database cache:
```bash
./target/release/lumenyx-node --chain mainnet --db-cache 512
```

