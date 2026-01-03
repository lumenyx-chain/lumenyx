#!/usr/bin/env python3
"""
LUMENYX - Become a Validator (One Command Setup)

This script does EVERYTHING automatically:
1. Generates a new account with seed phrase
2. Calculates Proof-of-Work
3. Claims 2 LUMENYX from faucet
4. Uses SAME seed for AURA session key (rewards go to your account!)
5. Registers keys on-chain

After running this, you ARE a validator. Nothing else needed.

Requirements:
    pip install substrate-interface

Usage:
    python3 become_validator.py
"""

import sys
import time
import os
import json
from hashlib import blake2b

try:
    from substrateinterface import SubstrateInterface, Keypair
    from substrateinterface.utils.ss58 import ss58_encode
except ImportError:
    print("ERROR: Missing dependency. Run:")
    print("  pip install substrate-interface")
    sys.exit(1)

# Configuration
MAINNET_BOOTNODE = "ws://89.147.111.102:9944"
LOCAL_NODE = "ws://127.0.0.1:9944"
POW_DIFFICULTY = 18


def print_banner():
    print("=" * 60)
    print("  LUMENYX - Automatic Validator Setup")
    print("  UNSTOPPABLE like Bitcoin. Fast like Solana.")
    print("=" * 60)
    print("  • 3 second blocks")
    print("  • 18 second finality")
    print("  • Network NEVER stops")


def calculate_pow(address_bytes: bytes, difficulty: int):
    """Calculate Proof-of-Work nonce and hash for faucet claim"""
    target = 1 << (256 - difficulty)
    nonce = 0
    
    while True:
        data = address_bytes + nonce.to_bytes(8, 'little')
        hash_result = blake2b(data, digest_size=32).digest()
        hash_int = int.from_bytes(hash_result, 'big')
        
        if hash_int < target:
            return nonce, "0x" + hash_result.hex()
        nonce += 1
        
        if nonce % 100000 == 0:
            print(f"   ... trying nonce {nonce}")


