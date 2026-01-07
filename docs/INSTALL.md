# LUMENYX Installation

---

## Requirements

### Minimum
- CPU: 4 cores
- RAM: 8 GB
- Storage: 100 GB SSD
- Network: 10 Mbps

### Recommended
- CPU: 8+ cores
- RAM: 16 GB
- Storage: 500 GB NVMe SSD
- Network: 100 Mbps

---

## Build

### Ubuntu/Debian
```bash
# Install dependencies
sudo apt update
sudo apt install -y build-essential git clang curl libssl-dev llvm libudev-dev protobuf-compiler

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build
git clone https://github.com/lumenyx-chain/lumenyx
cd lumenyx
cargo build --release
```

### macOS
```bash
brew install openssl protobuf
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

git clone https://github.com/lumenyx-chain/lumenyx
cd lumenyx
cargo build --release
```

Build takes 10-30 minutes.

---

## Run

### Full Node (sync only)
```bash
./target/release/lumenyx-node --chain mainnet --name "your-node"
```

### Miner
```bash
./target/release/lumenyx-node --chain mainnet --mine --name "your-miner"
```

That's it. You're mining.

---

## Options

| Flag | Description |
|------|-------------|
| `--chain mainnet` | Mainnet |
| `--chain dev` | Development mode |
| `--mine` | Enable mining |
| `--name "name"` | Node name |
| `--rpc-cors all` | Allow RPC from any origin |
| `--rpc-external` | Expose RPC externally |

---

## Verify
```bash
./target/release/lumenyx-node --version
```

---

## Data Directories

| OS | Path |
|----|------|
| Linux | `~/.local/share/lumenyx-node/` |
| macOS | `~/Library/Application Support/lumenyx-node/` |
| Windows | `%APPDATA%\lumenyx-node\` |

To reset: delete the data directory and restart.

---

## Troubleshooting

**Build fails with memory error**
```bash
cargo build --release -j 2
```

**Cannot connect to peers**
- Check firewall: port 30333 must be open

**RPC not accessible**
```bash
--rpc-external --rpc-cors all
```

---

No registration. No staking. No permission.

Just run and mine.
