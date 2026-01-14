#!/usr/bin/env bash
set -euo pipefail

VERSION="1.7.1"

# ---------- Paths ----------
LUMENYX_DIR="$HOME/.lumenyx"
BIN="$LUMENYX_DIR/lumenyx-node"
KEYS="$LUMENYX_DIR/keys"
WALLET_TXT="$LUMENYX_DIR/wallet.txt"

BASE_PATH="$HOME/.local/share/lumenyx-node"
MINER_KEY_FILE="$BASE_PATH/miner-key"

SERVICE="lumenyx.service"
SERVICE_FILE="/etc/systemd/system/lumenyx.service"

ETC_DIR="/etc/lumenyx"
BOOTFILE="$ETC_DIR/bootnodes.txt"
ENVFILE="$ETC_DIR/node.env"
BOOTGEN="/usr/local/bin/lumenyx-bootnodes.sh"

RPC="http://127.0.0.1:9944"

# Bootnode (Iceland)
OFFICIAL_BOOTNODE="/ip4/89.147.111.102/tcp/30333/p2p/12D3KooWNWLGaBDB9WwCTuG4fDT2rb3AY4WaweF6TBF4YWgZTtrY"

# Download URLs
BINARY_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v${VERSION}/lumenyx-node-linux-x86_64"
CHECKSUM_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v${VERSION}/lumenyx-node-sha256.txt"

# ---------- Colors ----------
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
ok(){ echo -e "${GREEN}✓${NC} $*"; }
warn(){ echo -e "${YELLOW}!${NC} $*"; }
die(){ echo -e "${RED}✗${NC} $*" >&2; exit 1; }
pause(){ echo; read -r -p "Press ENTER to continue..."; }

banner(){
  clear
  echo -e "${CYAN}"
  echo "╔══════════════════════════════════════════════════════════════╗"
  echo "║                                                              ║"
  echo "║   ██╗     ██╗   ██╗███╗   ███╗███████╗███╗   ██╗██╗   ██╗   ║"
  echo "║   ██║     ██║   ██║████╗ ████║██╔════╝████╗  ██║╚██╗ ██╔╝   ║"
  echo "║   ██║     ██║   ██║██╔████╔██║█████╗  ██╔██╗ ██║ ╚████╔╝    ║"
  echo "║   ██║     ██║   ██║██║╚██╔╝██║██╔══╝  ██║╚██╗██║  ╚██╔╝     ║"
  echo "║   ███████╗╚██████╔╝██║ ╚═╝ ██║███████╗██║ ╚████║   ██║      ║"
  echo "║   ╚══════╝ ╚═════╝ ╚═╝     ╚═╝╚══════╝╚═╝  ╚═══╝   ╚═╝      ║"
  echo "║                                                              ║"
  echo "║              Peer-to-Peer Electronic Cash                    ║"
  printf "║                     Version %-6s                           ║\n" "$VERSION"
  echo "╚══════════════════════════════════════════════════════════════╝"
  echo -e "${NC}"
}

have(){ command -v "$1" >/dev/null 2>&1; }
need(){ have "$1" || die "Missing: $1"; }

ensure_dirs(){
  mkdir -p "$LUMENYX_DIR" "$KEYS" "$BASE_PATH"
  chmod 700 "$KEYS"
}

ask_yes_no(){
  while true; do
    read -r -p "$1 [y/n]: " a
    case "$a" in
      y|Y) return 0 ;; n|N) return 1 ;; *) echo "y or n" ;;
    esac
  done
}

rpc_call(){
  curl -s -H 'Content-Type: application/json' \
    -d "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"$1\",\"params\":${2:-[]}}" \
    "$RPC" 2>/dev/null || echo "{}"
}

