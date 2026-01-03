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
P = 21888242871839275222246405745257275088548364400416034343698204186575808495617

def poseidon_hash(inputs: list[int]) -> int:
    state = 0
    for i, inp in enumerate(inputs):
        state = (state + inp) % P
        x2 = (state * state) % P
        x4 = (x2 * x2) % P
        state = (x4 * state) % P
        state = (state + (i + 1)) % P
    return state

def h256_to_int(h: str) -> int:
    if h.startswith("0x"):
        h = h[2:]
    return int(h, 16)

def int_to_hex(value: int) -> str:
    return value.to_bytes(32, 'big').hex()

def int_to_h256(value: int) -> str:
    return "0x" + int_to_hex(value)

def build_merkle_tree(leaves: list[int]) -> list[list[int]]:
    tree = [leaves[:]]
    current = leaves[:]
    while len(current) > 1:
        next_level = []
        for i in range(0, len(current), 2):
            left = current[i]
            right = current[i + 1] if i + 1 < len(current) else 0
            next_level.append(poseidon_hash([left, right]))
        tree.append(next_level)
        current = next_level
    return tree

def get_merkle_path(tree: list[list[int]], leaf_index: int) -> tuple[list[int], list[bool]]:
    path = []
    indices = []
    idx = leaf_index
    for level in tree[:-1]:
        is_right = idx % 2 == 1
        sibling_idx = idx - 1 if is_right else idx + 1
        sibling = level[sibling_idx] if sibling_idx < len(level) else 0
        path.append(sibling)
        indices.append(is_right)
        idx //= 2
    return path, indices

def compute_nullifier(commitment: int, secret: int) -> int:
    return poseidon_hash([commitment, secret])

def main():
    import argparse
    parser = argparse.ArgumentParser(description="Unshield LUMENYX funds with ZK proof")
    parser.add_argument("--note", type=str, required=True, help="Path to note JSON file")
    parser.add_argument("--seed", type=str, required=True, help="Recipient seed phrase")
    parser.add_argument("--local", action="store_true", help="Use local node")
    args = parser.parse_args()

    print("=" * 60)
    print("  LUMENYX - UNSHIELD FUNDS (ZK PROOF)")
    print("=" * 60)
    
    # Load note
    print(f"\n📂 Loading note from {args.note}...")
    with open(args.note, 'r') as f:
        note = json.load(f)
    
    leaf_index = note["leaf_index"]
    amount = note["amount"]
    amount_scaled = note["amount_scaled"]
    secret = int(note["secret"], 16)
    blinding = int(note["blinding"], 16)
    commitment_int = h256_to_int(note["commitment"])
    
    print(f"   Leaf index: {leaf_index}")
    print(f"   Amount: {amount} LUMENYX")
    
    # Connect
    url = LOCAL if args.local else MAINNET
    print(f"\n🔌 Connecting to {url}...")
    substrate = SubstrateInterface(url=url)
    
    # Check if already spent
    nullifier = compute_nullifier(commitment_int, secret)
    nullifier_hex = int_to_h256(nullifier)
    
    is_spent = substrate.query("Privacy", "SpentNullifiers", [nullifier_hex])
    if is_spent.value:
        print(f"❌ This note has already been spent!")
        sys.exit(1)
    
    # Get all commitments and build tree
    print(f"\n🌳 Building Merkle tree...")
    next_index = substrate.query("Privacy", "NextIndex")
    num_leaves = next_index.value
    
    leaves = []
    for i in range(num_leaves):
        commitment = substrate.query("Privacy", "Commitments", [i])
        if commitment.value:
            leaves.append(h256_to_int(commitment.value))
        else:
            leaves.append(0)
    
    tree_size = 1 << TREE_DEPTH
    while len(leaves) < tree_size:
        leaves.append(0)
    
    tree = build_merkle_tree(leaves)
    root = tree[-1][0]
    root_hex = int_to_h256(root)
    
    # Get Merkle path
    path, indices = get_merkle_path(tree, leaf_index)
    
    # Save merkle path for CLI
    merkle_path_file = "/tmp/merkle_path.json"
    merkle_data = {
        "root": int_to_hex(root),
        "path": [int_to_hex(p) for p in path],
        "indices": indices
    }
    with open(merkle_path_file, 'w') as f:
        json.dump(merkle_data, f)
    
    print(f"   Root: {root_hex[:20]}...")
    print(f"   Nullifier: {nullifier_hex[:20]}...")
    
    # Generate ZK proof using CLI
    print(f"\n🔮 Generating ZK proof...")
    
    zk_cli = Path.home() / "lumenyx/tools/zk-cli/target/release/lumenyx-zk"
    if not zk_cli.exists():
        print(f"❌ ZK CLI not found at {zk_cli}")
        print(f"   Build it: cd ~/lumenyx/tools/zk-cli && cargo build --release")
        sys.exit(1)
    
    # Run CLI to generate proof
    cmd = [
        str(zk_cli), "prove-unshield",
        "--amount", str(amount_scaled),
        "--secret", note["secret"].replace("0x", ""),
        "--blinding", note["blinding"].replace("0x", ""),
        "--merkle-path", merkle_path_file,
        "--pk-file", str(Path.home() / "lumenyx/tools/zk-cli/proving_key.bin")
    ]
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
        if result.returncode != 0:
            print(f"❌ Proof generation failed:")
            print(result.stderr)
            sys.exit(1)
        
        # Parse proof from output
        output = result.stdout
        print(output)
        
        # Extract proof hex from output
        proof_hex = None
        for line in output.split('\n'):
            if 'proof: 0x' in line:
                proof_hex = line.split('proof: ')[1].strip()
                break
        
        if not proof_hex:
            print(f"❌ Could not parse proof from CLI output")
            sys.exit(1)
            
    except subprocess.TimeoutExpired:
        print(f"❌ Proof generation timed out")
        sys.exit(1)
    except Exception as e:
        print(f"❌ Error running ZK CLI: {e}")
        sys.exit(1)
    
    print(f"   ✅ Proof generated!")
    
    # Submit unshield transaction
    print(f"\n📤 Submitting unshield transaction...")
    
    keypair = Keypair.create_from_mnemonic(args.seed)
    print(f"   Recipient: {keypair.ss58_address}")
    
    call = substrate.compose_call(
        call_module='Privacy',
        call_function='unshield',
        call_params={
            'amount': amount_scaled,
            'nullifier': nullifier_hex,
            'root': root_hex,
            'proof': proof_hex
        }
    )
    
    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)
    
    try:
        receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        if receipt.is_success:
            print(f"   ✅ Unshield successful! Block: {receipt.block_hash[:18]}...")
        else:
            print(f"   ❌ Failed: {receipt.error_message}")
            sys.exit(1)
    except Exception as e:
        print(f"   ❌ Transaction failed: {e}")
        sys.exit(1)
    
    # Check new balance
    account = substrate.query("System", "Account", [keypair.ss58_address])
    balance = account.value["data"]["free"] / 10**12
    print(f"\n💰 New balance: {balance} LUMENYX")
    
    print("\n" + "=" * 60)
    print("  ✅ UNSHIELD COMPLETE!")
    print("  Your funds are now transparent again.")
    print("=" * 60)

if __name__ == "__main__":
    main()
