#!/usr/bin/env python3
"""
LUMENYX - Shield Funds (Deposit to Private Pool)

This script:
1. Generates secret and blinding factors
2. Computes commitment
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

def poseidon_hash(inputs: list[int]) -> int:
    """Poseidon-like hash matching on-chain implementation"""
    # BN254 field modulus
    P = 21888242871839275222246405745257275088548364400416034343698204186575808495617
    
    state = 0
    for i, inp in enumerate(inputs):
        state = (state + inp) % P
        x2 = (state * state) % P
        x4 = (x2 * x2) % P
        state = (x4 * state) % P  # x^5
        state = (state + (i + 1)) % P
    return state

def compute_commitment(amount: int, secret: int, blinding: int) -> int:
    """Compute commitment = PoseidonHash(amount, secret, blinding)"""
    return poseidon_hash([amount, secret, blinding])

def int_to_h256(value: int) -> str:
    """Convert integer to 0x-prefixed 32-byte hex"""
    return "0x" + value.to_bytes(32, 'big').hex()

def main():
    import argparse
    parser = argparse.ArgumentParser(description="Shield LUMENYX funds")
    parser.add_argument("--amount", type=int, required=True, help="Amount to shield")
    parser.add_argument("--seed", type=str, required=True, help="Your seed phrase")
    parser.add_argument("--local", action="store_true", help="Use local node")
    args = parser.parse_args()

    print("=" * 60)
    print("  LUMENYX - SHIELD FUNDS")
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
    
    if balance < args.amount:
        print(f"❌ Insufficient balance!")
        sys.exit(1)
    
    # Generate secret and blinding
    P = 21888242871839275222246405745257275088548364400416034343698204186575808495617
    secret = secrets.randbelow(P)
    blinding = secrets.randbelow(P)
    
    # Compute commitment
    amount_scaled = args.amount * 10**12  # Convert to smallest unit
    commitment = compute_commitment(amount_scaled, secret, blinding)
    commitment_hex = int_to_h256(commitment)
    
    print(f"\n🔐 Secret: {hex(secret)}")
    print(f"🎲 Blinding: {hex(blinding)}")
    print(f"📦 Commitment: {commitment_hex}")
    
    # Get current leaf index (for saving)
    next_index = substrate.query("Privacy", "NextIndex")
    leaf_index = next_index.value
    
    # Submit shield transaction
    print(f"\n📤 Submitting shield transaction...")
    
    call = substrate.compose_call(
        call_module='Privacy',
        call_function='shield',
        call_params={
            'amount': amount_scaled,
            'commitment': commitment_hex
        }
    )
    
    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)
    
    try:
        receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        if receipt.is_success:
            print(f"✅ Shield successful! Block: {receipt.block_hash[:18]}...")
        else:
            print(f"❌ Failed: {receipt.error_message}")
            sys.exit(1)
    except Exception as e:
        print(f"❌ Transaction failed: {e}")
        sys.exit(1)
    
    # Save note data
    NOTES_DIR.mkdir(exist_ok=True)
    note_file = NOTES_DIR / f"note_{leaf_index}.json"
    
    note_data = {
        "leaf_index": leaf_index,
        "amount": args.amount,
        "amount_scaled": amount_scaled,
        "secret": hex(secret),
        "blinding": hex(blinding),
        "commitment": commitment_hex,
        "block_hash": receipt.block_hash
    }
    
    with open(note_file, 'w') as f:
        json.dump(note_data, f, indent=2)
    
    print(f"\n💾 Note saved to: {note_file}")
    print(f"\n⚠️  SAVE YOUR SECRET AND BLINDING!")
    print(f"    Without them, funds are LOST FOREVER!")
    
    print("\n" + "=" * 60)
    print("  ✅ SHIELD COMPLETE!")
    print("=" * 60)

if __name__ == "__main__":
    main()
