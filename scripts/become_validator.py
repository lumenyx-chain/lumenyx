#!/usr/bin/env python3
"""
LUMENYX - Become a Validator (One Command Setup)

This script does EVERYTHING automatically:
1. Generates a new account with seed phrase
2. Calculates Proof-of-Work
3. Claims 2 LUMENYX from faucet
4. Generates and inserts session keys (AURA only - no GRANDPA!)
5. Registers keys on-chain

After running this, you ARE a validator. Nothing else needed.

LUMENYX is UNSTOPPABLE like Bitcoin:
- No GRANDPA = Network never stops
- 3 second blocks
- 18 second finality (6 blocks)

Requirements:
    pip install substrate-interface

Usage:
    python3 become_validator.py

Your node must be running:
    ./target/release/lumenyx-node --validator
"""

import sys
import time
import os
from hashlib import blake2b

try:
    from substrateinterface import SubstrateInterface, Keypair
except ImportError:
    print("ERROR: Missing dependency. Run:")
    print("  pip install substrate-interface")
    sys.exit(1)

# Configuration
MAINNET_BOOTNODE = "ws://89.147.111.102:9944"
LOCAL_NODE = "ws://127.0.0.1:9944"
POW_DIFFICULTY = 18  # Must match runtime

def print_banner():
    print("=" * 60)
    print("  LUMENYX - Automatic Validator Setup")
    print("  UNSTOPPABLE like Bitcoin. Fast like Solana.")
    print("=" * 60)
    print()
    print("  • 3 second blocks")
    print("  • 18 second finality")
    print("  • Network NEVER stops")
    print()

def connect_to_node():
    """Connect to local node, fallback to mainnet bootnode"""
    print("🔌 Connecting to node...")
    
    # Try local first
    try:
        substrate = SubstrateInterface(url=LOCAL_NODE)
        print(f"   ✅ Connected to local node")
        return substrate, True
    except:
        pass
    
    # Fallback to mainnet bootnode
    try:
        substrate = SubstrateInterface(url=MAINNET_BOOTNODE)
        print(f"   ✅ Connected to mainnet bootnode")
        print(f"   ⚠️  WARNING: Your node is not running locally!")
        print(f"   ⚠️  Start it with: ./target/release/lumenyx-node --validator")
        return substrate, False
    except Exception as e:
        print(f"   ❌ Cannot connect to any node: {e}")
        sys.exit(1)

def find_pow(public_key: bytes, difficulty: int = 18) -> tuple:
    """Calculate Proof-of-Work for faucet claim"""
    print(f"⛏️  Calculating Proof-of-Work (difficulty: {difficulty} bits)...")
    print(f"   This may take 5-30 seconds...")
    
    nonce = 0
    start_time = time.time()
    
    while True:
        data = public_key + nonce.to_bytes(8, 'little')
        hash_result = blake2b(data, digest_size=32).digest()
        
        # Count leading zero bits
        zeros = 0
        for byte in hash_result:
            if byte == 0:
                zeros += 8
            else:
                zeros += (8 - byte.bit_length())
                break
        
        if zeros >= difficulty:
            elapsed = time.time() - start_time
            print(f"   ✅ PoW found in {elapsed:.1f} seconds (nonce: {nonce})")
            return nonce, hash_result
        
        nonce += 1
        
        if nonce % 500000 == 0:
            elapsed = time.time() - start_time
            print(f"   ... tried {nonce} nonces ({elapsed:.0f}s)")

def claim_from_faucet(substrate, keypair):
    """Claim 2 LUMENYX from the validator faucet"""
    print("\n💰 Claiming from faucet...")
    
    # Check current balance
    account_info = substrate.query("System", "Account", [keypair.ss58_address])
    balance_before = account_info.value['data']['free']
    
    if balance_before > 0:
        print(f"   ✅ Already have balance: {balance_before / 10**12} LUMENYX")
        return True
    
    # Calculate PoW
    nonce, pow_hash = find_pow(keypair.public_key, POW_DIFFICULTY)
    
    # Compose faucet claim
    print("   📤 Submitting claim transaction...")
    call = substrate.compose_call(
        call_module='ValidatorFaucet',
        call_function='claim_for_validator',
        call_params={
            'target': keypair.ss58_address,
            'nonce': nonce,
            'pow_hash': f'0x{pow_hash.hex()}'
        }
    )
    
    # Submit unsigned (faucet accepts unsigned with valid PoW)
    extrinsic = substrate.create_unsigned_extrinsic(call)
    
    try:
        receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        print(f"   ✅ Claim successful! Block: {receipt.block_hash[:18]}...")
    except Exception as e:
        if "AlreadyClaimed" in str(e):
            print(f"   ⚠️  Already claimed before. Checking balance...")
        else:
            print(f"   ❌ Claim failed: {e}")
            return False
    
    # Verify balance
    time.sleep(1)
    account_info = substrate.query("System", "Account", [keypair.ss58_address])
    balance_after = account_info.value['data']['free']
    print(f"   💰 Balance: {balance_after / 10**12} LUMENYX")
    
    return balance_after > 0

