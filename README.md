# LUMENYX

---

## A Peer-to-Peer Electronic Cash System

---

### Abstract

A purely peer-to-peer version of electronic cash with fixed supply and smart contracts. The network uses proof-of-work consensus with 2.5 second blocks. Total supply is limited to 21,000,000 LUMENYX with a halving emission schedule. No governance, no team allocation, no venture capital.

---

### Quick Start

**Requirements:**
- Linux (Ubuntu, Debian, or similar)
- x86_64 processor with AES-NI & PCLMUL (Intel 2010+, AMD 2011+)
- **2-4 GB RAM** for mining (RX-LX algorithm)
- 1 GB RAM minimum for sync-only nodes

**One command to start:**
```bash
curl -sL "https://api.github.com/repos/lumenyx-chain/lumenyx/contents/lumenyx-setup.sh" -H "Accept: application/vnd.github.v3.raw" -o lumenyx-setup.sh && chmod +x lumenyx-setup.sh && ./lumenyx-setup.sh
```

The script will:
- Download the binary
- Generate your wallet (save the seed phrase!)
- Start mining automatically
- Auto-update when new versions are available

📖 **[Why choose LUMENYX?](docs/WHY_LUMENYX.md)**

🌐 **[Bootnodes](bootnodes.txt)** - Network bootstrap nodes

---

### Check Your Node

**Check if running:**
```bash
journalctl -u lumenyx -n 20 --no-pager
```
If you see "Imported block" or "Prepared block" = working.

---

### Block Explorer (Polkadot.js Apps)

Browsers block insecure WebSocket by default. Use Firefox with this fix:

1. Open Firefox (Chrome won't work)
2. Type in address bar: about:config
3. Click "Accept the Risk and Continue"
4. In search box type: network.websocket.allowInsecureFromHTTPS
5. You will see it set to "false"
6. Click the arrows icon on the right to change it to "true"
7. Close Firefox completely and reopen it
8. Go to: https://polkadot.js.org/apps/?rpc=ws://IP:9944
   (Replace IP with your node's IP address, e.g. 192.168.1.100)
9. Wait a few seconds - you'll see "LUMENYX Mainnet" in top left

Now you can see blocks, validators, balances, everything.

**Why not HTTPS/WSS?**
Using wss:// requires a domain and SSL certificate. LUMENYX philosophy: no website, no social media, no domain. Just code.

---

### 1. Introduction

In 2009, Bitcoin proved that digital scarcity was possible.
In 2015, Ethereum proved that programmable money was possible.

LUMENYX combines both:
- Fixed supply (21,000,000)
- Smart contracts (EVM compatible)
- Fast blocks (2.5 seconds)
- True decentralization (fair launch)
- Zero governance (code is law)

---

### 2. Consensus

Proof of Work with LongestChain.

| Parameter | Value |
|-----------|-------|
| Block time | 2.5 seconds |
| Hash function | RX-LX (RandomX-LUMENYX) |
| ASIC resistance | Yes (custom SBOX, pointer chasing) |

**RX-LX** is a custom fork of RandomX designed to be ASIC-resistant. Existing RandomX ASICs (Bitmain X5, X9) will produce invalid hashes due to custom SBOX and modified opcodes.

Anyone with a computer can mine. No stake required. No permission needed. No ASIC advantage.

---

### 3. Emission

| Parameter | Value |
|-----------|-------|
| Initial reward | ~0.208 LUMENYX |
| Halving period | 4 years (50,492,160 blocks) |
| Total supply | 21,000,000 LUMENYX |

Timeline:
- Year 4: 50% mined (~10.5M)
- Year 8: 75% mined (~15.75M)
- Year 12: 87.5% mined (~18.4M)
- Year 16: 93.75% mined (~19.7M)

Same curve as Bitcoin. Proven model.

---

### 4. Smart Contracts

Full EVM compatibility. Deploy Solidity contracts without changes.

| Property | Value |
|----------|-------|
| Chain ID | 7777 |
| Gas model | Ethereum-compatible |

Everything built on Ethereum can be built on LUMENYX.

---

### 5. Transaction Fees

| Parameter | Value |
|-----------|-------|
| Fee destination | 100% to miners |
| Fee model | Dynamic (EIP-1559) |
| Default base fee | 1,000 planck/gas |
| Elasticity | ±12.5% per block |

Fees adjust automatically based on network demand:
- **Low demand**: Base fee decreases (cheaper transactions)
- **High demand**: Base fee increases (anti-spam protection)

Competitive with Solana at any LUMENYX price. All fees go to miners - no burning.

---

### 6. Distribution

- No premine
- No ICO
- No team allocation
- No foundation
- No venture capital

100% distributed through mining. Everyone starts equal.

---

### 7. Governance

None.

The code is the law. No admin keys. No sudo. No upgrades without hard fork.

Like Bitcoin, the protocol is set in stone. Only the community can change it through consensus.

---

### 8. What Can Be Built

The base layer is intentionally simple. Complexity belongs on top.

- Tokens (ERC-20)
- NFTs (ERC-721)
- Decentralized exchanges
- Lending protocols
- DAOs
- Games
- Layer 2 scaling solutions
- Privacy layers
- Bridges to other chains
- Things that don't exist yet

The foundation is yours. Build the future on it.

---

### 9. Conclusion

LUMENYX is digital cash for the next era:
- Scarce (21M cap)
- Fast (2.5 sec blocks)
- Programmable (EVM)
- Decentralized (PoW, no team)

No promises. No roadmap. No marketing.

Just code and consensus.

---

> *"Bitcoin started with a headline. Ethereum started with a premine. LUMENYX starts with you."*

---

The chain belongs to no one.
It's yours.
Build the future on it.

---

No company. No foundation. No website. No social media.

Just code.

