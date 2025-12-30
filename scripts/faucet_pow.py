#!/usr/bin/env python3
"""
LUMENYX Validator Faucet - PoW Calculator

This script calculates the nonce and pow_hash required to claim
from the validator faucet.

Requirements:
    pip install base58 substrate-interface

Usage:
    python3 faucet_pow.py <your_substrate_address>

Example:
    python3 faucet_pow.py 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY

After getting nonce and pow_hash, use substrate-interface to submit:
    See claim_faucet.py for full example.
"""

import sys
import time

try:
    import base58
except ImportError:
    print("Please install base58: pip install base58")
    sys.exit(1)

try:
    from hashlib import blake2b
except ImportError:
    print("Python 3.6+ required for blake2b")
    sys.exit(1)


def ss58_decode(address: str) -> bytes:
    """Decode SS58 address to raw public key bytes"""
    decoded = base58.b58decode(address)
    if len(decoded) == 35:
        return decoded[1:33]
    elif len(decoded) == 36:
        return decoded[2:34]
    else:
        raise ValueError(f"Invalid SS58 address length: {len(decoded)}")


def blake2_256(data: bytes) -> bytes:
    """Calculate blake2b-256 hash (same as Substrate)"""
    return blake2b(data, digest_size=32).digest()


def count_leading_zero_bits(hash_bytes: bytes) -> int:
    """Count leading zero bits in hash"""
    zeros = 0
    for byte in hash_bytes:
        if byte == 0:
            zeros += 8
        else:
            zeros += (8 - byte.bit_length())
            break
    return zeros


def find_pow(account_bytes: bytes, difficulty: int = 18) -> tuple:
    """Find nonce that produces hash with required leading zeros."""
    print(f"Searching for PoW with difficulty {difficulty} bits...")
    start_time = time.time()
    nonce = 0
    
    while True:
        data = account_bytes + nonce.to_bytes(8, 'little')
        hash_result = blake2_256(data)
        leading_zeros = count_leading_zero_bits(hash_result)
        
        if leading_zeros >= difficulty:
            elapsed = time.time() - start_time
            print(f"Found valid PoW in {elapsed:.2f} seconds")
            return nonce, hash_result
        
        if nonce % 500000 == 0 and nonce > 0:
            print(f"  Tried {nonce} nonces...")
        
        nonce += 1


def main():
    if len(sys.argv) != 2:
        print("LUMENYX Validator Faucet - PoW Calculator")
        print("")
        print("Usage: python3 faucet_pow.py <your_substrate_address>")
        print("")
        print("Example:")
        print("  python3 faucet_pow.py 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY")
        sys.exit(1)
    
    address = sys.argv[1]
    
    print("=" * 60)
    print("LUMENYX Validator Faucet - PoW Calculator")
    print("=" * 60)
    print(f"Address: {address}")
    
    try:
        account_bytes = ss58_decode(address)
        print(f"Public key: 0x{account_bytes.hex()}")
        print()
    except Exception as e:
        print(f"Error decoding address: {e}")
        sys.exit(1)
    
    nonce, pow_hash = find_pow(account_bytes, difficulty=18)
    
    print()
    print("=" * 60)
    print("RESULTS - Use these values to claim from faucet:")
    print("=" * 60)
    print(f"  nonce: {nonce}")
    print(f"  pow_hash: 0x{pow_hash.hex()}")
    print()
    print("See claim_faucet.py for how to submit the claim transaction.")
    print("=" * 60)


if __name__ == "__main__":
    main()
