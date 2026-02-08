#!/bin/bash

# ═══════════════════════════════════════════════════════════════════════════════
# LUMENYX SETUP SCRIPT v2.3.4 - Sync-safe mode + Bug fixes
# ═══════════════════════════════════════════════════════════════════════════════

set -e

VERSION="2.3.3"
SCRIPT_VERSION="2.3.4"

# Colors - LUMO brand palette
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[38;5;51m'        # Bright Cyan #00F0FF
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'
VIOLET='\033[38;5;129m'     # Viola #6A0DAD
PINK='\033[38;5;200m'       # Pink #FF00E5
BRIGHT_VIOLET='\033[38;5;165m' # #A100FF

# Configuration
LUMENYX_DIR="$HOME/.lumenyx"
BINARY_NAME="lumenyx-node"
DATA_DIR="$HOME/.local/share/lumenyx-node"
PID_FILE="$LUMENYX_DIR/lumenyx.pid"
LOG_FILE="$LUMENYX_DIR/lumenyx.log"
RPC="http://127.0.0.1:9944"
WS="ws://127.0.0.1:9944"
RPC_TIMEOUT=5
RPC_RETRIES=3

# Mining threads (empty = auto/all cores)
THREADS_FILE="$LUMENYX_DIR/mining_threads.conf"

# Daemon mode (systemd 24/7)
DAEMON_CONF="$LUMENYX_DIR/daemon.conf"
SYSTEMD_SERVICE="/etc/systemd/system/lumenyx.service"
SYSTEMD_WATCHDOG="/etc/systemd/system/lumenyx-watchdog.service"
SYSTEMD_WATCHDOG_TIMER="/etc/systemd/system/lumenyx-watchdog.timer"
WATCHDOG_SCRIPT="/usr/local/bin/lumenyx-watchdog.sh"
AUTOSTART_FILE="$HOME/.config/autostart/lumenyx.desktop"

# Helpers
HELPERS_DIR="$LUMENYX_DIR/helpers"
SUBSTRATE_SEND_PY="$HELPERS_DIR/substrate_send.py"
SUBSTRATE_DASH_PY="$HELPERS_DIR/substrate_dashboard.py"
SUBSTRATE_TX_PY="$HELPERS_DIR/substrate_tx.py"

# Download URLs
BINARY_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v${VERSION}/lumenyx-node"
CHECKSUM_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v${VERSION}/lumenyx-node-sha256.txt"
BOOTNODES_URL="https://raw.githubusercontent.com/lumenyx-chain/lumenyx/main/bootnodes.txt"

# ═══════════════════════════════════════════════════════════════════════════════
# AUTO-UPDATE CHECK
# ═══════════════════════════════════════════════════════════════════════════════

REMOTE_VERSION_URL="https://raw.githubusercontent.com/lumenyx-chain/lumenyx/main/lumenyx-setup.sh"

# Version comparison: returns 0 if $1 > $2
version_gt() {
    local v1="$1" v2="$2"
    # Split by dots
    local IFS='.'
    read -ra V1 <<< "$v1"
    read -ra V2 <<< "$v2"

    local i
    for ((i=0; i<${#V1[@]} || i<${#V2[@]}; i++)); do
        local n1="${V1[i]:-0}"
        local n2="${V2[i]:-0}"
        if ((n1 > n2)); then
            return 0
        elif ((n1 < n2)); then
            return 1
        fi
    done
    return 1  # Equal, not greater
}

check_for_updates() {
    local remote_version
    remote_version=$(curl -sL --connect-timeout 5 "$REMOTE_VERSION_URL" 2>/dev/null | grep '^SCRIPT_VERSION=' | cut -d'"' -f2)

    if [[ -z "$remote_version" ]]; then
        return 0
    fi

    if version_gt "$remote_version" "$SCRIPT_VERSION"; then
        clear
        print_logo
        echo -e "${YELLOW}╔════════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${YELLOW}║                    UPDATE AVAILABLE                                ║${NC}"
        echo -e "${YELLOW}╚════════════════════════════════════════════════════════════════════╝${NC}"
        echo ""
        echo -e "  Current version: ${RED}$SCRIPT_VERSION${NC}"
        echo -e "  New version:     ${GREEN}$remote_version${NC}"
        echo ""

        if ask_yes_no "Update to latest version?"; then
            print_info "Downloading update..."

            local script_path="$0"
            if curl -sL -o "${script_path}.new" "$REMOTE_VERSION_URL" 2>/dev/null; then
                mv "${script_path}.new" "$script_path"
                chmod +x "$script_path"
                print_ok "Updated to v$remote_version!"
                echo ""
                print_info "Restarting script..."
                sleep 1
                exec "$script_path" --updated
            else
                print_error "Update failed - continuing with current version"
                rm -f "${script_path}.new"
            fi
        else
            print_info "Skipping update..."
            sleep 1
        fi
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# BINARY VERSION CHECK (runs even when skipping clean install)
# ═══════════════════════════════════════════════════════════════════════════════

check_binary_update() {
    # Only check if binary exists
    if [[ ! -f "$LUMENYX_DIR/$BINARY_NAME" ]]; then
        return 0
    fi

    local current_version
    current_version=$("$LUMENYX_DIR/$BINARY_NAME" --version 2>/dev/null | awk '{print $2}' || echo "unknown")

    # If version matches, nothing to do
    if [[ "$current_version" == "$VERSION" ]]; then
        return 0
    fi

    # Version mismatch - need to update
    clear
    print_logo
    echo -e "${YELLOW}╔════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${YELLOW}║                  BINARY UPDATE AVAILABLE                           ║${NC}"
    echo -e "${YELLOW}╚════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "  Current binary: ${RED}v$current_version${NC}"
    echo -e "  Latest version: ${GREEN}v$VERSION${NC}"
    echo ""

    if ask_yes_no "Update binary to v$VERSION?"; then
        echo ""
        print_info "Downloading lumenyx-node v$VERSION (~65MB)..."
        echo ""

        # Stop node if running (to allow binary replacement)
        if node_running; then
            print_info "Stopping node for update..."
            stop_node
            sleep 2
        fi

        if curl -L -o "$LUMENYX_DIR/$BINARY_NAME" "$BINARY_URL" --progress-bar; then
            echo ""
            print_ok "Download complete"

            # Verify checksum
            print_info "Verifying checksum..."
            local expected actual
            expected=$(curl -sL "$CHECKSUM_URL" | grep -E "lumenyx-node" | awk '{print $1}' | head -1)
            actual=$(sha256sum "$LUMENYX_DIR/$BINARY_NAME" | awk '{print $1}')

            if [[ -n "$expected" ]] && [[ "$expected" == "$actual" ]]; then
                print_ok "Checksum verified"
            else
                print_warning "Checksum verification skipped/failed"
            fi

            chmod +x "$LUMENYX_DIR/$BINARY_NAME"
            print_ok "Binary updated to v$VERSION!"
            sleep 2
        else
            print_error "Download failed - continuing with current binary"
            sleep 2
        fi
    else
        print_info "Skipping binary update..."
        sleep 1
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# UI FUNCTIONS
# ═══════════════════════════════════════════════════════════════════════════════

print_logo() {
    echo -e "${VIOLET}"
    echo "╔════════════════════════════════════════════════════════════════════╗"
    echo "║                                                                    ║"
    echo -e "║   ${BRIGHT_VIOLET}██╗     ██╗   ██╗███╗   ███╗███████╗███╗   ██╗██╗   ██╗██╗  ██╗${VIOLET}  ║"
    echo -e "║   ${BRIGHT_VIOLET}██║     ██║   ██║████╗ ████║██╔════╝████╗  ██║╚██╗ ██╔╝╚██╗██╔╝${VIOLET}  ║"
    echo -e "║   ${PINK}██║     ██║   ██║██╔████╔██║█████╗  ██╔██╗ ██║ ╚████╔╝  ╚███╔╝ ${VIOLET}  ║"
    echo -e "║   ${PINK}██║     ██║   ██║██║╚██╔╝██║██╔══╝  ██║╚██╗██║  ╚██╔╝   ██╔██╗ ${VIOLET}  ║"
    echo -e "║   ${CYAN}███████╗╚██████╔╝██║ ╚═╝ ██║███████╗██║ ╚████║   ██║   ██╔╝ ██╗${VIOLET}  ║"
    echo -e "║   ${CYAN}╚══════╝ ╚═════╝ ╚═╝     ╚═╝╚══════╝╚═╝  ╚═══╝   ╚═╝   ╚═╝  ╚═╝${VIOLET}  ║"
    echo "║                                                                    ║"
    echo -e "╚════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

print_ok() { echo -e "${GREEN}✓${NC} $1"; }
print_error() { echo -e "${RED}✗${NC} $1"; }
print_warning() { echo -e "${YELLOW}!${NC} $1"; }
print_info() { echo -e "${CYAN}ℹ${NC} $1"; }

wait_enter() {
    echo ""
    read -r -p "Press ENTER to continue..."
}

ask_yes_no() {
    while true; do
        read -r -p "$1 [y/n]: " answer
        case $answer in
            [Yy]* ) return 0;;
            [Nn]* ) return 1;;
            * ) echo "Please answer y or n";;
        esac
    done
}

# ═══════════════════════════════════════════════════════════════════════════════
# MINING THREADS FUNCTIONS
# ═══════════════════════════════════════════════════════════════════════════════

get_threads() {
    if [[ -f "$THREADS_FILE" ]]; then
        cat "$THREADS_FILE" 2>/dev/null | tr -d ' \t\r\n'
    fi
}

get_threads_display() {
    local t
    t=$(get_threads)
    if [[ -z "$t" ]]; then
        echo "AUTO (all cores)"
    else
        echo "$t threads"
    fi
}

set_threads_menu() {
    local cores
    cores=$(command -v nproc >/dev/null 2>&1 && nproc || echo 1)
    
    echo ""
    echo -e "${CYAN}═══ SET MINING THREADS ═══${NC}"
    echo ""
    print_info "CPU cores detected: $cores"
    echo ""
    echo "  Current setting: $(get_threads_display)"
    echo ""
    echo "  Choose mining threads:"
    echo ""
    echo "    [0] Auto (use all $cores cores)"
    echo "    [1] 1 thread"
    echo "    [2] 2 threads"
    echo "    [4] 4 threads"
    echo "    [N] Custom number"
    echo ""
    read -r -p "  Selection (0/1/2/4/N): " sel

    local t=""
    case "$sel" in
        0) t="";;
        1) t="1";;
        2) t="2";;
        4) t="4";;
        [Nn])
            echo ""
            read -r -p "  Enter threads (1-$cores): " t
            ;;
        *)
            print_warning "Invalid choice. Keeping current setting."
            return
            ;;
    esac

    # Validate custom input
    if [[ -n "$t" ]]; then
        if ! echo "$t" | grep -Eq '^[0-9]+$'; then
            print_error "Threads must be a number."
            return
        fi
        if [[ "$t" -lt 1 ]]; then
            print_error "Threads must be >= 1."
            return
        fi
    fi

    # Save setting
    mkdir -p "$LUMENYX_DIR"
    echo -n "$t" > "$THREADS_FILE"
    
    echo ""
    if [[ -z "$t" ]]; then
        print_ok "Mining threads set to AUTO (all $cores cores)."
    else
        print_ok "Mining threads set to $t."
    fi

    # Offer to restart if node is running
    if node_running; then
        echo ""
        if ask_yes_no "Restart node to apply new thread setting?"; then
            stop_node
            sleep 2
            start_node
        else
            print_info "New setting will apply on next restart."
        fi
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# RPC FUNCTIONS (Robust with retries)
# ═══════════════════════════════════════════════════════════════════════════════

