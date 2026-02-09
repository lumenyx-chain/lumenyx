# LUMENYX

---

## A Peer-to-Peer Electronic Cash System

---

### Abstract

A purely peer-to-peer version of electronic cash with fixed supply and smart contracts. The network uses proof-of-work consensus with 2.5 second blocks. Total supply is limited to 21,000,000 LUMENYX (ticker: LUMO) with a halving emission schedule. No governance, no team allocation, no venture capital.

---

### Quick Start

**Requirements:**
- Linux (Ubuntu, Debian, or similar)
- x86_64 processor with AES-NI & PCLMUL (Intel 2010+, AMD 2011+)
- **4 GB RAM** for mining (RX-LX algorithm, fast mode)
- 1 GB RAM minimum for sync-only nodes

**One command to start:**
```bash
curl -sL "https://raw.githubusercontent.com/lumenyx-chain/lumenyx/main/lumenyx-setup.sh" -o lumenyx-setup.sh && chmod +x lumenyx-setup.sh && ./lumenyx-setup.sh
```

The script will:
- Download the binary
- Generate your wallet (save the seed phrase!)
- Start mining automatically
- Auto-update when new versions are available

📖 **[Why choose LUMENYX?](docs/WHY_LUMENYX.md)**

🌐 **[Bootnodes](bootnodes.txt)** - Network bootstrap nodes

🔧 **[Build from source](docs/INSTALL.md)** - Compile yourself

---

### Check Your Node

**Check if running:**
```bash
tail -n 20 ~/.lumenyx/lumenyx.log
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
8. Go to: https://polkadot.js.org/apps/?rpc=ws://207.180.204.4:9944
   (This is the public Archive node. Or use your own node's IP.)
9. Wait a few seconds - you'll see "LUMENYX Mainnet" in top left

Now you can see blocks, validators, balances, everything.

---

### 1. Introduction

In 2009, Bitcoin proved that digital scarcity was possible.
In 2015, Ethereum proved that programmable money was possible.

LUMENYX combines both:
- Fixed supply (21,000,000)
- Smart contracts (EVM compatible — standard 18 decimals)
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
| Mining mode | Fast (Dataset, ~2GB RAM, 5-7x faster) |
| ASIC resistance | Yes (custom SBOX, pointer chasing) |

**RX-LX** is a custom fork of RandomX designed to be ASIC-resistant. Existing RandomX ASICs (Bitmain X5, X9) will produce invalid hashes due to custom SBOX and modified opcodes.

Anyone with a computer can mine. No stake required. No permission needed. No ASIC advantage.

---

### 3. Emission

| Parameter | Value |
|-----------|-------|
| Initial reward | ~0.208 LUMO |
| Halving period | 4 years (50,492,160 blocks) |
| Total supply | 21,000,000 LUMO |

Timeline:
- Year 4: 50% mined (~10.5M)
- Year 8: 75% mined (~15.75M)
- Year 12: 87.5% mined (~18.4M)
- Year 16: 93.75% mined (~19.7M)

Same curve as Bitcoin. Proven model.

---

### 4. Smart Contracts

Full EVM compatibility with standard 18 decimals. Deploy Solidity contracts without changes.

| Property | Value |
|----------|-------|
| Chain ID | 7777 |
| Decimals | 18 (standard EVM) |
| Ticker | LUMO |
| Gas model | Ethereum-compatible (EIP-1559) |

Everything built on Ethereum can be built on LUMENYX — no decimal workarounds needed.

---

### 5. Transaction Fees

| Parameter | Value |
|-----------|-------|
| Fee destination | 100% to miners |
| Fee model | Dynamic (EIP-1559) |
| Elasticity | ±12.5% per block |

Fees adjust automatically based on network demand. All fees go to miners - no burning.

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

### 9. Hard Fork History

| Version | Block | Changes |
|---------|-------|---------|
| v2.2.5 | 125,000 | ASERT difficulty fix |
| v2.4.1 | 490,000 | 18 decimals, fast mining, ticker LUMO |

---

### 10. Conclusion

LUMENYX is digital cash for the next era:
- Scarce (21M cap)
- Fast (2.5 sec blocks)
- Programmable (EVM, 18 decimals)
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

---

---

## Who This Is For

LUMENYX is not for everyone.

It's for whoever has an idea in their head that no one else will let them build.

If you're looking for a "project" with a team, roadmap, community manager, and airdrop — close this page.

If you're looking for a place where code is the only authority — keep reading.

---

### The Space Is Yours

I know what you feel.

That thing in your head that won't turn off. That idea that follows you everywhere — in the shower, while eating, at 4 AM when you should be sleeping but instead you're staring at a screen with burning eyes.

You tried to explain it. You stopped. They don't understand anyway.

They see someone wasting time. A weirdo. Someone who should get a real job, a normal life, stop chasing things that don't exist.

They don't see what you see.

The connections. The systems. The cracks in everything around us — and the ways to rebuild it from scratch, better, different.

You know you could create something big. You feel it in your bones. But every time you try, someone says no. Rules. Limits. Permissions. People who never built anything in their lives explaining why it can't be done.

So you keep it all inside. The idea stays there, burning.

---

### Listen

While you were waiting for the right space, we were building it.

Nights fixing bugs. Accusations of premine — and us publishing every line of code saying: "look, verify, we have nothing to hide." The chain freezes. We restart it. An error in genesis. We delete everything and start from zero.

No team. No investors. No marketing.

Just obsession.

The result is here: a chain that works. Fixed scarcity like Bitcoin (21M). Real smart contracts via EVM — standard 18 decimals, compatible with everything. Zero premine. CPU mining — your computer, not billionaire server farms. Open code, verifiable, belonging to no one.

The foundation is ready.

What's missing is you.

---

### Technical Parameters For Builders

| Parameter | Value |
|-----------|-------|
| **RPC** | `http://207.180.204.4:9944` |
| **Chain ID** | `7777` |
| **Decimals** | `18` (standard EVM) |
| **Consensus** | PoW with RX-LX algorithm (RandomX fork, CPU only) |
| **Block time** | ~2.5 seconds |
| **Chain** | LUMENYX |
| **Ticker** | LUMO — meaning "light" in Esperanto |
| **EVM** | Full Solidity compatibility |

