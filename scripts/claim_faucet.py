#!/usr/bin/env python3
"""
LUMENYX Validator Faucet - Complete Claim Script

This script claims 2 LUMENYX from the validator faucet.
The tokens can be used to pay for session.setKeys() to become a validator.

Requirements:
    pip install base58 substrate-interface

Usage:
    python3 claim_faucet.py

This will:
1. Generate a new account (or you can modify to use existing)
2. Calculate the required Proof-of-Work
3. Submit the claim transaction
4. Show your new balance
"""

from substrateinterface import SubstrateInterface, Keypair
from hashlib import blake2b

# Configuration
NODE_URL = "ws://89.147.111.102:9944"  # Public mainnet node
POW_DIFFICULTY = 18  # Required leading zero bits

def find_pow(account_bytes: bytes, difficulty: int = 18) -> tuple:
    """Find nonce that produces hash with required leading zeros."""
    print(f"Calculating Proof-of-Work (difficulty: {difficulty} bits)...")
    nonce = 0
    while True:
        data = account_bytes + nonce.to_bytes(8, 'little')
        hash_result = blake2b(data, digest_size=32).digest()
        zeros = 0
        for byte in hash_result:
            if byte == 0:
                zeros += 8
            else:
                zeros += (8 - byte.bit_length())
                break
        if zeros >= difficulty:
            return nonce, hash_result
        if nonce % 500000 == 0 and nonce > 0:
            print(f"  Tried {nonce} nonces...")
        nonce += 1

def main():
    print("=" * 60)
    print("LUMENYX Validator Faucet - Claim Script")
    print("=" * 60)
    
    # Connect to node
    print(f"\nConnecting to {NODE_URL}...")
    substrate = SubstrateInterface(url=NODE_URL)
    print(f"Connected to: {substrate.chain}")
    print(f"Current block: #{substrate.get_block_number(None)}")
    
    # Generate new account
    print("\n--- Generating New Account ---")
    mnemonic = Keypair.generate_mnemonic()
    keypair = Keypair.create_from_mnemonic(mnemonic)
    
    print(f"Mnemonic (SAVE THIS!): {mnemonic}")
    print(f"Address: {keypair.ss58_address}")
    print(f"Public key: 0x{keypair.public_key.hex()}")
    
    # Check balance before
    balance = substrate.query('System', 'Account', [keypair.ss58_address])
    print(f"Balance before: {balance.value['data']['free']} planck")
    
    # Calculate PoW
    print("\n--- Calculating Proof-of-Work ---")
    nonce, pow_hash = find_pow(keypair.public_key, POW_DIFFICULTY)
    print(f"Nonce: {nonce}")
    print(f"Hash: 0x{pow_hash.hex()}")
    
    # Compose call
    print("\n--- Submitting Claim ---")
    call = substrate.compose_call(
        call_module='ValidatorFaucet',
        call_function='claim_for_validator',
        call_params={
            'target': keypair.ss58_address,
            'nonce': nonce,
            'pow_hash': f'0x{pow_hash.hex()}'
        }
    )
    
    # Create and submit unsigned extrinsic
    extrinsic = substrate.create_unsigned_extrinsic(call)
    result = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
    print(f"Success! Block hash: {result.block_hash}")
    
    # Check balance after
    balance = substrate.query('System', 'Account', [keypair.ss58_address])
    balance_lumenyx = balance.value['data']['free'] / 1_000_000_000_000
    print(f"\n--- Result ---")
    print(f"Balance after: {balance_lumenyx} LUMENYX")
    print(f"\nYou can now use this account to become a validator!")
    print(f"See README.md for next steps.")
    
    print("\n" + "=" * 60)
    print("IMPORTANT: Save your mnemonic phrase securely!")
    print(mnemonic)
    print("=" * 60)

if __name__ == "__main__":
    main()
