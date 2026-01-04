#!/usr/bin/env python3
"""
LUMENYX Transfer Script
Transfer LUMENYX between accounts
"""

import requests
import json
from substrateinterface import SubstrateInterface, Keypair
from substrateinterface.exceptions import SubstrateRequestException

# Node RPC
NODE_URL = "ws://89.147.111.102:9944"

# Miner Islanda seed phrase
SENDER_SEED = "announce dust normal canoe erode kitten have job slender music sing pact"

# Miner Finlandia address
RECEIVER = "5DA7xBQQE3gUGQJHmo3Ui359yBzJDYbJAdKCBN9KczXwH1xr"

# Amount to send (in LUMENYX, will be converted to planck)
AMOUNT = 10  # 10 LUMENYX

def main():
    print("🔗 Connecting to LUMENYX node...")
    
    try:
        substrate = SubstrateInterface(url=NODE_URL)
        print(f"✅ Connected to {substrate.chain} ({substrate.version})")
    except Exception as e:
        print(f"❌ Connection failed: {e}")
        return
    
    # Create keypair from seed
    keypair = Keypair.create_from_mnemonic(SENDER_SEED)
    print(f"📤 Sender: {keypair.ss58_address}")
    print(f"📥 Receiver: {RECEIVER}")
    print(f"💰 Amount: {AMOUNT} LUMENYX")
    
    # Convert to planck (12 decimals)
    amount_planck = int(AMOUNT * 10**12)
    
    # Create transfer call
    call = substrate.compose_call(
        call_module='Balances',
        call_function='transfer_keep_alive',
        call_params={
            'dest': RECEIVER,
            'value': amount_planck
        }
    )
    
    # Create and sign extrinsic
    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)
    
    print("📝 Submitting transaction...")
    
    try:
        receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        print(f"✅ Transaction included in block: {receipt.block_hash}")
        print(f"✅ Extrinsic hash: {receipt.extrinsic_hash}")
        
        if receipt.is_success:
            print("🎉 Transfer successful!")
        else:
            print(f"❌ Transfer failed: {receipt.error_message}")
            
    except SubstrateRequestException as e:
        print(f"❌ Transaction failed: {e}")

if __name__ == "__main__":
    main()