def main():
    print_banner()
    
    # Connect to local node first, fallback to mainnet for queries
    print("\n🔌 Connecting to node...")
    
    substrate = None
    is_local = False
    
    try:
        substrate = SubstrateInterface(url=LOCAL_NODE)
        is_local = True
        print("   ✅ Connected to local node")
    except:
        try:
            substrate = SubstrateInterface(url=MAINNET_BOOTNODE)
            print("   ✅ Connected to mainnet (local node not running)")
        except Exception as e:
            print(f"   ❌ Cannot connect: {e}")
            sys.exit(1)
    
    block = substrate.get_block_number(None)
    print(f"   📦 Current block: #{block}")
    
    # Generate new keypair
    print("\n🆕 Generating new validator account...")
    keypair = Keypair.create_from_mnemonic(Keypair.generate_mnemonic())
    
    print(f"   ✅ Address: {keypair.ss58_address}")
    
    # Save key to file
    key_file = os.path.expanduser("~/.lumenyx-validator-key")
    with open(key_file, 'w') as f:
        f.write(f"{keypair.mnemonic}\n")
    os.chmod(key_file, 0o600)
    print(f"   💾 Key saved to: {key_file}")
    
    print("\n" + "=" * 60)
    print("⚠️  YOUR SEED PHRASE (SAVE THIS SECURELY!):")
    print(f"   {keypair.mnemonic}")
    print("   This is the ONLY way to recover your validator account!")
    print("   Rewards will go directly to this account!")
    print("=" * 60)
    
    # Claim from faucet
    print("\n💰 Claiming from faucet...")
    
    # Calculate PoW
    print(f"⛏️  Calculating Proof-of-Work (difficulty: {POW_DIFFICULTY} bits)...")
    print("   This may take 5-30 seconds...")
    
    start_time = time.time()
    address_bytes = bytes(keypair.public_key)
    nonce, pow_hash = calculate_pow(address_bytes, POW_DIFFICULTY)
    elapsed = time.time() - start_time
    
    print(f"   ✅ PoW found in {elapsed:.1f} seconds (nonce: {nonce})")
    
    # Submit faucet claim (unsigned extrinsic)
    print("   📤 Submitting claim transaction...")
    
    call = substrate.compose_call(
        call_module='ValidatorFaucet',
        call_function='claim_for_validator',
        call_params={
            'target': keypair.ss58_address,
            'nonce': nonce,
            'pow_hash': pow_hash
        }
    )
    
    # Create unsigned extrinsic
    extrinsic = substrate.create_unsigned_extrinsic(call=call)
    
    try:
        receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        print(f"   ✅ Claim successful! Block: {receipt.block_hash[:20]}...")
    except Exception as e:
        print(f"   ❌ Claim failed: {e}")
        print("   ⚠️  You may have already claimed or faucet is empty")
    
    # Check balance
    time.sleep(3)  # Wait for block
    account_info = substrate.query("System", "Account", [keypair.ss58_address])
    balance = account_info.value["data"]["free"] / 10**12
    print(f"   💰 Balance: {balance} LUMENYX")
    
    if balance < 1:
        print("   ⚠️  Low balance - claim may have failed")
        return False
    
    # Setup session keys - USE SAME SEED for AURA!
    print("\n🔐 Setting up session keys...")
    
    if not is_local:
        print("   ❌ Cannot setup keys - local node not running!")
        print("   ⚠️  Start your node and run this script again.")
        return False
    
    # Check if already registered
    next_keys = substrate.query("Session", "NextKeys", [keypair.ss58_address])
    if next_keys.value is not None:
        print(f"   ✅ Already registered as validator!")
        return True
    
    # IMPORTANT: Use the SAME keypair for AURA (sr25519)
    # This way rewards go to the same account as the seed!
    print("   🔑 Using your account key as AURA session key...")
    print("   📍 Rewards will go to: " + keypair.ss58_address)
    
    # Insert the key into local keystore via RPC
    try:
        # Insert AURA key (sr25519) - using the same seed
        result = substrate.rpc_request("author_insertKey", [
            "aura",  # key type
            keypair.mnemonic,  # seed phrase
            "0x" + keypair.public_key.hex()  # public key
        ])
        print(f"   ✅ AURA key inserted into keystore")
    except Exception as e:
        print(f"   ❌ Failed to insert key: {e}")
        return False
    
    # The session key is just the public key (for AURA only, no GRANDPA)
    aura_key = "0x" + keypair.public_key.hex()
    
    # Submit setKeys transaction
    print("   📝 Registering AURA key on-chain...")
    
    call = substrate.compose_call(
        call_module='Session',
        call_function='set_keys',
        call_params={
            'keys': {'aura': aura_key},
            'proof': '0x'
        }
    )
    
    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)
    
    try:
        receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        print(f"   ✅ AURA key registered! TX: {receipt.extrinsic_hash[:20]}...")
    except Exception as e:
        print(f"   ❌ Failed to register: {e}")
        return False
    
    # Success!
    print("\n" + "=" * 60)
    print("🎉 VALIDATOR SETUP COMPLETE!")
    print("=" * 60)
    print(f"   Account:  {keypair.ss58_address}")
    print(f"   Key file: ~/.lumenyx-validator-key")
    print(f"")
    print(f"   Your node will start producing blocks in the next session")
    print(f"   (approximately 30 seconds).")
    print(f"")
    print(f"   💰 ALL REWARDS GO TO: {keypair.ss58_address}")
    print(f"")
    print(f"   Monitor your node logs for:")
    print(f"   '✅ Prepared block' - you're producing blocks!")
    print(f"")
    print(f"   NO GRANDPA = Network NEVER stops!")
    print(f"   Finality: 6 blocks = 18 seconds (like Bitcoin but 200x faster)")
    print("=" * 60)
    print(f"   Welcome to LUMENYX! You are now a validator.")
    print(f"   The network is UNSTOPPABLE. 🚀")
    print("=" * 60)
    
    return True


if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