With MetaMask (or any EVM wallet):

1. Add a new network with the RPC above
2. Chain ID: `7777`, Decimals: `18`, Symbol: `LUMO`
3. Deploy with Hardhat, Foundry, Remix — no workarounds needed

---

### How To Get LUMO

There is no faucet. There is no airdrop.

You want LUMO? Two paths:

1. **Mine.** Run the setup script, wait 10 minutes, you have your coins.
2. **Buy from someone who mines.**

There is no other way.

---

### What You Can Build Here

We won't tell you what to build.
We'll only tell you that LUMENYX is made for those who want to push beyond normal limits:

- DEXs and DeFi protocols with no admin key
- P2P escrow systems where no one can run away with the funds
- On-chain gambling where the house is just code
- Opt-in privacy tools built on top of a transparent layer
- Censorship-resistant marketplaces
- Anything that elsewhere would be "too risky", "too uncomfortable", or "too political"

If elsewhere they tell you "no", here you can at least try.

---

### First Quest: Enter LUMENYX in 30 Minutes

If you're a developer, this is the minimum path:

**1. Get LUMO**

Mine for 10 minutes. Or buy from a miner. No shortcuts.

**2. Connect to the network**

Add LUMENYX to your EVM wallet:
- RPC: `http://207.180.204.4:9944`
- Chain ID: `7777`
- Decimals: `18`
- Symbol: `LUMO`

Verify you see the network online (blocks advancing).

**3. Deploy something real**

Take a simple Solidity contract (ERC-20, ERC-721, or whatever you want).
Deploy it on LUMENYX using your preferred tool.

**4. Document**

Open an issue or a Discussion in this repo with:
- Contract address
- Toolchain used (Hardhat, Foundry, etc.)
- Problems encountered / fixes needed

If you get this far, you're no longer a spectator. You've left a mark on-chain.

---

### If You're "The One"

Maybe you're reading this and thinking: "it's not me."

Fine.

But maybe you know someone.

That weird guy.
The one you see online at impossible hours.
The one who talks about things you don't understand but you feel there's something different about him.

Send this to them.

Send it where people who think different hang out: forums, boards, tech chats, forgotten corners of the net.

Let it spread until it reaches the right person.

---

Satoshi was one person. He launched Bitcoin and the world was never the same.

Vitalik was 19. He wrote a whitepaper and created an ecosystem.

You don't need masses.
You don't need armies.

Just one.

One crazy enough to see what others don't see.
One obsessed enough to build what others call impossible.
One who doesn't ask permission.

Is that you?

Then stop reading.

**Build.**

---

**There is no Discord. There is no Telegram. There is no website.**

**Only this repo and the chain that runs.**

**If you think someone should see this, send it to them yourself.**

**If you're the one, stop reading. Build.**
