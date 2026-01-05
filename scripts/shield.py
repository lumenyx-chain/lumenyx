#!/usr/bin/env python3
"""
LUMENYX - Shield Funds (Deposit to Private Pool)

v3.0: Merkle root calculated OFF-CHAIN
- Downloads existing commitments from chain
- Computes new Merkle root locally using Poseidon
- Submits shield with pre-computed root
"""

import sys
import os
import json
from pathlib import Path

try:
    from substrateinterface import SubstrateInterface, Keypair
except ImportError:
    print("ERROR: pip install substrate-interface")
    sys.exit(1)

MAINNET = "ws://89.147.111.102:9944"
LOCAL = "ws://127.0.0.1:9944"
NOTES_DIR = Path.home() / ".lumenyx-notes"

# BN254 scalar field modulus
FIELD_MODULUS = 21888242871839275222246405745257275088548364400416034343698204186575808495617

# Merkle tree depth
TREE_DEPTH = 20

def fr_to_hex_le(x: int) -> str:
    """Convert field element to hex (little-endian, ark-serialize compatible)"""
    x = x % FIELD_MODULUS
    return x.to_bytes(32, 'little').hex()

def hex_to_fr_le(hex_str: str) -> int:
    """Convert hex to field element (little-endian)"""
    if hex_str.startswith("0x"):
        hex_str = hex_str[2:]
    b = bytes.fromhex(hex_str)
    if len(b) != 32:
        raise ValueError(f"Expected 32 bytes, got {len(b)}")
    x = int.from_bytes(b, 'little')
    if x >= FIELD_MODULUS:
        raise ValueError("Non-canonical field element")
    return x

def fr_to_h256(x: int) -> str:
    """Convert field element to H256 hex (big-endian, for chain storage)"""
    x = x % FIELD_MODULUS
    return "0x" + x.to_bytes(32, 'big').hex()

def h256_to_fr(hex_str: str) -> int:
    """Convert H256 hex to field element (big-endian input)"""
    if hex_str.startswith("0x"):
        hex_str = hex_str[2:]
    b = bytes.fromhex(hex_str)
    return int.from_bytes(b, 'big') % FIELD_MODULUS

def poseidon_hash(inputs: list) -> int:
    """
    Poseidon hash matching ZK circuit and on-chain implementation.
    """
    state = 0
    for i, inp in enumerate(inputs):
        state = (state + inp) % FIELD_MODULUS
        state = pow(state, 5, FIELD_MODULUS)
        state = (state + (i + 1)) % FIELD_MODULUS
    return state

def poseidon_hash_pair(left: int, right: int) -> int:
    """Hash two field elements together"""
    return poseidon_hash([left, right])

def compute_commitment(amount: int, secret: int, blinding: int) -> int:
    """Compute commitment = Poseidon(amount, secret, blinding)"""
    return poseidon_hash([amount, secret, blinding])

def compute_nullifier(commitment: int, secret: int) -> int:
    """Compute nullifier = Poseidon(commitment, secret)"""
    return poseidon_hash([commitment, secret])

def compute_merkle_root(leaves: list, new_leaf: int, new_index: int) -> int:
    """
    Compute Merkle root after inserting new_leaf at new_index.
    Uses Poseidon hash for ZK compatibility.
    """
    # Add new leaf to the list
    all_leaves = leaves + [new_leaf]
    
    # Pad to power of 2 with zeros
    tree_size = 2 ** TREE_DEPTH
    while len(all_leaves) < tree_size:
        all_leaves.append(0)
    
    # Build tree bottom-up
    current_level = all_leaves
    for level in range(TREE_DEPTH):
        next_level = []
        for i in range(0, len(current_level), 2):
            left = current_level[i]
            right = current_level[i + 1] if i + 1 < len(current_level) else 0
            next_level.append(poseidon_hash_pair(left, right))
        current_level = next_level
    
    return current_level[0] if current_level else 0

def compute_merkle_root_incremental(existing_commitments: list, new_commitment: int) -> int:
    """
    Compute Merkle root incrementally (more efficient for large trees).
    Uses frontier algorithm like the on-chain version used to.
    """
    num_leaves = len(existing_commitments) + 1
    all_commitments = existing_commitments + [new_commitment]
    
    # For small trees, just compute directly
    if num_leaves <= 1024:  # 2^10
        return compute_merkle_root(existing_commitments, new_commitment, len(existing_commitments))
    
    # For larger trees, use frontier algorithm
    # This matches what the chain would compute
    depth = TREE_DEPTH
    filled_subtrees = [0] * depth
    
    for idx, leaf in enumerate(all_commitments):
        node = leaf
        current_idx = idx
        
        for level in range(depth):
            if (current_idx & 1) == 0:
                # Even index - this is a left child, save to frontier
                filled_subtrees[level] = node
                break
            else:
                # Odd index - combine with left sibling from frontier
                left = filled_subtrees[level]
                node = poseidon_hash_pair(left, node)
            current_idx >>= 1
    
    # Compute root from frontier
    node = None
    for level in range(depth):
        level_node = filled_subtrees[level] if filled_subtrees[level] != 0 else None
        if node is None:
            node = level_node
        elif level_node is not None:
            node = poseidon_hash_pair(level_node, node)
        # If level_node is None, we use zero hash
        elif node is not None:
            # Hash with zero
            zero = 0
            for _ in range(level):
                zero = poseidon_hash_pair(zero, zero)
            node = poseidon_hash_pair(zero, node)
    
    return node if node else 0

