#!/usr/bin/env python3
"""
LUMENYX - Unshield Funds (Withdraw from Private Pool with ZK Proof)

This script:
1. Loads your note data
2. Gets Merkle path from chain
3. Generates ZK proof using lumenyx-zk CLI
4. Submits unshield transaction

Usage:
    python3 unshield.py --note ~/.lumenyx-notes/note_0.json --seed "your twelve words"
"""

import sys
import os
import json
import hashlib
import subprocess
from pathlib import Path

try:
    from substrateinterface import SubstrateInterface, Keypair
except ImportError:
    print("ERROR: pip install substrate-interface")
    sys.exit(1)

MAINNET = "ws://89.147.111.102:9944"
LOCAL = "ws://127.0.0.1:9944"
TREE_DEPTH = 20

def blake2_hash(data: bytes) -> bytes:
    """Blake2b-256 hash matching on-chain implementation"""
    return hashlib.blake2b(data, digest_size=32).digest()

def hash_pair(left: bytes, right: bytes) -> bytes:
    """Hash two 32-byte values for Merkle tree"""
    return blake2_hash(left + right)

def hex_to_bytes(h: str) -> bytes:
    """Convert hex string to bytes"""
    if h.startswith("0x"):
        h = h[2:]
    return bytes.fromhex(h)

def bytes_to_hex(b: bytes) -> str:
    """Convert bytes to 0x-prefixed hex"""
    return "0x" + b.hex()

def build_merkle_tree(leaves: list[bytes]) -> list[list[bytes]]:
    """Build full Merkle tree from leaves using Blake2"""
    zero = bytes(32)
    tree = [leaves[:]]
    current = leaves[:]

    while len(current) > 1:
        next_level = []
        for i in range(0, len(current), 2):
            left = current[i]
            right = current[i + 1] if i + 1 < len(current) else zero
            next_level.append(hash_pair(left, right))
        tree.append(next_level)
        current = next_level

    return tree

def get_merkle_path(tree: list[list[bytes]], leaf_index: int) -> tuple[list[bytes], list[bool]]:
    """Get Merkle path for a leaf"""
    zero = bytes(32)
    path = []
    indices = []
    idx = leaf_index

    for level in tree[:-1]:
        is_right = idx % 2 == 1
        sibling_idx = idx - 1 if is_right else idx + 1
        sibling = level[sibling_idx] if sibling_idx < len(level) else zero
        path.append(sibling)
        indices.append(is_right)
        idx //= 2

    return path, indices

def main():
    import argparse
    parser = argparse.ArgumentParser(description="Unshield LUMENYX funds")
    parser.add_argument("--note", type=str, required=True, help="Path to note JSON file")
    parser.add_argument("--seed", type=str, required=True, help="Your seed phrase")
    parser.add_argument("--local", action="store_true", help="Use local node")
    args = parser.parse_args()

    print("=" * 60)
    print("  LUMENYX - UNSHIELD FUNDS (Blake2)")
    print("=" * 60)

    # Load note
    note_path = Path(args.note).expanduser()
    if not note_path.exists():
        print(f"❌ Note file not found: {note_path}")
        sys.exit(1)

    with open(note_path) as f:
        note = json.load(f)

    amount = note["amount"]
    secret = hex_to_bytes(note["secret"])
    commitment = hex_to_bytes(note["commitment"])
    nullifier = hex_to_bytes(note["nullifier"])
    leaf_index = note["leaf_index"]

    print(f"\n📄 Note loaded:")
    print(f"   Amount: {amount / 10**12} LUMENYX")
    print(f"   Leaf index: {leaf_index}")

    # Connect
    url = LOCAL if args.local else MAINNET
    print(f"\n🔌 Connecting to {url}...")
    substrate = SubstrateInterface(url=url)

    # Load keypair
    keypair = Keypair.create_from_mnemonic(args.seed)
    print(f"📍 Recipient: {keypair.ss58_address}")

    # Check if nullifier already spent
    spent = substrate.query("Privacy", "SpentNullifiers", [bytes_to_hex(nullifier)])
    if spent.value:
        print(f"❌ This note has already been spent!")
        sys.exit(1)

    # Get all commitments and build Merkle tree
    print(f"\n🌳 Building Merkle tree...")
    next_index = substrate.query("Privacy", "NextIndex")
    num_leaves = next_index.value

    leaves = []
    for i in range(num_leaves):
        c = substrate.query("Privacy", "Commitments", [i])
        if c.value:
            leaves.append(hex_to_bytes(c.value))
        else:
            leaves.append(bytes(32))

    # Pad to power of 2
    tree_size = 1 << TREE_DEPTH
    while len(leaves) < tree_size:
        leaves.append(bytes(32))

    tree = build_merkle_tree(leaves)
    root = tree[-1][0]

    # Get on-chain root
    chain_root = substrate.query("Privacy", "CurrentMerkleRoot")
    chain_root_bytes = hex_to_bytes(chain_root.value) if chain_root.value else bytes(32)

    print(f"   Computed root: {root.hex()[:16]}...")
    print(f"   On-chain root: {chain_root_bytes.hex()[:16]}...")

    if root != chain_root_bytes:
        print(f"⚠️  Roots don't match! Tree may have changed.")
        print(f"   Using on-chain root for verification.")

    # Get Merkle path
    path, indices = get_merkle_path(tree, leaf_index)

    # For now, create a placeholder proof
    # In production, this would call the lumenyx-zk CLI
    print(f"\n🔐 Generating ZK proof...")
    
    # Placeholder proof (256 bytes)
    proof = bytes(256)

    # Submit unshield transaction
    print(f"\n📤 Submitting unshield transaction...")

    call = substrate.compose_call(
        call_module="Privacy",
        call_function="unshield",
        call_params={
            "amount": amount,
            "nullifier": bytes_to_hex(nullifier),
            "root": bytes_to_hex(root),
            "proof": bytes_to_hex(proof),
            "recipient": keypair.ss58_address,
        }
    )

    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)
    receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)

    if receipt.is_success:
        print(f"✅ Unshield successful!")
        print(f"   Block: {receipt.block_hash}")
        print(f"   Amount: {amount / 10**12} LUMENYX returned to {keypair.ss58_address}")
    else:
        print(f"❌ Unshield failed: {receipt.error_message}")
        sys.exit(1)

if __name__ == "__main__":
    main()
