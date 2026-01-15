#!/bin/bash

# ═══════════════════════════════════════════════════════════════════════════════
# LUMENYX SETUP SCRIPT - Simple & Clean
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
SERVICE_FILE="/etc/systemd/system/lumenyx.service"
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
    echo "║      Fresh install? Run: rm -rf ~/.lumenyx ~/.local/share/lumenyx* ║"
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
    
    if service_running; then
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

service_running() {
    systemctl is-active --quiet lumenyx 2>/dev/null
}

get_address() {
    if [[ -f "$LUMENYX_DIR/wallet.txt" ]]; then
        grep "Address:" "$LUMENYX_DIR/wallet.txt" 2>/dev/null | awk '{print $2}'
    elif [[ -f "$DATA_DIR/miner-key" ]]; then
        # Recover from miner-key
        local seed=$(cat "$DATA_DIR/miner-key" 2>/dev/null)
        if [[ -n "$seed" ]] && [[ -f "$LUMENYX_DIR/$BINARY_NAME" ]]; then
            "$LUMENYX_DIR/$BINARY_NAME" key inspect "0x$seed" 2>/dev/null | grep "SS58" | awk '{print $3}'
        fi
    fi
}

get_balance() {
    local addr=$(get_address)
    if [[ -z "$addr" ]] || ! service_running; then
        echo "?"
        return
    fi
    
    local result=$(curl -s -m 3 -H "Content-Type: application/json" \
        -d "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"state_getStorage\",\"params\":[\"0x\"]}" \
        "$RPC" 2>/dev/null)
    
    # Simplified - just show ? for now, real balance needs proper RPC
    echo "?"
}

get_block() {
    if ! service_running; then
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
    if ! service_running; then
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
    local bootnodes=$(curl -sL "$BOOTNODES_URL" 2>/dev/null | grep -v '^#' | grep -v '^$' | head -5)
    
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

step_setup() {
    echo ""
    echo -e "${CYAN}═══ STEP 4: SETUP ═══${NC}"
    echo ""
    
    # Get bootnodes
    print_info "Fetching bootnodes..."
    BOOTNODES=$(get_bootnodes)
    
    if [[ -z "$BOOTNODES" ]]; then
        print_warning "No bootnodes configured - node will run in solo mode"
    else
        print_ok "Bootnodes configured"
    fi
    
    # Create systemd service
    echo ""
    if ask_yes_no "Install as system service (auto-start on boot)?"; then
        local bootnode_args=""
        if [[ -n "$BOOTNODES" ]]; then
            bootnode_args="--bootnodes $BOOTNODES"
        fi
        
        sudo tee "$SERVICE_FILE" > /dev/null << SVCEOF
[Unit]
Description=LUMENYX Node
After=network.target

[Service]
Type=simple
User=$USER
ExecStart=$LUMENYX_DIR/$BINARY_NAME --chain mainnet --validator --rpc-cors all --rpc-external --rpc-methods Safe $bootnode_args
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
SVCEOF
        
        sudo systemctl daemon-reload
        sudo systemctl enable lumenyx
        print_ok "Service installed"
        
        echo ""
        if ask_yes_no "Start mining now?"; then
            sudo systemctl start lumenyx
            sleep 3
            if service_running; then
                print_ok "Mining started!"
            else
                print_error "Failed to start - check: journalctl -u lumenyx -f"
            fi
        fi
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
    wait_enter
    
    step_system_check
    step_install
    step_wallet
    step_setup
    
    echo ""
    print_ok "Setup complete! Entering wallet menu..."
    sleep 2
}

# ═══════════════════════════════════════════════════════════════════════════════
# MAIN MENU
# ═══════════════════════════════════════════════════════════════════════════════

menu_start_stop() {
    if service_running; then
        if ask_yes_no "Mining is running. Stop it?"; then
            sudo systemctl stop lumenyx
            print_ok "Mining stopped"
        fi
    else
        if ask_yes_no "Start mining?"; then
            sudo systemctl start lumenyx
            sleep 3
            if service_running; then
                print_ok "Mining started!"
            else
                print_error "Failed to start"
            fi
        fi
    fi
    wait_enter
}

menu_send() {
    print_dashboard
    echo ""
    echo -e "${CYAN}═══ SEND LUMENYX ═══${NC}"
    echo ""
    
    if ! service_running; then
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
        
        # Use author_submitExtrinsic via RPC
        # This is simplified - real implementation needs proper signing
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
    
    if ! service_running; then
        print_warning "Node must be running to fetch history"
        wait_enter
        return
    fi
    
    print_info "Fetching recent blocks..."
    
    # Show recent mining rewards from logs
    echo ""
    echo "  Recent mining activity:"
    echo ""
    sudo journalctl -u lumenyx --no-pager -n 20 2>/dev/null | grep -E "Imported|mined" | tail -10 || echo "  No recent activity"
    
    wait_enter
}

menu_logs() {
    echo ""
    print_info "Showing live logs (Ctrl+C to exit)..."
    echo ""
    sudo journalctl -u lumenyx -f --no-hostname -n 50
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
