#!/bin/bash

# ═══════════════════════════════════════════════════════════════════════════════
# LUMENYX SETUP SCRIPT v1.8.0 - Simple & Clean (No root required)
# ═══════════════════════════════════════════════════════════════════════════════

set -e

VERSION="1.7.1"
SCRIPT_VERSION="1.8.0"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# Configuration
LUMENYX_DIR="$HOME/.lumenyx"
BINARY_NAME="lumenyx-node"
DATA_DIR="$HOME/.local/share/lumenyx-node"
PID_FILE="$LUMENYX_DIR/lumenyx.pid"
LOG_FILE="$LUMENYX_DIR/lumenyx.log"
RPC="http://127.0.0.1:9944"
RPC_TIMEOUT=5
RPC_RETRIES=3

# Download URLs
BINARY_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v${VERSION}/lumenyx-node-linux-x86_64"
CHECKSUM_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v${VERSION}/lumenyx-node-sha256.txt"
BOOTNODES_URL="https://raw.githubusercontent.com/lumenyx-chain/lumenyx/main/bootnodes.txt"

# ═══════════════════════════════════════════════════════════════════════════════
# UI FUNCTIONS
# ═══════════════════════════════════════════════════════════════════════════════

print_logo() {
    echo -e "${CYAN}"
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
# UTILITY FUNCTIONS
# ═══════════════════════════════════════════════════════════════════════════════

node_running() {
    if [[ -f "$PID_FILE" ]]; then
        local pid=$(cat "$PID_FILE" 2>/dev/null)
        if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
            return 0
        fi
    fi
    pgrep -f "lumenyx-node.*--validator" > /dev/null 2>&1
}

get_address() {
    if [[ -f "$LUMENYX_DIR/wallet.txt" ]]; then
        grep "Address:" "$LUMENYX_DIR/wallet.txt" 2>/dev/null | awk '{print $2}'
    elif [[ -f "$DATA_DIR/miner-key" ]]; then
        local seed=$(cat "$DATA_DIR/miner-key" 2>/dev/null)
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
    
    local addr=$(get_address)
    if [[ -z "$addr" ]]; then
        echo "?"
        return
    fi
    
    # Convert SS58 to hex account ID using the node
    local account_hex=$("$LUMENYX_DIR/$BINARY_NAME" key inspect "$addr" 2>/dev/null | grep "Account ID" | awk '{print $3}')
    
    if [[ -z "$account_hex" ]]; then
        echo "?"
        return
    fi
    
    # Remove 0x prefix
    account_hex="${account_hex#0x}"
    
    # Build storage key for System.Account
    local module_hash="26aa394eea5630e07c48ae0c9558cef7"
    local storage_hash="b99d880ec681799c0cf30e8886371da9"
    local key_hash=$(echo -n "$account_hex" | xxd -r -p | b2sum -l 128 | awk '{print $1}')
    local storage_key="0x${module_hash}${storage_hash}${key_hash}${account_hex}"
    
    local result=$(rpc_call "state_getStorage" "[\"$storage_key\"]")
    
    if [[ -z "$result" ]] || [[ "$result" == "null" ]]; then
        echo "0.000"
        return
    fi
    
    # Extract the free balance from AccountInfo
    local data=$(echo "$result" | grep -o '"result":"[^"]*"' | cut -d'"' -f4)
    
    if [[ -z "$data" ]] || [[ "$data" == "null" ]]; then
        echo "0.000"
        return
    fi
    
    # AccountInfo structure: nonce (4) + consumers (4) + providers (4) + sufficients (4) + free (16) + ...
    # Skip first 32 chars (16 bytes) to get to free balance
    local free_hex="${data:34:32}"
    
    if [[ -z "$free_hex" ]]; then
        echo "0.000"
        return
    fi
    
    # Convert little-endian hex to decimal
    local reversed=""
    for ((i=${#free_hex}-2; i>=0; i-=2)); do
        reversed+="${free_hex:$i:2}"
    done
    
    local balance_planck=$(printf "%d" "0x$reversed" 2>/dev/null || echo "0")
    
    # Convert from planck (12 decimals) to LMX
    if [[ "$balance_planck" -gt 0 ]]; then
        local balance_lmx=$(echo "scale=3; $balance_planck / 1000000000000" | bc 2>/dev/null || echo "0.000")
        echo "$balance_lmx"
    else
        echo "0.000"
    fi
}

get_block() {
    if ! node_running; then
        echo "offline"
        return
    fi
    
    local result=$(rpc_call "chain_getHeader")
    
    if [[ -z "$result" ]]; then
        echo "?"
        return
    fi
    
    local hex=$(echo "$result" | grep -o '"number":"[^"]*"' | cut -d'"' -f4)
    if [[ -n "$hex" ]]; then
        printf "%d" "$hex" 2>/dev/null || echo "?"
    else
        echo "?"
    fi
}

get_peers() {
    if ! node_running; then
        echo "0"
        return
    fi
    
    local result=$(rpc_call "system_health")
    
    if [[ -z "$result" ]]; then
        echo "0"
        return
    fi
    
    local peers=$(echo "$result" | grep -o '"peers":[0-9]*' | cut -d':' -f2)
    echo "${peers:-0}"
}

get_bootnodes() {
    local bootnodes=$(curl -sL "$BOOTNODES_URL" 2>/dev/null | grep -v '^#' | grep -v '^$' | tr '\n' ' ')
    
    if [[ -z "$bootnodes" ]]; then
        print_warning "No bootnodes found in repository."
        echo ""
        read -r -p "Enter bootnode manually (or ENTER to skip): " manual
        if [[ -n "$manual" ]]; then
            echo "$manual"
        fi
    else
        echo "$bootnodes"
    fi
}

has_existing_data() {
    [[ -d "$LUMENYX_DIR" ]] || [[ -d "$DATA_DIR" ]]
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
    echo "  Found existing LUMENYX data on this machine:"
    echo ""
    [[ -d "$LUMENYX_DIR" ]] && echo -e "    ${CYAN}•${NC} $LUMENYX_DIR (binary, config, logs)"
    [[ -d "$DATA_DIR" ]] && echo -e "    ${CYAN}•${NC} $DATA_DIR (blockchain data, wallet)"
    echo ""
    echo -e "  ${GREEN}RECOMMENDED:${NC} Clean install for best experience"
    echo ""
    echo -e "  ${RED}⚠️  WARNING: This will delete your existing wallet!${NC}"
    echo -e "  ${RED}   Make sure you have saved your seed phrase!${NC}"
    echo ""
    
    if ask_yes_no "Perform clean install?"; then
        print_info "Cleaning existing data..."
        
        # Stop node if running
        if node_running; then
            print_info "Stopping running node..."
            if [[ -f "$PID_FILE" ]]; then
                local pid=$(cat "$PID_FILE")
                kill "$pid" 2>/dev/null
            fi
            pkill -f "lumenyx-node" 2>/dev/null || true
            sleep 2
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
    
    if command -v curl &> /dev/null; then
        print_ok "curl: installed"
    else
        print_error "curl not found"
        errors=$((errors + 1))
    fi
    
    if command -v bc &> /dev/null; then
        print_ok "bc: installed"
    else
        print_warning "bc not found (balance display may not work)"
    fi
    
    if curl -s --connect-timeout 5 https://github.com > /dev/null 2>&1; then
        print_ok "Internet: OK"
    else
        print_error "Cannot reach GitHub"
        errors=$((errors + 1))
    fi
    
    local available=$(df -BG "$HOME" 2>/dev/null | awk 'NR==2 {print $4}' | tr -d 'G')
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
    local expected=$(curl -sL "$CHECKSUM_URL" | awk '{print $1}')
    local actual=$(sha256sum "$LUMENYX_DIR/$BINARY_NAME" | awk '{print $1}')
    
    if [[ -n "$expected" ]] && [[ "$expected" == "$actual" ]]; then
        print_ok "Checksum verified"
    else
        print_warning "Checksum verification skipped"
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
        local addr=$(get_address)
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
        
        local output=$("$LUMENYX_DIR/$BINARY_NAME" key generate --words 12 2>&1)
        
        local seed_phrase=$(echo "$output" | grep "Secret phrase:" | sed 's/.*Secret phrase:[[:space:]]*//')
        local address=$(echo "$output" | grep "SS58 Address:" | sed 's/.*SS58 Address:[[:space:]]*//')
        local secret_seed=$(echo "$output" | grep "Secret seed:" | sed 's/.*Secret seed:[[:space:]]*//' | sed 's/0x//')
        
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
        
        local output=$("$LUMENYX_DIR/$BINARY_NAME" key inspect "$seed_phrase" 2>&1)
        local address=$(echo "$output" | grep "SS58 Address:" | sed 's/.*SS58 Address:[[:space:]]*//')
        local secret_seed=$(echo "$output" | grep "Secret seed:" | sed 's/.*Secret seed:[[:space:]]*//' | sed 's/0x//')
        
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
    
    local bootnode_args=""
    local bootnodes=$(curl -sL "$BOOTNODES_URL" 2>/dev/null | grep -v '^#' | grep -v '^$' | tr '\n' ' ')
    if [[ -n "$bootnodes" ]]; then
        for bn in $bootnodes; do
            bootnode_args="$bootnode_args --bootnodes $bn"
        done
    fi
    
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
        print_ok "Mining started! (PID: $(cat $PID_FILE))"
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
    
    # Method 1: Kill by PID file
    if [[ -f "$PID_FILE" ]]; then
        local pid=$(cat "$PID_FILE")
        if [[ -n "$pid" ]]; then
            kill -TERM "$pid" 2>/dev/null
            sleep 1
            kill -KILL "$pid" 2>/dev/null
        fi
        rm -f "$PID_FILE"
    fi
    
    # Method 2: Kill by process name (aggressive)
    pkill -TERM -f "lumenyx-node" 2>/dev/null
    sleep 1
    pkill -KILL -f "lumenyx-node" 2>/dev/null
    
    sleep 1
    
    # Verify
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
    local addr=$(get_address)
    local short_addr=""
    if [[ -n "$addr" ]]; then
        short_addr="${addr:0:8}...${addr: -6}"
    else
        short_addr="Not set"
    fi
    
    local balance=$(get_balance)
    local block=$(get_block)
    local peers=$(get_peers)
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
    echo -e "  Balance:  ${GREEN}$balance LMX${NC}"
    echo -e "  Block:    #$block"
    echo -e "  Status:   ${status_color} ${status}${NC}"
    echo -e "  Peers:    $peers"
    echo ""
    echo "════════════════════════════════════════════════════════════════════"
}

dashboard_loop() {
    local last_input=""
    
    while true; do
        print_dashboard
        echo ""
        echo "  [1] ⛏️  Start/Stop Mining"
        echo "  [2] 💸 Send LUMENYX"
        echo "  [3] 📥 Receive (show address)"
        echo "  [4] 📜 History"
        echo "  [5] 📊 Live Logs"
        echo "  [6] 🛠️  Useful Commands"
        echo "  [0] 🚪 Exit"
        echo ""
        echo -e "  ${CYAN}Auto-refresh in 10s - Press a key to select${NC}"
        echo ""
        
        # Read with timeout for auto-refresh
        read -r -t 10 -n 1 choice || choice="refresh"
        
        case $choice in
            1) 
                echo ""
                menu_start_stop 
                ;;
            2) 
                echo ""
                menu_send 
                ;;
            3) 
                echo ""
                menu_receive 
                ;;
            4) 
                echo ""
                menu_history 
                ;;
            5) 
                echo ""
                menu_logs 
                ;;
            6) 
                echo ""
                menu_commands 
                ;;
            0) 
                echo ""
                echo "Goodbye!"
                exit 0 
                ;;
            "refresh")
                # Auto-refresh, just continue loop
                ;;
            *)
                # Any other key, just refresh
                ;;
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
    print_dashboard
    echo ""
    echo -e "${CYAN}═══ SEND LUMENYX ═══${NC}"
    echo ""
    
    if ! node_running; then
        print_error "Node must be running to send transactions"
        wait_enter
        return
    fi
    
    read -r -p "Recipient address: " recipient
    if [[ -z "$recipient" ]]; then
        print_warning "Cancelled"
        wait_enter
        return
    fi
    
    read -r -p "Amount (LMX): " amount
    if [[ -z "$amount" ]]; then
        print_warning "Cancelled"
        wait_enter
        return
    fi
    
    echo ""
    echo "  To: $recipient"
    echo "  Amount: $amount LMX"
    echo ""
    
    if ask_yes_no "Confirm transaction?"; then
        print_info "Sending transaction..."
        print_warning "Send feature coming soon - use polkadot.js for now"
    fi
    
    wait_enter
}