shorten(){
  local s="$1" n="${2:-8}"
  [[ -z "$s" ]] && echo "-" && return
  [[ ${#s} -le $((n*2+3)) ]] && echo "$s" && return
  echo "${s:0:n}...${s: -n}"
}

first_run_needed(){
  [[ ! -x "$BIN" ]] && return 0
  [[ ! -f "$MINER_KEY_FILE" ]] && return 0
  return 1
}

service_active(){ systemctl is-active --quiet "$SERVICE" 2>/dev/null; }
start_service(){ sudo systemctl restart "$SERVICE"; }
stop_service(){ sudo systemctl stop "$SERVICE" 2>/dev/null || true; }

# ==============================================================================
# Download & Install
# ==============================================================================
check_requirements(){
  banner
  echo "  Checking requirements..."
  echo ""
  need curl; need sha256sum; need sudo; need systemctl
  ok "All dependencies found"
  
  if grep -qE 'aes' /proc/cpuinfo 2>/dev/null; then
    ok "CPU: AES-NI supported"
  else
    warn "CPU: AES-NI not detected"
  fi
  
  local ram=$(awk '/MemTotal/{print int($2/1024)}' /proc/meminfo 2>/dev/null)
  ok "RAM: ${ram:-?}MB"
  pause
}

download_binary(){
  ensure_dirs
  banner
  echo "  Downloading LUMENYX node v${VERSION}..."
  echo ""

  curl -L -o "$BIN.tmp" "$BINARY_URL" --progress-bar || die "Download failed"
  curl -sL -o "$LUMENYX_DIR/sha256.txt" "$CHECKSUM_URL" || die "Checksum download failed"

  local expected=$(awk '{print $1}' "$LUMENYX_DIR/sha256.txt" | head -1)
  local actual=$(sha256sum "$BIN.tmp" | awk '{print $1}')
  
  [[ "$expected" == "$actual" ]] || die "Checksum mismatch!"
  
  mv "$BIN.tmp" "$BIN"
  chmod +x "$BIN"
  rm -f "$LUMENYX_DIR/sha256.txt"
  
  ok "Binary installed: $BIN"
  pause
}

# ==============================================================================
# Wallet Creation
# ==============================================================================
create_wallet(){
  banner
  echo "  Creating your wallet..."
  echo ""

  local output mnemonic address seed_hex
  output=$("$BIN" key generate --scheme Sr25519 --words 12 2>&1) || die "Key generation failed"
  
  # Parse output - handle different formats
  mnemonic=$(echo "$output" | grep -i "phrase" | sed 's/.*: *//' | sed 's/^ *//' | head -1)
  [[ -z "$mnemonic" ]] && mnemonic=$(echo "$output" | tail -n +2 | head -1 | sed 's/^ *//')
  [[ -z "$mnemonic" ]] && die "Failed to generate mnemonic"

  # Get seed from mnemonic
  local inspect=$("$BIN" key inspect "$mnemonic" 2>&1) || die "Key inspect failed"
  
  address=$(echo "$inspect" | grep -E "(SS58|Public key \(SS58\))" | sed 's/.*: *//' | head -1)
  seed_hex=$(echo "$inspect" | grep -i "Secret seed" | sed 's/.*0x//' | head -1)
  
  [[ -z "$address" ]] && die "Failed to get address"
  [[ -z "$seed_hex" ]] && die "Failed to get seed"

  # Show seed phrase
  echo ""
  echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════════╗${NC}"
  echo -e "${CYAN}║  ${YELLOW}YOUR SEED PHRASE - WRITE THIS DOWN!${CYAN}                                ║${NC}"
  echo -e "${CYAN}╠══════════════════════════════════════════════════════════════════════╣${NC}"
  echo -e "${CYAN}║${NC}"
  echo -e "${CYAN}║${NC}  ${GREEN}$mnemonic${NC}"
  echo -e "${CYAN}║${NC}"
  echo -e "${CYAN}╠══════════════════════════════════════════════════════════════════════╣${NC}"
  echo -e "${CYAN}║${NC}  Mining Address: ${GREEN}$address${NC}"
  echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════════╝${NC}"
  echo ""
  echo -e "${RED}⚠️  This is your ONLY way to recover your wallet!${NC}"
  echo ""

  read -r -p "Type 'YES' to confirm you saved it: " confirm
  [[ "$confirm" != "YES" ]] && die "Please save your seed phrase first"

  # Save miner-key (32 bytes hex, no 0x)
  echo "$seed_hex" > "$MINER_KEY_FILE"
  chmod 600 "$MINER_KEY_FILE"
  
  # Save wallet info
  echo "$address" > "$KEYS/address.txt"
  cat > "$WALLET_TXT" <<EOF
LUMENYX Wallet
Mining Address: $address

YOUR SEED PHRASE (12 words):
$mnemonic
EOF
  chmod 600 "$WALLET_TXT" "$KEYS/address.txt"

  ok "Wallet created: $address"
  pause
}

import_wallet(){
  banner
  echo "  Import existing wallet"
  echo ""
  read -r -p "Enter 12-word seed phrase: " mnemonic
  
  local inspect=$("$BIN" key inspect "$mnemonic" 2>&1) || die "Invalid seed phrase"
  
  local address=$(echo "$inspect" | grep -E "(SS58|Public key \(SS58\))" | sed 's/.*: *//' | head -1)
  local seed_hex=$(echo "$inspect" | grep -i "Secret seed" | sed 's/.*0x//' | head -1)
  
  [[ -z "$address" || -z "$seed_hex" ]] && die "Failed to parse seed phrase"

  echo "$seed_hex" > "$MINER_KEY_FILE"
  chmod 600 "$MINER_KEY_FILE"
  
  echo "$address" > "$KEYS/address.txt"
  cat > "$WALLET_TXT" <<EOF
LUMENYX Wallet (Imported)
Mining Address: $address
EOF
  chmod 600 "$WALLET_TXT" "$KEYS/address.txt"

  ok "Imported: $address"
  pause
}

# ==============================================================================
# Systemd Service
# ==============================================================================
install_systemd(){
  sudo mkdir -p "$ETC_DIR"
  
  # Bootnodes file
  [[ ! -f "$BOOTFILE" ]] && echo "$OFFICIAL_BOOTNODE" | sudo tee "$BOOTFILE" >/dev/null
  
  # Environment file
  [[ ! -f "$ENVFILE" ]] && sudo tee "$ENVFILE" >/dev/null <<EOF
LUMENYX_NAME="lumenyx-node"
LUMENYX_ROLE="--validator"
EOF

  # Bootnode generator script
  sudo tee "$BOOTGEN" >/dev/null <<'SCRIPT'
#!/bin/bash
[[ -f /etc/lumenyx/bootnodes.txt ]] || exit 0
while read -r line; do
  [[ -z "$line" || "$line" =~ ^# ]] && continue
  echo -n "--bootnodes $line "
done < /etc/lumenyx/bootnodes.txt
SCRIPT
  sudo chmod +x "$BOOTGEN"

  # Service file
  sudo tee "$SERVICE_FILE" >/dev/null <<EOF
[Unit]
Description=LUMENYX Node
After=network.target

[Service]
Type=simple
User=root
EnvironmentFile=$ENVFILE
ExecStart=/bin/bash -c '$BIN --base-path $BASE_PATH --chain mainnet --name "\$LUMENYX_NAME" \$LUMENYX_ROLE --rpc-cors all --unsafe-rpc-external --rpc-methods Safe \$($BOOTGEN)'
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

  sudo systemctl daemon-reload
  sudo systemctl enable "$SERVICE" >/dev/null 2>&1
  ok "Service installed"
}

# ==============================================================================
# First Run
# ==============================================================================
first_run(){
  check_requirements
  download_binary
  
  banner
  echo "  Wallet Setup"
  echo ""
  echo "  [1] Create NEW wallet"
  echo "  [2] Import EXISTING wallet"
  echo ""
  read -r -p "Choice [1/2]: " c
  
  case "$c" in
    2) import_wallet ;;
    *) create_wallet ;;
  esac
  
  banner
  echo "  Node Configuration"
  echo ""
  read -r -p "Node name [lumenyx-node]: " name
  name="${name:-lumenyx-node}"
  
  echo ""
  echo "  [1] Mining mode (earn LUMENYX)"
  echo "  [2] Sync only (no mining)"
  read -r -p "Choice [1/2]: " mode
  
  local role="--validator"
  [[ "$mode" == "2" ]] && role=""
  
  sudo mkdir -p "$ETC_DIR"
  sudo tee "$ENVFILE" >/dev/null <<EOF
LUMENYX_NAME="$name"
LUMENYX_ROLE="$role"
EOF

  install_systemd
  
  banner
  echo "  Starting LUMENYX..."
  echo ""
  start_service
  sleep 3
  
  if service_active; then
    ok "Node started!"
    ok "Mining to: $(cat "$KEYS/address.txt" 2>/dev/null || echo "unknown")"
  else
    warn "Node may have issues - check logs"
  fi
  pause
  
  main_menu
}

# ==============================================================================
# Dashboard
# ==============================================================================
get_status(){
  local addr block peers diff status
  
  addr=$(cat "$KEYS/address.txt" 2>/dev/null || echo "Not set")
  
  if service_active; then
    status="● MINING"
    local hdr=$(rpc_call chain_getHeader)
    local num=$(echo "$hdr" | grep -oP '"number"\s*:\s*"0x\K[0-9a-fA-F]+' | head -1)
    block="${num:+$((16#$num))}"
    block="${block:-?}"
    
    local health=$(rpc_call system_health)
    peers=$(echo "$health" | grep -oP '"peers"\s*:\s*\K[0-9]+' | head -1)
    peers="${peers:-0}"
    
    diff=$(sudo journalctl -u "$SERVICE" -n 30 --no-pager 2>/dev/null | grep -oP 'difficulty=\K[0-9]+' | tail -1)
    diff="${diff:-?}"
  else
    status="○ STOPPED"
    block="?" peers="0" diff="?"
  fi
  
  echo "addr=$addr"
  echo "block=$block"
  echo "peers=$peers"
  echo "diff=$diff"
  echo "status=$status"
}

render_dashboard(){
  local addr block peers diff status
  eval "$(get_status)"
  
  banner
  printf "  Address:  %s\n" "$(shorten "$addr" 12)"
  printf "  Block:    #%s\n" "$block"
  printf "  Status:   %s (diff: %s)\n" "$status" "$diff"
  printf "  Network:  %s peers\n" "$peers"
  echo "════════════════════════════════════════════════════════════════"
}

# ==============================================================================
# Menus
# ==============================================================================
menu_mining(){
  while true; do
    render_dashboard
    echo ""
    echo "  [1] Start mining"
    echo "  [2] Stop mining"
    echo "  [3] Status details"
    echo "  [0] Back"
    echo ""
    read -r -p "Choice: " c
    case "$c" in
      1) service_active && warn "Already running" || { start_service; sleep 2; ok "Started"; }; pause ;;
      2) service_active && { stop_service; ok "Stopped"; } || warn "Already stopped"; pause ;;
      3) sudo systemctl status "$SERVICE" --no-pager || true; pause ;;
      0) return ;;
    esac
  done
}

