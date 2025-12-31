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

**Bootstrap Phase:**
- 5000 LUMENYX allocated to validator faucet
- ~840,000 LUMENYX mined in first ~350,000 blocks (~12 days)
- Required to initialize network security
- Total: ~4% of supply

**Public Distribution:**
- ~96% of supply via halving schedule (100+ years)
- Fully permissionless

## Genesis Block

> "Banks ended up in the headlines. Today control over digital money sits in a few hands: founders, large holders, intermediaries and those who write the rules."

## Quick Start
```bash
# Clone and build
git clone https://github.com/lumenyx-chain/lumenyx
cd lumenyx
cargo build --release

# Run as full node
./target/release/lumenyx-node --chain mainnet-spec.json --bootnodes /ip4/89.147.111.102/tcp/30333/p2p/12D3KooWSnyAJBQoKRQL3SWgYu5LKqEsAYfN5JWtzoc2NKgGb5ZJ

# Run as validator
./target/release/lumenyx-node --chain mainnet-spec.json --validator --bootnodes /ip4/89.147.111.102/tcp/30333/p2p/12D3KooWSnyAJBQoKRQL3SWgYu5LKqEsAYfN5JWtzoc2NKgGb5ZJ
```

## Become a Validator (Permissionless)

Anyone can become a validator without permission or contact.

### Step 1: Generate Keys
```bash
# Generate AURA key (block production)
./target/release/lumenyx-node key generate --scheme sr25519

# Generate GRANDPA key (finalization)
./target/release/lumenyx-node key generate --scheme ed25519

# Save both secret phrases securely
```

### Step 2: Claim Free LUMENYX

The validator faucet provides **2 LUMENYX for free** to cover registration fees.

**Option A: Automatic (Recommended)**
```bash
pip install base58 substrate-interface
python3 scripts/claim_faucet.py
```
This generates a new account, calculates PoW, and claims automatically.

**Option B: Manual**
```bash
# Calculate PoW for your existing address
pip install base58
python3 scripts/faucet_pow.py YOUR_ADDRESS
```
Then submit via substrate-interface or Polkadot.js Apps.

**Details:**
- Requires proof-of-work (~2 seconds to compute)
- One claim per account
- Pool: 5000 LUMENYX total

### Step 3: Register Your Keys
```bash
# Insert AURA key
./target/release/lumenyx-node key insert --chain mainnet-spec.json \
  --scheme sr25519 --key-type aura \
  --suri "your twelve word secret phrase"

# Insert GRANDPA key
./target/release/lumenyx-node key insert --chain mainnet-spec.json \
  --scheme ed25519 --key-type gran \
  --suri "your twelve word secret phrase"
```

### Step 4: Set Session Keys

Using Polkadot.js Apps:
1. Call `session.setKeys(keys, proof)` with your AURA + GRANDPA public keys
2. Costs ~0.001 LUMENYX (paid from faucet claim)

### Step 5: Start Validating
```bash
./target/release/lumenyx-node --chain mainnet-spec.json --validator --name "your-name" --bootnodes /ip4/89.147.111.102/tcp/30333/p2p/12D3KooWSnyAJBQoKRQL3SWgYu5LKqEsAYfN5JWtzoc2NKgGb5ZJ
```

Your node will start producing blocks in the next era (~1 hour).
No permission needed. No contact required.

## Technical Details

- **Consensus:** AURA (block production) + GRANDPA (finality)
- **Block time:** 3 seconds
- **Block reward:** 2.4 LUMENYX, halving every ~4 years
- **Chain ID:** 7777
- **Framework:** Substrate + Frontier EVM
- **Privacy:** Groth16 ZK-SNARKs (optional)

## Documentation

- [Installation Guide](docs/INSTALL.md)
- [Whitepaper](docs/WHITEPAPER.md)
- [ZK Privacy](docs/ZK_PRIVACY.md)

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