def setup_session_keys(substrate, keypair, is_local):
    """Generate and register session keys (AURA only - no GRANDPA!)"""
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
    
    # Generate session keys via RPC (this inserts them into keystore)
    # With no GRANDPA, rotateKeys only generates AURA key
    print("   🔑 Generating AURA session key...")
    try:
        result = substrate.rpc_request("author_rotateKeys", [])
        session_keys = result['result']
        print(f"   ✅ AURA key generated: {session_keys[:20]}...{session_keys[-10:]}")
    except Exception as e:
        print(f"   ❌ Failed to generate keys: {e}")
        print("   ⚠️  Make sure your node is running with --validator flag")
        return False
    
    # With no GRANDPA, session_keys is just the AURA key (66 chars with 0x prefix)
    aura_key = session_keys
    
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
        if receipt.is_success:
            print(f"   ✅ AURA key registered! TX: {receipt.extrinsic_hash[:18]}...")
            return True
        else:
            print(f"   ❌ Failed: {receipt.error_message}")
            return False
    except Exception as e:
        print(f"   ❌ Transaction failed: {e}")
        return False

def check_existing_keys(substrate):
    """Check if there's an existing validator key file"""
    key_file = os.path.expanduser("~/.lumenyx-validator-key")
    
    if os.path.exists(key_file):
        print("🔍 Found existing validator key file...")
        with open(key_file, 'r') as f:
            mnemonic = f.read().strip()
        
        try:
            keypair = Keypair.create_from_mnemonic(mnemonic)
            print(f"   ✅ Loaded existing account: {keypair.ss58_address}")
            return keypair, mnemonic, False
        except:
            print("   ⚠️  Invalid key file, generating new account...")
    
    return None, None, True

def save_key(mnemonic):
    """Save mnemonic to file"""
    key_file = os.path.expanduser("~/.lumenyx-validator-key")
    with open(key_file, 'w') as f:
        f.write(mnemonic)
    os.chmod(key_file, 0o600)  # Read/write only for owner
    print(f"   💾 Key saved to: {key_file}")

def main():
    print_banner()
    
    # Connect
    substrate, is_local = connect_to_node()
    
    block = substrate.get_block_number(None)
    print(f"   📦 Current block: #{block}")
    print()
    
    # Check for existing keys
    keypair, mnemonic, is_new = check_existing_keys(substrate)
    
    if keypair is None:
        # Generate new account
        print("🆕 Generating new validator account...")
        mnemonic = Keypair.generate_mnemonic()
        keypair = Keypair.create_from_mnemonic(mnemonic)
        print(f"   ✅ Address: {keypair.ss58_address}")
        save_key(mnemonic)
        is_new = True
    
    # Print seed phrase warning
    print()
    print("=" * 60)
    print("⚠️  YOUR SEED PHRASE (SAVE THIS SECURELY!):")
    print()
    print(f"   {mnemonic}")
    print()
    print("   This is the ONLY way to recover your validator account!")
    print("=" * 60)
    print()
    
    # Claim from faucet
    if not claim_from_faucet(substrate, keypair):
        print("\n❌ Failed to get balance from faucet.")
        sys.exit(1)
    
    # Setup session keys (AURA only)
    if is_local:
        if not setup_session_keys(substrate, keypair, is_local):
            print("\n⚠️  Could not complete key registration.")
            print("    Your account has balance. Try running this script again")
            print("    after your node is fully synced.")
            sys.exit(1)
    else:
        print("\n⚠️  Session key setup skipped (no local node)")
        print("    Start your node and run this script again to complete setup.")
    
    # Success!
    print()
    print("=" * 60)
    print("🎉 VALIDATOR SETUP COMPLETE!")
    print("=" * 60)
    print()
    print(f"   Account:  {keypair.ss58_address}")
    print(f"   Key file: ~/.lumenyx-validator-key")
    print()
    
    if is_local:
        print("   Your node will start producing blocks in the next session")
        print("   (approximately 30 seconds).")
        print()
        print("   Monitor your node logs for:")
        print("   '✅ Prepared block' - you're producing blocks!")
        print()
        print("   NO GRANDPA = Network NEVER stops!")
        print("   Finality: 6 blocks = 18 seconds (like Bitcoin but 200x faster)")
    else:
        print("   Next steps:")
        print("   1. Start your node: ./target/release/lumenyx-node --validator")
        print("   2. Run this script again to complete key registration")
    
    print()
    print("=" * 60)
    print("   Welcome to LUMENYX! You are now a validator.")
    print("   The network is UNSTOPPABLE. 🚀")
    print("=" * 60)

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\nAborted by user.")
        sys.exit(1)
    except Exception as e:
        print(f"\n❌ Error: {e}")
        sys.exit(1)