menu_receive() {
    print_dashboard
    echo ""
    echo -e "${CYAN}═══ RECEIVE LUMENYX ═══${NC}"
    echo ""
    
    local addr=$(get_address)
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

menu_logs() {
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


menu_commands() {
    clear
    print_logo
    echo ""
    echo -e "${CYAN}═══ USEFUL COMMANDS ═══${NC}"
    echo ""
    echo -e "  ${YELLOW}🧹 CLEAN INSTALL (reset everything):${NC}"
    echo "     rm -rf ~/.lumenyx ~/.local/share/lumenyx*"
    echo ""
    echo -e "  ${YELLOW}📋 VIEW FULL LOGS:${NC}"
    echo "     tail -100 ~/.lumenyx/lumenyx.log"
    echo ""
    echo -e "  ${YELLOW}🔍 FIND YOUR PEER ID:${NC}"
    echo '     grep "Local node identity" ~/.lumenyx/lumenyx.log'
    echo ""
    echo -e "  ${YELLOW}🔄 UPDATE SCRIPT:${NC}"
    echo "     curl -O https://raw.githubusercontent.com/lumenyx-chain/lumenyx/main/lumenyx-setup.sh"
    echo ""
    echo -e "  ${YELLOW}🌐 POLKADOT.JS EXPLORER:${NC}"
    echo "     https://polkadot.js.org/apps/?rpc=ws://YOUR_IP:9944"
    echo ""
    wait_enter
}

# ═══════════════════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════════════════

main() {
    # Check for existing data on first run
    if is_first_run && has_existing_data; then
        prompt_clean_install
    fi
    
    # Run first-time setup if needed
    if is_first_run; then
        first_run
    fi
    
    # Enter dashboard
    dashboard_loop
}

main "$@"


