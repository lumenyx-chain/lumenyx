#!/bin/bash

# ═══════════════════════════════════════════════════════════════════════════════
# LUMENYX SETUP SCRIPT v1.9.29 - Simple & Clean (No root required)
# ═══════════════════════════════════════════════════════════════════════════════

set -e

VERSION="1.7.1"
SCRIPT_VERSION="1.9.29"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

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

# Helpers
HELPERS_DIR="$LUMENYX_DIR/helpers"
SUBSTRATE_SEND_PY="$HELPERS_DIR/substrate_send.py"
SUBSTRATE_DASH_PY="$HELPERS_DIR/substrate_dashboard.py"
SUBSTRATE_TX_PY="$HELPERS_DIR/substrate_tx.py"

# Download URLs
BINARY_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v${VERSION}/lumenyx-node-linux-x86_64"
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
# UI FUNCTIONS
# ═══════════════════════════════════════════════════════════════════════════════

print_logo() {
    echo -e "${BLUE}"
    echo "╔════════════════════════════════════════════════════════════════════╗"
    echo "║                                                                    ║"
    echo "║   ██╗     ██╗   ██╗███╗   ███╗███████╗███╗   ██╗██╗   ██╗██╗  ██╗  ║"
    echo "║   ██║     ██║   ██║████╗ ████║██╔════╝████╗  ██║╚██╗ ██╔╝╚██╗██╔╝  ║"
    echo "║   ██║     ██║   ██║██╔████╔██║█████╗  ██╔██╗ ██║ ╚████╔╝  ╚███╔╝   ║"
    echo "║   ██║     ██║   ██║██║╚██╔╝██║██╔══╝  ██║╚██╗██║  ╚██╔╝   ██╔██╗   ║"
    echo "║   ███████╗╚██████╔╝██║ ╚═╝ ██║███████╗██║ ╚████║   ██║   ██╔╝ ██╗  ║"
    echo "║   ╚══════╝ ╚═════╝ ╚═╝     ╚═╝╚══════╝╚═╝  ╚═══╝   ╚═╝   ╚═╝  ╚═╝  ║"
    echo "║                                                                    ║"
    echo "╚════════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
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

def amount_to_planck(amount_str, decimals=12):
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
    ap.add_argument("--decimals", type=int, default=12)
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
    ap.add_argument("--decimals", type=int, default=12)
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
    ap.add_argument("--decimals", type=int, default=12)
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

    if python3 -c 'import substrateinterface' >/dev/null 2>&1; then
        return 0
    fi

    print_info "Installing Python dependency (substrate-interface)..."
    python3 -m pip install --user substrate-interface >/dev/null 2>&1 || {
        print_error "Failed to install substrate-interface. Run: pip3 install --user substrate-interface"
        return 1
    }
    return 0
}

# ═══════════════════════════════════════════════════════════════════════════════
# UTILITY FUNCTIONS
# ═══════════════════════════════════════════════════════════════════════════════

node_running() {
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
        local seed
        seed=$(cat "$DATA_DIR/miner-key" 2>/dev/null)
        if [[ -n "$seed" ]] && [[ -f "$LUMENYX_DIR/$BINARY_NAME" ]]; then
            "$LUMENYX_DIR/$BINARY_NAME" key inspect "0x$seed" 2>/dev/null | grep "SS58" | awk '{print $3}'
        fi
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
    out=$(python3 "$SUBSTRATE_DASH_PY" --ws "$WS" --mode balance --decimals 12 2>/dev/null || true)
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

    if [[ -f "$LUMENYX_DIR/$BINARY_NAME" ]]; then
        print_ok "Binary already exists"
        wait_enter
        return
    fi

    print_info "Downloading lumenyx-node (~65MB)..."
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
    print_ok "Binary ready: $LUMENYX_DIR/$BINARY_NAME"
    wait_enter
}