def get_existing_commitments(substrate) -> list:
    """Download all existing commitments from chain"""
    next_index = substrate.query("Privacy", "NextIndex")
    num_commitments = next_index.value if next_index.value else 0
    
    print(f"📥 Downloading {num_commitments} existing commitments...")
    
    commitments = []
    for i in range(num_commitments):
        result = substrate.query("Privacy", "Commitments", [i])
        if result.value:
            commitment_fr = h256_to_fr(result.value)
            commitments.append(commitment_fr)
        else:
            commitments.append(0)  # Empty slot
    
    return commitments

def main():
    import argparse
    import secrets as secrets_module

    parser = argparse.ArgumentParser(description="Shield LUMENYX funds")
    parser.add_argument("--amount", type=float, required=True, help="Amount to shield")
    parser.add_argument("--seed", type=str, required=True, help="Your seed phrase")
    parser.add_argument("--local", action="store_true", help="Use local node")
    args = parser.parse_args()

    print("=" * 60)
    print("  LUMENYX - SHIELD FUNDS v3.0 (Off-chain Merkle)")
    print("=" * 60)

    # Connect
    url = LOCAL if args.local else MAINNET
    print(f"\n🔌 Connecting to {url}...")

    try:
        substrate = SubstrateInterface(url=url)
    except Exception as e:
        print(f"❌ Connection failed: {e}")
        sys.exit(1)

    # Load keypair
    keypair = Keypair.create_from_mnemonic(args.seed)
    print(f"📍 Account: {keypair.ss58_address}")

    # Check balance
    account = substrate.query("System", "Account", [keypair.ss58_address])
    balance = account.value["data"]["free"] / 10**12
    print(f"💰 Balance: {balance} LUMENYX")

    amount_raw = int(args.amount * 10**12)
    if balance < args.amount + 0.1:
        print(f"❌ Insufficient balance (need {args.amount} + fees)")
        sys.exit(1)

    # Generate secrets (random field elements)
    secret = secrets_module.randbelow(FIELD_MODULUS)
    blinding = secrets_module.randbelow(FIELD_MODULUS)

    print(f"\n🔐 Generated secrets (SAVE THESE!):")
    print(f"   Secret: {fr_to_hex_le(secret)}")
    print(f"   Blinding: {fr_to_hex_le(blinding)}")

    # Compute commitment using Poseidon
    commitment = compute_commitment(amount_raw, secret, blinding)
    commitment_h256 = fr_to_h256(commitment)

    print(f"📦 Commitment: {commitment_h256}")

    # Get existing commitments and compute new Merkle root OFF-CHAIN
    existing_commitments = get_existing_commitments(substrate)
    
    print(f"🌲 Computing Merkle root off-chain...")
    new_merkle_root = compute_merkle_root_incremental(existing_commitments, commitment)
    merkle_root_h256 = fr_to_h256(new_merkle_root)
    
    print(f"🌲 New Merkle root: {merkle_root_h256}")

    # Submit shield transaction with pre-computed merkle root
    print(f"\n📤 Submitting shield transaction...")

    call = substrate.compose_call(
        call_module="Privacy",
        call_function="shield",
        call_params={
            "amount": amount_raw,
            "commitment": commitment_h256,
            "merkle_root": merkle_root_h256,
        }
    )

    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)

    try:
        receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        if receipt.is_success:
            print(f"✅ Shield successful!")
            print(f"   Block: {receipt.block_hash}")
        else:
            print(f"❌ Shield failed: {receipt.error_message}")
            sys.exit(1)
    except Exception as e:
        print(f"❌ Transaction error: {e}")
        sys.exit(1)

    # Get leaf index
    leaf_index = len(existing_commitments)

    # Save note
    NOTES_DIR.mkdir(exist_ok=True)
    note_file = NOTES_DIR / f"note_{leaf_index}.json"

    note_data = {
        "amount": amount_raw,
        "secret": fr_to_hex_le(secret),
        "blinding": fr_to_hex_le(blinding),
        "commitment": commitment_h256,
        "leaf_index": leaf_index,
        "merkle_root": merkle_root_h256,
        "nullifier": fr_to_hex_le(compute_nullifier(commitment, secret)),
    }

    with open(note_file, 'w') as f:
        json.dump(note_data, f, indent=2)

    print(f"\n💾 Note saved to: {note_file}")
    print(f"\n⚠️  IMPORTANT: Keep your note file safe!")
    print(f"   You need it to unshield your funds.")


if __name__ == "__main__":
    main()
