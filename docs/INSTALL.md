# LUMENYX Installation

---

## Requirements

### Minimum (sync-only node)
- CPU: 1+ cores
- RAM: 1 GB
- Storage: 20 GB SSD
- Network: 10 Mbps

### Recommended (mining with pruning)
- CPU: 2+ cores
- RAM: 2-4 GB
- Storage: 30 GB SSD
- Network: 100 Mbps

### Archive Node (full history)
- CPU: 2+ cores
- RAM: 4 GB
- Storage: 100+ GB SSD (grows ~50 GB/month)
- Network: 100 Mbps

---

## Build

### Ubuntu/Debian

**Step 1: Install dependencies**
```bash
sudo apt update && sudo apt install -y build-essential git clang curl libssl-dev llvm libudev-dev protobuf-compiler cmake
```

**Step 2: Install Rust**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**Step 3: Clone and build**
```bash
git clone --recursive https://github.com/lumenyx-chain/lumenyx
cd lumenyx
cargo build --release
```

Build takes 10-30 minutes.

### macOS

**Step 1: Install dependencies**
```bash
brew install openssl protobuf cmake
```

**Step 2: Install Rust**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**Step 3: Clone and build**
```bash
git clone --recursive https://github.com/lumenyx-chain/lumenyx
cd lumenyx
cargo build --release
```

Build takes 10-30 minutes.

---

## Update

If you already have LUMENYX cloned and want to update:

```bash
cd ~/lumenyx
git pull
git submodule update --init --recursive
cargo build --release
```

**Important:** The `git submodule update` step is required. Without it, you may have an outdated RX-LX algorithm and your node will not sync with the network.

After updating, delete old chain data:
```bash
rm -rf ~/.local/share/lumenyx-node/chains/
```

---

## Run

After building, make sure you are in the lumenyx folder:
```bash
cd ~/lumenyx
```

**Important:** To connect to the network, you need bootnodes. Get them from [bootnodes.txt](../bootnodes.txt) and add the `--bootnodes` flag to your command.

---

## Node Types

### 1. Pruned Miner (recommended)

Keeps only ~1 week of history. Lighter storage (~15 GB max).

```bash
./target/release/lumenyx-node \
    --chain mainnet \
    --validator \
    --state-pruning 250000 \
    --blocks-pruning 250000 \
    --bootnodes "/ip4/IP/tcp/30333/p2p/PEER_ID" \
    --name "your-miner"
```

### 2. Full Node (sync only, no mining)

Syncs the blockchain but doesn't mine. Good for wallets and explorers.

```bash
./target/release/lumenyx-node \
    --chain mainnet \
    --state-pruning 250000 \
    --blocks-pruning 250000 \
    --bootnodes "/ip4/IP/tcp/30333/p2p/PEER_ID" \
    --name "your-node"
```

### 3. Archive Node (full history)

Keeps ALL blocks forever. Required to become an official bootnode.

```bash
./target/release/lumenyx-node \
    --chain mainnet \
    --bootnodes "/ip4/IP/tcp/30333/p2p/PEER_ID" \
    --name "your-archive"
```

**Want to become an official bootnode?** 
Run an Archive Node 24/7 with a stable IP and contact us. We'll add your bootnode to the network so others can sync from you.

---

## Options

| Flag | Description |
|------|-------------|
| `--chain mainnet` | Connect to mainnet |
| `--chain dev` | Development mode (local) |
| `--validator` | Enable mining |
| `--name "name"` | Node name (visible to peers) |
| `--bootnodes "addr"` | Connect to bootnode |
| `--state-pruning N` | Keep only last N states (default: archive) |
| `--blocks-pruning N` | Keep only last N blocks (default: archive) |
| `--rpc-cors all` | Allow RPC from any origin |
| `--rpc-external` | Expose RPC externally |
| `--rpc-methods Safe` | Expose only safe RPC methods |

---

## Verify

```bash
./target/release/lumenyx-node --version
```

Should output the current version number.

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

**Build fails with CMake error**
```bash
git submodule update --init --recursive
cargo build --release
```

**Cannot connect to peers**
- Check firewall: port 30333 must be open (TCP)
- Make sure you have the correct bootnode from [bootnodes.txt](../bootnodes.txt)

**Genesis mismatch / Node won't sync**
```bash
rm -rf ~/.local/share/lumenyx-node/chains/
git submodule update --init --recursive
cargo build --release
```
Then restart your node.

**RPC not accessible**
Add flags: `--rpc-external --rpc-cors all`

**High memory usage warning**
If you see "Large pruning window detected", you can ignore it or switch to a smaller pruning value.

---

## Mining Wallet

When you start with `--validator`, the node automatically creates a mining wallet:

- **Key file:** `~/.local/share/lumenyx-node/miner-key`
- **Address:** Shown in terminal as "Mining rewards to: ..."

**Important:** Back up your `miner-key` file! It contains your private key. If you lose it, you lose access to your mined coins.

---

## Summary

| Node Type | Pruning | Mining | Storage | Use Case |
|-----------|---------|--------|---------|----------|
| Pruned Miner | 250000 | Yes | ~15 GB | Normal mining |
| Full Node | 250000 | No | ~15 GB | Wallet, explorer |
| Archive Node | None | Optional | 100+ GB | Bootnode, full history |

---

No registration. No staking. No permission.

Just run and mine.
