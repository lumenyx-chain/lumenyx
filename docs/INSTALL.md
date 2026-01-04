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

## Build from Source

### 1. Install Dependencies

#### Ubuntu/Debian
```bash
sudo apt update
sudo apt install -y build-essential git clang curl libssl-dev llvm libudev-dev protobuf-compiler

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

#### macOS
```bash
brew install openssl protobuf
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. Clone and Build
```bash
git clone https://github.com/lumenyx-chain/lumenyx
cd lumenyx
cargo build --release
```

Build takes 10-30 minutes depending on hardware.

### 3. Run

#### Full Node (sync only)
```bash
./target/release/lumenyx-node --chain mainnet --name "my-node"
```

#### Miner (produces blocks)
```bash
./target/release/lumenyx-node --chain mainnet --mine --name "my-miner"
```

## Configuration Options

| Flag | Description |
|------|-------------|
| `--chain mainnet` | Use mainnet (default) |
| `--chain dev` | Development mode (local) |
| `--mine` | Enable mining |
| `--name "name"` | Node name (shows in telemetry) |
| `--rpc-cors all` | Allow RPC from any origin |
| `--rpc-external` | Expose RPC externally |

## Verify Installation
```bash
# Check version
./target/release/lumenyx-node --version

# Run in dev mode (instant blocks, local only)
./target/release/lumenyx-node --dev --tmp
```

You should see:
```
🔷 GHOSTDAG: K=18, target=1000ms, difficulty=100
⛏️  Starting GHOSTDAG block production...
✅ Block #1 imported!
```

## Troubleshooting

### Build fails with memory error
Increase swap or use `cargo build --release -j 2` (fewer parallel jobs)

### Cannot connect to peers
Check firewall - port 30333 must be open for P2P

### RPC not accessible
Add `--rpc-external --rpc-cors all`

## Data Directories

| OS | Path |
|----|------|
| Linux | `~/.local/share/lumenyx-node/` |
| macOS | `~/Library/Application Support/lumenyx-node/` |
| Windows | `%APPDATA%\lumenyx-node\` |

To reset: delete the data directory and restart.
