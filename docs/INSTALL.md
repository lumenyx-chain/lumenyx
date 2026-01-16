# LUMENYX Installation

---

## Requirements

### Minimum (sync-only)
- CPU: 1+ cores
- RAM: 1 GB
- Storage: 20 GB SSD
- Network: 10 Mbps

### Recommended (mining)
- CPU: 2+ cores
- RAM: 2-4 GB
- Storage: 50 GB SSD
- Network: 100 Mbps

---

## Build

### Ubuntu/Debian
```bash
# Install dependencies
sudo apt update
sudo apt install -y build-essential git clang curl libssl-dev llvm libudev-dev protobuf-compiler cmake

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build
git clone --recursive https://github.com/lumenyx-chain/lumenyx
cd lumenyx
cargo build --release
```

### macOS
```bash
brew install openssl protobuf cmake
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

git clone --recursive https://github.com/lumenyx-chain/lumenyx
cd lumenyx
cargo build --release
```

Build takes 10-30 minutes.

---

## Run

> **Important:** To connect to the network, you need bootnodes.
> Get them from [`bootnodes.txt`](../bootnodes.txt) and add `--bootnodes <address>` to your command.


### Full Node (sync only)
```bash
./target/release/lumenyx-node --chain mainnet --name "your-node"
```

### Miner
```bash
./target/release/lumenyx-node --chain mainnet --validator --name "your-miner"
```

That's it. You're mining.

---

## Options

| Flag | Description |
|------|-------------|
| `--chain mainnet` | Mainnet |
| `--chain dev` | Development mode |
| `--validator` | Enable mining (validator mode) |
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

---

## Mining Wallet

When you start with `--validator`, the node automatically creates a mining wallet:

- **Key file:** `~/.local/share/lumenyx-node/miner-key`
- **Address:** Shown in terminal as "💰 Mining rewards to: ..."

**Important:** Back up your `miner-key` file! It contains your private key.