step_wallet() {
    clear
    print_logo
    echo -e "${CYAN}═══ STEP 3: WALLET ═══${NC}"
    echo ""

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

        echo ""
        echo -e "${YELLOW}╔════════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${YELLOW}║  YOUR SEED PHRASE (write it down NOW!):                            ║${NC}"
        echo -e "${YELLOW}╚════════════════════════════════════════════════════════════════════╝${NC}"
        echo ""
        echo -e "  ${GREEN}${BOLD}$seed_phrase${NC}"
        echo ""
        echo -e "  Your address: ${CYAN}$address${NC}"
        echo ""

        mkdir -p "$DATA_DIR"
        echo "$secret_seed" > "$DATA_DIR/miner-key"
        chmod 600 "$DATA_DIR/miner-key"

        echo "Address: $address" > "$LUMENYX_DIR/wallet.txt"

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

        mkdir -p "$DATA_DIR"
        echo "$secret_seed" > "$DATA_DIR/miner-key"
        chmod 600 "$DATA_DIR/miner-key"

        echo "Address: $address" > "$LUMENYX_DIR/wallet.txt"

        echo ""
        echo -e "  Your address: ${GREEN}$address${NC}"
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

start_node() {
    if node_running; then
        print_warning "Node is already running"
        return
    fi

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
            bootnode_args="$bootnode_args --bootnodes $bn"
        done
    fi

    # Ensure log file exists
    mkdir -p "$LUMENYX_DIR"
    touch "$LOG_FILE"

    print_info "Starting node..."

    nohup "$LUMENYX_DIR/$BINARY_NAME" \
        --chain mainnet \
        --validator \
        --rpc-cors all \
        --unsafe-rpc-external \
        --rpc-methods Unsafe \
        $bootnode_args \
        >> "$LOG_FILE" 2>&1 &

    echo $! > "$PID_FILE"

    sleep 3

    if node_running; then
        print_ok "Mining started! (PID: $(cat "$PID_FILE"))"
    else
        print_error "Failed to start - check: tail -50 $LOG_FILE"
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

print_dashboard() {
    local addr short_addr
    addr=$(get_address)
    if [[ -n "$addr" ]]; then
        short_addr="${addr:0:8}...${addr: -6}"
    else
        short_addr="Not set"
    fi

    local balance block_info peers
    balance=$(get_balance)
    block_info=$(get_block)
    peers=$(get_peers)
    
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
    if node_running; then
        status="MINING"
        status_color="${GREEN}●"
    fi

    clear
    print_logo
    echo ""
    echo -e "  Wallet:   ${GREEN}$short_addr${NC}"
    echo -e "  Balance:  ${GREEN}$balance LUMENYX${NC}"
    echo -e "  Block:    $block_display"
    echo -e "  Status:   ${status_color} ${status}${NC}"
    echo -e "  Peers:    $peers"
    echo ""
    echo "════════════════════════════════════════════════════════════════════"
}

dashboard_loop() {
    while true; do
        print_dashboard
        echo ""
        echo "  [1] ⛏️  Start/Stop Mining"
        echo "  [2] 💸 Send LUMENYX"
        echo "  [3] 📥 Receive (show address)"
        echo "  [4] 📜 History"
        echo "  [5] 📊 Live Logs"
        echo "  [6] 💰 Transaction History"
        echo "  [7] 🛠️  Useful Commands"
        echo "  [0] 🚪 Exit"
        echo ""
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
    if node_running; then
        if ask_yes_no "Mining is running. Stop it?"; then
            stop_node
        fi
    else
        if ask_yes_no "Start mining?"; then
            start_node
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
    echo -e "${CYAN}═══ SEND LUMENYX (Balances.transfer_keep_alive) ═══${NC}"
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

    read -r -p "Recipient address (SS58): " recipient
    if [[ -z "$recipient" ]]; then
        print_warning "Cancelled"
        wait_enter
        return
    fi

    read -r -p "Amount (LUMENYX): " amount
    if [[ -z "$amount" ]]; then
        print_warning "Cancelled"
        wait_enter
        return
    fi

    echo ""
    echo "  To:     $recipient"
    echo "  Amount: $amount LUMENYX"
    echo ""

    if ask_yes_no "Confirm transaction?"; then
        print_info "Signing & submitting extrinsic..."
        local out ok hash err
        out=$(python3 "$SUBSTRATE_SEND_PY" --ws "$WS" --to "$recipient" --amount "$amount" --decimals 12 --wait inclusion 2>/dev/null || true)
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

    wait_enter
}

menu_receive() {
    echo ""
    if ! ask_yes_no "Show address?"; then
        return
    fi
    print_dashboard
    echo ""
    echo -e "${CYAN}═══ RECEIVE LUMENYX ═══${NC}"
    echo ""

    local addr
    addr=$(get_address)
    if [[ -n "$addr" ]]; then
        echo "  Share this address to receive LUMENYX:"
        echo ""
        echo -e "  ${GREEN}${BOLD}$addr${NC}"
        echo ""
        echo "  (Copy the address above)"
    else
        print_error "No wallet found"
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
    out=$(python3 "$SUBSTRATE_TX_PY" --ws "$WS" --blocks 0 --decimals 12 2>&1 || true)
    
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
        print("  %s Block #%d | %12.3f LUMENYX | %s %s" % (sym,tx["block"],tx["amount"],label,addr))'
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

    if [[ -f "$LOG_FILE" ]]; then
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

    if [[ ! -f "$LOG_FILE" ]]; then
        print_error "Log file not found: $LOG_FILE"
        wait_enter
        return
    fi

    local peer_id
    peer_id=$(grep "Local node identity" "$LOG_FILE" 2>/dev/null | tail -1 | awk '{print $NF}')

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

    if is_first_run; then
        first_run
    fi

    dashboard_loop
}

main "$@"
