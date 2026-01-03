#!/usr/bin/env python3
"""
LUMENYX - Get Merkle Path for ZK Proof

Retrieves the Merkle path for a shielded note from the chain.
Uses Blake2 hash matching on-chain implementation.

Usage:
    python3 get_merkle_path.py --leaf-index 0
"""

import sys
import json
import hashlib
from pathlib import Path

try:
    from substrateinterface import SubstrateInterface
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
    """Convert bytes to hex string (no 0x prefix)"""
    return b.hex()

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
    parser = argparse.ArgumentParser(description="Get Merkle path for ZK proof")
    parser.add_argument("--leaf-index", type=int, required=True, help="Leaf index of your note")
    parser.add_argument("--local", action="store_true", help="Use local node")
    parser.add_argument("--output", type=str, default="merkle_path.json", help="Output file")
    args = parser.parse_args()

    print("=" * 60)
    print("  LUMENYX - GET MERKLE PATH (Blake2)")
    print("=" * 60)

    # Connect
    url = LOCAL if args.local else MAINNET
    print(f"\n🔌 Connecting to {url}...")
    substrate = SubstrateInterface(url=url)

    # Get tree info
    next_index = substrate.query("Privacy", "NextIndex")
    num_leaves = next_index.value
    print(f"🌳 Tree has {num_leaves} leaves")

    if args.leaf_index >= num_leaves:
        print(f"❌ Leaf index {args.leaf_index} doesn't exist (max: {num_leaves - 1})")
        sys.exit(1)

    # Get all commitments
    print(f"📥 Fetching commitments...")
    leaves = []
    for i in range(num_leaves):
        commitment = substrate.query("Privacy", "Commitments", [i])
        if commitment.value:
            leaves.append(hex_to_bytes(commitment.value))
        else:
            leaves.append(bytes(32))

    # Pad to tree size
    tree_size = 1 << TREE_DEPTH
    while len(leaves) < tree_size:
        leaves.append(bytes(32))

    # Build tree
    print(f"🔨 Building Merkle tree (depth {TREE_DEPTH})...")
    tree = build_merkle_tree(leaves)
    root = tree[-1][0]

    # Get path
    path, indices = get_merkle_path(tree, args.leaf_index)

    # Get on-chain root for verification
    chain_root = substrate.query("Privacy", "CurrentMerkleRoot")
    chain_root_bytes = hex_to_bytes(chain_root.value) if chain_root.value else bytes(32)

    print(f"\n📊 Results:")
    print(f"   Leaf index: {args.leaf_index}")
    print(f"   Computed root: {bytes_to_hex(root)[:20]}...")
    print(f"   On-chain root: {bytes_to_hex(chain_root_bytes)[:20]}...")

    if root == chain_root_bytes:
        print(f"   ✅ Roots match!")
    else:
        print(f"   ⚠️  Roots don't match - tree may have changed")

    # Save to file
    output = {
        "leaf_index": args.leaf_index,
        "root": bytes_to_hex(root),
        "path": [bytes_to_hex(p) for p in path],
        "indices": indices
    }

    with open(args.output, 'w') as f:
        json.dump(output, f, indent=2)

    print(f"\n💾 Merkle path saved to: {args.output}")
    print(f"\n📋 Use with lumenyx-zk CLI:")
    print(f"   ./lumenyx-zk prove-unshield \\")
    print(f"     --amount <amount> \\")
    print(f"     --secret <your-secret> \\")
    print(f"     --blinding <your-blinding> \\")
    print(f"     --merkle-path {args.output}")

if __name__ == "__main__":
    main()
