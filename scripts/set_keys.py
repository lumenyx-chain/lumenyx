#!/usr/bin/env python3
"""
LUMENYX - Set Session Keys Script
Registers your validator keys on-chain after claiming from faucet.

Requirements:
    pip install substrate-interface

Usage:
    python3 set_keys.py
"""

import sys
from substrateinterface import SubstrateInterface, Keypair

def main():
    print("=" * 50)
    print("LUMENYX - Set Session Keys")
    print("=" * 50)
    
    # Connect to local node
    try:
        substrate = SubstrateInterface(url="ws://127.0.0.1:9944")
        print("Connected to local node")
    except:
        print("Cannot connect. Is your node running?")
        print("Start with: ./target/release/lumenyx-node --chain mainnet-spec.json --validator")
        sys.exit(1)
    
    # Get mnemonic
    print("\nEnter your 12-word secret phrase:")
    mnemonic = input("> ").strip()
    
    if len(mnemonic.split()) != 12:
        print("Invalid mnemonic. Must be 12 words.")
        sys.exit(1)
    
    keypair = Keypair.create_from_mnemonic(mnemonic)
    print(f"Account: {keypair.ss58_address}")
    
    # Check balance
    account_info = substrate.query("System", "Account", [keypair.ss58_address])
    balance = account_info.value['data']['free']
    print(f"Balance: {balance / 10**12} LUMENYX")
    
    if balance == 0:
        print("No balance. Run claim_faucet.py first.")
        sys.exit(1)
    
    # Generate session keys
    print("\nGenerating session keys...")
    result = substrate.rpc_request("author_rotateKeys", [])
    session_keys = result['result']
    
    aura_key = session_keys[:66]
    grandpa_key = "0x" + session_keys[66:]
    
    # Submit setKeys
    print("Registering on-chain...")
    call = substrate.compose_call(
        call_module='Session',
        call_function='set_keys',
        call_params={
            'keys': {'aura': aura_key, 'grandpa': grandpa_key},
            'proof': '0x'
        }
    )
    
    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)
    receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
    
    if receipt.is_success:
        print(f"\nSuccess! TX: {receipt.extrinsic_hash}")
        print("Your node will start validating in ~30 seconds.")
    else:
        print(f"Failed: {receipt.error_message}")
        sys.exit(1)

if __name__ == "__main__":
    main()
