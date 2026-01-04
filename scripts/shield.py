#!/usr/bin/env python3
"""
LUMENYX - Shield Funds (Deposit to Private Pool)

Uses Poseidon hash (ZK-friendly) with little-endian serialization
matching on-chain and ZK-CLI implementation.
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
    state = 0
    for i, input in enumerate(inputs):
        state = (state + input) % p
        state = state^5 % p
        state = (state + (i+1)) % p
    """
    state = 0
    for i, inp in enumerate(inputs):
        state = (state + inp) % FIELD_MODULUS
        state = pow(state, 5, FIELD_MODULUS)
        state = (state + (i + 1)) % FIELD_MODULUS
    return state

def compute_commitment(amount: int, secret: int, blinding: int) -> int:
    """Compute commitment = Poseidon(amount, secret, blinding)"""
    return poseidon_hash([amount, secret, blinding])

def compute_nullifier(commitment: int, secret: int) -> int:
    """Compute nullifier = Poseidon(commitment, secret)"""
    return poseidon_hash([commitment, secret])

def main():
    import argparse
    import secrets as secrets_module
    
    parser = argparse.ArgumentParser(description="Shield LUMENYX funds")
    parser.add_argument("--amount", type=float, required=True, help="Amount to shield")
    parser.add_argument("--seed", type=str, required=True, help="Your seed phrase")
    parser.add_argument("--local", action="store_true", help="Use local node")
    args = parser.parse_args()

    print("=" * 60)
    print("  LUMENYX - SHIELD FUNDS (Poseidon)")
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

    # Submit shield transaction
    print(f"\n📤 Submitting shield transaction...")

    call = substrate.compose_call(
        call_module="Privacy",
        call_function="shield",
        call_params={
            "amount": amount_raw,
            "commitment": commitment_h256,
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
    note_count = substrate.query("Privacy", "NoteCount")
    leaf_index = note_count.value - 1 if note_count.value else 0

    # Save note (secrets in little-endian hex for ZK-CLI compatibility)
    NOTES_DIR.mkdir(exist_ok=True)
    note_file = NOTES_DIR / f"note_{leaf_index}.json"
    
    note_data = {
        "amount": amount_raw,
        "secret": fr_to_hex_le(secret),
        "blinding": fr_to_hex_le(blinding),
        "commitment": commitment_h256,
        "leaf_index": leaf_index,
        "nullifier": fr_to_hex_le(compute_nullifier(commitment, secret)),
    }
    
    with open(note_file, 'w') as f:
        json.dump(note_data, f, indent=2)

    print(f"\n💾 Note saved to: {note_file}")
    print(f"\n⚠️  IMPORTANT: Keep your note file safe!")
    print(f"   You need it to unshield your funds.")


if __name__ == "__main__":
    main()