menu_logs(){
  render_dashboard
  echo ""
  echo "  Live logs (Ctrl+C to return)..."
  echo ""
  sudo journalctl -u "$SERVICE" -f --no-hostname -n 50 || true
}

menu_wallet(){
  while true; do
    render_dashboard
    echo ""
    echo "  [1] Show address"
    echo "  [2] Show miner-key path"
    echo "  [3] View wallet.txt"
    echo "  [4] Import wallet (replaces current!)"
    echo "  [0] Back"
    echo ""
    read -r -p "Choice: " c
    case "$c" in
      1) banner; echo "  Address: $(cat "$KEYS/address.txt" 2>/dev/null || echo "Not set")"; pause ;;
      2) banner; echo "  Miner-key: $MINER_KEY_FILE"; [[ -f "$MINER_KEY_FILE" ]] && echo "  Status: EXISTS" || echo "  Status: MISSING"; pause ;;
      3) banner; [[ -f "$WALLET_TXT" ]] && cat "$WALLET_TXT" || echo "  Not found"; pause ;;
      4) warn "This replaces your wallet!"; ask_yes_no "Continue?" && import_wallet ;;
      0) return ;;
    esac
  done
}

menu_network(){
  render_dashboard
  echo ""
  echo "  system_health:"
  rpc_call system_health | python3 -m json.tool 2>/dev/null || rpc_call system_health
  echo ""
  pause
}

