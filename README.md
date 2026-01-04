# LUMENYX

*I've been working on a new electronic cash system that runs without central control.*

## What is LUMENYX?

A peer-to-peer electronic cash system with:

- **Fixed supply**: 21,000,000 LUMENYX, never more
- **Fast blocks**: 1-3 seconds with GHOSTDAG PoW
- **Smart contracts**: Full EVM compatibility
- **Optional privacy**: ZK-SNARKs when you need it
- **Zero premine**: 100% mined, no allocation
- **No team**: Code is law, no governance

## Distribution

- No ICO, no pre-sale, no VC
- No team allocation, no foundation
- No governance, no admin keys
- No premine

100% of supply distributed via mining rewards over 100+ years.

## Quick Start

### Build
```bash
git clone https://github.com/lumenyx-chain/lumenyx
cd lumenyx
cargo build --release
```

### Run a Full Node
```bash
./target/release/lumenyx-node --chain mainnet --name "your-node"
```

### Mine LUMENYX
```bash
./target/release/lumenyx-node --chain mainnet --mine --name "your-miner"
```

No registration. No staking. No permission. Just mine.

## Technical Specifications

| Property | Value |
|----------|-------|
| Consensus | GHOSTDAG PoW |
| Block time | 1-3 seconds |
| Supply | 21,000,000 |
| Decimals | 12 |
| Chain ID (EVM) | 7777 |
| Hash algorithm | Blake3 |
| Finality | ~18 seconds (6 blocks) |

## Emission Schedule

| Phase | Block Reward | Duration |
|-------|--------------|----------|
| Bootstrap | 2.4 LUMENYX | ~12 days |
| Early | 0.3 LUMENYX | ~30 days |
| Standard | 0.25 LUMENYX | Halving every ~4 years |

## Privacy (Optional)

Private transactions using Groth16 ZK-SNARKs:
```bash
python3 scripts/shield.py --amount 100
python3 scripts/unshield.py --amount 50
```

See [docs/ZK_PRIVACY.md](docs/ZK_PRIVACY.md) for details.

## EVM Compatibility

Deploy Ethereum smart contracts. MetaMask settings:

| Setting | Value |
|---------|-------|
| Network Name | LUMENYX |
| RPC URL | http://localhost:8545 |
| Chain ID | 7777 |
| Symbol | LUMENYX |

## Genesis Block

> *"Banks ended up in the headlines. Today control over digital money sits in a few hands: founders, large holders, intermediaries and those who write the rules."*

## Building from Source
```bash
# Requirements: Rust, Cargo, LLVM/Clang
cargo build --release
cargo test --all
```

## Documentation

- [Installation Guide](docs/INSTALL.md)
- [Whitepaper](docs/WHITEPAPER.md)
- [ZK Privacy](docs/ZK_PRIVACY.md)

## License

GPL-3.0

---

No company. No foundation. No website. No social media. Just code and consensus.

## Community Bootnodes

The network must survive without its creator. Help keep it alive by running a bootnode.

**[Bootnodes List](https://github.com/lumenyx-chain/lumenyx/wiki/Bootnodes)** - Add your node here

When the original nodes go offline, your nodes keep the network running.