rpc_call() {
    local method="$1"
    local params="${2:-[]}"
    local result=""
    local attempt=1

    while [[ $attempt -le $RPC_RETRIES ]]; do
        result=$(curl -s -m $RPC_TIMEOUT -H "Content-Type: application/json" \
            -d "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params}" \
            "$RPC" 2>/dev/null)

        if [[ -n "$result" ]] && [[ "$result" != *"error"* ]]; then
            echo "$result"
            return 0
        fi

        attempt=$((attempt + 1))
        sleep 0.5
    done

    echo ""
    return 1
}

# Set pool mode via RPC (no restart needed)
rpc_set_pool_mode() {
    local enabled="$1"  # true or false
    local result
    result=$(curl -s -m 5 -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"lumenyx_setPoolMode\",\"params\":[$enabled]}" \
        "$RPC" 2>/dev/null)
    if [[ -n "$result" ]] && [[ "$result" == *"result"* ]]; then
        return 0
    fi
    return 1
}

# ═══════════════════════════════════════════════════════════════════════════════
# HELPERS (Python: send + dashboard)
# ═══════════════════════════════════════════════════════════════════════════════

ensure_helpers() {
    mkdir -p "$HELPERS_DIR"

    # SEND helper
    cat > "$SUBSTRATE_SEND_PY" <<'PY'
#!/usr/bin/env python3
import argparse, json, os, sys

SEED_FILE = os.path.expanduser("~/.local/share/lumenyx-node/miner-key")

def read_seed_hex():
    if not os.path.exists(SEED_FILE):
        raise SystemExit("miner-key not found: " + SEED_FILE)
    s = open(SEED_FILE, "r").read().strip().lower()
    s = s[2:] if s.startswith("0x") else s
    if len(s) != 64:
        raise SystemExit("miner-key must be 32 bytes hex (64 chars), without 0x")
    int(s, 16)
    return s

def amount_to_planck(amount_str, decimals=18):
    s = amount_str.strip().replace(",", ".")
    if s.count(".") > 1:
        raise SystemExit("Invalid amount")
    if "." in s:
        a, b = s.split(".", 1)
        b = (b + "0" * decimals)[:decimals]
        return int(a) * (10 ** decimals) + int(b)
    return int(s) * (10 ** decimals)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ws", required=True)
    ap.add_argument("--to", required=True)
    ap.add_argument("--amount", required=True)
    ap.add_argument("--decimals", type=int, default=18)
    ap.add_argument("--wait", choices=["none","inclusion","finalization"], default="inclusion")
    args = ap.parse_args()

    seed_hex = read_seed_hex()
    value = amount_to_planck(args.amount, args.decimals)

    try:
        from substrateinterface import SubstrateInterface, Keypair, KeypairType
        from substrateinterface.exceptions import SubstrateRequestException
    except Exception:
        raise SystemExit("Missing dependency: substrate-interface. Install with: pip3 install --user substrate-interface")

    substrate = SubstrateInterface(url=args.ws)
    kp = Keypair.create_from_seed(bytes.fromhex(seed_hex), crypto_type=KeypairType.SR25519)

    call = substrate.compose_call(
        call_module="Balances",
        call_function="transfer_keep_alive",
        call_params={"dest": args.to, "value": value},
    )

    extrinsic = substrate.create_signed_extrinsic(call=call, keypair=kp)

    try:
        if args.wait == "finalization":
            receipt = substrate.submit_extrinsic(extrinsic, wait_for_finalization=True)
        elif args.wait == "inclusion":
            receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
        else:
            receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=False)
    except SubstrateRequestException as e:
        print(json.dumps({"ok": False, "error": str(e)}))
        sys.exit(1)

    out = {
        "ok": bool(getattr(receipt, "is_success", True)),
        "hash": getattr(receipt, "extrinsic_hash", None),
        "block_hash": getattr(receipt, "block_hash", None),
        "error": (getattr(receipt, "error_message", None) if not getattr(receipt, "is_success", True) else None),
    }
    print(json.dumps(out))

if __name__ == "__main__":
    main()
PY
    chmod +x "$SUBSTRATE_SEND_PY"

    # DASH helper (balance + block + peers) via metadata
    cat > "$SUBSTRATE_DASH_PY" <<'PY'
#!/usr/bin/env python3
import argparse, json, os, sys

WALLET_FILE = os.path.expanduser("~/.lumenyx/wallet.txt")
SEED_FILE = os.path.expanduser("~/.local/share/lumenyx-node/miner-key")

def read_address():
    if os.path.exists(WALLET_FILE):
        for line in open(WALLET_FILE, "r"):
            line = line.strip()
            if line.startswith("Address:"):
                parts = line.split()
                if len(parts) >= 2:
                    return parts[1].strip()
    return None

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ws", required=True)
    ap.add_argument("--mode", choices=["balance","block","peers"], required=True)
    ap.add_argument("--decimals", type=int, default=18)
    args = ap.parse_args()

    try:
        from substrateinterface import SubstrateInterface
    except Exception:
        print(json.dumps({"ok": False, "error": "Missing dependency: substrate-interface"}))
        sys.exit(2)

    try:
        substrate = SubstrateInterface(url=args.ws)
    except Exception as e:
        print(json.dumps({"ok": False, "error": "Connect failed: " + str(e)}))
        sys.exit(1)

    if args.mode == "balance":
        addr = read_address()
        if not addr:
            print(json.dumps({"ok": False, "error": "No address (wallet.txt missing?)"}))
            sys.exit(1)

        try:
            account_info = substrate.query("System", "Account", [addr])
            free = int(account_info.value["data"]["free"])
        except Exception as e:
            print(json.dumps({"ok": False, "error": "Balance query failed: " + str(e)}))
            sys.exit(1)

        human = free / (10 ** args.decimals)
        print(json.dumps({"ok": True, "free_planck": free, "free": human}))
        return

    if args.mode == "block":
        try:
            hdr = substrate.rpc_request("chain_getHeader", [])
            n_hex = hdr.get("result", {}).get("number")
            if not n_hex:
                raise Exception("Missing header number")
            best = int(n_hex, 16)
            # Get sync state for target block
            target = best
            syncing = False
            try:
                sync = substrate.rpc_request("system_syncState", [])
                if sync.get("result"):
                    current = sync["result"].get("currentBlock", best)
                    highest = sync["result"].get("highestBlock", best)
                    if highest > current:
                        target = highest
                        syncing = True
            except:
                pass
            print(json.dumps({"ok": True, "best": best, "target": target, "syncing": syncing}))
        except Exception as e:
            print(json.dumps({"ok": False, "error": "Header failed: " + str(e)}))
            sys.exit(1)
        return

    if args.mode == "peers":
        try:
            h = substrate.rpc_request("system_health", [])
            peers = int(h.get("result", {}).get("peers", 0))
            print(json.dumps({"ok": True, "peers": peers}))
        except Exception as e:
            print(json.dumps({"ok": False, "error": "system_health failed: " + str(e)}))
            sys.exit(1)
        return

if __name__ == "__main__":
    main()
PY
    chmod +x "$SUBSTRATE_DASH_PY"

    # TX HISTORY helper
    cat > "$SUBSTRATE_TX_PY" <<'PY'
#!/usr/bin/env python3
import argparse, json, os, sys

WALLET_FILE = os.path.expanduser("~/.lumenyx/wallet.txt")

def read_address():
    if os.path.exists(WALLET_FILE):
        for line in open(WALLET_FILE, "r"):
            line = line.strip()
            if line.startswith("Address:"):
                parts = line.split()
                if len(parts) >= 2:
                    return parts[1].strip()
    return None

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ws", required=True)
    ap.add_argument("--blocks", type=int, default=0)
    ap.add_argument("--decimals", type=int, default=18)
    args = ap.parse_args()

    try:
        from substrateinterface import SubstrateInterface
    except Exception:
        print(json.dumps({"ok": False, "error": "Missing substrate-interface"}))
        sys.exit(2)

    addr = read_address()
    if not addr:
        print(json.dumps({"ok": False, "error": "No wallet address found"}))
        sys.exit(1)

    try:
        substrate = SubstrateInterface(url=args.ws)
    except Exception as e:
        print(json.dumps({"ok": False, "error": "Connect failed: " + str(e)}))
        sys.exit(1)

    try:
        head = substrate.get_block()
        current_block = head['header']['number']

        transactions = []
        start_block = 1 if args.blocks == 0 else max(1, current_block - args.blocks)

        for block_num in range(current_block, start_block - 1, -1):
            try:
                block_hash = substrate.get_block_hash(block_num)
                events = substrate.get_events(block_hash)

                for event in events:
                    if event.value.get('event_id') == 'Transfer' and event.value.get('module_id') == 'Balances':
                        attrs = event.value.get('attributes', {})
                        if isinstance(attrs, dict):
                            from_addr = str(attrs.get('from', ''))
                            to_addr = str(attrs.get('to', ''))
                            amount = int(attrs.get('amount', 0))
                        else:
                            continue

                        if from_addr == addr or to_addr == addr:
                            tx_type = "SENT" if from_addr == addr else "RECV"
                            human_amount = amount / (10 ** args.decimals)
                            transactions.append({
                                "block": block_num,
                                "type": tx_type,
                                "amount": human_amount,
                                "from": from_addr[:8] + "..." + from_addr[-6:],
                                "to": to_addr[:8] + "..." + to_addr[-6:]
                            })
            except Exception:
                continue

            if len(transactions) >= 20:
                break

        print(json.dumps({"ok": True, "transactions": transactions}))

    except Exception as e:
        print(json.dumps({"ok": False, "error": str(e)}))
        sys.exit(1)

if __name__ == "__main__":
    main()
PY
    chmod +x "$SUBSTRATE_TX_PY"
}

ensure_python_deps() {
    if ! command -v python3 >/dev/null 2>&1; then
        print_error "python3 is required for dashboard + send"
        return 1
    fi

    local need_install=false

    if ! python3 -c 'import substrateinterface' >/dev/null 2>&1; then
        need_install=true
    fi

    if ! python3 -c 'from eth_account import Account' >/dev/null 2>&1; then
        need_install=true
    fi

    if $need_install; then
        print_info "Installing Python dependencies..."
        python3 -m pip install --user substrate-interface eth-account >/dev/null 2>&1 || \
        python3 -m pip install --break-system-packages substrate-interface eth-account >/dev/null 2>&1 || {
            print_error "Failed to install Python dependencies. Run: pip3 install --break-system-packages substrate-interface eth-account"
            return 1
        }
    fi
    return 0
}

# ═══════════════════════════════════════════════════════════════════════════════
# UTILITY FUNCTIONS
# ═══════════════════════════════════════════════════════════════════════════════

node_running() {
    # Check systemd first if daemon mode is enabled
    if systemctl is-active lumenyx.service >/dev/null 2>&1; then
        return 0
    fi
    
    # Check PID file
    if [[ -f "$PID_FILE" ]]; then
        local pid
        pid=$(cat "$PID_FILE" 2>/dev/null)
        if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
            return 0
        fi
    fi
    pgrep -f "lumenyx-node" > /dev/null 2>&1
}

get_address() {
    if [[ -f "$LUMENYX_DIR/wallet.txt" ]]; then
        grep "Address:" "$LUMENYX_DIR/wallet.txt" 2>/dev/null | awk '{print $2}'
    elif [[ -f "$DATA_DIR/miner-key" ]]; then
        local seed addr
        seed=$(cat "$DATA_DIR/miner-key" 2>/dev/null)
        if [[ -n "$seed" ]] && [[ -f "$LUMENYX_DIR/$BINARY_NAME" ]]; then
            addr=$("$LUMENYX_DIR/$BINARY_NAME" key inspect "0x$seed" 2>/dev/null | grep "SS58 Address:" | awk '{print $3}')
            if [[ -n "$addr" ]]; then
                # Auto-create wallet.txt so balance works
                echo "Address: $addr" > "$LUMENYX_DIR/wallet.txt"
                echo "$addr"
            fi
        fi
    fi
}

