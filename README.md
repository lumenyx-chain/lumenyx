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

# Run as full node (sync only)
./target/release/lumenyx-node --chain mainnet --bootnodes /ip4/89.147.111.102/tcp/30333/p2p/12D3KooWFvS5t55gVhjtzP4egiWMZKtjdPDQg9HxCkXDXNmeH2V1

# Run as validator (produces blocks)
./target/release/lumenyx-node --chain mainnet --validator --name "your-name" --bootnodes /ip4/89.147.111.102/tcp/30333/p2p/12D3KooWFvS5t55gVhjtzP4egiWMZKtjdPDQg9HxCkXDXNmeH2V1
```

## Become a Validator (3 Simple Steps)

Anyone can become a validator without permission or contact.

### Step 1: Build and Start Your Node
```bash
# Clone and build
git clone https://github.com/lumenyx-chain/lumenyx
cd lumenyx
cargo build --release

# Start as validator (keep this running!)
./target/release/lumenyx-node \
  --chain mainnet \
  --validator \
  --name "your-name" \
  --rpc-cors=all \
  --rpc-methods=unsafe \
  --bootnodes /ip4/89.147.111.102/tcp/30333/p2p/12D3KooWFvS5t55gVhjtzP4egiWMZKtjdPDQg9HxCkXDXNmeH2V1
```

Wait until you see `Idle (1 peers)` - your node is synced.

### Step 2: Run the Automatic Setup Script

Open a **new terminal** (keep the node running) and run:
```bash
pip install substrate-interface
cd lumenyx
python3 scripts/become_validator.py
```

This script automatically:
- ✅ Generates a new account for you
- ✅ Calculates proof-of-work (~2 seconds)
- ✅ Claims 2 LUMENYX from faucet (free!)
- ✅ Inserts your AURA key into the node
- ✅ Registers you as validator (session.setKeys)
- ✅ Saves your seed phrase to `~/.lumenyx-validator-key`

**IMPORTANT:** Save the seed phrase shown on screen! It's the only way to recover your account.

### Step 3: Wait and Mine

Your node will start producing blocks in the next era (~10 minutes).

You'll see logs like:
```
🎁 Prepared block for proposing at 1234
🏆 Imported #1234
```

**Congratulations! You're now earning LUMENYX rewards!**

---

## Manual Setup (Advanced)

If you prefer manual control, here are the individual steps:

### Generate Key
```bash
./target/release/lumenyx-node key generate --scheme sr25519
# Save the secret phrase and public key!
```

### Insert Key into Node
```bash
curl -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"author_insertKey","params":["aura","your twelve word phrase","0xYOUR_PUBLIC_KEY"],"id":1}' \
  http://localhost:9944
```

### Register as Validator
Using Python:
```python
from substrateinterface import SubstrateInterface, Keypair

substrate = SubstrateInterface(url="ws://localhost:9944")
keypair = Keypair.create_from_mnemonic("your twelve word phrase")

call = substrate.compose_call(
    call_module='Session',
    call_function='set_keys',
    call_params={
        'keys': {'aura': f'0x{keypair.public_key.hex()}'},
        'proof': '0x'
    }
)
extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)
substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
```

---

## Technical Details

| Property | Value |
|----------|-------|
| Consensus | AURA (block production) |
| Finality | Probabilistic (~18 seconds / 6 blocks) |
| Block time | 3 seconds |
| Chain ID | 7777 |
| Decimals | 12 |
| Framework | Substrate + Frontier EVM |
| Privacy | Groth16 ZK-SNARKs (optional) |

**Note:** No GRANDPA finality - the chain NEVER stops, like Bitcoin.

## Emission Schedule

| Phase | Reward | Duration | Purpose |
|-------|--------|----------|---------|
| 0 - Bootstrap | 2.4 LUMENYX | ~12 days | High incentive when network needs security most |
| 1 - Early Adoption | 0.3 LUMENYX | ~30 days | Gradual transition |
| 2 - Standard | 0.25 LUMENYX | Forever | Halving every ~4 years |

Early validators take the highest risk. Higher rewards compensate for this uncertainty.
Same principle as Bitcoin's early 50 BTC blocks - those who believe first, earn most.

## Validator Faucet

| Property | Value |
|----------|-------|
| Pool | 5000 LUMENYX total |
| Claim amount | 2 LUMENYX per account |
| Anti-spam | Proof-of-work (18-bit, ~2 seconds) |
| Limit | One claim per account |
| Max validators | ~2,500 |

Truly permissionless. No gatekeeper.

## EVM Compatibility

LUMENYX is fully EVM-compatible. Deploy Ethereum smart contracts as-is.

**Connect MetaMask:**
| Setting | Value |
|---------|-------|
| Network Name | LUMENYX |
| RPC URL | http://localhost:8545 |
| Chain ID | 7777 |
| Symbol | LUMENYX |

## Privacy (Optional)

Private transactions use zero-knowledge proofs. Your public key remains hidden.
See [ZK_PRIVACY.md](docs/ZK_PRIVACY.md) for details.

## Building from Source

**Requirements:**
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

No company. No foundation. No website. No social media.
Just code and consensus.
Use at your own risk.

## Community Bootnodes

Help decentralize the network by running a bootnode:
- [Bootnodes List](https://github.com/lumenyx-chain/lumenyx/wiki) - Community-maintained

To add yours, edit the wiki page with your bootnode info.
