LUMENYX: A Peer-to-Peer Electronic Cash System with Privacy




Abstract

A purely peer-to-peer version of electronic cash with fixed supply, smart
contracts, and optional privacy. The network uses proof-of-stake consensus 
with 3-second block finality. Total supply is limited to 21,000,000 LUMENYX 
with a halving emission schedule. Users can choose between transparent or 
shielded transactions. No governance, no team allocation, no venture capital. 
The code is the law.


1. Introduction

LUMENYX is designed to be a complete solution:

  - Fixed supply (21,000,000)
  - Smart contracts (EVM compatible)
  - Optional privacy (zero-knowledge proofs)
  - Fast blocks (3 seconds)
  - True decentralization (fair launch)
  - Zero governance (code is law)


2. Transactions

Standard transactions are transparent and traceable.
For users requiring privacy, shielded transactions use zero-knowledge proofs
to hide sender, receiver, and amount information.

Transparent transfer:
  Alice → 100 LUMENYX → Bob (visible on chain)

Shielded transfer:
  [nullifier] → [commitment] (only proofs visible, no identities)


3. Proof-of-Stake

The network uses Aura for block production and GRANDPA for finality.
Any participant can become a validator by running a node and staking LUMENYX.

Block time: 3 seconds
Finality: ~3 seconds
Minimum stake: 1 LUMENYX
Slashing: 30% for misbehavior
Unbonding: 28 days


4. Emission Schedule

Total supply: 21,000,000 LUMENYX (fixed, immutable)

The emission follows a three-phase schedule:

Phase 0 - Bootstrap (~12 days):
  Block reward: 2.4 LUMENYX
  Total blocks: 350,000
  Total emission: 840,000 LUMENYX (4.0%)
  Purpose: Network security initialization

Phase 1 - Early Adoption (30 days):
  Block reward: 0.3 LUMENYX
  Total blocks: 864,000
  Total emission: 259,200 LUMENYX (1.2%)
  Purpose: Early adopter incentives

Phase 2 - Standard (forever):
  Block reward: 0.25 LUMENYX (with halving)
  Daily emission: 7,200 LUMENYX
  Halving: Every 42,076,800 blocks (~4 years)
  Purpose: Long-term scarcity

Emission distribution over time:

  Year 1:   ~2,628,000 LUMENYX (12.5%)
  Year 4:  ~10,500,000 LUMENYX (50%)
  Year 8:  ~15,750,000 LUMENYX (75%)
  Year 50: ~21,000,000 LUMENYX (100%)


5. Fee Structure

Transaction fees are designed to remain low regardless of token price.

Base transfer fee: 0.0001 LUMENYX (~$0.001)
Smart contract fee: 0.001 LUMENYX (~$0.01)
Privacy (ZK) fee: 0.01 LUMENYX (~$0.05)

All fees are paid to validators who produce blocks.


6. Privacy Mechanism

The privacy system uses a shielded pool with note commitments and nullifiers.

Shield: Convert transparent LUMENYX to shielded
  - Sender visible (last time identity linked)
  - Creates note commitment
  - LUMENYX enters shielded pool

Shielded Transfer: Anonymous transfer
  - No sender or receiver visible
  - Only cryptographic proofs on chain
  - Nullifier prevents double-spend

Unshield: Convert shielded LUMENYX to transparent
  - Recipient visible (identity revealed)
  - Note consumed via nullifier
  - LUMENYX exits shielded pool

The system ensures:
  - Conservation of value (inputs = outputs)
  - No double-spending (nullifier uniqueness)
  - Unlinkability (transactions cannot be traced)


7. Smart Contracts

Full EVM and WASM compatibility allows deployment of existing contracts.
Chain ID: 7777

Supported standards:
  - ERC-20 (fungible tokens)
  - ERC-721 (NFTs)
  - ERC-1155 (multi-token)
  - Custom contracts

Precompiled contracts:
  - 0x01: ecrecover (signature recovery)
  - 0x02: SHA-256
  - 0x03: RIPEMD-160
  - 0x04: Identity
  - 0x05: Modexp
  - 0x06-08: BN128 (elliptic curve)
  - 0x09: Blake2F

Developers can build:
  - Decentralized exchanges
  - Lending protocols
  - DAOs
  - Games
  - Any application


8. Network Parameters

Block time:           3,000 ms (3 seconds)
Blocks per day:       28,800
Blocks per year:      10,519,200
Max validators:       100
SS58 prefix:          42
Token decimals:       12
Token symbol:         LUMENYX


9. Governance

There is no governance mechanism. No voting. No proposals. No council.

The code is the law. If users disagree, they can fork.

This design ensures:
  - No centralization of power
  - No political capture
  - No regulatory target
  - True immutability


10. Conclusion

LUMENYX combines fixed supply, smart contracts, optional privacy, and speed
into one chain.

The fair launch model, with documented bootstrap phase and no pre-allocation,
ensures equitable distribution. The absence of governance prevents centralization.

The network is live.
