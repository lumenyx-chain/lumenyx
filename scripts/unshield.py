#!/usr/bin/env python3
"""
LUMENYX - Unshield Funds (Withdraw from Private Pool)

Uses Poseidon hash with little-endian serialization matching ZK-CLI.
"""

import sys
import os
import json
import subprocess
import tempfile
from pathlib import Path

try:
    from substrateinterface import SubstrateInterface, Keypair
except ImportError:
    print("ERROR: pip install substrate-interface")
    sys.exit(1)

MAINNET = "ws://89.147.111.102:9944"
LOCAL = "ws://127.0.0.1:9944"
ZK_CLI = Path(__file__).parent.parent / "tools" / "zk-cli" / "target" / "release" / "lumenyx-zk"
PK_FILE = Path(__file__).parent.parent / "tools" / "zk-cli" / "proving_key.bin"

# BN254 scalar field modulus
FIELD_MODULUS = 21888242871839275222246405745257275088548364400416034343698204186575808495617

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
    """Poseidon hash matching ZK circuit and on-chain"""
    state = 0
    for i, inp in enumerate(inputs):
        state = (state + inp) % FIELD_MODULUS
        state = pow(state, 5, FIELD_MODULUS)
        state = (state + (i + 1)) % FIELD_MODULUS
    return state

def poseidon_hash_pair(left: int, right: int) -> int:
    """Hash two values for Merkle tree"""
    return poseidon_hash([left, right])

def build_merkle_tree(leaves: list, depth: int = 20) -> list:
    """Build Merkle tree with Poseidon hash"""
    size = 1 << depth
    padded = leaves + [0] * (size - len(leaves))
    
    tree = [padded]
    current = padded
    
    while len(current) > 1:
        next_level = []
        for i in range(0, len(current), 2):
            left = current[i]
            right = current[i + 1] if i + 1 < len(current) else 0
            next_level.append(poseidon_hash_pair(left, right))
        tree.append(next_level)
        current = next_level
    
    return tree

def get_merkle_path(tree: list, leaf_index: int) -> tuple:
    """Get Merkle path for a leaf"""
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

def main():
    import argparse
    parser = argparse.ArgumentParser(description="Unshield LUMENYX funds")
    parser.add_argument("--note", type=str, required=True, help="Path to note JSON file")
    parser.add_argument("--seed", type=str, required=True, help="Recipient seed phrase")
    parser.add_argument("--local", action="store_true", help="Use local node")
    args = parser.parse_args()

    print("=" * 60)
    print("  LUMENYX - UNSHIELD FUNDS (ZK Proof)")
    print("=" * 60)

    # Load note
    note_path = Path(args.note).expanduser()
    if not note_path.exists():
        print(f"❌ Note file not found: {note_path}")
        sys.exit(1)

    with open(note_path) as f:
        note = json.load(f)

    amount = note["amount"]
    secret = hex_to_fr_le(note["secret"])
    blinding = hex_to_fr_le(note["blinding"])
    leaf_index = note["leaf_index"]
    
    print(f"\n📄 Note loaded:")
    print(f"   Amount: {amount / 10**12} LUMENYX")
    print(f"   Leaf index: {leaf_index}")

    # Connect
    url = LOCAL if args.local else MAINNET
    print(f"\n🔌 Connecting to {url}...")
    
    try:
        substrate = SubstrateInterface(url=url)
    except Exception as e:
        print(f"❌ Connection failed: {e}")
        sys.exit(1)

    # Load recipient keypair
    keypair = Keypair.create_from_mnemonic(args.seed)
    print(f"📍 Recipient: {keypair.ss58_address}")

    # Get all commitments from chain and convert to field elements
    print(f"\n🌳 Building Merkle tree...")
    note_count = substrate.query("Privacy", "NoteCount").value or 0
    
    commitments = []
    for i in range(note_count):
        c = substrate.query("Privacy", "Commitments", [i])
        if c.value:
            # H256 is big-endian, convert to field element
            c_int = h256_to_fr(c.value)
            commitments.append(c_int)
        else:
            commitments.append(0)

    # Build tree and get path
    tree = build_merkle_tree(commitments, depth=20)
    root = tree[-1][0] if tree[-1] else 0
    path, indices = get_merkle_path(tree, leaf_index)

    # Verify on-chain root matches
    on_chain_root = substrate.query("Privacy", "CurrentMerkleRoot").value
    on_chain_root_int = h256_to_fr(on_chain_root) if on_chain_root else 0
    
    print(f"   Computed root: {fr_to_hex_le(root)[:32]}...")
    print(f"   On-chain root: {fr_to_hex_le(on_chain_root_int)[:32]}...")

    if root != on_chain_root_int:
        print(f"⚠️  Root mismatch! Computed vs on-chain differ.")
        print(f"   This may indicate hash function mismatch.")

    # Compute nullifier
    commitment = poseidon_hash([amount, secret, blinding])
    nullifier = poseidon_hash([commitment, secret])

    print(f"\n🔐 Nullifier: {fr_to_hex_le(nullifier)[:32]}...")

    # Check if already spent
    nullifier_h256 = fr_to_h256(nullifier)
    is_spent = substrate.query("Privacy", "SpentNullifiers", [nullifier_h256])
    if is_spent.value:
        print(f"❌ This note has already been spent!")
        sys.exit(1)

    # Generate ZK proof using CLI
    print(f"\n🔮 Generating ZK proof...")
    
    if not ZK_CLI.exists():
        print(f"❌ ZK CLI not found at {ZK_CLI}")
        print(f"   Run: cd tools/zk-cli && cargo build --release")
        sys.exit(1)

    # Create merkle path JSON file (little-endian hex for ark compatibility)
    with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
        merkle_data = {
            "path": [fr_to_hex_le(p) for p in path],
            "indices": indices,
            "root": fr_to_hex_le(root)
        }
        json.dump(merkle_data, f)
        merkle_path_file = f.name

    try:
        # Call ZK CLI with little-endian hex
        result = subprocess.run([
            str(ZK_CLI), "prove-unshield",
            "--amount", str(amount),
            "--secret", fr_to_hex_le(secret),
            "--blinding", fr_to_hex_le(blinding),
            "--merkle-path", merkle_path_file,
            "--pk-file", str(PK_FILE)
        ], capture_output=True, text=True, timeout=60)

        if result.returncode != 0:
            print(f"❌ Proof generation failed:")
            print(result.stderr)
            sys.exit(1)

        # Parse proof from output
        output = result.stdout
        proof_line = [l for l in output.split('\n') if 'proof: 0x' in l]
        if not proof_line:
            print(f"❌ Could not parse proof from output")
            print(output)
            sys.exit(1)
        
        proof_hex = proof_line[0].split('proof: ')[1].strip().rstrip(')')
        print(f"   ✅ Proof generated ({len(proof_hex)//2 - 1} bytes)")

    finally:
        os.unlink(merkle_path_file)

    # Submit unshield transaction
    print(f"\n📤 Submitting unshield transaction...")

    call = substrate.compose_call(
        call_module="Privacy",
        call_function="unshield",
        call_params={
            "amount": amount,
            "nullifier": nullifier_h256,
            "root": fr_to_h256(root),
            "proof": proof_hex,
        }
    )

    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)

    try:
        receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        if receipt.is_success:
            print(f"✅ Unshield successful!")
            print(f"   Block: {receipt.block_hash}")
            print(f"   Amount: {amount / 10**12} LUMENYX returned to {keypair.ss58_address}")
        else:
            print(f"❌ Unshield failed: {receipt.error_message}")
            sys.exit(1)
    except Exception as e:
        print(f"❌ Transaction error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
