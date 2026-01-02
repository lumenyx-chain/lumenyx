# LUMENYX

I've been working on a new electronic cash system that runs without central control.

## Properties

- **21,000,000** fixed supply
- **3 second** block time
- **Zero-knowledge privacy** (optional)
- **Full EVM compatibility** (Chain ID: 7777)
- **Proof-of-Stake** consensus
- **Permissionless** validation

## Distribution

- No ICO, no pre-sale, no VC
- No team allocation, no foundation
- No governance, no admin keys
- No premine

**Original Plan:**
A 4% bootstrap allocation (~840,000 LUMENYX) was planned to initialize network security.

**What Changed:**
Consensus issues during multi-validator testing required a network reset. We decided to start fresh - zero allocation, mining from block 0 like everyone else.

**Current Reality:**
- 5000 LUMENYX in validator faucet (from genesis)
- We run one validator now, mining alongside you
- When enough validators join, we leave
- Before disappearing, anything above 5% will be burned to:
  `5Gbh1MkL3KSAMmwx7wxYyCYRtzHhXocSAAvcT6gD21L4Q978`

**Supply Distribution:**
- 100% via halving schedule (100+ years)
- Fully permissionless

This chain belongs to no one. Run a node. Become a validator. It's yours.

## Genesis Block

> "Banks ended up in the headlines. Today control over digital money sits in a few hands: founders, large holders, intermediaries and those who write the rules."

## Quick Start
```bash
# Clone and build
git clone https://github.com/lumenyx-chain/lumenyx
cd lumenyx
cargo build --release

# Run as full node
./target/release/lumenyx-node --chain mainnet --bootnodes /ip4/89.147.111.102/tcp/30333/p2p/12D3KooWSnyAJBQoKRQL3SWgYu5LKqEsAYfN5JWtzoc2NKgGb5ZJ

# Run as validator
./target/release/lumenyx-node --chain mainnet --validator --bootnodes /ip4/89.147.111.102/tcp/30333/p2p/12D3KooWSnyAJBQoKRQL3SWgYu5LKqEsAYfN5JWtzoc2NKgGb5ZJ
```

## Become a Validator (Permissionless)

Anyone can become a validator without permission or contact.

### Step 1: Generate Key
```bash
# Generate AURA key (block production)
./target/release/lumenyx-node key generate --scheme sr25519

# Save the secret phrase securely
```

### Step 2: Claim Free LUMENYX

The validator faucet provides **2 LUMENYX for free** to cover registration fees.
```bash
pip install substrate-interface
python3 scripts/become_validator.py
```

This script generates a new account, calculates PoW, and claims automatically.

**Details:**
- Requires proof-of-work (~2 seconds to compute)
- One claim per account
- Pool: 5000 LUMENYX total

### Step 3: Register Your Key
```bash
# Insert AURA key
./target/release/lumenyx-node key insert --chain mainnet \
  --scheme sr25519 --key-type aura \
  --suri "your twelve word secret phrase"
```

### Step 4: Set Session Keys

Using Polkadot.js Apps:
1. Call `session.setKeys(keys, proof)` with your AURA public key
2. Costs ~0.001 LUMENYX (paid from faucet claim)

### Step 5: Start Validating
```bash
./target/release/lumenyx-node --chain mainnet --validator --name "your-name" --bootnodes /ip4/89.147.111.102/tcp/30333/p2p/12D3KooWSnyAJBQoKRQL3SWgYu5LKqEsAYfN5JWtzoc2NKgGb5ZJ
```

Your node will start producing blocks in the next era (~1 hour).
No permission needed. No contact required.

## Technical Details

- **Consensus:** AURA (block production) with probabilistic finality
- **Finality:** ~18 seconds (6 blocks) - like Bitcoin
- **Block time:** 3 seconds
- **Chain ID:** 7777
- **Framework:** Substrate + Frontier EVM
- **Privacy:** Groth16 ZK-SNARKs (optional)

## Emission Schedule

| Phase | Reward | Duration | Purpose |
|-------|--------|----------|---------|
| 0 - Bootstrap | 2.4 LUMENYX | ~12 days | High incentive when network needs security most |
| 1 - Early Adoption | 0.3 LUMENYX | ~30 days | Gradual transition |
| 2 - Standard | 0.25 LUMENYX | Forever | Halving every ~4 years |

Early validators take the highest risk. The network has no value yet, no guarantee it will work. Higher rewards compensate for this uncertainty.

Same principle as Bitcoin's early 50 BTC blocks - those who believe first, earn most.

The code is public. The rules are visible. Anyone can join now and earn the same rewards.

## Documentation

- [Installation Guide](docs/INSTALL.md)
- [Whitepaper](docs/WHITEPAPER.md)
- [ZK Privacy](docs/ZK_PRIVACY.md)
- [Bootnodes List](https://github.com/lumenyx-chain/lumenyx/wiki) - Community-maintained

## EVM Compatibility

LUMENYX is fully EVM-compatible. Deploy Ethereum smart contracts as-is.

**Connect MetaMask:**
- Network Name: LUMENYX
- RPC URL: http://localhost:8545
- Chain ID: 7777
- Symbol: LUMENYX

## Privacy (Optional)

Private transactions use zero-knowledge proofs. Your public key remains hidden.
See [ZK_PRIVACY.md](docs/ZK_PRIVACY.md) for details.

## Validator Faucet Details

**Purpose:** Solve the chicken-and-egg problem for new validators.
**Pool:** 5000 LUMENYX total (~2,500 validators)
**Claim amount:** 2 LUMENYX per account
**Anti-spam:** Proof-of-work required (18-bit difficulty, ~2 seconds)
**Limit:** One claim per account

Truly permissionless. No gatekeeper.

## Building from Source

**Requirements:**
- Rust (stable)
- Cargo
- LLVM/Clang

**Build:**
```bash
cargo build --release
```

**Test:**
```bash
cargo test --all
```

## License

GPL-3.0

## Disclaimer

No company. No foundation. No website. No social media.
Just code and consensus.
Use at your own risk.
