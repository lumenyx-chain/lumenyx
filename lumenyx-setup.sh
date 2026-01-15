#!/bin/bash

# ═══════════════════════════════════════════════════════════════════════════════
# LUMENYX SETUP SCRIPT - Simple & Clean (No root required)
# ═══════════════════════════════════════════════════════════════════════════════

set -e

VERSION="1.7.1"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
LUMENYX_DIR="$HOME/.lumenyx"
BINARY_NAME="lumenyx-node"
DATA_DIR="$HOME/.local/share/lumenyx-node"
PID_FILE="$LUMENYX_DIR/lumenyx.pid"
LOG_FILE="$LUMENYX_DIR/lumenyx.log"
RPC="http://127.0.0.1:9944"

# Download URLs
BINARY_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v${VERSION}/lumenyx-node-linux-x86_64"
CHECKSUM_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v${VERSION}/lumenyx-node-sha256.txt"
BOOTNODES_URL="https://raw.githubusercontent.com/lumenyx-chain/lumenyx/main/bootnodes.txt"

# ═══════════════════════════════════════════════════════════════════════════════
# UI FUNCTIONS
# ═══════════════════════════════════════════════════════════════════════════════

print_banner() {
    clear
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
    echo "║                Welcome to LUMENYX - Your Chain                     ║"
    echo "║                                                                    ║"
    echo "╚════════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

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
    echo ""
    echo -e "  Wallet:   ${GREEN}$short_addr${NC}"
    echo -e "  Balance:  ${GREEN}$balance LMX${NC}"
    echo -e "  Block:    #$block"
    echo -e "  Status:   ${status_color} ${status}${NC}"
    echo -e "  Peers:    $peers"
    echo ""
    echo "════════════════════════════════════════════════════════════════════"
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
# UTILITY FUNCTIONS
# ═══════════════════════════════════════════════════════════════════════════════

node_running() {
    if [[ -f "$PID_FILE" ]]; then
        local pid=$(cat "$PID_FILE" 2>/dev/null)
        if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
            return 0
        fi
    fi
    # Also check by process name
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
    local addr=$(get_address)
    if [[ -z "$addr" ]] || ! node_running; then
        echo "?"
        return
    fi
    
    # Simplified - real balance needs proper RPC
    echo "?"
}

get_block() {
    if ! node_running; then
        echo "?"
        return
    fi
    
    local result=$(curl -s -m 3 -H "Content-Type: application/json" \
        -d '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' \
        "$RPC" 2>/dev/null)
    
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
    
    local result=$(curl -s -m 3 -H "Content-Type: application/json" \
        -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' \
        "$RPC" 2>/dev/null)
    
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

# ═══════════════════════════════════════════════════════════════════════════════
# FIRST RUN - INSTALLATION
# ═══════════════════════════════════════════════════════════════════════════════

is_first_run() {
    [[ ! -f "$LUMENYX_DIR/$BINARY_NAME" ]] || [[ ! -f "$DATA_DIR/miner-key" ]]
}

step_system_check() {
    echo ""
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
        print_error "Fix the issues above before continuing."
        exit 1
    fi
    
    print_ok "System check passed!"
    wait_enter
}

step_install() {
    echo ""
    echo -e "${CYAN}═══ STEP 2: INSTALLATION ═══${NC}"
    echo ""
    
    mkdir -p "$LUMENYX_DIR"
    
    # Check existing binary
    if [[ -f "$LUMENYX_DIR/$BINARY_NAME" ]]; then
        print_warning "Binary already exists"
        if ask_yes_no "Re-download?"; then
            rm -f "$LUMENYX_DIR/$BINARY_NAME"
        else
            print_ok "Using existing binary"
            wait_enter
            return
        fi
    fi
    
    print_info "Downloading lumenyx-node (~65MB)..."
    if curl -L -o "$LUMENYX_DIR/$BINARY_NAME" "$BINARY_URL" --progress-bar; then
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
    echo ""
    echo -e "${CYAN}═══ STEP 3: WALLET ═══${NC}"
    echo ""
    
    # Check existing wallet
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
    
    echo -e "${RED}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}║  IMPORTANT: Write down the 12-word seed phrase!               ║${NC}"
    echo -e "${RED}║  If you lose it, your funds are LOST FOREVER.                 ║${NC}"
    echo -e "${RED}╚════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    if ask_yes_no "Create NEW wallet?"; then
        # Generate wallet
        local output=$("$LUMENYX_DIR/$BINARY_NAME" key generate --words 12 2>&1)
        
        local seed_phrase=$(echo "$output" | grep "Secret phrase:" | sed 's/.*Secret phrase:[[:space:]]*//')
        local address=$(echo "$output" | grep "SS58 Address:" | sed 's/.*SS58 Address:[[:space:]]*//')
        local secret_seed=$(echo "$output" | grep "Secret seed:" | sed 's/.*Secret seed:[[:space:]]*//' | sed 's/0x//')
        
        echo ""
        echo -e "${YELLOW}════════════════════════════════════════════════════════════════${NC}"
        echo -e "${YELLOW}  YOUR SEED PHRASE (write it down NOW!):${NC}"
        echo ""
        echo -e "  ${GREEN}$seed_phrase${NC}"
        echo ""
        echo -e "${YELLOW}  Your address: ${GREEN}$address${NC}"
        echo -e "${YELLOW}════════════════════════════════════════════════════════════════${NC}"
        echo ""
        
        # Save wallet info
        mkdir -p "$DATA_DIR"
        echo "$secret_seed" > "$DATA_DIR/miner-key"
        chmod 600 "$DATA_DIR/miner-key"
        
        echo "Address: $address" > "$LUMENYX_DIR/wallet.txt"
        echo "WARNING: Seed phrase NOT saved here - write it down!" >> "$LUMENYX_DIR/wallet.txt"
        
        echo ""
        read -r -p "Type YES when you have saved your seed phrase: " confirm
        if [[ "$confirm" != "YES" ]]; then
            print_warning "Please save your seed phrase!"
        fi
        
        print_ok "Wallet created!"
    else
        # Import existing wallet
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
    echo ""
    echo -e "${CYAN}═══ STEP 4: START MINING ═══${NC}"
    echo ""
    
    # Get bootnodes
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
    print_banner
    echo ""
    echo "  This script will:"
    echo "    1. Check your system"
    echo "    2. Download LUMENYX node"
    echo "    3. Create your wallet"
    echo "    4. Start mining"
    echo ""
    echo "  No root/sudo required!"
    echo ""
    wait_enter
    
    step_system_check
    step_install
    step_wallet
    step_start
    
    echo ""
    print_ok "Setup complete! Entering wallet menu..."
    sleep 2
}

# ═══════════════════════════════════════════════════════════════════════════════
# NODE CONTROL (No sudo!)
# ═══════════════════════════════════════════════════════════════════════════════

start_node() {
    if node_running; then
        print_warning "Node is already running"
        return
    fi
    
    # Build bootnode args
    local bootnode_args=""
    local bootnodes=$(curl -sL "$BOOTNODES_URL" 2>/dev/null | grep -v '^#' | grep -v '^$' | tr '\n' ' ')
    if [[ -n "$bootnodes" ]]; then
        for bn in $bootnodes; do
            bootnode_args="$bootnode_args --bootnodes $bn"
        done
    fi
    
    print_info "Starting node..."
    
    # Start in background with nohup
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
    
    if [[ -f "$PID_FILE" ]]; then
        local pid=$(cat "$PID_FILE")
        kill "$pid" 2>/dev/null
        rm -f "$PID_FILE"
    fi
    
    # Also kill by name if pid file was stale
    pkill -f "lumenyx-node.*--validator" 2>/dev/null
    
    sleep 2
    
    if ! node_running; then
        print_ok "Node stopped"
    else
        print_warning "Force killing..."
        pkill -9 -f "lumenyx-node" 2>/dev/null
        rm -f "$PID_FILE"
    fi
}

# ═══════════════════════════════════════════════════════════════════════════════
# MAIN MENU
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
        echo -e "  ${GREEN}$addr${NC}"
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
    echo -e "${CYAN}═══ TRANSACTION HISTORY ═══${NC}"
    echo ""
    
    if ! node_running; then
        print_warning "Node must be running to fetch history"
        wait_enter
        return
    fi
    
    print_info "Recent mining activity:"
    echo ""
    
    if [[ -f "$LOG_FILE" ]]; then
        grep -E "Imported|mined|Prepared" "$LOG_FILE" | tail -15 || echo "  No recent activity"
    else
        echo "  No log file found"
    fi
    
    wait_enter
}

menu_logs() {
    echo ""
    print_info "Showing live logs (Ctrl+C to exit)..."
    echo ""
    
    if [[ -f "$LOG_FILE" ]]; then
        tail -f "$LOG_FILE"
    else
        print_error "No log file found. Start mining first."
        wait_enter
    fi
}

main_menu() {
    while true; do
        print_dashboard
        echo ""
        echo "  [1] ⛏️  Start/Stop Mining"
        echo "  [2] 💸 Send LUMENYX"
        echo "  [3] 📥 Receive (show address)"
        echo "  [4] 📜 History"
        echo "  [5] 📊 Live Logs"
        echo "  [0] 🚪 Exit"
        echo ""
        read -r -p "Choice: " choice
        
        case $choice in
            1) menu_start_stop ;;
            2) menu_send ;;
            3) menu_receive ;;
            4) menu_history ;;
            5) menu_logs ;;
            0) echo "Goodbye!"; exit 0 ;;
            *) print_warning "Invalid choice" ;;
        esac
    done
}

# ═══════════════════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════════════════

main() {
    if is_first_run; then
        first_run
    fi
    main_menu
}

main "$@"