derive_evm_from_mnemonic() {
    # Derive real EVM address from BIP39 mnemonic using BIP44 path (m/44'/60'/0'/0/0)
    # This produces the same address MetaMask would generate from the same seed phrase
    local mnemonic="$1"
    python3 -c "
from eth_account import Account
Account.enable_unaudited_hdwallet_features()
acct = Account.from_mnemonic('$mnemonic', account_path=\"m/44'/60'/0'/0/0\")
print(acct.address)
print(acct.key.hex())
" 2>/dev/null
}

get_evm_address() {
    # Return cached EVM address if available
    if [[ -f "$LUMENYX_DIR/wallet_evm.txt" ]]; then
        local cached
        cached=$(cat "$LUMENYX_DIR/wallet_evm.txt" 2>/dev/null)
        # Validate it's a real address (not the old fake derivation)
        if [[ -n "$cached" ]] && [[ -f "$LUMENYX_DIR/evm-key" ]]; then
            echo "$cached"
            return
        fi
        # Old fake address or missing evm-key — need re-derivation
        rm -f "$LUMENYX_DIR/wallet_evm.txt" 2>/dev/null
    fi
    echo ""
}

get_evm_balance() {
    if ! node_running; then
        echo "offline"
        return
    fi

    local evm_addr
    evm_addr=$(get_evm_address)
    if [[ -z "$evm_addr" ]]; then
        echo "0.000"
        return
    fi

    ensure_helpers
    ensure_python_deps >/dev/null || { echo "offline"; return; }

    local out dec
    dec=$(get_decimals)
    out=$(python3 -c "
import json, sys
try:
    from substrateinterface import SubstrateInterface
    substrate = SubstrateInterface(url='$WS')
    result = substrate.rpc_request('eth_getBalance', ['$evm_addr', 'latest'])
    bal_hex = result.get('result', '0x0')
    bal = int(bal_hex, 16)
    human = bal / (10 ** $dec)
    print(json.dumps({'ok': True, 'balance': human}))
except Exception as e:
    print(json.dumps({'ok': False, 'error': str(e)}))
" 2>/dev/null || true)

    local ok bal
    ok=$(echo "$out" | python3 -c 'import sys,json; d=json.load(sys.stdin); print(d.get("ok",False))' 2>/dev/null || echo "False")
    if [[ "$ok" == "True" ]]; then
        bal=$(echo "$out" | python3 -c 'import sys,json; d=json.load(sys.stdin); print("{:.3f}".format(d.get("balance",0)))' 2>/dev/null || echo "0.000")
        echo "$bal"
    else
        echo "0.000"
    fi
}

# Fork height for decimal migration (12 → 18 decimals)
FORK_HEIGHT=450000

# Get current decimals based on best block number
# Before block 450,000: 12 decimals (LUMENYX era)
# After block 450,000: 18 decimals (LUMO era)
get_decimals() {
    local best
    best=$(curl -s -m 2 -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
        http://127.0.0.1:9944 2>/dev/null | python3 -c 'import sys,json; h=json.load(sys.stdin).get("result",{}).get("number","0x0"); print(int(h,16))' 2>/dev/null || echo "0")
    if [[ "$best" -ge "$FORK_HEIGHT" ]]; then
        echo "18"
    else
        echo "12"
    fi
}

get_balance() {
    if ! node_running; then
        echo "offline"
        return
    fi

    ensure_helpers
    ensure_python_deps >/dev/null || { echo "offline"; return; }

    local out ok free
    local dec=$(get_decimals)
    out=$(python3 "$SUBSTRATE_DASH_PY" --ws "$WS" --mode balance --decimals "$dec" 2>/dev/null || true)
    ok=$(echo "$out" | grep -o '"ok": *[^,]*' | cut -d':' -f2 | tr -d ' }')

    if [[ "$ok" == "true" ]]; then
        free=$(echo "$out" | python3 -c 'import sys,json; d=json.load(sys.stdin); print("{:.3f}".format(d.get("free",0)))' 2>/dev/null || echo "")
        if [[ -n "$free" ]]; then
            echo "$free"
            return
        fi
    fi

    echo "offline"
}

get_block() {
    if ! node_running; then
        echo "offline|0|false"
        return
    fi

    ensure_helpers
    ensure_python_deps >/dev/null || { echo "offline|0|false"; return; }

    local out ok best target syncing
    out=$(python3 "$SUBSTRATE_DASH_PY" --ws "$WS" --mode block 2>/dev/null || true)
    ok=$(echo "$out" | grep -o '"ok": *[^,]*' | cut -d':' -f2 | tr -d ' }')
    if [[ "$ok" == "true" ]]; then
        best=$(echo "$out" | python3 -c 'import sys,json; d=json.load(sys.stdin); print(d.get("best",""))' 2>/dev/null || true)
        target=$(echo "$out" | python3 -c 'import sys,json; d=json.load(sys.stdin); print(d.get("target",""))' 2>/dev/null || true)
        syncing=$(echo "$out" | python3 -c 'import sys,json; d=json.load(sys.stdin); print(d.get("syncing",False))' 2>/dev/null || true)
        [[ -n "$best" ]] && { echo "$best|${target:-$best}|${syncing:-false}"; return; }
    fi
    echo "offline|0|false"
}

get_peers() {
    if ! node_running; then
        echo "0"
        return
    fi

    ensure_helpers
    ensure_python_deps >/dev/null || { echo "0"; return; }

    local out ok peers
    out=$(python3 "$SUBSTRATE_DASH_PY" --ws "$WS" --mode peers 2>/dev/null || true)
    ok=$(echo "$out" | grep -o '"ok": *[^,]*' | cut -d':' -f2 | tr -d ' }')
    if [[ "$ok" == "true" ]]; then
        peers=$(echo "$out" | python3 -c 'import sys,json; d=json.load(sys.stdin); print(d.get("peers",0))' 2>/dev/null || echo "0")
        echo "${peers:-0}"
        return
    fi

    echo "0"
}

get_bootnodes() {
    echo "" >&2
    echo -e "${CYAN}═══ BOOTNODE SETUP ═══${NC}" >&2
    echo "" >&2
    echo "  To connect to the network, you need a bootnode address." >&2
    echo "  Get it from someone already running LUMENYX." >&2
    echo "" >&2
    echo "  Format: /ip4/IP/tcp/30333/p2p/PEER_ID" >&2
    echo "" >&2
    read -r -p "Paste bootnode address (or ENTER to skip): " manual
    if [[ -n "$manual" ]]; then
        echo "$manual"
    fi
}

has_existing_data() {
    [[ -d "$LUMENYX_DIR" ]] || [[ -d "$DATA_DIR" ]] || pgrep -f "lumenyx-node" > /dev/null 2>&1 || systemctl is-active --quiet lumenyx 2>/dev/null
}

# ═══════════════════════════════════════════════════════════════════════════════
# CLEAN INSTALL
# ═══════════════════════════════════════════════════════════════════════════════

prompt_clean_install() {
    clear
    print_logo
    echo -e "${YELLOW}╔════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${YELLOW}║                  EXISTING DATA DETECTED                            ║${NC}"
    echo -e "${YELLOW}╚════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "  Found existing LUMENYX on this machine:"
    echo ""
    [[ -d "$LUMENYX_DIR" ]] && echo -e "    ${CYAN}•${NC} $LUMENYX_DIR (binary, config, logs)"
    [[ -d "$DATA_DIR" ]] && echo -e "    ${CYAN}•${NC} $DATA_DIR (blockchain data, wallet)"
    pgrep -f "lumenyx-node" > /dev/null 2>&1 && echo -e "    ${RED}•${NC} lumenyx-node process is RUNNING"
    systemctl is-active --quiet lumenyx 2>/dev/null && echo -e "    ${RED}•${NC} systemd service is ACTIVE"
    echo ""
    echo -e "  ${GREEN}RECOMMENDED:${NC} Clean install for best experience"
    echo ""
    echo -e "${RED}⚠️  WARNING: This will delete your existing wallet!${NC}"
    echo -e "${RED}   Make sure you have saved your seed phrase!${NC}"
    echo ""

    if ask_yes_no "Perform clean install?"; then
        print_info "Cleaning existing data..."

        if systemctl is-active --quiet lumenyx 2>/dev/null; then
            print_info "Stopping systemd service..."
            systemctl stop lumenyx 2>/dev/null || true
            systemctl disable lumenyx 2>/dev/null || true
            rm -f /etc/systemd/system/lumenyx.service 2>/dev/null || true
            systemctl daemon-reload 2>/dev/null || true
            sleep 1
        fi

        if pgrep -f "lumenyx-node" > /dev/null 2>&1; then
            print_info "Stopping running node..."
            pkill -TERM -f "lumenyx-node" 2>/dev/null || true
            sleep 2
            pkill -KILL -f "lumenyx-node" 2>/dev/null || true
            sleep 1
        fi

        rm -f "$PID_FILE" 2>/dev/null

        if pgrep -f "lumenyx-node" > /dev/null 2>&1; then
            print_error "Could not stop node. Please run: pkill -9 -f lumenyx-node"
            wait_enter
            return 1
        fi

        rm -rf "$LUMENYX_DIR" "$DATA_DIR"
        rm -f "$SYNC_SAFE_FILE" 2>/dev/null
        print_ok "Clean install complete!"
        sleep 1
        return 0
    else
        print_info "Keeping existing data..."
        sleep 1
        return 1
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# FIRST RUN - INSTALLATION
# ═══════════════════════════════════════════════════════════════════════════════

is_first_run() {
    [[ ! -f "$LUMENYX_DIR/$BINARY_NAME" ]] || [[ ! -f "$DATA_DIR/miner-key" ]]
}

step_welcome() {
    clear
    print_logo
    echo -e "${BOLD}                    Welcome to LUMENYX${NC}"
    echo ""
    echo -e "  ${CYAN}\"Bitcoin started with a headline. Ethereum started with a premine.${NC}"
    echo -e "  ${CYAN} LUMENYX starts with you.\"${NC}"
    echo ""
    echo "  This script will:"
    echo ""
    echo -e "    ${GREEN}1.${NC} Check your system"
    echo -e "    ${GREEN}2.${NC} Download LUMENYX node"
    echo -e "    ${GREEN}3.${NC} Create your wallet"
    echo -e "    ${GREEN}4.${NC} Start mining"
    echo ""
    echo -e "  ${CYAN}No root/sudo required!${NC}"
    echo ""
    wait_enter
}

step_system_check() {
    clear
    print_logo
    echo -e "${CYAN}═══ STEP 1: SYSTEM CHECK ═══${NC}"
    echo ""

    local errors=0

    if [[ "$(uname -s)" == "Linux" ]]; then
        print_ok "Operating system: Linux"
    else
        print_error "Linux required!"
        errors=$((errors + 1))
    fi

    if [[ "$(uname -m)" == "x86_64" ]]; then
        print_ok "Architecture: x86_64"
    else
        print_error "x86_64 required"
        errors=$((errors + 1))
    fi

    command -v curl >/dev/null 2>&1 && print_ok "curl: installed" || { print_error "curl not found"; errors=$((errors + 1)); }
    command -v python3 >/dev/null 2>&1 && print_ok "python3: installed (dashboard + send)" || print_warning "python3 not found (dashboard + send will not work)"

    if curl -s --connect-timeout 5 https://github.com > /dev/null 2>&1; then
        print_ok "Internet: OK"
    else
        print_error "Cannot reach GitHub"
        errors=$((errors + 1))
    fi

    local available
    available=$(df -BG "$HOME" 2>/dev/null | awk 'NR==2 {print $4}' | tr -d 'G')
    if [[ "$available" -ge 1 ]] 2>/dev/null; then
        print_ok "Disk space: ${available}GB available"
    else
        print_error "Disk space check failed"
        errors=$((errors + 1))
    fi

    if [[ $errors -gt 0 ]]; then
        echo ""
        print_error "Fix the issues above before continuing."
        exit 1
    fi

    echo ""
    print_ok "System check passed!"
    wait_enter
}

step_install() {
    clear
    print_logo
    echo -e "${CYAN}═══ STEP 2: INSTALLATION ═══${NC}"
    echo ""

    mkdir -p "$LUMENYX_DIR"

    # Check if binary exists and verify version
    if [[ -f "$LUMENYX_DIR/$BINARY_NAME" ]]; then
        local current_version
        current_version=$("$LUMENYX_DIR/$BINARY_NAME" --version 2>/dev/null | awk '{print $2}' || echo "unknown")
        
        if [[ "$current_version" == "$VERSION" ]]; then
            print_ok "Binary v$VERSION already installed"
            wait_enter
            return
        else
            echo -e "${YELLOW}Binary update needed: v$current_version → v$VERSION${NC}"
            echo ""
            print_info "Downloading lumenyx-node v$VERSION (~65MB)..."
        fi
    else
        print_info "Downloading lumenyx-node v$VERSION (~65MB)..."
    fi
    
    echo ""

    if curl -L -o "$LUMENYX_DIR/$BINARY_NAME" "$BINARY_URL" --progress-bar; then
        echo ""
        print_ok "Download complete"
    else
        print_error "Download failed"
        exit 1
    fi

    print_info "Verifying checksum..."
    local expected actual
    expected=$(curl -sL "$CHECKSUM_URL" | grep -E "lumenyx-node" | awk '{print $1}' | head -1)
    actual=$(sha256sum "$LUMENYX_DIR/$BINARY_NAME" | awk '{print $1}')

    if [[ -n "$expected" ]] && [[ "$expected" == "$actual" ]]; then
        print_ok "Checksum verified"
    else
        print_warning "Checksum verification skipped/failed"
    fi

    chmod +x "$LUMENYX_DIR/$BINARY_NAME"
    print_ok "Binary ready: $LUMENYX_DIR/$BINARY_NAME (v$VERSION)"
    wait_enter
}

step_wallet() {
    clear
    print_logo
    echo -e "${CYAN}═══ STEP 3: WALLET ═══${NC}"
    echo ""

    # Install Python deps first (needed for EVM wallet derivation)
    ensure_python_deps >/dev/null 2>&1 || true

    if [[ -f "$DATA_DIR/miner-key" ]]; then
        print_ok "Wallet already exists"
        local addr
        addr=$(get_address)
        if [[ -n "$addr" ]]; then
            echo ""
            echo -e "  Your address: ${GREEN}$addr${NC}"
        fi
        wait_enter
        return
    fi

    echo -e "${RED}╔════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}║  ⚠️  IMPORTANT: Write down the 12-word seed phrase!                ║${NC}"
    echo -e "${RED}║     If you lose it, your funds are LOST FOREVER.                  ║${NC}"
    echo -e "${RED}╚════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    if ask_yes_no "Create NEW wallet?"; then
        echo ""
        print_info "Generating wallet..."

        local output seed_phrase address secret_seed
        output=$("$LUMENYX_DIR/$BINARY_NAME" key generate --words 12 2>&1)

        seed_phrase=$(echo "$output" | grep "Secret phrase:" | sed 's/.*Secret phrase:[[:space:]]*//')
        address=$(echo "$output" | grep "SS58 Address:" | sed 's/.*SS58 Address:[[:space:]]*//')
        secret_seed=$(echo "$output" | grep "Secret seed:" | sed 's/.*Secret seed:[[:space:]]*//' | sed 's/0x//')

        # Derive real EVM address from mnemonic (BIP44 - same as MetaMask)
        local evm_output evm_address evm_privkey
        evm_output=$(derive_evm_from_mnemonic "$seed_phrase" || true)
        evm_address=$(echo "$evm_output" | head -1)
        evm_privkey=$(echo "$evm_output" | tail -1)

        echo ""
        echo -e "${YELLOW}╔════════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${YELLOW}║  YOUR SEED PHRASE (write it down NOW!):                            ║${NC}"
        echo -e "${YELLOW}╚════════════════════════════════════════════════════════════════════╝${NC}"
        echo ""
        echo -e "  ${GREEN}${BOLD}$seed_phrase${NC}"
        echo ""
        echo -e "  SS58 (mining):   ${CYAN}$address${NC}"
        if [[ -n "$evm_address" ]]; then
            echo -e "  EVM  (MetaMask): ${CYAN}$evm_address${NC}"
        fi
        echo ""
        echo -e "  ${YELLOW}→ Import the same 12 words in MetaMask to use the DEX${NC}"
        echo ""

        mkdir -p "$DATA_DIR"
        echo "$secret_seed" > "$DATA_DIR/miner-key"
        chmod 600 "$DATA_DIR/miner-key"

        echo "Address: $address" > "$LUMENYX_DIR/wallet.txt"

        # Save EVM wallet
        if [[ -n "$evm_address" ]]; then
            echo "$evm_address" > "$LUMENYX_DIR/wallet_evm.txt"
            echo "$evm_privkey" > "$LUMENYX_DIR/evm-key"
            chmod 600 "$LUMENYX_DIR/evm-key"
        fi

        echo ""
        read -r -p "Type YES when you have saved your seed phrase: " confirm
        if [[ "$confirm" != "YES" ]]; then
            echo ""
            print_warning "Please make sure to save your seed phrase!"
        fi

        print_ok "Wallet created!"
    else
        echo ""
        read -r -p "Enter your 12-word seed phrase: " seed_phrase

        local output address secret_seed
        output=$("$LUMENYX_DIR/$BINARY_NAME" key inspect "$seed_phrase" 2>&1)
        address=$(echo "$output" | grep "SS58 Address:" | sed 's/.*SS58 Address:[[:space:]]*//')
        secret_seed=$(echo "$output" | grep "Secret seed:" | sed 's/.*Secret seed:[[:space:]]*//' | sed 's/0x//')

        if [[ -z "$address" ]]; then
            print_error "Invalid seed phrase"
            exit 1
        fi

        # Derive real EVM address from mnemonic (BIP44 - same as MetaMask)
        local evm_output evm_address evm_privkey
        evm_output=$(derive_evm_from_mnemonic "$seed_phrase" || true)
        evm_address=$(echo "$evm_output" | head -1)
        evm_privkey=$(echo "$evm_output" | tail -1)

        mkdir -p "$DATA_DIR"
        echo "$secret_seed" > "$DATA_DIR/miner-key"
        chmod 600 "$DATA_DIR/miner-key"

        echo "Address: $address" > "$LUMENYX_DIR/wallet.txt"

        # Save EVM wallet
        if [[ -n "$evm_address" ]]; then
            echo "$evm_address" > "$LUMENYX_DIR/wallet_evm.txt"
            echo "$evm_privkey" > "$LUMENYX_DIR/evm-key"
            chmod 600 "$LUMENYX_DIR/evm-key"
        fi

        echo ""
        echo -e "  SS58 (mining):   ${GREEN}$address${NC}"
        if [[ -n "$evm_address" ]]; then
            echo -e "  EVM  (MetaMask): ${GREEN}$evm_address${NC}"
        fi
        print_ok "Wallet imported!"
    fi

    wait_enter
}

step_start() {
    clear
    print_logo
    echo -e "${CYAN}═══ STEP 4: START MINING ═══${NC}"
    echo ""

    print_info "Fetching bootnodes..."
    BOOTNODES=$(get_bootnodes)

    if [[ -z "$BOOTNODES" ]]; then
        print_warning "No bootnodes - node will wait for connections"
    else
        print_ok "Bootnodes configured"
    fi

    echo ""
    if ask_yes_no "Start mining now?"; then
        start_node
    else
        print_info "You can start mining later from the menu"
    fi

    wait_enter
}

first_run() {
    step_welcome
    step_system_check
    step_install
    step_wallet
    step_start

    clear
    print_logo
    echo ""
    print_ok "Setup complete!"
    echo ""
    echo -e "  ${CYAN}Entering dashboard...${NC}"
    sleep 2
}

# ═══════════════════════════════════════════════════════════════════════════════
# NODE CONTROL
# ═══════════════════════════════════════════════════════════════════════════════

## Sync-safe mode: detect if node needs initial sync
## Mining during sync causes AnnouncePin worker freeze + DB corruption (BUG 1)
## Solution: start without --validator during sync, restart with mining once synced

SYNC_SAFE_FILE="$LUMENYX_DIR/sync_complete"

is_sync_complete() {
    [[ -f "$SYNC_SAFE_FILE" ]]
}

check_if_synced() {
    # Quick check via RPC if node is synced
    local result
    result=$(curl -s -m 3 -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"system_syncState","params":[],"id":1}' \
        http://127.0.0.1:9944 2>/dev/null || true)
    
    if [[ -z "$result" ]]; then
        return 1
    fi
    
    local current highest
    current=$(echo "$result" | python3 -c 'import sys,json; d=json.load(sys.stdin).get("result",{}); print(d.get("currentBlock",0))' 2>/dev/null || echo "0")
    highest=$(echo "$result" | python3 -c 'import sys,json; d=json.load(sys.stdin).get("result",{}); print(d.get("highestBlock",0))' 2>/dev/null || echo "0")
    
    # Synced if within 50 blocks of highest (allow small gap)
    if [[ "$highest" -gt 0 ]] && [[ "$current" -gt 0 ]]; then
        local gap=$((highest - current))
        if [[ "$gap" -le 50 ]]; then
            return 0
        fi
    fi
    return 1
}

mark_sync_complete() {
    echo "$(date -Iseconds)" > "$SYNC_SAFE_FILE"
}

build_bootnode_args() {
    local bootnode_args="" bootnodes=""
    
    if [[ -n "${BOOTNODES:-}" ]]; then
        bootnodes="$BOOTNODES"
    elif [[ -f "$LUMENYX_DIR/bootnodes.conf" ]]; then
        bootnodes=$(cat "$LUMENYX_DIR/bootnodes.conf" 2>/dev/null)
    else
        bootnodes=$(curl -sL "$BOOTNODES_URL" 2>/dev/null | grep -v '^#' | grep -v '^$' | tr '\n' ' ')
    fi
    
    if [[ -n "$bootnodes" ]]; then
        echo "$bootnodes" > "$LUMENYX_DIR/bootnodes.conf"
        for bn in $bootnodes; do
            bootnode_args="$bootnode_args --bootnodes $bn --reserved-nodes $bn"
        done
    fi
    
    echo "$bootnode_args"
}

start_node() {
    if node_running; then
        print_warning "Node is already running"
        return
    fi

    local bootnode_args
    bootnode_args=$(build_bootnode_args)

    # Ensure log file exists
    mkdir -p "$LUMENYX_DIR"
    touch "$LOG_FILE"

    # Determine if we need sync-safe mode (no mining during sync)
    local use_validator=true
    if ! is_sync_complete; then
        # Check if DB has substantial data (if not, it's a fresh sync)
        local db_size=0
        if [[ -d "$DATA_DIR/chains" ]]; then
            db_size=$(du -sm "$DATA_DIR/chains" 2>/dev/null | awk '{print $1}' || echo "0")
        fi
        # Fresh or small DB = needs full sync = no mining
        # Large DB but not marked complete = possibly corrupted, sync again safely
        use_validator=false
        print_warning "Sync-safe mode: syncing WITHOUT mining to prevent freeze"
        print_info "Mining will start automatically once sync is complete"
    fi

    # Get mining threads setting
    local threads
    threads=$(get_threads)
    
    if $use_validator; then
        if [[ -n "$threads" ]]; then
            export LUMENYX_MINING_THREADS="$threads"
            print_info "Mining with $threads thread(s)"
        else
            unset LUMENYX_MINING_THREADS
            local cores
            cores=$(command -v nproc >/dev/null 2>&1 && nproc || echo "?")
            print_info "Mining with AUTO threads (all $cores cores)"
        fi
    fi

    print_info "Starting node..."

    local validator_flag=""
    local pool_flag=""
    if $use_validator; then
        validator_flag="--validator"
        if pool_is_enabled; then
            pool_flag="--pool-mode"
        fi
    fi

    nohup "$LUMENYX_DIR/$BINARY_NAME" \
        --base-path "$DATA_DIR" \
        --chain mainnet \
        $validator_flag \
        $pool_flag \
        --rpc-cors all \
        --unsafe-rpc-external \
        --rpc-methods Unsafe \
        --state-pruning 250000 \
        --blocks-pruning 250000 \
        $bootnode_args \
        >> "$LOG_FILE" 2>&1 &

    echo $! > "$PID_FILE"
    disown

    sleep 3

    if node_running; then
        if $use_validator; then
            print_ok "Mining started! (PID: $(cat "$PID_FILE"))"
        else
            print_ok "Syncing started! (PID: $(cat "$PID_FILE"))"
            print_info "Node will switch to mining mode once fully synced"
        fi
    else
        print_error "Failed to start - check: tail -50 $LOG_FILE"
    fi
}

## Background sync monitor: when sync completes, restart with mining
check_sync_and_upgrade() {
    # Only relevant if sync not yet marked complete
    if is_sync_complete; then
        return
    fi
    
    # Only check if node is running
    if ! node_running; then
        return
    fi
    
    if check_if_synced; then
        mark_sync_complete
        print_ok "Sync complete! Restarting with mining enabled..."
        stop_node
        sleep 3
        start_node
    fi
}

stop_node() {
    if ! node_running; then
        print_warning "Node is not running"
        return
    fi

    print_info "Stopping node..."

    if [[ -f "$PID_FILE" ]]; then
        local pid
        pid=$(cat "$PID_FILE")
        if [[ -n "$pid" ]]; then
            kill -TERM "$pid" 2>/dev/null || true
            sleep 1
            kill -KILL "$pid" 2>/dev/null || true
        fi
        rm -f "$PID_FILE"
    fi

    pkill -TERM -f "lumenyx-node" 2>/dev/null || true
    sleep 1
    pkill -KILL -f "lumenyx-node" 2>/dev/null || true

    sleep 1

    if ! node_running; then
        print_ok "Node stopped"
    else
        print_error "Failed to stop node - try: pkill -9 -f lumenyx-node"
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# DASHBOARD (Auto-refresh)
# ═══════════════════════════════════════════════════════════════════════════════


# ═══════════════════════════════════════════════════════════════════════════════
# POOL MODE FUNCTIONS
# ═══════════════════════════════════════════════════════════════════════════════

POOL_CONF="$LUMENYX_DIR/pool.conf"

pool_is_enabled() {
    [ -f "$POOL_CONF" ] && grep -q '^POOL_MODE=1' "$POOL_CONF"
}

pool_enable() {
    mkdir -p "$LUMENYX_DIR"
    cat > "$POOL_CONF" <<EOF
# LUMO Pool Configuration
POOL_MODE=1
EOF
    print_ok "Pool mode ENABLED"
    echo ""
    echo -e "${YELLOW}⚠️  You must restart mining for changes to take effect${NC}"
}

pool_disable() {
    rm -f "$POOL_CONF"
    print_ok "Pool mode DISABLED"
    echo ""
    echo -e "${YELLOW}⚠️  You must restart mining for changes to take effect${NC}"
}

# Toggle functions for quick mode switching
toggle_solo_mode() {
    if ! pool_is_enabled; then
        echo ""
        echo -e "${GREEN}✓ Already in SOLO mode${NC}"
        sleep 1
        return
    fi
    # Try RPC first (no restart needed)
    if node_running && rpc_set_pool_mode false; then
        rm -f "$POOL_CONF"
        echo ""
        echo -e "${GREEN}✓ Switched to SOLO mode (no restart)${NC}"
        sleep 1
        return
    fi
    # Fallback: update config and restart
    rm -f "$POOL_CONF"
    echo ""
    echo -e "${GREEN}✓ Switched to SOLO mode${NC}"
    if node_running; then
        echo -e "${YELLOW}Restarting node with SOLO mode...${NC}"
        stop_node
        sleep 2
        start_node
    fi
    sleep 1
}
toggle_pool_mode() {
    if pool_is_enabled; then
        echo ""
        echo -e "${GREEN}✓ Already in POOL mode${NC}"
        sleep 1
        return
    fi
    # Try RPC first (no restart needed)
    if node_running && rpc_set_pool_mode true; then
        mkdir -p "$LUMENYX_DIR"
        echo "POOL_MODE=1" > "$POOL_CONF"
        echo ""
        echo -e "${GREEN}✓ Switched to POOL mode (no restart)${NC}"
        sleep 1
        return
    fi
    # Fallback: update config and restart
    mkdir -p "$LUMENYX_DIR"
    echo "POOL_MODE=1" > "$POOL_CONF"
    echo ""
    echo -e "${GREEN}✓ Switched to POOL mode${NC}"
    if node_running; then
        echo -e "${YELLOW}Restarting node with POOL mode...${NC}"
        stop_node
        sleep 2
        start_node
    fi
    sleep 1
}

# ═══════════════════════════════════════════════════════════════════════════════
# DAEMON MODE (24/7 SYSTEMD) FUNCTIONS  [PATCHED]
# ═══════════════════════════════════════════════════════════════════════════════

daemon_is_enabled() {
    systemctl is-enabled lumenyx.service >/dev/null 2>&1
}

daemon_is_running() {
    systemctl is-active lumenyx.service >/dev/null 2>&1
}

# --- Robust user/home detection (works for root@VPS and fabri@desktop) ---
get_current_user() {
    id -un
}

get_user_home() {
    local user
    user="$(get_current_user)"
    # Do NOT trust $HOME blindly (can be inherited / wrong under sudo)
    getent passwd "$user" | cut -d: -f6
}

daemon_compute_paths() {
    DAEMON_USER="$(get_current_user)"
    DAEMON_HOME="$(get_user_home)"

    if [[ -z "$DAEMON_HOME" || ! -d "$DAEMON_HOME" ]]; then
        echo -e "${RED}ERROR:${NC} Cannot determine home for user '$DAEMON_USER'"
        return 1
    fi

    DAEMON_BIN="$DAEMON_HOME/.lumenyx/lumenyx-node"
    DAEMON_WALLET_TXT="$DAEMON_HOME/.lumenyx/wallet.txt"

    # Base path MUST be explicit to pin keystore location
    DAEMON_BASE_PATH="$DAEMON_HOME/.local/share/lumenyx-node"
    DAEMON_KEYSTORE_DIR="$DAEMON_BASE_PATH/chains/lumenyx_mainnet/keystore"

    # Watchdog reads this file
    DAEMON_LOGFILE="$DAEMON_HOME/.lumenyx/lumenyx.log"
}

daemon_print_paths() {
    echo -e "${CYAN}Daemon user:${NC}    $DAEMON_USER"
    echo -e "${CYAN}Daemon home:${NC}    $DAEMON_HOME"
    echo -e "${CYAN}Binary:${NC}         $DAEMON_BIN"
    echo -e "${CYAN}Base path:${NC}      $DAEMON_BASE_PATH"
    echo -e "${CYAN}Keystore dir:${NC}   $DAEMON_KEYSTORE_DIR"
    echo -e "${CYAN}wallet.txt:${NC}     $DAEMON_WALLET_TXT"
    echo -e "${CYAN}Logfile:${NC}        $DAEMON_LOGFILE"
}

daemon_guard_rails_or_die() {
    # Fail fast: NEVER allow systemd to start if wallet/keystore missing
    [[ -x "$DAEMON_BIN" ]] || { echo -e "${RED}ERROR:${NC} Missing binary: $DAEMON_BIN"; return 1; }
    [[ -s "$DAEMON_WALLET_TXT" ]] || { echo -e "${RED}ERROR:${NC} Missing wallet.txt: $DAEMON_WALLET_TXT"; return 1; }
    [[ -f "$DAEMON_BASE_PATH/miner-key" ]] || { echo -e "${RED}ERROR:${NC} Missing miner-key: $DAEMON_BASE_PATH/miner-key"; return 1; }

#POW_FIX:     # Require keystore not empty (directory exists is not enough)
#POW_FIX:     if ! ls -1 "$DAEMON_KEYSTORE_DIR"/* >/dev/null 2>&1; then
#POW_FIX:         echo -e "${RED}ERROR:${NC} Keystore directory is empty: $DAEMON_KEYSTORE_DIR"
#POW_FIX:         echo -e "${YELLOW}Hint:${NC} Start in normal mode once or import/insert keys, then retry daemon mode."
#POW_FIX:         return 1
#POW_FIX:     fi

    # Ensure logfile directory exists (watchdog expects it)
    mkdir -p "$DAEMON_HOME/.lumenyx" >/dev/null 2>&1 || true
}

create_systemd_service() {
    daemon_compute_paths || return 1

    local validator_flag=""
    local pool_flag=""
    
    # Sync-safe: only add --validator if sync is complete
    if is_sync_complete; then
        validator_flag=" --validator"
        if pool_is_enabled; then
            pool_flag=" --pool-mode"
        fi
    fi

    local threads_env=""
    local threads
    threads="$(get_threads)"
    if [[ -n "$threads" ]] && [[ -n "$validator_flag" ]]; then
        threads_env="Environment=LUMENYX_MINING_THREADS=$threads"
    fi


    # Build bootnode args (same logic as start_node)
    local bootnode_args="" bootnodes=""
    if [[ -n "${BOOTNODES:-}" ]]; then
        bootnodes="$BOOTNODES"
    elif [[ -f "$LUMENYX_DIR/bootnodes.conf" ]]; then
        bootnodes=$(cat "$LUMENYX_DIR/bootnodes.conf" 2>/dev/null)
    else
        bootnodes=$(curl -sL "$BOOTNODES_URL" 2>/dev/null | grep -v '^#' | grep -v '^$' | tr '\n' ' ')
    fi
    if [[ -n "$bootnodes" ]]; then
        for bn in $bootnodes; do
            bootnode_args="$bootnode_args --bootnodes $bn"
        done
    fi

    # Reserved nodes: mantiene connessione stabile in rete piccola
    local reserved_nodes_args=""
    if [[ -n "$bootnodes" ]]; then
        for bn in $bootnodes; do
            reserved_nodes_args="$reserved_nodes_args --reserved-nodes $bn"
        done
    fi

    # RPC flags (same as normal mode)
    local rpc_args="--rpc-cors all --unsafe-rpc-external --rpc-methods Unsafe"
    # IMPORTANT:
    # - systemd does not expand "~"
    # - pin --base-path so keystore is stable
    # - use ExecStartPre guards
    cat > /tmp/lumenyx.service <<EOF
[Unit]
Description=LUMENYX Node (24/7 Mining)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$DAEMON_USER
Group=$DAEMON_USER
WorkingDirectory=$DAEMON_HOME

ExecStartPre=/usr/bin/test -s $DAEMON_WALLET_TXT

ExecStart=$DAEMON_BIN --chain mainnet --base-path $DAEMON_BASE_PATH${validator_flag}${pool_flag} --state-pruning 250000 --blocks-pruning 250000 $rpc_args $bootnode_args $reserved_nodes_args

Restart=always
RestartSec=3
TimeoutStopSec=30
KillSignal=SIGINT

# Prefer journald (robust across distros)
StandardOutput=journal
StandardError=journal

$threads_env

[Install]
WantedBy=multi-user.target
EOF

    sudo mv /tmp/lumenyx.service "$SYSTEMD_SERVICE"
    sudo chmod 644 "$SYSTEMD_SERVICE"
}

create_watchdog_script() {
    daemon_compute_paths || return 1
    
    cat > /tmp/lumenyx-watchdog.sh <<'WATCHDOG_EOF'
#!/usr/bin/env bash
set -euo pipefail

SERVICE_NAME="lumenyx.service"

# Thresholds (seconds)
NO_MINING_SECS="${NO_MINING_SECS:-60}"
HASHRATE_ZERO_SECS="${HASHRATE_ZERO_SECS:-60}"
COOLDOWN_SECS="${COOLDOWN_SECS:-120}"

STATE_DIR="/run/lumenyx-watchdog"
LAST_RESTART_FILE="${STATE_DIR}/last_restart_epoch"
ZERO_START_FILE="${STATE_DIR}/zero_start_epoch"

mkdir -p "${STATE_DIR}"
chmod 755 "${STATE_DIR}" 2>/dev/null || true

now_epoch() { date +%s; }

# Read recent logs from journalctl (daemon mode) with fallback to log file
get_recent_logs() {
    journalctl -u "$SERVICE_NAME" --since "2 min ago" --no-pager 2>/dev/null || true
}

last_line_matching() {
    local re="$1"
    get_recent_logs | grep -E "$re" | tail -n1 || true
}

line_epoch() {
    local line="$1"
    local ts
    ts="$(echo "$line" | awk '{print $1" "$2" "$3}')"
    date -d "$ts" +%s 2>/dev/null || echo 0
}

last_mining_epoch() {
    local line
    line="$(last_line_matching 'Mining #[0-9]+ with difficulty')"
    [ -n "$line" ] || { echo 0; return; }
    line_epoch "$line"
}

last_hashrate_info() {
    local line epoch hr
    line="$(last_line_matching 'Hashrate total=')"
    [ -n "$line" ] || { echo "0 -1"; return; }
    epoch="$(line_epoch "$line")"
    hr="$(echo "$line" | sed -n 's/.*Hashrate total=\([0-9]\+\) H\/s.*/\1/p')"
    [ -n "$hr" ] || hr="-1"
    echo "$epoch $hr"
}

cooldown_ok() {
    local now last
    now="$(now_epoch)"
    last="$(cat "$LAST_RESTART_FILE" 2>/dev/null || echo 0)"
    [ $((now - last)) -ge "$COOLDOWN_SECS" ]
}

mark_restarted() {
    now_epoch >"$LAST_RESTART_FILE"
    rm -f "$ZERO_START_FILE" 2>/dev/null || true
}

restart_node() {
    logger -t lumenyx-watchdog "Restarting ${SERVICE_NAME} due to watchdog condition"
    systemctl restart "$SERVICE_NAME"
    mark_restarted
}

main() {
    if ! systemctl is-active --quiet "$SERVICE_NAME"; then
        exit 0
    fi

    # ---- Sync-safe upgrade: if sync completed, recreate service with --validator ----
    local sync_file="__SYNC_SAFE_FILE__"
    local service_has_validator
    service_has_validator=$(grep -c "\-\-validator" /etc/systemd/system/lumenyx.service 2>/dev/null || echo "0")
    
    if [[ "$service_has_validator" -eq 0 ]] && [[ -f "$sync_file" ]]; then
        # Sync is marked complete but service doesn't have --validator
        # Signal the main script to recreate the service (we don't have the full logic here)
        logger -t lumenyx-watchdog "Sync complete, signaling service upgrade to mining mode"
        touch "/tmp/lumenyx-needs-mining-upgrade"
    fi

    if [[ "$service_has_validator" -eq 0 ]]; then
        # In sync-only mode, check if sync is done via RPC
        local result
        result=$(curl -s -m 3 -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","method":"system_syncState","params":[],"id":1}' \
            http://127.0.0.1:9944 2>/dev/null || true)
        
        if [[ -n "$result" ]]; then
            local current highest
            current=$(echo "$result" | python3 -c 'import sys,json; d=json.load(sys.stdin).get("result",{}); print(d.get("currentBlock",0))' 2>/dev/null || echo "0")
            highest=$(echo "$result" | python3 -c 'import sys,json; d=json.load(sys.stdin).get("result",{}); print(d.get("highestBlock",0))' 2>/dev/null || echo "0")
            
            if [[ "$highest" -gt 0 ]] && [[ "$current" -gt 0 ]]; then
                local gap=$((highest - current))
                if [[ "$gap" -le 50 ]]; then
                    # Sync complete! Mark it and signal upgrade
                    echo "$(date -Iseconds)" > "$sync_file"
                    logger -t lumenyx-watchdog "Sync complete at block $current. Service needs restart with --validator."
                    touch "/tmp/lumenyx-needs-mining-upgrade"
                fi
            fi
        fi
        # In sync-only mode, don't do mining checks - just exit
        exit 0
    fi
    # ---- End sync-safe upgrade ----

    local now lm
    now="$(now_epoch)"
    lm="$(last_mining_epoch)"

    # Condition 1: no "Mining #..." for too long
    if [ "$lm" -gt 0 ]; then
        if [ $((now - lm)) -ge "$NO_MINING_SECS" ]; then
            cooldown_ok && restart_node
            exit 0
        fi
    fi

    # Condition 2: hashrate = 0 for too long
    local le_hr hr
    read -r le_hr hr < <(last_hashrate_info)

    if [ "$le_hr" -gt 0 ] && [ "$hr" = "0" ]; then
        local zero_start
        zero_start="$(cat "$ZERO_START_FILE" 2>/dev/null || echo 0)"
        if [ "$zero_start" -eq 0 ]; then
            echo "$now" >"$ZERO_START_FILE"
        else
            if [ $((now - zero_start)) -ge "$HASHRATE_ZERO_SECS" ]; then
                cooldown_ok && restart_node
                exit 0
            fi
        fi
    else
        rm -f "$ZERO_START_FILE" 2>/dev/null || true
    fi
}

main "$@"
WATCHDOG_EOF
    
    # Replace placeholder with actual sync_safe file path
    sed -i "s|__SYNC_SAFE_FILE__|$DAEMON_HOME/.lumenyx/sync_complete|g" /tmp/lumenyx-watchdog.sh
    
    sudo mv /tmp/lumenyx-watchdog.sh "$WATCHDOG_SCRIPT"
    sudo chmod +x "$WATCHDOG_SCRIPT"
}

create_watchdog_service() {
    cat > /tmp/lumenyx-watchdog.service <<EOF
[Unit]
Description=LUMENYX Watchdog
After=lumenyx.service
Wants=lumenyx.service

[Service]
Type=oneshot
ExecStart=$WATCHDOG_SCRIPT
EOF

    sudo mv /tmp/lumenyx-watchdog.service "$SYSTEMD_WATCHDOG"
    sudo chmod 644 "$SYSTEMD_WATCHDOG"
}

create_watchdog_timer() {
    cat > /tmp/lumenyx-watchdog.timer <<EOF
[Unit]
Description=Run LUMENYX watchdog periodically

[Timer]
OnBootSec=60s
OnUnitActiveSec=15s
AccuracySec=1s
Unit=lumenyx-watchdog.service

[Install]
WantedBy=timers.target
EOF

    sudo mv /tmp/lumenyx-watchdog.timer "$SYSTEMD_WATCHDOG_TIMER"
    sudo chmod 644 "$SYSTEMD_WATCHDOG_TIMER"
}

is_real_desktop_session() {
    # Must have a GUI session
    [[ -n "${XDG_CURRENT_DESKTOP:-}" || -n "${DESKTOP_SESSION:-}" ]] || return 1
    # Must have a display
    [[ -n "${DISPLAY:-}" || -n "${WAYLAND_DISPLAY:-}" ]] || return 1
    # Must have gnome-terminal available
    command -v gnome-terminal >/dev/null 2>&1 || return 1
    return 0
}

create_autostart_desktop() {
    if ! is_real_desktop_session; then
        print_info "Desktop autostart skipped (no GUI desktop session detected)."
        return 0
    fi

    local script_path="$(realpath "$0")"
    
    mkdir -p "$HOME/.config/autostart"
    
    cat > "$AUTOSTART_FILE" <<EOF
[Desktop Entry]
Type=Application
Name=LUMENYX Wallet
Comment=LUMENYX Mining Dashboard
Exec=gnome-terminal -- bash -c '$script_path; exec bash'
Terminal=false
Hidden=false
X-GNOME-Autostart-enabled=true
EOF
    
    chmod +x "$AUTOSTART_FILE"
}

remove_autostart_desktop() {
    rm -f "$AUTOSTART_FILE" 2>/dev/null || true
}

daemon_enable() {
    echo ""
    echo -e "${CYAN}Setting up 24/7 daemon mode...${NC}"
    echo ""

    # Need sudo for /etc/systemd/system + systemctl enable
    if ! sudo -n true 2>/dev/null; then
        echo -e "${YELLOW}This requires sudo access. Please enter your password:${NC}"
    fi

    # Compute paths + show them BEFORE doing anything
    daemon_compute_paths || return 1
    echo -e "${CYAN}Daemon mode will use:${NC}"
    daemon_print_paths
    echo ""

    # Guard rails: NEVER start if wallet/keystore missing
    if ! daemon_guard_rails_or_die; then
        echo ""
        echo -e "${YELLOW}Daemon mode NOT enabled.${NC} Create/import wallet/keystore first."
        sleep 2
        return 1
    fi

    # Stop the script-managed node if running (avoid double node)
    if node_running && ! daemon_is_running; then
        echo -e "${YELLOW}Stopping script-managed node...${NC}"
        stop_node
        sleep 2
    fi

    echo "Creating systemd service..."
    create_systemd_service || return 1

    echo "Creating watchdog script..."
    create_watchdog_script

    echo "Creating watchdog service..."
    create_watchdog_service

    echo "Creating watchdog timer..."
    create_watchdog_timer

    # Create autostart for desktop
    echo "Creating desktop autostart..."
    create_autostart_desktop

    # Reload and enable
    echo "Enabling services..."
    sudo systemctl daemon-reload
    sudo systemctl enable lumenyx.service
    sudo systemctl enable lumenyx-watchdog.timer

    # Start
    echo "Starting services..."
    sudo systemctl start lumenyx.service
    sudo systemctl start lumenyx-watchdog.timer

    # Mark as enabled
    echo "1" > "$DAEMON_CONF"

    echo ""
    echo -e "${GREEN}✓ 24/7 Daemon mode ENABLED${NC}"
    if is_sync_complete; then
        echo -e "${GREEN}✓ Node will start mining immediately${NC}"
    else
        echo -e "${YELLOW}✓ Node will sync first, then start mining automatically${NC}"
    fi
    echo -e "${GREEN}✓ Node will start automatically on boot${NC}"
    echo -e "${GREEN}✓ Watchdog will monitor and restart if needed${NC}"
    echo -e "${GREEN}✓ Script will open automatically on login (desktop)${NC}"
    sleep 2
}

daemon_disable() {
    echo ""
    echo -e "${CYAN}Disabling 24/7 daemon mode...${NC}"
    echo ""
    
    # Stop and disable services
    echo "Stopping services..."
    sudo systemctl stop lumenyx-watchdog.timer 2>/dev/null || true
    sudo systemctl stop lumenyx.service 2>/dev/null || true
    
    echo "Disabling services..."
    sudo systemctl disable lumenyx-watchdog.timer 2>/dev/null || true
    sudo systemctl disable lumenyx.service 2>/dev/null || true
    
    # Remove autostart
    echo "Removing desktop autostart..."
    remove_autostart_desktop
    
    # Remove daemon conf
    rm -f "$DAEMON_CONF" 2>/dev/null || true
    
    echo ""
    echo -e "${GREEN}✓ 24/7 Daemon mode DISABLED${NC}"
    echo -e "${YELLOW}Node will only run when script is open${NC}"
    sleep 2
}

toggle_daemon_mode() {
    if daemon_is_enabled; then
        # Already enabled, disable it
        if ask_yes_no "Disable 24/7 mode? Node will stop when you close the script."; then
            daemon_disable
        fi
    else
        # Not enabled, enable it
        echo ""
        echo -e "${CYAN}═══ 24/7 DAEMON MODE ═══${NC}"
        echo ""
        echo "This will:"
        echo "  • Run the node as a system service (systemd)"
        echo "  • Start automatically when PC/VPS boots"
        echo "  • Continue running even if you close this script"
        echo "  • Auto-restart if mining gets stuck (watchdog)"
        echo "  • Open this dashboard automatically on desktop login"
        echo ""
        if ask_yes_no "Enable 24/7 daemon mode?"; then
            daemon_enable
        fi
    fi
}

menu_pool() {
    clear
    print_logo
    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
    echo -e "${BOLD}                    🏊 MINING POOL SETTINGS                        ${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
    echo ""
    
    if pool_is_enabled; then
        echo -e "  Status: ${GREEN}● POOL MODE ENABLED${NC}"
        echo ""
        echo "  Your node will:"
        echo "    • Share mining work with other pool miners"
        echo "    • Receive proportional rewards based on your hashrate"
        echo "    • Connect to other --pool-mode nodes automatically"
    else
        echo -e "  Status: ${CYAN}○ SOLO MODE (default)${NC}"
        echo ""
        echo "  Your node will:"
        echo "    • Mine blocks independently"
        echo "    • Receive 100% of block rewards when you find a block"
    fi
    
    echo ""
    echo -e "${CYAN}───────────────────────────────────────────────────────────────────${NC}"
    echo ""
    
    if pool_is_enabled; then
        echo -e "${YELLOW}Pool mode is currently ENABLED${NC}"
        echo ""
        if ask_yes_no "Disable pool mode and switch to solo mining?"; then
            pool_disable
        else
            echo "Cancelled."
        fi
    else
        echo -e "${CYAN}Pool mode is currently DISABLED (solo mining)${NC}"
        echo ""
        echo "Enabling pool mode will:"
        echo "  • Share your hashrate with the decentralized pool"
        echo "  • Get smaller but more frequent rewards"
        echo "  • Reduce reward variance (less luck-dependent)"
        echo ""
        if ask_yes_no "Enable pool mode?"; then
            pool_enable
        else
            echo "Cancelled."
        fi
    fi
    
    wait_enter
}

print_dashboard() {
    local addr short_addr
    addr=$(get_address)
    if [[ -n "$addr" ]]; then
        short_addr="${addr:0:8}...${addr: -6}"
    else
        short_addr="Not set"
    fi

    # EVM address
    local evm_addr short_evm
    evm_addr=$(get_evm_address)
    if [[ -n "$evm_addr" ]]; then
        short_evm="${evm_addr:0:8}...${evm_addr: -4}"
    else
        short_evm="N/A"
    fi

    local balance evm_balance block_info peers
    balance=$(get_balance)
    evm_balance=$(get_evm_balance)
    block_info=$(get_block)
    peers=$(get_peers)

    # Calculate total
    local total="N/A"
    if [[ "$balance" != "offline" && "$evm_balance" != "offline" ]]; then
        total=$(python3 -c "print('{:.3f}'.format($balance + $evm_balance))" 2>/dev/null || echo "N/A")
    fi

    # Parse block info: best|target|syncing
    local block target syncing block_display
    block=$(echo "$block_info" | cut -d'|' -f1)
    target=$(echo "$block_info" | cut -d'|' -f2)
    syncing=$(echo "$block_info" | cut -d'|' -f3)

    if [[ "$block" == "offline" ]]; then
        block_display="#offline"
    elif [[ "$syncing" == "True" && "$target" -gt "$block" ]]; then
        local pct=$((block * 100 / target))
        block_display="#${block} / #${target} (${pct}%)"
    else
        block_display="#${block} ✓"
    fi

    local status="STOPPED"
    local status_color="${RED}○"
    if daemon_is_running; then
        if ! is_sync_complete; then
            status="SYNCING (no mining)"
            status_color="${YELLOW}◉"
        else
            status="MINING"; if pool_is_enabled; then status="MINING (POOL)"; else status="MINING (SOLO)"; fi
            status="${status} [systemd]"
            status_color="${GREEN}●"
        fi
    elif node_running; then
        if ! is_sync_complete; then
            status="SYNCING (no mining)"
            status_color="${YELLOW}◉"
        else
            status="MINING"; if pool_is_enabled; then status="MINING (POOL)"; else status="MINING (SOLO)"; fi
            status_color="${GREEN}●"
        fi
    fi

    # Get threads display
    local threads_display
    threads_display=$(get_threads_display)

    # Mining mode toggle display
    local solo_indicator pool_indicator daemon_indicator
    if pool_is_enabled; then
        solo_indicator="${CYAN}○ SOLO${NC}"
        pool_indicator="${GREEN}● POOL${NC}"
    else
        solo_indicator="${GREEN}● SOLO${NC}"
        pool_indicator="${CYAN}○ POOL${NC}"
    fi
    
    if daemon_is_enabled; then
        daemon_indicator="${GREEN}● 24/7${NC}"
    else
        daemon_indicator="${CYAN}○ 24/7${NC}"
    fi

    clear
    print_logo
    echo -e "${VIOLET}╔════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${VIOLET}║${NC}   [S] $solo_indicator      [P] $pool_indicator      [D] $daemon_indicator               ${VIOLET}║${NC}"
    echo -e "${VIOLET}╚════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "  ${PINK}SS58:${NC}     ${GREEN}$short_addr${NC}    ${CYAN}$balance LUMO${NC}"
    echo -e "  ${PINK}EVM:${NC}      ${GREEN}$short_evm${NC}      ${CYAN}$evm_balance LUMO${NC}"
    echo -e "  ${PINK}Total:${NC}                         ${BRIGHT_VIOLET}${BOLD}$total LUMO${NC}"
    echo ""
    echo -e "  Block:    $block_display"
    echo -e "  Status:   ${status_color} ${status}${NC}"
    echo -e "  Peers:    $peers"
    if [[ "$block" == "offline" ]]; then
        echo -e "  Network:  ${RED}✗ Offline${NC}"
    elif [[ "$syncing" == "True" ]]; then
        echo -e "  Network:  ${YELLOW}⚠ Syncing...${NC}"
    else
        echo -e "  Network:  ${GREEN}✓ Synced${NC}"
    fi
    echo -e "  Threads:  ${CYAN}$threads_display${NC}"
    echo ""
    echo -e "${VIOLET}════════════════════════════════════════════════════════════════════${NC}"
}

check_evm_wallet() {
    # For existing users: if we have miner-key but no valid EVM wallet, prompt for seed phrase
    if [[ -f "$DATA_DIR/miner-key" ]] && [[ ! -f "$LUMENYX_DIR/evm-key" ]]; then
        # Check if eth-account is available
        if ! python3 -c 'from eth_account import Account' >/dev/null 2>&1; then
            return  # deps not installed yet, will be done on next ensure_python_deps
        fi

        echo ""
        echo -e "${YELLOW}╔════════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${YELLOW}║  EVM WALLET SETUP (one-time)                                      ║${NC}"
        echo -e "${YELLOW}║  Enter your 12-word seed phrase to derive your MetaMask address.   ║${NC}"
        echo -e "${YELLOW}║  Press ENTER to skip (you can do this later from Useful Commands). ║${NC}"
        echo -e "${YELLOW}╚════════════════════════════════════════════════════════════════════╝${NC}"
        echo ""
        read -r -p "  12-word seed phrase (or ENTER to skip): " seed_phrase

        if [[ -z "$seed_phrase" ]]; then
            print_warning "Skipped. EVM address not configured. Use [7] Useful Commands to set up later."
            sleep 2
            return
        fi

        # Validate the seed phrase matches our SS58
        local output address_check
        output=$("$LUMENYX_DIR/$BINARY_NAME" key inspect "$seed_phrase" 2>&1)
        address_check=$(echo "$output" | grep "SS58 Address:" | sed 's/.*SS58 Address:[[:space:]]*//')
        local current_addr
        current_addr=$(get_address)

        if [[ "$address_check" != "$current_addr" ]]; then
            print_error "Seed phrase does not match your current wallet ($current_addr)."
            print_error "Got: $address_check"
            sleep 3
            return
        fi

        # Derive EVM address
        local evm_output evm_address evm_privkey
        evm_output=$(derive_evm_from_mnemonic "$seed_phrase")
        evm_address=$(echo "$evm_output" | head -1)
        evm_privkey=$(echo "$evm_output" | tail -1)

        if [[ -n "$evm_address" ]]; then
            echo "$evm_address" > "$LUMENYX_DIR/wallet_evm.txt"
            echo "$evm_privkey" > "$LUMENYX_DIR/evm-key"
            chmod 600 "$LUMENYX_DIR/evm-key"
            # Remove old fake wallet_evm.txt if it existed
            echo ""
            print_ok "EVM wallet configured!"
            echo -e "  EVM (MetaMask): ${GREEN}$evm_address${NC}"
            echo -e "  ${YELLOW}→ Import the same 12 words in MetaMask to use the DEX${NC}"
            sleep 3
        else
            print_error "Failed to derive EVM address. Check Python dependencies."
            sleep 2
        fi
    fi
}

dashboard_loop() {
    # One-time check: migrate existing users to real EVM wallet
    check_evm_wallet

    while true; do
        # Check if sync completed and node needs restart with mining
        check_sync_and_upgrade
        
        # For daemon mode: if watchdog signaled sync complete, recreate service with --validator
        if [[ -f "/tmp/lumenyx-needs-mining-upgrade" ]] && daemon_is_enabled; then
            rm -f "/tmp/lumenyx-needs-mining-upgrade"
            print_ok "Sync complete! Upgrading daemon to mining mode..."
            create_systemd_service || true
            sudo systemctl daemon-reload
            sudo systemctl restart lumenyx.service
        fi

        print_dashboard
        echo ""
        echo -e "  ${VIOLET}[1]${NC} ⛏️  Start/Stop Mining"
        echo -e "  ${VIOLET}[2]${NC} 💸 Send LUMO"
        echo -e "  ${VIOLET}[3]${NC} 📥 Receive (show addresses)"
        echo -e "  ${VIOLET}[4]${NC} 📜 History"
        echo -e "  ${VIOLET}[5]${NC} 📊 Live Logs"
        echo -e "  ${VIOLET}[6]${NC} 💰 Transaction History"
        echo -e "  ${VIOLET}[7]${NC} 🛠️  Useful Commands"
        echo -e "  ${VIOLET}[8]${NC} ⚙️  Set Mining Threads"
        echo -e "  ${VIOLET}[0]${NC} 🚪 Exit"
        echo ""
        echo -e "  ${PINK}[S] SOLO  [P] POOL  [D] 24/7 Daemon${NC}"
        echo -e "  ${CYAN}Auto-refresh in 10s - Press a key to select${NC}"
        echo ""

        read -r -t 10 -n 1 choice || choice="refresh"

        # Clear input buffer
        read -r -t 0.1 -n 10000 discard 2>/dev/null || true

        case $choice in
            1) echo ""; echo "Loading..."; menu_start_stop ;;
            2) echo ""; echo "Loading..."; menu_send ;;
            3) echo ""; echo "Loading..."; menu_receive ;;
            4) echo ""; echo "Loading..."; menu_history ;;
            5) echo ""; echo "Loading..."; menu_logs ;;
            6) echo ""; echo "Loading..."; menu_tx_history ;;
            7) echo ""; echo "Loading..."; menu_commands ;;
            8) echo ""; echo "Loading..."; set_threads_menu; wait_enter ;;
            [Ss]) toggle_solo_mode ;;
            [Pp]) toggle_pool_mode ;;
            [Dd]) toggle_daemon_mode ;;
            0) echo ""; echo "Goodbye!"; exit 0 ;;
            refresh) ;;
            *) ;;
        esac
    done
}

