#!/usr/bin/env bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════════════
# LUMENYX NODE SETUP + WALLET CLI
# ══════════════════════════════════════════════════════════════════════════════
# - First run: Download, wallet generation, systemd setup
# - Subsequent runs: Auto-update check + Full wallet CLI menu
# ══════════════════════════════════════════════════════════════════════════════

VERSION="1.7.1"
LUMENYX_DIR="$HOME/.lumenyx"
HELPERS_DIR="$LUMENYX_DIR/helpers"
KEYS_DIR="$LUMENYX_DIR/keys"
BINARY_NAME="lumenyx-node"
SERVICE_NAME="lumenyx"

BINARY_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v${VERSION}/lumenyx-node-linux-x86_64"
CHECKSUM_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v${VERSION}/lumenyx-node-sha256.txt"

BOOTNODE="/ip4/89.147.111.102/tcp/30333/p2p/12D3KooWNWLGaBDB9WwCTuG4fDT2rb3AY4WaweF6TBF4YWgZTtrY"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

print_banner() {
    clear
    echo -e "${CYAN}"
    echo "╔════════════════════════════════════════════════════════════════════════╗"
    echo "║                                                                        ║"
    echo "║   ██╗     ██╗   ██╗███╗   ███╗███████╗███╗   ██╗██╗   ██╗██╗  ██╗    ║"
    echo "║   ██║     ██║   ██║████╗ ████║██╔════╝████╗  ██║╚██╗ ██╔╝╚██╗██╔╝    ║"
    echo "║   ██║     ██║   ██║██╔████╔██║█████╗  ██╔██╗ ██║ ╚████╔╝  ╚███╔╝     ║"
    echo "║   ██║     ██║   ██║██║╚██╔╝██║██╔══╝  ██║╚██╗██║  ╚██╔╝   ██╔██╗     ║"
    echo "║   ███████╗╚██████╔╝██║ ╚═╝ ██║███████╗██║ ╚████║   ██║   ██╔╝ ██╗    ║"
    echo "║   ╚══════╝ ╚═════╝ ╚═╝     ╚═╝╚══════╝╚═╝  ╚═══╝   ╚═╝   ╚═╝  ╚═╝    ║"
    echo "║                                                                        ║"
    echo "║                     Peer-to-Peer Electronic Cash                       ║"
    echo "║                          Version ${VERSION}                               ║"
    echo "╚════════════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}
print_success() { echo -e "${GREEN}✓${NC} $1"; }
print_error() { echo -e "${RED}✗${NC} $1"; }
print_warn() { echo -e "${YELLOW}!${NC} $1"; }
print_info() { echo -e "${BLUE}ℹ${NC} $1"; }

press_enter() {
    echo ""
    read -r -p "Press ENTER to continue..."
}

# ══════════════════════════════════════════════════════════════════════════════
# VERSION CHECK AND AUTO-UPDATE
# ══════════════════════════════════════════════════════════════════════════════

is_first_run() {
    [[ ! -f "$LUMENYX_DIR/$BINARY_NAME" ]]
}

is_node_running() {
    pgrep -f "$BINARY_NAME" > /dev/null 2>&1
}

get_installed_version() {
    if [[ -f "$LUMENYX_DIR/$BINARY_NAME" ]]; then
        # Try to get version from binary
        local ver
        ver=$("$LUMENYX_DIR/$BINARY_NAME" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || echo "unknown")
        echo "$ver"
    else
        echo "none"
    fi
}

check_for_updates() {
    local installed_ver
    installed_ver=$(get_installed_version)
    
    if [[ "$installed_ver" == "unknown" || "$installed_ver" == "none" ]]; then
        return 1  # Can't determine, assume needs update
    fi
    
    if [[ "$installed_ver" != "$VERSION" ]]; then
        return 1  # Needs update
    fi
    
    return 0  # Up to date
}

do_update() {
    print_banner
    echo -e "${CYAN}═══ AUTO-UPDATE ═══${NC}"
    echo ""
    
    local installed_ver
    installed_ver=$(get_installed_version)
    
    echo "Installed version: $installed_ver"
    echo "Latest version:    $VERSION"
    echo ""
    
    if [[ "$installed_ver" == "$VERSION" ]]; then
        print_success "Already up to date!"
        return 0
    fi
    
    print_info "Downloading update..."
    
    # Stop node if running
    if is_node_running; then
        print_info "Stopping node..."
        if systemctl is-active --quiet "$SERVICE_NAME" 2>/dev/null; then
            sudo systemctl stop "$SERVICE_NAME"
        else
            pkill -f "$BINARY_NAME" || true
        fi
        sleep 2
    fi
    
    # Download new binary
    local tmp_binary="/tmp/lumenyx-node-new"
    if ! curl -L --progress-bar -o "$tmp_binary" "$BINARY_URL"; then
        print_error "Failed to download update"
        return 1
    fi
    
    # Verify checksum
    print_info "Verifying checksum..."
    local expected_checksum
    expected_checksum=$(curl -sL "$CHECKSUM_URL" | awk '{print $1}')
    local actual_checksum
    actual_checksum=$(sha256sum "$tmp_binary" | awk '{print $1}')
    
    if [[ "$expected_checksum" != "$actual_checksum" ]]; then
        print_error "Checksum mismatch! Update aborted."
        rm -f "$tmp_binary"
        return 1
    fi
    
    print_success "Checksum verified!"
    
    # Replace binary
    mv "$tmp_binary" "$LUMENYX_DIR/$BINARY_NAME"
    chmod +x "$LUMENYX_DIR/$BINARY_NAME"
    
    print_success "Updated to version $VERSION!"
    
    # Restart node if it was running
    if systemctl is-enabled --quiet "$SERVICE_NAME" 2>/dev/null; then
        print_info "Restarting node..."
        sudo systemctl start "$SERVICE_NAME"
        print_success "Node restarted!"
    fi
    
    return 0
}

is_node_running() {
    pgrep -f "$BINARY_NAME" > /dev/null 2>&1
}

# ══════════════════════════════════════════════════════════════════════════════
# PYTHON HELPERS INSTALLATION
# ══════════════════════════════════════════════════════════════════════════════

install_python_helpers() {
    print_info "Installing Python helpers..."
    
    mkdir -p "$HELPERS_DIR"
    mkdir -p "$KEYS_DIR"
    chmod 700 "$KEYS_DIR"
    
    # Check/install dependencies
    if ! command -v python3 &> /dev/null; then
        print_error "Python3 not found. Please install python3."
        exit 1
    fi
    
    # Install required packages
    pip3 install --quiet --break-system-packages \
        substrate-interface scalecodec cryptography eth-account web3 bip-utils 2>/dev/null || \
    pip3 install --quiet \
        substrate-interface scalecodec cryptography eth-account web3 bip-utils 2>/dev/null || true

    # ─────────────────────────────────────────────────────────────────────────
    # key_manager.py
    # ─────────────────────────────────────────────────────────────────────────
    cat > "$HELPERS_DIR/key_manager.py" << 'PYEOF'
#!/usr/bin/env python3
"""Key management with encrypted keystores."""

import os
import json
import getpass
from pathlib import Path
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.scrypt import Scrypt
from cryptography.hazmat.backends import default_backend

KEYS_DIR = Path.home() / ".lumenyx" / "keys"
KEYS_DIR.mkdir(parents=True, exist_ok=True)
os.chmod(KEYS_DIR, 0o700)

def derive_key(password: str, salt: bytes) -> bytes:
    kdf = Scrypt(salt=salt, length=32, n=2**14, r=8, p=1, backend=default_backend())
    return kdf.derive(password.encode())

def encrypt_data(data: bytes, password: str) -> dict:
    salt = os.urandom(16)
    nonce = os.urandom(12)
    key = derive_key(password, salt)
    aesgcm = AESGCM(key)
    ciphertext = aesgcm.encrypt(nonce, data, None)
    return {
        "salt": salt.hex(),
        "nonce": nonce.hex(),
        "ciphertext": ciphertext.hex()
    }

def decrypt_data(encrypted: dict, password: str) -> bytes:
    salt = bytes.fromhex(encrypted["salt"])
    nonce = bytes.fromhex(encrypted["nonce"])
    ciphertext = bytes.fromhex(encrypted["ciphertext"])
    key = derive_key(password, salt)
    aesgcm = AESGCM(key)
    return aesgcm.decrypt(nonce, ciphertext, None)

def save_substrate_keystore(mnemonic: str, address: str, password: str):
    encrypted = encrypt_data(mnemonic.encode(), password)
    keystore = {
        "address": address,
        "crypto": encrypted,
        "version": 1
    }
    path = KEYS_DIR / "substrate_keystore.json"
    with open(path, "w") as f:
        json.dump(keystore, f, indent=2)
    os.chmod(path, 0o600)
    # Cache address for password-free display
    (KEYS_DIR / "substrate_address.txt").write_text(address)

def load_substrate_mnemonic(password: str) -> str:
    path = KEYS_DIR / "substrate_keystore.json"
    with open(path) as f:
        keystore = json.load(f)
    return decrypt_data(keystore["crypto"], password).decode()

def get_substrate_address() -> str:
    path = KEYS_DIR / "substrate_address.txt"
    if path.exists():
        return path.read_text().strip()
    return None

def save_evm_keystore(private_key: bytes, address: str, password: str):
    from eth_account import Account
    keystore = Account.encrypt(private_key, password)
    path = KEYS_DIR / "evm_keystore.json"
    with open(path, "w") as f:
        json.dump(keystore, f, indent=2)
    os.chmod(path, 0o600)

def load_evm_private_key(password: str) -> bytes:
    from eth_account import Account
    path = KEYS_DIR / "evm_keystore.json"
    with open(path) as f:
        keystore = json.load(f)
    return Account.decrypt(keystore, password)

def get_evm_address() -> str:
    path = KEYS_DIR / "evm_keystore.json"
    if path.exists():
        with open(path) as f:
            keystore = json.load(f)
        return "0x" + keystore.get("address", "")
    return None
PYEOF

    # ─────────────────────────────────────────────────────────────────────────
    # wallet_init.py
    # ─────────────────────────────────────────────────────────────────────────
    cat > "$HELPERS_DIR/wallet_init.py" << 'PYEOF'
#!/usr/bin/env python3
"""Initialize wallet: generate SR25519 + EVM keys from same mnemonic."""

import sys
import getpass
from substrateinterface import Keypair

sys.path.insert(0, str(__file__).rsplit("/", 1)[0])
from key_manager import save_substrate_keystore, save_evm_keystore

def main():
    print("\n" + "═" * 60)
    print("  WALLET GENERATION")
    print("═" * 60 + "\n")
    
    # Generate mnemonic
    mnemonic = Keypair.generate_mnemonic()
    keypair = Keypair.create_from_mnemonic(mnemonic)
    ss58_address = keypair.ss58_address
    
    # Derive EVM key from same mnemonic using BIP44
    try:
        from bip_utils import Bip39SeedGenerator, Bip44, Bip44Coins
        seed = Bip39SeedGenerator(mnemonic).Generate()
        bip44 = Bip44.FromSeed(seed, Bip44Coins.ETHEREUM)
        evm_key = bip44.Purpose().Coin().Account(0).Change(0).AddressIndex(0)
        evm_private = evm_key.PrivateKey().Raw().ToBytes()
        evm_address = evm_key.PublicKey().ToAddress()
    except Exception as e:
        # Fallback: generate separate EVM key
        from eth_account import Account
        acct = Account.create()
        evm_private = acct.key
        evm_address = acct.address
    
    # Show seed phrase
    print("╔═══════════════════════════════════════════════════════════════════╗")
    print("║  CRITICAL: Write down these 12 words on paper!                   ║")
    print("║  If you lose them, your funds are LOST FOREVER.                  ║")
    print("╚═══════════════════════════════════════════════════════════════════╝")
    print("")
    print(f"  {mnemonic}")
    print("")
    print("═" * 60)
    
    # Double confirmation
    confirm1 = input("\nHave you written down your seed phrase? Type 'YES' to confirm: ")
    if confirm1.upper() != "YES":
        print("Aborted. Please write down your seed phrase.")
        sys.exit(1)
    
    confirm2 = input("Are you SURE you saved it safely? Type 'YES' again: ")
    if confirm2.upper() != "YES":
        print("Aborted. Please save your seed phrase safely.")
        sys.exit(1)
    
    # Get password
    print("\nNow choose a password to encrypt your wallet.")
    print("This password will be needed to send transactions.")
    while True:
        password = getpass.getpass("Enter password: ")
        if len(password) < 6:
            print("Password must be at least 6 characters.")
            continue
        password2 = getpass.getpass("Confirm password: ")
        if password != password2:
            print("Passwords don't match. Try again.")
            continue
        break
    
    # Save keystores
    save_substrate_keystore(mnemonic, ss58_address, password)
    save_evm_keystore(evm_private, evm_address, password)
    
    print("\n" + "═" * 60)
    print("  WALLET CREATED SUCCESSFULLY!")
    print("═" * 60)
    print(f"\n  Substrate Address: {ss58_address}")
    print(f"  EVM Address:       {evm_address}")
    print("\n" + "═" * 60)
    
    # Output for bash script
    print(f"\n__SS58__:{ss58_address}")
    print(f"__EVM__:{evm_address}")

if __name__ == "__main__":
    main()
PYEOF

    # ─────────────────────────────────────────────────────────────────────────
    # receive_info.py
    # ─────────────────────────────────────────────────────────────────────────
    cat > "$HELPERS_DIR/receive_info.py" << 'PYEOF'
#!/usr/bin/env python3
"""Show receive addresses without password."""

import sys
sys.path.insert(0, str(__file__).rsplit("/", 1)[0])
from key_manager import get_substrate_address, get_evm_address

def main():
    ss58 = get_substrate_address()
    evm = get_evm_address()
    
    print("\n" + "═" * 60)
    print("  YOUR LUMENYX ADDRESSES")
    print("═" * 60)
    
    if ss58:
        print(f"\n  Substrate (SS58): {ss58}")
        print("  └─ Use this for native LUMENYX transfers")
    else:
        print("\n  Substrate: Not found")
    
    if evm:
        print(f"\n  EVM (0x):         {evm}")
        print("  └─ Use this for ERC-20 tokens and contracts")
    else:
        print("\n  EVM: Not found")
    
    print("\n" + "═" * 60 + "\n")

if __name__ == "__main__":
    main()
PYEOF

    # ─────────────────────────────────────────────────────────────────────────
    # wallet_info.py
    # ─────────────────────────────────────────────────────────────────────────
    cat > "$HELPERS_DIR/wallet_info.py" << 'PYEOF'
#!/usr/bin/env python3
"""Get wallet info: balance, peers, blocks."""

import sys
import json
from substrateinterface import SubstrateInterface

sys.path.insert(0, str(__file__).rsplit("/", 1)[0])
from key_manager import get_substrate_address, get_evm_address

RPC_URL = "ws://127.0.0.1:9944"
DECIMALS = 12

def main():
    ss58 = get_substrate_address()
    evm = get_evm_address()
    
    try:
        substrate = SubstrateInterface(url=RPC_URL)
        
        # Get balance
        balance_raw = 0
        if ss58:
            result = substrate.query("System", "Account", [ss58])
            balance_raw = result.value["data"]["free"]
        
        balance = balance_raw / (10 ** DECIMALS)
        
        # Get chain info
        header = substrate.get_block_header()
        block_number = header["header"]["number"]
        
        # Get peers
        health = substrate.rpc_request("system_health", [])
        peers = health["result"]["peers"]
        syncing = health["result"]["isSyncing"]
        
        # Count mined blocks (scan last 100 blocks for rewards to our address)
        mined = 0
        if ss58:
            for i in range(max(0, block_number - 100), block_number + 1):
                try:
                    events = substrate.get_events(block_hash=substrate.get_block_hash(i))
                    for event in events:
                        if event.value["event_id"] == "BlockRewardIssued":
                            if str(event.value["attributes"][0]) == ss58:
                                mined += 1
                except:
                    pass
        
        print("\n" + "═" * 60)
        print("  WALLET INFO")
        print("═" * 60)
        print(f"\n  Balance:     {balance:.6f} LUMENYX")
        print(f"  Block:       #{block_number}")
        print(f"  Peers:       {peers}")
        print(f"  Syncing:     {'Yes' if syncing else 'No'}")
        print(f"  Mined (100): {mined} blocks")
        print("\n" + "═" * 60 + "\n")
        
    except Exception as e:
        print(f"\n  Error connecting to node: {e}")
        print("  Make sure the node is running.\n")
        sys.exit(1)

if __name__ == "__main__":
    main()
PYEOF

    # ─────────────────────────────────────────────────────────────────────────
    # send_substrate.py
    # ─────────────────────────────────────────────────────────────────────────
    cat > "$HELPERS_DIR/send_substrate.py" << 'PYEOF'
#!/usr/bin/env python3
"""Send LUMENYX via Substrate."""

import sys
import getpass
from decimal import Decimal
from substrateinterface import SubstrateInterface, Keypair

sys.path.insert(0, str(__file__).rsplit("/", 1)[0])
from key_manager import load_substrate_mnemonic

RPC_URL = "ws://127.0.0.1:9944"
DECIMALS = 12

def main():
    if len(sys.argv) < 3:
        print("Usage: send_substrate.py <destination> <amount>")
        sys.exit(1)
    
    destination = sys.argv[1]
    amount_str = sys.argv[2]
    
    try:
        amount = Decimal(amount_str)
        amount_planck = int(amount * (10 ** DECIMALS))
    except:
        print("Invalid amount")
        sys.exit(1)
    
    if amount_planck <= 0:
        print("Amount must be positive")
        sys.exit(1)
    
    # Get password and load key
    password = getpass.getpass("Enter wallet password: ")
    try:
        mnemonic = load_substrate_mnemonic(password)
        keypair = Keypair.create_from_mnemonic(mnemonic)
    except Exception as e:
        print(f"Failed to decrypt wallet: {e}")
        sys.exit(1)
    
    # Connect and send
    try:
        substrate = SubstrateInterface(url=RPC_URL)
        
        # Check balance
        result = substrate.query("System", "Account", [keypair.ss58_address])
        balance = result.value["data"]["free"]
        
        if balance < amount_planck:
            print(f"Insufficient balance. Have: {balance / 10**DECIMALS:.6f}, Need: {amount}")
            sys.exit(1)
        
        # Create and sign transaction
        call = substrate.compose_call(
            call_module="Balances",
            call_function="transfer_keep_alive",
            call_params={
                "dest": destination,
                "value": amount_planck
            }
        )
        
        extrinsic = substrate.create_signed_extrinsic(call=call, keypair=keypair)
        receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        
        if receipt.is_success:
            print("\n" + "═" * 60)
            print("  TRANSACTION SUCCESSFUL!")
            print("═" * 60)
            print(f"\n  To:     {destination}")
            print(f"  Amount: {amount} LUMENYX")
            print(f"  Hash:   {receipt.extrinsic_hash}")
            print(f"  Block:  #{receipt.block_number}")
            print("\n" + "═" * 60 + "\n")
        else:
            print(f"\nTransaction failed: {receipt.error_message}")
            sys.exit(1)
            
    except Exception as e:
        print(f"\nError: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
PYEOF

    # ─────────────────────────────────────────────────────────────────────────
    # bridge_withdraw.py
    # ─────────────────────────────────────────────────────────────────────────
    cat > "$HELPERS_DIR/bridge_withdraw.py" << 'PYEOF'
#!/usr/bin/env python3
"""Bridge: Withdraw from EVM to Substrate."""

import sys
import getpass
from decimal import Decimal
from substrateinterface import SubstrateInterface, Keypair
from eth_account import Account
from eth_account.messages import encode_defunct
from web3 import Web3

sys.path.insert(0, str(__file__).rsplit("/", 1)[0])
from key_manager import load_substrate_mnemonic, load_evm_private_key, get_substrate_address

RPC_URL = "ws://127.0.0.1:9944"
DECIMALS = 12
CHAIN_ID = 7777

def main():
    if len(sys.argv) < 2:
        print("Usage: bridge_withdraw.py <amount>")
        sys.exit(1)
    
    amount_str = sys.argv[1]
    
    try:
        amount = Decimal(amount_str)
        amount_planck = int(amount * (10 ** DECIMALS))
    except:
        print("Invalid amount")
        sys.exit(1)
    
    destination = get_substrate_address()
    if not destination:
        print("Substrate address not found")
        sys.exit(1)
    
    # Get password
    password = getpass.getpass("Enter wallet password: ")
    
    try:
        # Load keys
        mnemonic = load_substrate_mnemonic(password)
        substrate_keypair = Keypair.create_from_mnemonic(mnemonic)
        evm_private = load_evm_private_key(password)
        evm_account = Account.from_key(evm_private)
    except Exception as e:
        print(f"Failed to decrypt wallet: {e}")
        sys.exit(1)
    
    try:
        substrate = SubstrateInterface(url=RPC_URL)
        
        # Get nonce for EVM address
        nonce_result = substrate.query("EvmBridge", "Nonce", [evm_account.address])
        nonce = nonce_result.value if nonce_result.value else 0
        
        # Build payload for signature
        # Format: tag(1) + chain_id(8) + evm_addr(20) + substrate_addr(32) + amount(compact) + nonce(4)
        import struct
        from scalecodec import ScaleBytes
        from scalecodec.types import Compact
        
        payload = b'\x00'  # tag
        payload += struct.pack('<Q', CHAIN_ID)  # chain_id u64 LE
        payload += bytes.fromhex(evm_account.address[2:])  # H160
        payload += bytes.fromhex(substrate_keypair.public_key.hex())  # AccountId32
        
        # Compact encode amount
        compact = Compact()
        compact.encode(amount_planck)
        payload += compact.data.data
        
        payload += struct.pack('<I', nonce)  # nonce u32 LE
        
        # Sign with EIP-191
        message_hash = Web3.keccak(payload)
        message = encode_defunct(message_hash)
        signed = evm_account.sign_message(message)
        
        # Submit to bridge
        call = substrate.compose_call(
            call_module="EvmBridge",
            call_function="withdraw",
            call_params={
                "source": evm_account.address,
                "dest": destination,
                "value": amount_planck,
                "signature": signed.signature.hex()
            }
        )
        
        extrinsic = substrate.create_signed_extrinsic(call=call, keypair=substrate_keypair)
        receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        
        if receipt.is_success:
            print("\n" + "═" * 60)
            print("  BRIDGE WITHDRAWAL SUCCESSFUL!")
            print("═" * 60)
            print(f"\n  From EVM:      {evm_account.address}")
            print(f"  To Substrate:  {destination}")
            print(f"  Amount:        {amount} LUMENYX")
            print(f"  Hash:          {receipt.extrinsic_hash}")
            print("\n" + "═" * 60 + "\n")
        else:
            print(f"\nBridge withdrawal failed: {receipt.error_message}")
            sys.exit(1)
            
    except Exception as e:
        print(f"\nError: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
PYEOF

    # ─────────────────────────────────────────────────────────────────────────
    # backup_restore.py
    # ─────────────────────────────────────────────────────────────────────────
    cat > "$HELPERS_DIR/backup_restore.py" << 'PYEOF'
#!/usr/bin/env python3
"""Backup and restore wallet seed."""

import sys
import getpass
import shutil
from pathlib import Path
from substrateinterface import Keypair

sys.path.insert(0, str(__file__).rsplit("/", 1)[0])
from key_manager import (
    save_substrate_keystore, save_evm_keystore,
    load_substrate_mnemonic, get_substrate_address, get_evm_address,
    KEYS_DIR
)

def cmd_show_address():
    """Show addresses without password."""
    ss58 = get_substrate_address()
    evm = get_evm_address()
    print(f"\nSubstrate: {ss58 or 'Not found'}")
    print(f"EVM:       {evm or 'Not found'}\n")

def cmd_backup_seed():
    """Show seed phrase (requires password + double confirm)."""
    print("\n⚠️  WARNING: Your seed phrase gives FULL ACCESS to your funds!")
    print("   Never share it with anyone. Never enter it on websites.\n")
    
    confirm1 = input("Are you in a private place? Type 'YES': ")
    if confirm1.upper() != "YES":
        print("Aborted.")
        return
    
    confirm2 = input("Are you SURE you want to see your seed? Type 'YES': ")
    if confirm2.upper() != "YES":
        print("Aborted.")
        return
    
    password = getpass.getpass("Enter wallet password: ")
    
    try:
        mnemonic = load_substrate_mnemonic(password)
        print("\n" + "═" * 60)
        print("  YOUR SEED PHRASE (12 words)")
        print("═" * 60)
        print(f"\n  {mnemonic}\n")
        print("═" * 60 + "\n")
    except Exception as e:
        print(f"Failed to decrypt: {e}")

def cmd_import_seed():
    """Import existing seed phrase."""
    print("\nEnter your 12 or 24 word seed phrase:")
    mnemonic = input("> ").strip()
    
    words = mnemonic.split()
    if len(words) not in [12, 24]:
        print(f"Invalid: expected 12 or 24 words, got {len(words)}")
        return
    
    try:
        keypair = Keypair.create_from_mnemonic(mnemonic)
        ss58 = keypair.ss58_address
    except Exception as e:
        print(f"Invalid mnemonic: {e}")
        return
    
    # Derive EVM key
    try:
        from bip_utils import Bip39SeedGenerator, Bip44, Bip44Coins
        seed = Bip39SeedGenerator(mnemonic).Generate()
        bip44 = Bip44.FromSeed(seed, Bip44Coins.ETHEREUM)
        evm_key = bip44.Purpose().Coin().Account(0).Change(0).AddressIndex(0)
        evm_private = evm_key.PrivateKey().Raw().ToBytes()
        evm_address = evm_key.PublicKey().ToAddress()
    except:
        from eth_account import Account
        acct = Account.create()
        evm_private = acct.key
        evm_address = acct.address
    
    print(f"\nSubstrate address: {ss58}")
    print(f"EVM address:       {evm_address}")
    
    confirm = input("\nImport this wallet? Type 'YES': ")
    if confirm.upper() != "YES":
        print("Aborted.")
        return
    
    while True:
        password = getpass.getpass("Choose password: ")
        if len(password) < 6:
            print("Password must be at least 6 characters.")
            continue
        password2 = getpass.getpass("Confirm password: ")
        if password != password2:
            print("Passwords don't match.")
            continue
        break
    
    save_substrate_keystore(mnemonic, ss58, password)
    save_evm_keystore(evm_private, evm_address, password)
    
    print("\n✓ Wallet imported successfully!\n")

def cmd_wipe():
    """Delete all keys (triple confirm)."""
    print("\n" + "═" * 60)
    print("  ⚠️  DANGER: This will DELETE ALL YOUR KEYS!")
    print("  You will LOSE ACCESS to your funds unless you have a backup!")
    print("═" * 60 + "\n")
    
    confirm1 = input("Type 'YES' to continue: ")
    if confirm1.upper() != "YES":
        print("Aborted.")
        return
    
    confirm2 = input("Type 'DELETE' to confirm deletion: ")
    if confirm2.upper() != "DELETE":
        print("Aborted.")
        return
    
    confirm3 = input("Type 'YES DELETE' to permanently delete keys: ")
    if confirm3.upper() != "YES DELETE":
        print("Aborted.")
        return
    
    try:
        shutil.rmtree(KEYS_DIR)
        KEYS_DIR.mkdir(parents=True, exist_ok=True)
        print("\n✓ All keys deleted.\n")
    except Exception as e:
        print(f"Error: {e}")

def main():
    if len(sys.argv) < 2:
        print("Usage: backup_restore.py <command>")
        print("Commands: show-address, backup-seed, import, wipe")
        sys.exit(1)
    
    cmd = sys.argv[1]
    
    if cmd == "show-address":
        cmd_show_address()
    elif cmd == "backup-seed":
        cmd_backup_seed()
    elif cmd == "import":
        cmd_import_seed()
    elif cmd == "wipe":
        cmd_wipe()
    else:
        print(f"Unknown command: {cmd}")
        sys.exit(1)

if __name__ == "__main__":
    main()
PYEOF

    # ─────────────────────────────────────────────────────────────────────────
    # evm_send.py
    # ─────────────────────────────────────────────────────────────────────────
    cat > "$HELPERS_DIR/evm_send.py" << 'PYEOF'
#!/usr/bin/env python3
"""Send native LUMENYX or ERC-20 tokens via EVM."""

import sys
import getpass
from decimal import Decimal
from web3 import Web3
from eth_account import Account

sys.path.insert(0, str(__file__).rsplit("/", 1)[0])
from key_manager import load_evm_private_key, get_evm_address

RPC_URL = "http://127.0.0.1:9944"
CHAIN_ID = 7777

ERC20_ABI = [
    {"constant":True,"inputs":[],"name":"decimals","outputs":[{"name":"","type":"uint8"}],"type":"function"},
    {"constant":False,"inputs":[{"name":"to","type":"address"},{"name":"value","type":"uint256"}],"name":"transfer","outputs":[{"name":"","type":"bool"}],"type":"function"}
]

def main():
    if len(sys.argv) < 3:
        print("Usage: evm_send.py <to> <amount> [--token <contract>]")
        sys.exit(1)
    
    to_address = sys.argv[1]
    amount_str = sys.argv[2]
    token_contract = None
    
    if "--token" in sys.argv:
        idx = sys.argv.index("--token")
        token_contract = sys.argv[idx + 1]
    
    try:
        amount = Decimal(amount_str)
    except:
        print("Invalid amount")
        sys.exit(1)
    
    password = getpass.getpass("Enter wallet password: ")
    
    try:
        private_key = load_evm_private_key(password)
        account = Account.from_key(private_key)
    except Exception as e:
        print(f"Failed to decrypt wallet: {e}")
        sys.exit(1)
    
    try:
        w3 = Web3(Web3.HTTPProvider(RPC_URL))
        
        if token_contract:
            # ERC-20 transfer
            contract = w3.eth.contract(address=Web3.to_checksum_address(token_contract), abi=ERC20_ABI)
            decimals = contract.functions.decimals().call()
            amount_wei = int(amount * (10 ** decimals))
            
            tx = contract.functions.transfer(
                Web3.to_checksum_address(to_address),
                amount_wei
            ).build_transaction({
                'from': account.address,
                'nonce': w3.eth.get_transaction_count(account.address),
                'gas': 100000,
                'gasPrice': w3.eth.gas_price,
                'chainId': CHAIN_ID
            })
        else:
            # Native transfer
            amount_wei = int(amount * (10 ** 18))
            tx = {
                'to': Web3.to_checksum_address(to_address),
                'value': amount_wei,
                'nonce': w3.eth.get_transaction_count(account.address),
                'gas': 21000,
                'gasPrice': w3.eth.gas_price,
                'chainId': CHAIN_ID
            }
        
        signed = account.sign_transaction(tx)
        tx_hash = w3.eth.send_raw_transaction(signed.raw_transaction)
        receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
        
        print("\n" + "═" * 60)
        print("  EVM TRANSACTION SUCCESSFUL!")
        print("═" * 60)
        print(f"\n  To:     {to_address}")
        print(f"  Amount: {amount}")
        print(f"  Hash:   {tx_hash.hex()}")
        print(f"  Block:  #{receipt['blockNumber']}")
        print("\n" + "═" * 60 + "\n")
        
    except Exception as e:
        print(f"\nError: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
PYEOF

    # ─────────────────────────────────────────────────────────────────────────
    # evm_tokens.py
    # ─────────────────────────────────────────────────────────────────────────
    cat > "$HELPERS_DIR/evm_tokens.py" << 'PYEOF'
#!/usr/bin/env python3
"""Manage ERC-20 tokens."""

import sys
import json
from pathlib import Path
from web3 import Web3

sys.path.insert(0, str(__file__).rsplit("/", 1)[0])
from key_manager import get_evm_address

RPC_URL = "http://127.0.0.1:9944"
TOKENS_FILE = Path.home() / ".lumenyx" / "tokens.json"

ERC20_ABI = [
    {"constant":True,"inputs":[],"name":"name","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"decimals","outputs":[{"name":"","type":"uint8"}],"type":"function"},
    {"constant":True,"inputs":[{"name":"owner","type":"address"}],"name":"balanceOf","outputs":[{"name":"","type":"uint256"}],"type":"function"}
]

def load_tokens():
    if TOKENS_FILE.exists():
        return json.loads(TOKENS_FILE.read_text())
    return {}

def save_tokens(tokens):
    TOKENS_FILE.write_text(json.dumps(tokens, indent=2))

def cmd_list():
    tokens = load_tokens()
    if not tokens:
        print("\nNo tokens saved. Use 'add' to add a token.\n")
        return
    print("\n" + "═" * 60)
    print("  SAVED TOKENS")
    print("═" * 60)
    for addr, info in tokens.items():
        print(f"\n  {info['symbol']} - {info['name']}")
        print(f"  Contract: {addr}")
    print("\n" + "═" * 60 + "\n")

def cmd_add(contract_addr):
    try:
        w3 = Web3(Web3.HTTPProvider(RPC_URL))
        contract = w3.eth.contract(address=Web3.to_checksum_address(contract_addr), abi=ERC20_ABI)
        
        name = contract.functions.name().call()
        symbol = contract.functions.symbol().call()
        decimals = contract.functions.decimals().call()
        
        tokens = load_tokens()
        tokens[contract_addr.lower()] = {
            "name": name,
            "symbol": symbol,
            "decimals": decimals
        }
        save_tokens(tokens)
        
        print(f"\n✓ Added {symbol} ({name})\n")
    except Exception as e:
        print(f"\nError: {e}\n")

def cmd_balance(contract_addr=None):
    evm_addr = get_evm_address()
    if not evm_addr:
        print("\nEVM address not found.\n")
        return
    
    try:
        w3 = Web3(Web3.HTTPProvider(RPC_URL))
        tokens = load_tokens()
        
        print("\n" + "═" * 60)
        print("  TOKEN BALANCES")
        print("═" * 60)
        
        if contract_addr:
            tokens = {contract_addr: tokens.get(contract_addr.lower(), {"symbol": "TOKEN", "decimals": 18})}
        
        for addr, info in tokens.items():
            contract = w3.eth.contract(address=Web3.to_checksum_address(addr), abi=ERC20_ABI)
            balance = contract.functions.balanceOf(Web3.to_checksum_address(evm_addr)).call()
            balance_human = balance / (10 ** info.get("decimals", 18))
            print(f"\n  {info.get('symbol', 'TOKEN')}: {balance_human:.6f}")
        
        print("\n" + "═" * 60 + "\n")
    except Exception as e:
        print(f"\nError: {e}\n")

def main():
    if len(sys.argv) < 2:
        print("Usage: evm_tokens.py <command> [args]")
        print("Commands: list, add <contract>, balance [contract]")
        sys.exit(1)
    
    cmd = sys.argv[1]
    
    if cmd == "list":
        cmd_list()
    elif cmd == "add" and len(sys.argv) > 2:
        cmd_add(sys.argv[2])
    elif cmd == "balance":
        cmd_balance(sys.argv[2] if len(sys.argv) > 2 else None)
    else:
        print(f"Unknown command: {cmd}")

if __name__ == "__main__":
    main()
PYEOF

    # ─────────────────────────────────────────────────────────────────────────
    # contracts_helper.py
    # ─────────────────────────────────────────────────────────────────────────
    cat > "$HELPERS_DIR/contracts_helper.py" << 'PYEOF'
#!/usr/bin/env python3
"""Deploy and interact with smart contracts."""

import sys
import json
import getpass
from web3 import Web3
from eth_account import Account

sys.path.insert(0, str(__file__).rsplit("/", 1)[0])
from key_manager import load_evm_private_key

RPC_URL = "http://127.0.0.1:9944"
CHAIN_ID = 7777

def cmd_deploy(abi_file, bytecode_file, *constructor_args):
    password = getpass.getpass("Enter wallet password: ")
    
    try:
        private_key = load_evm_private_key(password)
        account = Account.from_key(private_key)
    except Exception as e:
        print(f"Failed to decrypt wallet: {e}")
        sys.exit(1)
    
    try:
        w3 = Web3(Web3.HTTPProvider(RPC_URL))
        
        with open(abi_file) as f:
            abi = json.load(f)
        with open(bytecode_file) as f:
            bytecode = f.read().strip()
        
        contract = w3.eth.contract(abi=abi, bytecode=bytecode)
        
        tx = contract.constructor(*constructor_args).build_transaction({
            'from': account.address,
            'nonce': w3.eth.get_transaction_count(account.address),
            'gas': 3000000,
            'gasPrice': w3.eth.gas_price,
            'chainId': CHAIN_ID
        })
        
        signed = account.sign_transaction(tx)
        tx_hash = w3.eth.send_raw_transaction(signed.raw_transaction)
        receipt = w3.eth.wait_for_transaction_receipt(tx_hash)
        
        print("\n" + "═" * 60)
        print("  CONTRACT DEPLOYED!")
        print("═" * 60)
        print(f"\n  Address:  {receipt['contractAddress']}")
        print(f"  TX Hash:  {tx_hash.hex()}")
        print(f"  Block:    #{receipt['blockNumber']}")
        print("\n" + "═" * 60 + "\n")
        
    except Exception as e:
        print(f"\nError: {e}")
        sys.exit(1)

def cmd_call(contract_addr, abi_file, function_name, *args):
    try:
        w3 = Web3(Web3.HTTPProvider(RPC_URL))
        
        with open(abi_file) as f:
            abi = json.load(f)
        
        contract = w3.eth.contract(address=Web3.to_checksum_address(contract_addr), abi=abi)
        func = getattr(contract.functions, function_name)
        result = func(*args).call()
        
        print(f"\nResult: {result}\n")
        
    except Exception as e:
        print(f"\nError: {e}")
        sys.exit(1)

def main():
    if len(sys.argv) < 2:
        print("Usage: contracts_helper.py <command> [args]")
        print("Commands:")
        print("  deploy <abi.json> <bytecode> [constructor_args...]")
        print("  call <contract> <abi.json> <function> [args...]")
        sys.exit(1)
    
    cmd = sys.argv[1]
    
    if cmd == "deploy" and len(sys.argv) >= 4:
        cmd_deploy(sys.argv[2], sys.argv[3], *sys.argv[4:])
    elif cmd == "call" and len(sys.argv) >= 5:
        cmd_call(sys.argv[2], sys.argv[3], sys.argv[4], *sys.argv[5:])
    else:
        print(f"Unknown command or missing args")

if __name__ == "__main__":
    main()
PYEOF

    # ─────────────────────────────────────────────────────────────────────────
    # history.py
    # ─────────────────────────────────────────────────────────────────────────
    cat > "$HELPERS_DIR/history.py" << 'PYEOF'
#!/usr/bin/env python3
"""Transaction history."""

import sys
from substrateinterface import SubstrateInterface

sys.path.insert(0, str(__file__).rsplit("/", 1)[0])
from key_manager import get_substrate_address

RPC_URL = "ws://127.0.0.1:9944"
DECIMALS = 12

def main():
    ss58 = get_substrate_address()
    if not ss58:
        print("\nNo wallet found.\n")
        return
    
    try:
        substrate = SubstrateInterface(url=RPC_URL)
        header = substrate.get_block_header()
        current_block = header["header"]["number"]
        
        print("\n" + "═" * 60)
        print("  RECENT TRANSACTIONS (last 200 blocks)")
        print("═" * 60 + "\n")
        
        found = 0
        for i in range(current_block, max(0, current_block - 200), -1):
            try:
                block_hash = substrate.get_block_hash(i)
                events = substrate.get_events(block_hash=block_hash)
                
                for event in events:
                    if event.value["event_id"] == "Transfer":
                        attrs = event.value["attributes"]
                        from_addr = str(attrs[0])
                        to_addr = str(attrs[1])
                        amount = attrs[2] / (10 ** DECIMALS)
                        
                        if from_addr == ss58:
                            print(f"  #{i} SENT    {amount:.6f} LUMENYX → {to_addr[:20]}...")
                            found += 1
                        elif to_addr == ss58:
                            print(f"  #{i} RECV    {amount:.6f} LUMENYX ← {from_addr[:20]}...")
                            found += 1
                    
                    elif event.value["event_id"] == "BlockRewardIssued":
                        if str(event.value["attributes"][0]) == ss58:
                            amount = event.value["attributes"][1] / (10 ** DECIMALS)
                            print(f"  #{i} MINED   {amount:.6f} LUMENYX")
                            found += 1
                
                if found >= 20:
                    break
            except:
                pass
        
        if found == 0:
            print("  No transactions found.")
        
        print("\n" + "═" * 60 + "\n")
        
    except Exception as e:
        print(f"\nError: {e}\n")

if __name__ == "__main__":
    main()
PYEOF

    chmod +x "$HELPERS_DIR"/*.py
    print_success "Python helpers installed"
}

# ══════════════════════════════════════════════════════════════════════════════
# FIRST RUN SETUP
# ══════════════════════════════════════════════════════════════════════════════

do_first_run() {
    print_banner
    echo "Welcome to LUMENYX setup!"
    echo ""
    echo "This script will:"
    echo "  1. Check your system"
    echo "  2. Download LUMENYX"
    echo "  3. Generate your mining wallet"  
    echo "  4. Start the node"
    echo ""
    press_enter

    # Pre-flight checks
    echo ""
    echo -e "${CYAN}═══ PRE-FLIGHT CHECKS ═══${NC}"
    echo ""
    
    if pgrep "$BINARY_NAME" > /dev/null 2>&1; then
        print_warn "LUMENYX node already running. Stopping..."
        pkill -f "$BINARY_NAME" || true
        sleep 2
    fi
    
    print_success "Pre-flight checks passed!"
    press_enter

    # System check
    echo ""
    echo -e "${CYAN}═══ STEP 1: SYSTEM CHECK ═══${NC}"
    echo ""
    
    if [[ "$(uname)" != "Linux" ]]; then
        print_error "This script requires Linux."
        exit 1
    fi
    print_success "Operating system: Linux"
    
    if [[ "$(uname -m)" != "x86_64" ]]; then
        print_error "This script requires x86_64 architecture."
        exit 1
    fi
    print_success "Architecture: x86_64"
    
    if ! command -v curl &> /dev/null; then
        print_error "curl not found. Please install curl."
        exit 1
    fi
    print_success "curl: installed"
    
    if ! curl -s --connect-timeout 5 https://github.com > /dev/null; then
        print_error "Cannot reach GitHub. Check your internet connection."
        exit 1
    fi
    print_success "Internet: OK"
    
    DISK_FREE=$(df -BG "$HOME" | tail -1 | awk '{print $4}' | tr -d 'G')
    if [[ "$DISK_FREE" -lt 5 ]]; then
        print_error "Insufficient disk space. Need at least 5GB."
        exit 1
    fi
    print_success "Disk space: ${DISK_FREE}GB available"
    
    print_success "System check passed!"
    press_enter

    # Download binary
    echo ""
    echo -e "${CYAN}═══ STEP 2: INSTALLATION ═══${NC}"
    echo ""
    
    mkdir -p "$LUMENYX_DIR"
    
    print_info "Downloading lumenyx-node (~65MB)..."
    curl -L --progress-bar -o "$LUMENYX_DIR/$BINARY_NAME" "$BINARY_URL"
    chmod +x "$LUMENYX_DIR/$BINARY_NAME"
    print_success "Download complete"
    
    print_info "Verifying checksum..."
    EXPECTED_CHECKSUM=$(curl -sL "$CHECKSUM_URL" | awk '{print $1}')
    ACTUAL_CHECKSUM=$(sha256sum "$LUMENYX_DIR/$BINARY_NAME" | awk '{print $1}')
    
    if [[ "$EXPECTED_CHECKSUM" != "$ACTUAL_CHECKSUM" ]]; then
        print_error "Checksum mismatch! Download may be corrupted."
        rm -f "$LUMENYX_DIR/$BINARY_NAME"
        exit 1
    fi
    print_success "Checksum verified"
    print_success "Binary ready: $LUMENYX_DIR/$BINARY_NAME"
    press_enter

    # Node mode selection
    echo ""
    echo -e "${CYAN}═══ STEP 3: NODE MODE ═══${NC}"
    echo ""
    echo "  [1] MINING - Earn LUMENYX (uses CPU)"
    echo "  [2] SYNC ONLY - Just verify (lightweight)"
    echo ""
    read -p "Your choice [1/2]: " MODE_CHOICE
    
    case "$MODE_CHOICE" in
        2) MINING_MODE=false ;;
        *) MINING_MODE=true ;;
    esac
    
    read -p "Node name: " NODE_NAME
    NODE_NAME=${NODE_NAME:-"lumenyx-node"}
    
    if $MINING_MODE; then
        print_success "Mode: mining"
    else
        print_success "Mode: sync-only"
    fi
    print_success "Name: $NODE_NAME"
    press_enter

    # Install Python helpers
    echo ""
    echo -e "${CYAN}═══ STEP 4: WALLET GENERATION ═══${NC}"
    echo ""
    
    install_python_helpers
    
    # Generate wallet
    echo ""
    python3 "$HELPERS_DIR/wallet_init.py"
    
    # Get the generated address
    SS58_ADDRESS=$(cat "$KEYS_DIR/substrate_address.txt" 2>/dev/null || echo "")
    
    print_success "Wallet ready!"
    press_enter

    # Create start script
    echo ""
    echo -e "${CYAN}═══ STEP 5: SETUP ═══${NC}"
    echo ""
    
    if $MINING_MODE; then
        FULL_CMD="$LUMENYX_DIR/$BINARY_NAME --chain mainnet --name \"$NODE_NAME\" --validator --rpc-cors all --unsafe-rpc-external --rpc-methods Safe --bootnodes $BOOTNODE"
    else
        FULL_CMD="$LUMENYX_DIR/$BINARY_NAME --chain mainnet --name \"$NODE_NAME\" --rpc-cors all --unsafe-rpc-external --rpc-methods Safe --bootnodes $BOOTNODE"
    fi
    
    cat > "$LUMENYX_DIR/start.sh" << EOF
#!/bin/bash
$FULL_CMD
EOF
    chmod +x "$LUMENYX_DIR/start.sh"
    print_success "Start script: $LUMENYX_DIR/start.sh"
    
    # Systemd option
    echo ""
    read -p "Create systemd service (auto-restart if node crashes)? [y/n]: " SYSTEMD_CHOICE
    
    if [[ "$SYSTEMD_CHOICE" == "y" || "$SYSTEMD_CHOICE" == "Y" ]]; then
        sudo mkdir -p /etc/lumenyx
        echo "$BOOTNODE" | sudo tee /etc/lumenyx/bootnodes.txt > /dev/null
        
        cat << EOF | sudo tee /etc/systemd/system/lumenyx.service > /dev/null
[Unit]
Description=LUMENYX Node
After=network.target

[Service]
Type=simple
User=$USER
ExecStart=$LUMENYX_DIR/$BINARY_NAME --chain mainnet --name "$NODE_NAME" $(if $MINING_MODE; then echo "--validator"; fi) --rpc-cors all --unsafe-rpc-external --rpc-methods Safe --bootnodes $BOOTNODE
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF
        
        sudo systemctl daemon-reload
        sudo systemctl enable lumenyx
        print_success "Systemd service created"
    fi

    # Start node
    echo ""
    echo -e "${CYAN}═══ STEP 6: START NODE ═══${NC}"
    echo ""
    read -p "Start node now? [y/n]: " START_CHOICE
    
    if [[ "$START_CHOICE" == "y" || "$START_CHOICE" == "Y" ]]; then
        if [[ "$SYSTEMD_CHOICE" == "y" || "$SYSTEMD_CHOICE" == "Y" ]]; then
            sudo systemctl start lumenyx
            print_success "Node started via systemd"
            echo ""
            print_info "View logs: journalctl -u lumenyx -f"
        else
            print_info "Starting node..."
            "$LUMENYX_DIR/start.sh"
        fi
    else
        echo ""
        print_info "To start later: $LUMENYX_DIR/start.sh"
    fi
}

# ══════════════════════════════════════════════════════════════════════════════
# WALLET MENU
# ══════════════════════════════════════════════════════════════════════════════

menu_send() {
    clear
    echo -e "${CYAN}═══ SEND LUMENYX ═══${NC}"
    echo ""
    
    # Check if node is running and has peers
    if ! is_node_running; then
        print_error "Node is not running. Start the node first."
        press_enter
        return
    fi
    
    PEERS=$(curl -s -X POST -H "Content-Type: application/json" \
        --data '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' \
        "$RPC_URL" 2>/dev/null | grep -o '"peers":[0-9]*' | cut -d: -f2)
    
    if [[ "$PEERS" == "0" ]]; then
        print_warn "No peers connected. Transaction may not propagate."
        read -p "Continue anyway? [y/n]: " CONTINUE
        [[ "$CONTINUE" != "y" ]] && return
    fi
    
    read -p "Destination address: " DEST
    read -p "Amount (LUMENYX): " AMOUNT
    
    echo ""
    python3 "$HELPERS_DIR/send_substrate.py" "$DEST" "$AMOUNT"
    press_enter
}

menu_receive() {
    clear
    python3 "$HELPERS_DIR/receive_info.py"
    press_enter
}

menu_history() {
    clear
    python3 "$HELPERS_DIR/history.py"
    press_enter
}

menu_tokens() {
    while true; do
        clear
        echo -e "${CYAN}═══ ERC-20 TOKENS ═══${NC}"
        echo ""
        echo "  [1] List saved tokens"
        echo "  [2] Add token"
        echo "  [3] Check balance"
        echo "  [0] Back"
        echo ""
        read -p "Choice: " CHOICE
        
        case "$CHOICE" in
            1) python3 "$HELPERS_DIR/evm_tokens.py" list; press_enter ;;
            2) 
                read -p "Token contract address: " CONTRACT
                python3 "$HELPERS_DIR/evm_tokens.py" add "$CONTRACT"
                press_enter
                ;;
            3) python3 "$HELPERS_DIR/evm_tokens.py" balance; press_enter ;;
            0) break ;;
        esac
    done
}

menu_contracts() {
    clear
    echo -e "${CYAN}═══ SMART CONTRACTS ═══${NC}"
    echo ""
    echo "  [1] Deploy contract"
    echo "  [2] Call function (read)"
    echo "  [0] Back"
    echo ""
    read -p "Choice: " CHOICE
    
    case "$CHOICE" in
        1)
            read -p "ABI file path: " ABI
            read -p "Bytecode file path: " BYTECODE
            python3 "$HELPERS_DIR/contracts_helper.py" deploy "$ABI" "$BYTECODE"
            press_enter
            ;;
        2)
            read -p "Contract address: " CONTRACT
            read -p "ABI file path: " ABI
            read -p "Function name: " FUNC
            python3 "$HELPERS_DIR/contracts_helper.py" call "$CONTRACT" "$ABI" "$FUNC"
            press_enter
            ;;
    esac
}

menu_mining() {
    while true; do
        clear
        echo -e "${CYAN}═══ MINING ═══${NC}"
        echo ""
        
        if is_node_running; then
            print_success "Node is RUNNING"
        else
            print_warn "Node is STOPPED"
        fi
        
        echo ""
        echo "  [1] Start mining"
        echo "  [2] Stop mining"
        echo "  [3] View stats"
        echo "  [0] Back"
        echo ""
        read -p "Choice: " CHOICE
        
        case "$CHOICE" in
            1)
                if is_node_running; then
                    print_warn "Node already running"
                else
                    if systemctl is-enabled lumenyx &>/dev/null; then
                        sudo systemctl start lumenyx
                        print_success "Started via systemd"
                    else
                        nohup "$LUMENYX_DIR/start.sh" > "$LOG_FILE" 2>&1 &
                        print_success "Started in background"
                    fi
                fi
                press_enter
                ;;
            2)
                if is_node_running; then
                    if systemctl is-active lumenyx &>/dev/null; then
                        sudo systemctl stop lumenyx
                    else
                        pkill -f "$BINARY_NAME"
                    fi
                    print_success "Node stopped"
                else
                    print_warn "Node not running"
                fi
                press_enter
                ;;
            3)
                python3 "$HELPERS_DIR/wallet_info.py"
                press_enter
                ;;
            0) break ;;
        esac
    done
}

menu_bridge() {
    clear
    echo -e "${CYAN}═══ BRIDGE: EVM → SUBSTRATE ═══${NC}"
    echo ""
    read -p "Amount to withdraw: " AMOUNT
    python3 "$HELPERS_DIR/bridge_withdraw.py" "$AMOUNT"
    press_enter
}

menu_backup() {
    while true; do
        clear
        echo -e "${CYAN}═══ BACKUP / RESTORE ═══${NC}"
        echo ""
        echo "  [1] Show addresses (no password)"
        echo "  [2] Backup seed phrase"
        echo "  [3] Import seed phrase"
        echo "  [4] WIPE ALL KEYS (danger!)"
        echo "  [0] Back"
        echo ""
        read -p "Choice: " CHOICE
        
        case "$CHOICE" in
            1) python3 "$HELPERS_DIR/backup_restore.py" show-address; press_enter ;;
            2) python3 "$HELPERS_DIR/backup_restore.py" backup-seed; press_enter ;;
            3) python3 "$HELPERS_DIR/backup_restore.py" import; press_enter ;;
            4) python3 "$HELPERS_DIR/backup_restore.py" wipe; press_enter ;;
            0) break ;;
        esac
    done
}

menu_network() {
    clear
    echo -e "${CYAN}═══ NETWORK STATUS ═══${NC}"
    echo ""
    
    if ! is_node_running; then
        print_error "Node is not running"
        press_enter
        return
    fi
    
    HEALTH=$(curl -s -X POST -H "Content-Type: application/json" \
        --data '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' \
        "$RPC_URL" 2>/dev/null)
    
    PEERS=$(echo "$HEALTH" | grep -o '"peers":[0-9]*' | cut -d: -f2)
    SYNCING=$(echo "$HEALTH" | grep -o '"isSyncing":[a-z]*' | cut -d: -f2)
    
    CHAIN=$(curl -s -X POST -H "Content-Type: application/json" \
        --data '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
        "$RPC_URL" 2>/dev/null)
    
    BLOCK=$(echo "$CHAIN" | grep -o '"number":"0x[0-9a-f]*"' | cut -d'"' -f4)
    BLOCK_DEC=$((BLOCK))
    
    echo "  Peers:    $PEERS"
    echo "  Syncing:  $SYNCING"
    echo "  Block:    #$BLOCK_DEC"
    echo ""
    press_enter
}

menu_settings() {
    while true; do
        clear
        echo -e "${CYAN}═══ SETTINGS ═══${NC}"
        echo ""
        echo "  [1] Edit bootnodes"
        echo "  [2] View current config"
        echo "  [0] Back"
        echo ""
        read -p "Choice: " CHOICE
        
        case "$CHOICE" in
            1)
                echo ""
                echo "Current bootnode:"
                echo "  $BOOTNODE"
                echo ""
                print_info "Edit start.sh to change bootnodes:"
                echo "  nano $LUMENYX_DIR/start.sh"
                press_enter
                ;;
            2)
                echo ""
                echo "Binary: $LUMENYX_DIR/$BINARY_NAME"
                echo "Data:   $DATA_DIR"
                echo "Keys:   $KEYS_DIR"
                echo "Logs:   $LOG_FILE"
                echo ""
                press_enter
                ;;
            0) break ;;
        esac
    done
}

menu_logs() {
    clear
    echo -e "${CYAN}═══ LOGS (Ctrl+C to exit) ═══${NC}"
    echo ""
    
    if systemctl is-active lumenyx &>/dev/null; then
        journalctl -u lumenyx -f
    elif [[ -f "$LOG_FILE" ]]; then
        tail -f "$LOG_FILE"
    else
        print_warn "No logs found"
        press_enter
    fi
}

menu_evm_send() {
    clear
    echo -e "${CYAN}═══ EVM SEND ═══${NC}"
    echo ""
    echo "  [1] Send native LUMENYX (EVM)"
    echo "  [2] Send ERC-20 token"
    echo "  [0] Back"
    echo ""
    read -p "Choice: " CHOICE
    
    case "$CHOICE" in
        1)
            read -p "To address (0x...): " TO
            read -p "Amount: " AMOUNT
            python3 "$HELPERS_DIR/evm_send.py" "$TO" "$AMOUNT"
            press_enter
            ;;
        2)
            read -p "To address (0x...): " TO
            read -p "Amount: " AMOUNT
            read -p "Token contract: " TOKEN
            python3 "$HELPERS_DIR/evm_send.py" "$TO" "$AMOUNT" --token "$TOKEN"
            press_enter
            ;;
    esac
}

show_main_menu() {
    while true; do
        print_banner
        
        SS58=$(cat "$KEYS_DIR/substrate_address.txt" 2>/dev/null || echo "Not set")
        echo -e "  Address: ${GREEN}$SS58${NC}"
        
        if is_node_running; then
            echo -e "  Status:  ${GREEN}● Running${NC}"
        else
            echo -e "  Status:  ${RED}○ Stopped${NC}"
        fi
        
        echo ""
        echo "═══════════════════════════════════════════════════════════════════"
        echo ""
        echo "  [1]  Send (Substrate)"
        echo "  [2]  Receive"
        echo "  [3]  History"
        echo "  [4]  Tokens (ERC-20)"
        echo "  [5]  Contracts"
        echo "  [6]  Mining"
        echo "  [7]  Bridge"
        echo "  [8]  Backup/Restore"
        echo "  [9]  Network"
        echo "  [10] Settings"
        echo "  [11] Logs"
        echo "  [12] EVM Send"
        echo "  [0]  Exit"
        echo ""
        read -p "Choice: " CHOICE
        
        case "$CHOICE" in
            1) menu_send ;;
            2) menu_receive ;;
            3) menu_history ;;
            4) menu_tokens ;;
            5) menu_contracts ;;
            6) menu_mining ;;
            7) menu_bridge ;;
            8) menu_backup ;;
            9) menu_network ;;
            10) menu_settings ;;
            11) menu_logs ;;
            12) menu_evm_send ;;
            0) echo ""; print_info "Goodbye!"; exit 0 ;;
        esac
    done
}

# ══════════════════════════════════════════════════════════════════════════════
# MAIN
# ══════════════════════════════════════════════════════════════════════════════

main() {
    # Check for updates on every run
    if [[ -f "$LUMENYX_DIR/$BINARY_NAME" ]]; then
        if ! check_for_updates; then
            echo ""
            print_warn "A new version is available!"
            echo ""
            read -r -p "Update to v$VERSION? [Y/n]: " update_choice
            if [[ ! "$update_choice" =~ ^[Nn] ]]; then
                do_update
                press_enter
            fi
        fi
    fi

    if is_first_run; then
        do_first_run
    else
        # Ensure helpers are installed
        if [[ ! -f "$HELPERS_DIR/wallet_init.py" ]]; then
            install_python_helpers
        fi
        show_main_menu
    fi
}

main "$@"
