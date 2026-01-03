#!/usr/bin/env python3
"""
LUMENYX - Shield Funds (Deposit to Private Pool)

This script:
1. Generates secret and blinding factors
2. Computes commitment using Blake2
3. Submits shield transaction
4. Saves note data for later unshield

Usage:
    python3 shield.py --amount 100 --seed "your twelve words"
"""

import sys
import os
import json
import hashlib
import secrets
from pathlib import Path

try:
    from substrateinterface import SubstrateInterface, Keypair
except ImportError:
    print("ERROR: pip install substrate-interface")
    sys.exit(1)

MAINNET = "ws://89.147.111.102:9944"
LOCAL = "ws://127.0.0.1:9944"
NOTES_DIR = Path.home() / ".lumenyx-notes"

def blake2_hash(data: bytes) -> bytes:
    """Blake2b-256 hash matching on-chain implementation"""
    return hashlib.blake2b(data, digest_size=32).digest()

def hash_pair(left: bytes, right: bytes) -> bytes:
    """Hash two 32-byte values for Merkle tree"""
    return blake2_hash(left + right)

def compute_commitment(amount: int, secret: bytes, blinding: bytes) -> bytes:
    """Compute commitment = Blake2(amount || secret || blinding)"""
    amount_bytes = amount.to_bytes(16, 'little')
    data = amount_bytes + secret + blinding
    return blake2_hash(data)

def compute_nullifier(commitment: bytes, secret: bytes) -> bytes:
    """Compute nullifier = Blake2(commitment || secret)"""
    return blake2_hash(commitment + secret)

def bytes_to_hex(b: bytes) -> str:
    """Convert bytes to 0x-prefixed hex"""
    return "0x" + b.hex()

def main():
    import argparse
    parser = argparse.ArgumentParser(description="Shield LUMENYX funds")
    parser.add_argument("--amount", type=int, required=True, help="Amount to shield (in LUMENYX)")
    parser.add_argument("--seed", type=str, required=True, help="Your seed phrase")
    parser.add_argument("--local", action="store_true", help="Use local node")
    args = parser.parse_args()

    print("=" * 60)
    print("  LUMENYX - SHIELD FUNDS (Blake2)")
    print("=" * 60)

    # Connect
    url = LOCAL if args.local else MAINNET
    print(f"\n🔌 Connecting to {url}...")
    substrate = SubstrateInterface(url=url)

    # Load keypair
    keypair = Keypair.create_from_mnemonic(args.seed)
    print(f"📍 Account: {keypair.ss58_address}")

    # Check balance
    account = substrate.query("System", "Account", [keypair.ss58_address])
    balance = account.value["data"]["free"] / 10**12
    print(f"💰 Balance: {balance} LUMENYX")

    amount_planck = args.amount * 10**12

    if balance < args.amount:
        print(f"❌ Insufficient balance! Need {args.amount}, have {balance}")
        sys.exit(1)

    # Generate secrets (32 bytes each)
    secret = secrets.token_bytes(32)
    blinding = secrets.token_bytes(32)

    print(f"\n🔐 Generated secrets (SAVE THESE!):")
    print(f"   Secret: {secret.hex()}")
    print(f"   Blinding: {blinding.hex()}")

    # Compute commitment
    commitment = compute_commitment(amount_planck, secret, blinding)
    print(f"\n📦 Commitment: {bytes_to_hex(commitment)}")

    # Create and submit transaction
    print(f"\n📤 Submitting shield transaction...")
    
    call = substrate.compose_call(
        call_module="Privacy",
        call_function="shield",
        call_params={
            "amount": amount_planck,
            "commitment": bytes_to_hex(commitment),
        }
    )

    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)
    receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)

    if receipt.is_success:
        print(f"✅ Shield successful!")
        print(f"   Block: {receipt.block_hash}")

        # Get leaf index from events
        leaf_index = None
        for event in receipt.triggered_events:
            if event.value["event_id"] == "Shielded":
                leaf_index = event.value["attributes"].get("leaf_index", 0)
                break

        # Save note
        NOTES_DIR.mkdir(exist_ok=True)
        note = {
            "amount": amount_planck,
            "secret": secret.hex(),
            "blinding": blinding.hex(),
            "commitment": commitment.hex(),
            "leaf_index": leaf_index,
            "nullifier": compute_nullifier(commitment, secret).hex(),
        }

        note_file = NOTES_DIR / f"note_{leaf_index}.json"
        with open(note_file, 'w') as f:
            json.dump(note, f, indent=2)

        print(f"\n💾 Note saved to: {note_file}")
        print(f"\n⚠️  IMPORTANT: Keep your note file safe!")
        print(f"   You need it to unshield your funds.")
    else:
        print(f"❌ Shield failed: {receipt.error_message}")
        sys.exit(1)

if __name__ == "__main__":
    main()
