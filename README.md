# LUMENYX

*I've been working on a new electronic cash system that runs without central control.*

## What is LUMENYX?

**Bitcoin + Ethereum + Kaspa + Zcash in one.**

| Feature | LUMENYX |
|---------|---------|
| Supply | 21,000,000 (fixed forever) |
| Consensus | GHOSTDAG PoW |
| Block time | 1-3 seconds |
| Smart Contracts | EVM compatible (Chain ID: 7777) |
| Privacy | ZK-SNARKs (optional) |
| Premine | Zero |
| Team | None |
| Governance | None |

## Distribution

- **No ICO, no pre-sale, no VC**
- **No team allocation, no foundation**
- **No governance, no admin keys**
- **No premine**

100% of supply distributed via mining rewards over 100+ years.

## Quick Start

### Build
```bash
git clone https://github.com/lumenyx-chain/lumenyx
cd lumenyx
cargo build --release
```

### Run a Full Node (sync only)
```bash
./target/release/lumenyx-node \
  --chain mainnet \
  --name "your-node-name"
```

### Mine LUMENYX
```bash
./target/release/lumenyx-node \
  --chain mainnet \
  --mine \
  --name "your-miner-name"
```

That's it. No registration, no staking, no permission needed. Pure PoW.

## Technical Specifications

| Property | Value |
|----------|-------|
| Consensus | GHOSTDAG (PoW + blockDAG) |
| Block time | 1-3 seconds |
| K parameter | 18 (anticone limit) |
| Hash algorithm | Blake3 |
| Finality | Probabilistic (~18 seconds / 6 blocks) |
| Chain ID (EVM) | 7777 |
| Decimals | 12 |
| Framework | Substrate + Frontier EVM |

## Emission Schedule

| Phase | Block Reward | Duration |
|-------|--------------|----------|
| Genesis | 2.4 LUMENYX | ~12 days |
| Early | 0.3 LUMENYX | ~30 days |
| Standard | 0.25 LUMENYX | Halving every ~4 years |

Early miners take the highest risk. Higher rewards compensate. Same as Bitcoin's early 50 BTC blocks.

## Why GHOSTDAG?

Traditional blockchains (Bitcoin, Ethereum) waste blocks when miners find them simultaneously. GHOSTDAG keeps ALL blocks in a DAG structure:

- **No wasted work** - All valid blocks contribute
- **Fast blocks** - 1-3 seconds without orphan problems
- **Truly permissionless** - Anyone can mine, anytime
- **Never stops** - No validator set, no coordination needed

Like Kaspa, but with EVM smart contracts and ZK privacy.

## Privacy (Optional)

LUMENYX supports optional private transactions using Groth16 ZK-SNARKs:
```bash
# Shield (make private)
python3 scripts/shield.py --amount 100

# Unshield (make public)
python3 scripts/unshield.py --amount 50
```

See [docs/ZK_PRIVACY.md](docs/ZK_PRIVACY.md) for details.

## EVM Compatibility

Deploy any Ethereum smart contract. Connect MetaMask:

| Setting | Value |
|---------|-------|
| Network Name | LUMENYX |
| RPC URL | http://localhost:8545 |
| Chain ID | 7777 |
| Symbol | LUMENYX |

## Genesis Block

> *"Banks ended up in the headlines. Today control over digital money sits in a few hands: founders, large holders, intermediaries and those who write the rules."*

## Principles

1. **21M supply** - Never changes
2. **PoW only** - Anyone can mine
3. **No premine** - Zero allocation
4. **No sudo** - No admin keys
5. **No governance** - Code is law
6. **Permissionless** - No permission needed
7. **Privacy optional** - Exchange-friendly
8. **Launch and disappear** - Satoshi-style

## Building from Source

Requirements:
- Rust (stable)
- Cargo
- LLVM/Clang
```bash
cargo build --release
cargo test --all
```

## Documentation

- [Installation Guide](docs/INSTALL.md)
- [Whitepaper](docs/WHITEPAPER.md)
- [ZK Privacy](docs/ZK_PRIVACY.md)

## License

GPL-3.0

## Disclaimer

No company. No foundation. No website. No social media. Just code and consensus. Use at your own risk.