# ═══════════════════════════════════════════════════════════════════════════════
# MENU FUNCTIONS
# ═══════════════════════════════════════════════════════════════════════════════

menu_start_stop() {
    # If daemon mode is enabled, use systemctl
    if daemon_is_enabled; then
        if daemon_is_running; then
            echo ""
            echo -e "${CYAN}Node is managed by systemd (24/7 mode)${NC}"
            if ask_yes_no "Stop mining? (will restart automatically on reboot)"; then
                sudo systemctl stop lumenyx.service
                echo -e "${GREEN}✓ Node stopped${NC}"
            fi
        else
            if ask_yes_no "Start mining? (managed by systemd)"; then
                sudo systemctl start lumenyx.service
                echo -e "${GREEN}✓ Node started${NC}"
            fi
        fi
    else
        # Normal script-managed mode
        if node_running; then
            if ask_yes_no "Mining is running. Stop it?"; then
                stop_node
            fi
        else
            if ask_yes_no "Start mining?"; then
                start_node
            fi
        fi
    fi
    wait_enter
}

menu_send() {
    echo ""
    if ! ask_yes_no "Open Send menu?"; then
        return
    fi
    print_dashboard
    echo ""
    echo -e "${VIOLET}═══ SEND LUMO ═══${NC}"
    echo ""

    if ! node_running; then
        print_error "Node must be running to send transactions"
        wait_enter
        return
    fi

    if [[ ! -f "$DATA_DIR/miner-key" ]]; then
        print_error "Wallet not found (missing $DATA_DIR/miner-key)"
        wait_enter
        return
    fi

    ensure_helpers
    ensure_python_deps || { wait_enter; return; }

    read -r -p "Recipient address (SS58 or 0x EVM): " recipient
    if [[ -z "$recipient" ]]; then
        print_warning "Cancelled"
        wait_enter
        return
    fi

    read -r -p "Amount (LUMO): " amount
    if [[ -z "$amount" ]]; then
        print_warning "Cancelled"
        wait_enter
        return
    fi

    echo ""
    echo "  To:     $recipient"
    echo "  Amount: $amount LUMO"
    echo ""

    if ask_yes_no "Confirm transaction?"; then
        # Detect if recipient is EVM (0x...) or SS58
        if [[ "$recipient" == 0x* ]]; then
            print_info "EVM address detected — using evm_bridge.deposit..."
            local dec_now
            dec_now=$(get_decimals)
            local out ok hash err
            out=$(python3 -c "
import json, sys, os
SEED_FILE = os.path.expanduser('~/.local/share/lumenyx-node/miner-key')
seed = open(SEED_FILE).read().strip().lower()
if seed.startswith('0x'): seed = seed[2:]

from substrateinterface import SubstrateInterface, Keypair, KeypairType
substrate = SubstrateInterface(url='$WS')
kp = Keypair.create_from_seed(bytes.fromhex(seed), crypto_type=KeypairType.SR25519)

dec = $dec_now
amount_str = '$amount'
s = amount_str.strip().replace(',', '.')
if '.' in s:
    a, b = s.split('.', 1)
    b = (b + '0' * dec)[:dec]
    value = int(a) * (10 ** dec) + int(b)
else:
    value = int(s) * (10 ** dec)

call = substrate.compose_call(
    call_module='EvmBridge',
    call_function='deposit',
    call_params={'evm_address': '$recipient', 'amount': value},
)
extrinsic = substrate.create_signed_extrinsic(call=call, keypair=kp)
try:
    receipt = substrate.submit_extrinsic(extrinsic, wait_for_inclusion=True)
    out = {'ok': bool(getattr(receipt, 'is_success', True)), 'hash': getattr(receipt, 'extrinsic_hash', None)}
    print(json.dumps(out))
except Exception as e:
    print(json.dumps({'ok': False, 'error': str(e)}))
" 2>/dev/null || true)
            ok=$(echo "$out" | grep -o '"ok": *[^,]*' | cut -d':' -f2 | tr -d ' }')
            hash=$(echo "$out" | grep -o '"hash": *"[^"]*"' | cut -d'"' -f4)
            err=$(echo "$out" | grep -o '"error":"[^"]*"' | cut -d'"' -f4)

            if [[ "$ok" == "true" ]] && [[ -n "$hash" ]]; then
                print_ok "Transaction submitted: $hash"
            else
                print_error "Send failed"
                [[ -n "$err" ]] && echo "  Error: $err"
                [[ -n "$out" ]] && echo "  Raw: $out"
            fi
        else
            print_info "Signing & submitting extrinsic..."
            local out ok hash err
            out=$(python3 "$SUBSTRATE_SEND_PY" --ws "$WS" --to "$recipient" --amount "$amount" --decimals "$(get_decimals)" --wait inclusion 2>/dev/null || true)
            ok=$(echo "$out" | grep -o '"ok": *[^,]*' | cut -d':' -f2 | tr -d ' }')
            hash=$(echo "$out" | grep -o '"hash": *"[^"]*"' | cut -d'"' -f4)
            err=$(echo "$out" | grep -o '"error":"[^"]*"' | cut -d'"' -f4)

            if [[ "$ok" == "true" ]] && [[ -n "$hash" ]]; then
                print_ok "Transaction submitted: $hash"
            else
                print_error "Send failed"
                [[ -n "$err" ]] && echo "  Error: $err"
                [[ -n "$out" ]] && echo "  Raw: $out"
            fi
        fi
    fi

    wait_enter
}

menu_receive() {
    echo ""
    if ! ask_yes_no "Show addresses?"; then
        return
    fi
    print_dashboard
    echo ""
    echo -e "${VIOLET}═══ RECEIVE LUMO ═══${NC}"
    echo ""

    local addr
    addr=$(get_address)
    if [[ -n "$addr" ]]; then
        echo -e "  ${PINK}SS58 Address (Substrate):${NC}"
        echo -e "  ${GREEN}${BOLD}$addr${NC}"
        echo ""
    fi

    local evm_addr
    evm_addr=$(get_evm_address)
    if [[ -n "$evm_addr" ]]; then
        echo -e "  ${PINK}EVM Address (MetaMask):${NC}"
        echo -e "  ${GREEN}${BOLD}$evm_addr${NC}"
        echo ""
    fi

    if [[ -z "$addr" && -z "$evm_addr" ]]; then
        print_error "No wallet found"
    else
        echo "  (Copy the address you need)"
    fi

    wait_enter
}

menu_history() {
    echo ""
    if ! ask_yes_no "Show history?"; then
        return
    fi
    print_dashboard
    echo ""
    echo -e "${CYAN}═══ MINING HISTORY ═══${NC}"
    echo ""

    if [[ -f "$LOG_FILE" ]]; then
        print_info "Recent mining activity:"
        echo ""
        grep -E "✅ Block.*mined|🏆 Imported" "$LOG_FILE" 2>/dev/null | tail -15 || echo "  No recent activity"
    else
        print_warning "No log file found"
    fi

    wait_enter
}

menu_tx_history() {
    echo ""
    if ! ask_yes_no "Show transaction history?"; then
        return
    fi

    print_dashboard
    echo ""
    echo -e "${CYAN}═══ TRANSACTION HISTORY (sent/received) ═══${NC}"
    echo ""

    if ! node_running; then
        print_error "Node must be running"
        wait_enter
        return
    fi

    ensure_helpers
    ensure_python_deps >/dev/null || { print_error "Python deps missing"; wait_enter; return; }

    print_info "Scanning recent blocks for transactions..."

    local out
    out=$(python3 "$SUBSTRATE_TX_PY" --ws "$WS" --blocks 5000 --decimals "$(get_decimals)" 2>&1 || true)

    if echo "$out" | grep -q '"ok": true'; then
        echo "$out" | python3 -c 'import sys,json
d=json.load(sys.stdin)
txs=d.get("transactions",[])
if not txs:
    print("  No transactions found in recent blocks")
else:
    for tx in txs:
        sym="[OUT]" if tx["type"]=="SENT" else "[IN] "
        addr = tx["from"] if tx["type"]=="RECV" else tx["to"]
        label = "From:" if tx["type"]=="RECV" else "To:  "
        print("  %s Block #%d | %12.3f LUMO | %s %s" % (sym,tx["block"],tx["amount"],label,addr))'
    else
        print_warning "Could not fetch transactions"
    fi

    wait_enter
}

menu_logs() {
    echo ""
    if ! ask_yes_no "Show logs?"; then
        return
    fi
    echo ""
    print_info "Showing live logs (Ctrl+C to exit)..."
    print_warning "Note: Ctrl+C will return to menu, mining continues in background"
    echo ""

    # If running as daemon (systemd), use journalctl
    if systemctl is-active --quiet lumenyx.service 2>/dev/null; then
        journalctl -u lumenyx -f --no-pager
    elif [[ -f "$LOG_FILE" ]] && [[ -s "$LOG_FILE" ]]; then
        tail -f "$LOG_FILE"
    else
        print_error "No log file found. Start mining first."
        wait_enter
    fi
}

show_my_bootnode() {
    clear
    print_logo
    echo ""
    echo -e "${CYAN}═══ SHOW MY BOOTNODE ═══${NC}"
    echo ""

    local peer_id=""

    # Try journalctl first (daemon mode), then log file
    if systemctl is-active --quiet lumenyx.service 2>/dev/null; then
        peer_id=$(journalctl -u lumenyx --no-pager -n 200 2>/dev/null | grep "Local node identity" | tail -1 | awk '{print $NF}')
    fi

    if [[ -z "$peer_id" ]] && [[ -f "$LOG_FILE" ]]; then
        peer_id=$(grep "Local node identity" "$LOG_FILE" 2>/dev/null | tail -1 | awk '{print $NF}')
    fi

    if [[ -z "$peer_id" ]]; then
        print_error "Peer ID not found in logs. Start the node and wait for 'Local node identity'."
        wait_enter
        return
    fi

    local ip
    ip=$(curl -s --connect-timeout 5 https://api.ipify.org 2>/dev/null || true)

    if [[ -z "$ip" ]]; then
        print_error "Could not detect public IP (api.ipify.org unreachable)."
        echo "Peer ID: $peer_id"
        wait_enter
        return
    fi

    local bootnode="/ip4/${ip}/tcp/30333/p2p/${peer_id}"
    print_ok "Your bootnode:"
    echo ""
    echo "  $bootnode"
    echo ""
    wait_enter
}

menu_commands() {
    clear
    print_logo
    echo ""
    echo -e "${CYAN}═══ USEFUL COMMANDS ═══${NC}"
    echo ""
    echo -e "  ${YELLOW}[1] Show my bootnode (IP + Peer ID)${NC}"
    echo -e "  ${YELLOW}[2] CLEAN INSTALL (reset everything):${NC}"
    echo "     rm -rf ~/.lumenyx ~/.local/share/lumenyx*"
    echo ""
    echo -e "  ${YELLOW}[3] VIEW FULL LOGS:${NC}"
    echo "     tail -100 ~/.lumenyx/lumenyx.log"
    echo ""
    echo -e "  ${YELLOW}[4] FIND YOUR PEER ID:${NC}"
    echo '     grep "Local node identity" ~/.lumenyx/lumenyx.log'
    echo ""
    echo -e "  ${YELLOW}[5] UPDATE SCRIPT:${NC}"
    echo "     curl -O https://raw.githubusercontent.com/lumenyx-chain/lumenyx/main/lumenyx-setup.sh"
    echo ""
    echo -e "  ${YELLOW}[6] POLKADOT.JS EXPLORER:${NC}"
    echo ""
    echo -e "  ${YELLOW}[7] START/STOP/RESTART NODE:${NC}"
    echo "     systemctl start lumenyx"
    echo "     systemctl stop lumenyx"
    echo "     systemctl restart lumenyx"
    echo "     https://polkadot.js.org/apps/?rpc=ws://YOUR_IP:9944"
    echo ""
    echo -e "  ${YELLOW}[0] Back${NC}"
    echo ""

    read -r -p "Choice: " c
    case "$c" in
        1) show_my_bootnode ;;
        2|3|4|5|6|0|"") ;;
        *) ;;
    esac
}

# ═══════════════════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════════════════

main() {
    check_for_updates

    if [[ "${1:-}" == "--updated" ]]; then
        print_ok "Script updated successfully!"
        sleep 1
    elif has_existing_data; then
        prompt_clean_install || true
    fi

    # Always check if binary needs update (even if skipping clean install)
    check_binary_update

    if is_first_run; then
        first_run
    fi

    dashboard_loop
}

main "$@"
