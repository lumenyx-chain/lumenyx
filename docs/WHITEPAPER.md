# LUMENYX: A Peer-to-Peer Electronic Cash System with Privacy

## Abstract

A purely peer-to-peer version of electronic cash with fixed supply, smart contracts, and optional privacy. The network uses GHOSTDAG proof-of-work consensus with 1-3 second blocks. Total supply is limited to 21,000,000 LUMENYX with a halving emission schedule. Users can choose between transparent or shielded transactions. No governance, no team allocation, no venture capital. The code is the law.

## 1. Introduction

LUMENYX combines the best properties of existing cryptocurrencies:

- **Bitcoin**: Fixed 21M supply, PoW consensus, halving schedule
- **Ethereum**: EVM smart contracts, programmability
- **Kaspa**: GHOSTDAG blockDAG, fast blocks without orphans
- **Zcash**: ZK-SNARK privacy (optional)

The result is a complete monetary system:

- Fixed supply (21,000,000)
- Smart contracts (EVM compatible)
- Optional privacy (zero-knowledge proofs)
- Fast blocks (1-3 seconds)
- True decentralization (fair launch)
- Zero governance (code is law)

## 2. GHOSTDAG Consensus

Traditional blockchains discard blocks found simultaneously ("orphans"). This limits throughput and wastes mining work.

GHOSTDAG (Greedy Heaviest-Observed Sub-DAG) solves this by organizing blocks in a Directed Acyclic Graph:
```
    [Block A]
       /  \
[Block B]  [Block C]  <- Both valid, both included
       \  /
    [Block D]
```

### Properties

| Parameter | Value |
|-----------|-------|
| K (anticone limit) | 18 |
| Max parents | 10 |
| Block time | 1-3 seconds |
| Hash function | Blake3 |

### Blue Set Selection

GHOSTDAG distinguishes "blue" (honest) blocks from "red" (potentially adversarial) blocks using the K-cluster algorithm. Transactions in blue blocks are confirmed; red block transactions may be rejected if conflicting.

### Finality

Probabilistic finality after ~6 blocks (~18 seconds). Like Bitcoin, but faster.

## 3. Proof of Work

Pure PoW mining. No staking, no validators, no permission required.
```
hash = Blake3(block_header || nonce)
if hash < target:
    block is valid
```

Anyone with a CPU can mine. No special hardware required (GPU/ASIC resistance through memory-hard hashing in future versions).

### Difficulty Adjustment

Dynamic difficulty adjustment every block to maintain 1-3 second block times.

## 4. Transactions

### Standard Transactions

Transparent, traceable on-chain. Compatible with Ethereum tooling.

### Private Transactions (Optional)

Using Groth16 ZK-SNARKs:

1. **Shield**: Convert public LUMENYX to private
2. **Transfer**: Move private LUMENYX (hidden sender, receiver, amount)
3. **Unshield**: Convert private LUMENYX back to public

Privacy is optional. Users choose per-transaction.

## 5. Smart Contracts

Full EVM compatibility via Frontier. Deploy Solidity contracts as-is.

| Property | Value |
|----------|-------|
| Chain ID | 7777 |
| Gas model | Ethereum-compatible |
| Opcodes | Full EVM support |

## 6. Emission Schedule

| Phase | Block Reward | Approximate Duration |
|-------|--------------|---------------------|
| Bootstrap | 2.4 LUMENYX | ~12 days |
| Early | 0.3 LUMENYX | ~30 days |
| Standard | 0.25 LUMENYX | Halving every ~4 years |

Total supply approaches 21,000,000 asymptotically over 100+ years.

## 7. Distribution

- No premine
- No ICO/IEO/IDO
- No team allocation
- No foundation
- No venture capital
- No airdrops

100% distributed through mining.

## 8. Governance

None. The code is the law. No upgrades without hard fork. No admin keys. No sudo. No democracy. No plutocracy.

## 9. Launch Philosophy

Satoshi-style:
1. Write the code
2. Launch the network
3. Disappear

The network must survive without its creator.

## 10. Conclusion

LUMENYX is digital cash for the 21st century:

- Scarce like gold (21M cap)
- Fast like digital payments (1-3 sec)
- Private when needed (ZK-SNARKs)
- Programmable (EVM)
- Decentralized (PoW, no team)

No promises. No roadmap. No marketing. Just code.

---

*"Banks ended up in the headlines. Today control over digital money sits in a few hands: founders, large holders, intermediaries and those who write the rules."*
