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

## Run

After building, make sure you are in the lumenyx folder:
```bash
cd ~/lumenyx
```

**Important:** To connect to the network, you need bootnodes. Get them from [bootnodes.txt](../bootnodes.txt) and add the `--bootnodes` flag to your command.

### Full Node (sync only)
```bash
./target/release/lumenyx-node --chain mainnet --bootnodes "/ip4/IP/tcp/30333/p2p/PEER_ID" --name "your-node"
```

### Miner
```bash
./target/release/lumenyx-node --chain mainnet --validator --bootnodes "/ip4/IP/tcp/30333/p2p/PEER_ID" --name "your-miner"
```

That's it. You're mining.

---

## Options

| Flag | Description |
|------|-------------|
| `--chain mainnet` | Mainnet |
| `--chain dev` | Development mode |
| `--validator` | Enable mining |
| `--name "name"` | Node name |
| `--bootnodes "addr"` | Connect to bootnode |
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

**Build fails with CMake error**
```bash
git submodule update --init --recursive
cargo build --release
```

**Cannot connect to peers**
- Check firewall: port 30333 must be open
- Make sure you have the correct bootnode from [bootnodes.txt](../bootnodes.txt)

**Genesis mismatch / Node won't sync**
```bash
rm -rf ~/.local/share/lumenyx-node/chains/
git submodule update --init --recursive
cargo build --release
```
Then restart your node.

**RPC not accessible**
- Add flags: `--rpc-external --rpc-cors all`

---

## Mining Wallet

When you start with `--validator`, the node automatically creates a mining wallet:

- **Key file:** `~/.local/share/lumenyx-node/miner-key`
- **Address:** Shown in terminal as "Mining rewards to: ..."

**Important:** Back up your `miner-key` file! It contains your private key.

---

No registration. No staking. No permission.

Just run and mine.
