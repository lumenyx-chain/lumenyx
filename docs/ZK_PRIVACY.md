# LUMENYX Privacy System v2.1 - Full BN254 Pairing Verification

## Overview

LUMENYX implements **TRUE zero-knowledge privacy** using Groth16 proofs on BN254 curve
with **FULL on-chain pairing verification**. This is the same cryptographic foundation 
used by Zcash and Tornado Cash.

### v2.1 Upgrade: Full On-Chain Verification

| Component | Implementation |
|-----------|----------------|
| **Field Arithmetic** | Pure Rust Fp, Fp2, Fp6, Fp12 tower |
| **Curve Operations** | BN254 G1, G2 point arithmetic |
| **Pairing** | Optimal Ate pairing with Miller loop |
| **Verification** | Full Groth16 equation check |

**Security Model**: NO trusted validators needed. Mathematical verification ensures
that ONLY valid proofs can be accepted. Fake proofs are REJECTED.

## Security Properties

| Property | Description |
|----------|-------------|
| **Unlinkability** | Shield and Unshield transactions CANNOT be linked |
| **Hiding** | Amounts are hidden in commitments |
| **Soundness** | Cannot create fake proofs |
| **Zero-Knowledge** | Verifier learns nothing except validity |

## Performance (Tested)

| Operation | Time |
|-----------|------|
| Trusted Setup | ~17ms |
| Proof Generation | ~13ms |
| Verification | ~3ms |
| Proof Size | 128 bytes |
| VK Size | 360 bytes |

## How It Works

### Shield (Deposit to Private Pool)

```
User side:
1. Generate random secret and blinding
2. Compute commitment = Hash(amount, secret, blinding)
3. Call privacy.shield(amount, commitment)
4. SAVE secret and blinding! (needed to withdraw)

On-chain:
1. Burns transparent funds
2. Adds commitment to Merkle tree
3. Updates root
4. Funds now ANONYMOUS
```

### Unshield (Withdraw with ZK Proof)

```
User side:
1. Generate ZK proof proving:
   - "I know a valid commitment in the tree"
   - "I know the secret for that commitment"
   - "This nullifier is correctly computed"
   - WITHOUT revealing which commitment!
2. Call privacy.unshield(amount, nullifier, root, proof)

On-chain:
1. Verifies ZK proof (Groth16)
2. Checks nullifier not spent
3. Mints transparent funds
4. NO LINK to original deposit!
```

## Privacy Guarantee

```
OBSERVER SEES:

Shield Event:
├── Commitment: 0xabc123...
├── Amount: 100 LUMENYX
└── Depositor: Alice

Unshield Event:
├── Nullifier: 0xdef456...  (DIFFERENT from commitment!)
├── Amount: 100 LUMENYX
└── Recipient: Bob

CAN OBSERVER LINK THEM?
├── ❌ Nullifier ≠ Commitment (different values)
├── ❌ No correlation visible
├── ❌ Cannot determine which commitment was spent
└── ✅ PRIVACY PRESERVED!

WHY?
├── Nullifier = Hash(commitment, secret)
├── Without knowing secret, IMPOSSIBLE to compute
└── ZK proof proves validity WITHOUT revealing link
```

## Setup Guide

### 1. Generate ZK Keys (One-time)

```bash
cd tools/zk-cli
cargo build --release

# Generate keys
./target/release/lumenyx-zk setup \
    --vk-output verification_key.bin \
    --pk-output proving_key.bin
```

### 2. Deploy Verification Key

```javascript
// Via Polkadot.js
const vkHex = '0x' + fs.readFileSync('verification_key.bin').toString('hex');
await api.tx.sudo.sudo(
    api.tx.privacy.setVerificationKey(vkHex)
).signAndSend(sudoAccount);
```

### 3. Shield Funds

```bash
# Generate commitment
./target/release/lumenyx-zk commitment --amount 100

# Output:
# 💰 Amount: 100 LUMENYX
# 🔑 Secret: abc123...     <- SAVE THIS!
# 🎲 Blinding: def456...   <- SAVE THIS!
# 📦 Commitment: 789xyz...
```

Then call on-chain:
```javascript
await api.tx.privacy.shield(100, commitment).signAndSend(account);
```

### 4. Unshield Funds

```bash
# Generate proof (need merkle path from chain)
./target/release/lumenyx-zk prove-unshield \
    --amount 100 \
    --secret abc123... \
    --blinding def456... \
    --merkle-path path.json \
    --pk-file proving_key.bin
```

Then call on-chain:
```javascript
await api.tx.privacy.unshield(
    amount,
    nullifier,
    root,
    proof
).signAndSend(account);
```

## Files Structure

```
lumenyx/
├── pallets/privacy/
│   ├── src/
│   │   ├── lib.rs     # Main pallet (shield, unshield, transfer)
│   │   ├── zk.rs      # ZK verifier interface
│   │   └── bn254.rs   # Full BN254 pairing implementation (29KB)
│   │                  # - Field tower: Fp → Fp2 → Fp6 → Fp12
│   │                  # - G1/G2 curve operations
│   │                  # - Optimal Ate pairing
│   │                  # - Groth16 verify_proof
│   └── Cargo.toml
│
├── tools/zk-cli/
│   ├── src/main.rs    # CLI tool (proof generation with arkworks)
│   ├── Cargo.toml
│   ├── verification_key.bin  # Deploy this on-chain!
│   └── proving_key.bin       # Keep safe (off-chain only)
│
└── docs/
    └── ZK_PRIVACY.md  # This file
```

### On-Chain vs Off-Chain Split

| Component | Location | Library |
|-----------|----------|---------|
| Proof Generation | Off-chain CLI | arkworks (full features) |
| Proof Verification | On-chain Runtime | Pure Rust bn254.rs (no_std) |
| Key Generation | Off-chain CLI | arkworks |
| Commitment Hash | Both | Blake2-256 |

## Comparison with Other Privacy Systems

| Feature | LUMENYX | Other Privacy | Other ZK | Mixer |
|---------|-------|--------|-------|---------|
| Privacy Type | Optional | Mandatory | Optional | Optional |
| Proof System | Groth16 | Ring Sigs | Groth16 | Groth16 |
| Verification | ~3ms | ~5ms | ~3ms | ~3ms |
| Exchange Compatible | ✅ Yes | ❌ No | ✅ Yes | ❌ No |
| Smart Contracts | ✅ Yes | ❌ No | ✅ Yes | ✅ Yes |

## Security Notes

1. **Trusted Setup**: The verification key is generated via trusted setup.
   In production, use MPC ceremony with multiple participants.

2. **Secret Management**: Users MUST save their secret and blinding.
   Lost secrets = lost funds forever.

3. **Merkle Tree**: Historical roots are kept to allow async proof generation.
   Proofs generated against old roots remain valid.

4. **Nullifier**: Double-spend protection. Once a nullifier is used,
   it cannot be reused.

## FAQ

**Q: Can exchanges list LUMENYX?**
A: Yes! Privacy is OPTIONAL. Default transactions are transparent.
   Same model as Zcash, which is listed on major exchanges.

**Q: Can someone link my shield and unshield?**
A: No. The nullifier cannot be linked to the commitment without
   knowing the secret, which only you know.

**Q: What if I lose my secret?**
A: Your funds are lost forever. There is no recovery mechanism.
   This is the price of true privacy.

**Q: How many notes can the tree hold?**
A: 2^20 = ~1 million notes. Can be upgraded if needed.