menu_settings(){
  while true; do
    render_dashboard
    echo ""
    echo "  [1] Edit bootnodes"
    echo "  [2] Restart node"
    echo "  [3] Reset chain (keep wallet)"
    echo "  [4] Full reset (NEW wallet)"
    echo "  [5] Update binary"
    echo "  [0] Back"
    echo ""
    read -r -p "Choice: " c
    case "$c" in
      1) sudo "${EDITOR:-nano}" "$BOOTFILE"; pause ;;
      2) start_service; ok "Restarted"; pause ;;
      3)
        warn "Deletes chain data, keeps wallet"
        read -r -p "Type RESET: " x
        [[ "$x" == "RESET" ]] && { stop_service; rm -rf "$BASE_PATH/chains"; ok "Reset"; start_service; } || warn "Cancelled"
        pause ;;
      4)
        echo -e "${RED}This DELETES your wallet!${NC}"
        read -r -p "Type DELETE: " x
        [[ "$x" == "DELETE" ]] && { stop_service; rm -rf "$BASE_PATH" "$KEYS"/* "$WALLET_TXT"; ok "Full reset. Run script again."; exit 0; } || warn "Cancelled"
        pause ;;
      5)
        ask_yes_no "Update to v$VERSION?" && { stop_service; download_binary; install_systemd; start_service; ok "Updated"; }
        pause ;;
      0) return ;;
    esac
  done
}

main_menu(){
  while true; do
    render_dashboard
    echo ""
    echo "  [1] ⛏️  Mining Control"
    echo "  [2] 📊 Live Logs"
    echo "  [3] 🔑 Wallet Info"
    echo "  [4] 🌐 Network Info"
    echo "  [5] ⚙️  Settings"
    echo "  [0] 🚪 Exit"
    echo ""
    read -r -p "Choice: " c
    case "$c" in
      1) menu_mining ;;
      2) menu_logs ;;
      3) menu_wallet ;;
      4) menu_network ;;
      5) menu_settings ;;
      0) echo "Goodbye!"; exit 0 ;;
    esac
  done
}

# ==============================================================================
# Entry
# ==============================================================================
main(){
  ensure_dirs
  first_run_needed && first_run || main_menu
}

main "$@"
