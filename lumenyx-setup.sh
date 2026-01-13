#!/bin/bash

# ═══════════════════════════════════════════════════════════════════════════════
# LUMENYX SETUP SCRIPT v1.5
# ═══════════════════════════════════════════════════════════════════════════════

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
LUMENYX_DIR="$HOME/.lumenyx"
BINARY_NAME="lumenyx-node-linux-x86_64"
BINARY_FINAL="lumenyx-node"
BINARY_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v1.7.0/lumenyx-node-linux-x86_64"
CHECKSUM_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v1.7.0/lumenyx-node-sha256.txt"
BOOTNODE="/ip4/89.147.111.102/tcp/30333/p2p/12D3KooWRz3czuibWRMthonHp1jbysuJ89duArvkEAYdef7SRJec"
GITHUB_REPO="https://github.com/lumenyx-chain/lumenyx.git"
SERVICE_FILE="/etc/systemd/system/lumenyx.service"
DATA_DIR="$HOME/.local/share/lumenyx-node"

print_banner() {
    echo -e "${CYAN}"
    echo "╔═══════════════════════════════════════════════════════════════════╗"
    echo "║                      L U M E N Y X                                ║"
    echo "║              PoW + EVM + 21M Supply + Fair Launch                 ║"
    echo "╚═══════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

print_ok() { echo -e "${GREEN}✓ $1${NC}"; }
print_error() { echo -e "${RED}✗ $1${NC}"; }
print_warning() { echo -e "${YELLOW}⚠ $1${NC}"; }
print_info() { echo -e "${CYAN}ℹ $1${NC}"; }

wait_enter() {
    echo ""
    echo -e "${YELLOW}Press ENTER to continue...${NC}"
    read -r
}

ask_yes_no() {
    while true; do
        echo -ne "${YELLOW}$1 [y/n]: ${NC}"
        read -r answer
        case $answer in
            [Yy]|[Yy][Ee][Ss] ) return 0;;
            [Nn]|[Nn][Oo] ) return 1;;
            * ) echo "Please answer y or n";;
        esac
    done
}

ask_confirm() {
    while true; do
        echo -ne "${RED}$1 Type 'YES' to confirm: ${NC}"
        read -r answer
        case $answer in
            [Yy][Ee][Ss] ) return 0;;
            * ) echo "Please type YES to confirm";;
        esac
    done
}

# STEP 0: PRE-CHECKS
step_prechecks() {
    echo ""
    echo -e "${BLUE}═══ PRE-FLIGHT CHECKS ═══${NC}"
    echo ""
    
    # Check systemd service
    if systemctl is-enabled lumenyx &>/dev/null; then
        print_warning "LUMENYX systemd service is enabled (auto-restart on)"
        if ask_yes_no "Disable it before continuing?"; then
            systemctl stop lumenyx 2>/dev/null || true
            systemctl disable lumenyx 2>/dev/null || true
            print_ok "Service disabled"
        else
            print_error "Cannot continue with service enabled. Exiting."
            exit 1
        fi
    fi
    
    # Check running process
    if pgrep -x "lumenyx-node-linux-x86_64" > /dev/null; then
        PID=$(pgrep -x "lumenyx-node-linux-x86_64")
        print_warning "LUMENYX is already running (PID: $PID)"
        if ask_yes_no "Stop it before continuing?"; then
            pkill -9 lumenyx-node 2>/dev/null || true
            sleep 2
            print_ok "Process stopped"
        else
            print_error "Cannot continue with node running. Exiting."
            exit 1
        fi
    fi
    
    # Check existing database (both old and new paths)
    OLD_DATA_DIR="$HOME/.local/share/lumenyx-node"
    FOUND_OLD=false
    FOUND_NEW=false
    
    if [[ -d "$OLD_DATA_DIR" ]]; then
        FOUND_OLD=true
    fi
    if [[ -d "$DATA_DIR" ]]; then
        FOUND_NEW=true
    fi
    
    if [[ "$FOUND_OLD" == "true" || "$FOUND_NEW" == "true" ]]; then
        print_warning "Existing LUMENYX database found"
        if ask_yes_no "Delete it and start fresh?"; then
            # === COMPLETE CLEANUP ===
            pkill -f lumenyx-node 2>/dev/null || true
            systemctl stop lumenyx 2>/dev/null || true
            systemctl disable lumenyx 2>/dev/null || true
            rm -f /etc/systemd/system/lumenyx.service
            systemctl daemon-reload 2>/dev/null || true
            rm -rf "$DATA_DIR"
            rm -rf "$OLD_DATA_DIR"
            rm -rf "$LUMENYX_DIR"
            rm -f "$HOME/lumenyx-node-linux-x86_64" "$HOME/sha256sum.txt" "$HOME/lumenyx" "$HOME/lumenyx-node" 2>/dev/null
            print_ok "Complete cleanup done"
        fi
    fi
    
    # Clean stale lock files
    if [[ -f "$DATA_DIR/chains/lumenyx_mainnet/db/full/LOCK" ]]; then
        rm -f "$DATA_DIR/chains/lumenyx_mainnet/db/full/LOCK"
        print_ok "Cleaned stale lock file"
    fi
    
    print_ok "Pre-flight checks passed!"
    wait_enter
}

# STEP 1: WELCOME
step_welcome() {
    clear
    print_banner
    echo "Welcome to LUMENYX setup!"
    echo ""
    echo "This script will:"
    echo "  1. Check your system"
    echo "  2. Download LUMENYX"
    echo "  3. Generate your mining wallet"
    echo "  4. Start the node"
    echo ""
    wait_enter
}

# STEP 2: SYSTEM CHECK
step_check_system() {
    echo ""
    echo -e "${BLUE}═══ STEP 1: SYSTEM CHECK ═══${NC}"
    echo ""
    
    local errors=0
    
    if [[ "$(uname -s)" == "Linux" ]]; then
        print_ok "Operating system: Linux"
    else
        print_error "Linux required!"
        errors=$((errors + 1))
    fi
    
    ARCH=$(uname -m)
    if [[ "$ARCH" == "x86_64" ]]; then
        print_ok "Architecture: x86_64"
    else
        print_error "x86_64 required for precompiled binary"
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
    
    AVAILABLE_GB=$(df -BG "$HOME" | awk 'NR==2 {print $4}' | tr -d 'G')
    if [[ "$AVAILABLE_GB" -ge 1 ]]; then
        print_ok "Disk space: ${AVAILABLE_GB}GB available"
    else
        print_error "Less than 1GB available"
        errors=$((errors + 1))
    fi
    
    if [[ $errors -gt 0 ]]; then
        print_error "Fix the issues above before continuing."
        exit 1
    fi
    
    print_ok "System check passed!"
    wait_enter
}

# STEP 3: INSTALLATION
step_install() {
    echo ""
    echo -e "${BLUE}═══ STEP 2: INSTALLATION ═══${NC}"
    echo ""
    
    mkdir -p "$LUMENYX_DIR"
    cd "$LUMENYX_DIR"
    
    # Also remove any binary in home directory
    rm -f "$HOME/lumenyx-node-linux-x86_64" 2>/dev/null

    # Check if binary already exists
    if [[ -f "$BINARY_NAME" ]] || [[ -f "$BINARY_FINAL" ]]; then
        print_warning "Binary already exists"
        if ask_yes_no "Re-download?"; then
            rm -f "$BINARY_NAME" "$BINARY_FINAL"
        else
            print_ok "Using existing binary"
            wait_enter
            return
        fi
    fi
    
    print_info "Downloading lumenyx-node (~65MB)..."
    if curl -L -o "$BINARY_NAME" "$BINARY_URL" --progress-bar; then
        print_ok "Download complete"
    else
        print_error "Download failed"
        exit 1
    fi
    
    print_info "Verifying checksum..."
    if curl -sL -o sha256sum.txt "$CHECKSUM_URL"; then
        EXPECTED=$(grep lumenyx-node sha256sum.txt | awk '{print $1}')
        ACTUAL=$(sha256sum "$BINARY_NAME" | awk '{print $1}')
        
        if [[ "$EXPECTED" == "$ACTUAL" ]]; then
            print_ok "Checksum verified"
        else
            print_error "Checksum mismatch!"
            exit 1
        fi
        rm sha256sum.txt
    fi
    
    chmod +x "$BINARY_NAME"
    
    # Rename to standard name (Substrate uses binary name for data folder)
    if [[ "$BINARY_NAME" != "$BINARY_FINAL" ]]; then
        mv "$BINARY_NAME" "$BINARY_FINAL"
        BINARY_NAME="$BINARY_FINAL"
    fi
    print_ok "Binary ready: $LUMENYX_DIR/$BINARY_FINAL"
    wait_enter
}

# STEP 4: NODE MODE
step_mode() {
    echo ""
    echo -e "${BLUE}═══ STEP 3: NODE MODE ═══${NC}"
    echo ""
    
    echo "  [1] MINING - Earn LUMENYX (uses CPU)"
    echo "  [2] SYNC ONLY - Just verify (lightweight)"
    echo ""
    
    while true; do
        echo -ne "${YELLOW}Your choice [1/2]: ${NC}"
        read -r MODE_CHOICE
        case $MODE_CHOICE in
            1 ) NODE_MODE="mining"; break;;
            2 ) NODE_MODE="sync"; break;;
            * ) echo "Please enter 1 or 2";;
        esac
    done
    
    echo ""
    echo -ne "${YELLOW}Node name: ${NC}"
    read -r NODE_NAME
    
    if [[ -z "$NODE_NAME" ]]; then
        NODE_NAME="LUMENYX-Node-$$"
    fi
    
    print_ok "Mode: $NODE_MODE"
    print_ok "Name: $NODE_NAME"
    wait_enter
}

# STEP 5: WALLET GENERATION (via node)
step_wallet() {
    echo ""
    echo -e "${BLUE}═══ STEP 4: WALLET GENERATION ═══${NC}"
    echo ""
    
    # Check if wallet already exists
    if [[ -f "$DATA_DIR/miner-key" ]]; then
        print_ok "Wallet already exists"
        print_info "Your mining address is in the node logs when it starts"
        wait_enter
        return
    fi
    
    echo -e "${RED}╔═══════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}║  CRITICAL: The node will now generate your mining wallet.        ║${NC}"
    echo -e "${RED}║  Write down the 12-word seed phrase on paper!                    ║${NC}"
    echo -e "${RED}║  If you lose it, your funds are LOST FOREVER.                    ║${NC}"
    echo -e "${RED}╚═══════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    print_info "Starting node to generate wallet..."
    echo ""
    
    # Start node briefly to generate wallet, capture output
    LOG_FILE="/tmp/lumenyx-wallet-$$.log"
    
    if [[ "$NODE_MODE" == "mining" ]]; then
        timeout 10 "$LUMENYX_DIR/$BINARY_NAME" --chain mainnet --name "$NODE_NAME" --validator 2>&1 | tee "$LOG_FILE" || true
    else
        timeout 10 "$LUMENYX_DIR/$BINARY_NAME" --chain mainnet --name "$NODE_NAME" 2>&1 | tee "$LOG_FILE" || true
    fi
    
    # Kill any remaining process
    pkill -9 lumenyx-node 2>/dev/null || true
    sleep 2
    
    # Extract wallet info from log
    SEED_PHRASE=$(grep "Seed phrase:" "$LOG_FILE" | sed 's/.*Seed phrase: //' | head -1)
    WALLET_ADDRESS=$(grep "Address:" "$LOG_FILE" | grep -v "public" | sed 's/.*Address: //' | head -1)
    
    if [[ -z "$SEED_PHRASE" ]]; then
        # Wallet might already exist, get address from Mining rewards line
        WALLET_ADDRESS=$(grep "Mining rewards to:" "$LOG_FILE" | sed 's/.*Mining rewards to: //' | head -1)
        if [[ -n "$WALLET_ADDRESS" ]]; then
            echo ""
            print_ok "Wallet already generated"
            echo ""
            echo -e "Your LUMENYX mining address:"
            echo -e "${GREEN}  $WALLET_ADDRESS${NC}"
            echo ""
        fi
    else
        echo ""
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
        echo ""
        echo -e "${YELLOW}YOUR SEED PHRASE (12 words):${NC}"
        echo ""
        echo -e "${GREEN}  $SEED_PHRASE${NC}"
        echo ""
        echo -e "Your LUMENYX mining address:"
        echo -e "${GREEN}  $WALLET_ADDRESS${NC}"
        echo ""
        echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}"
        echo ""
        
        # Save address to file (not seed phrase for security)
        echo "LUMENYX Mining Wallet" > "$LUMENYX_DIR/wallet.txt"
        echo "Address: $WALLET_ADDRESS" >> "$LUMENYX_DIR/wallet.txt"
        echo "SEED PHRASE NOT SAVED - YOU SHOULD HAVE WRITTEN IT DOWN!" >> "$LUMENYX_DIR/wallet.txt"
    fi
    
    rm -f "$LOG_FILE"
    
    ask_confirm "Have you written down your seed phrase?"
    
    print_ok "Wallet ready!"
    wait_enter
}

# STEP 6: CREATE START SCRIPT
step_create_scripts() {
    echo ""
    echo -e "${BLUE}═══ STEP 5: SETUP ═══${NC}"
    echo ""
    
    if [[ "$NODE_MODE" == "mining" ]]; then
        FULL_CMD="$LUMENYX_DIR/$BINARY_NAME --chain mainnet --name \"$NODE_NAME\" --validator --rpc-cors all --unsafe-rpc-external --rpc-methods Safe --bootnodes $BOOTNODE"
    else
        FULL_CMD="$LUMENYX_DIR/$BINARY_NAME --chain mainnet --name \"$NODE_NAME\" --rpc-cors all --unsafe-rpc-external --rpc-methods Safe --bootnodes $BOOTNODE"
    fi
    
    # Create start script
    echo "#!/bin/bash" > "$LUMENYX_DIR/start.sh"
    echo "$FULL_CMD" >> "$LUMENYX_DIR/start.sh"
    chmod +x "$LUMENYX_DIR/start.sh"
    print_ok "Start script: $LUMENYX_DIR/start.sh"
    
    # Ask about systemd service
    echo ""
    if ask_yes_no "Create systemd service (auto-restart if node crashes)?"; then
        create_systemd_service
    fi
}

create_systemd_service() {
    # Need root for systemd
    if [[ $EUID -ne 0 ]]; then
        print_warning "Need root to create systemd service"
        print_info "Run this command later as root to create the service:"
        echo ""
        echo "  sudo bash -c 'cat > $SERVICE_FILE << SVCEOF"
        echo "[Unit]"
        echo "Description=LUMENYX Node"
        echo "After=network.target"
        echo ""
        echo "[Service]"
        echo "Type=simple"
        echo "User=$USER"
        echo "ExecStart=$FULL_CMD"
        echo "Restart=always"
        echo "RestartSec=10"
        echo ""
        echo "[Install]"
        echo "WantedBy=multi-user.target"
        echo "SVCEOF'"
        echo ""
        return
    fi
    
    cat > "$SERVICE_FILE" << SVCEOF
[Unit]
Description=LUMENYX Node
After=network.target

[Service]
Type=simple
User=root
ExecStart=$FULL_CMD
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
SVCEOF

    systemctl daemon-reload
    print_ok "Systemd service created: $SERVICE_FILE"
    echo ""
    print_info "Service is NOT enabled yet. First test the node manually."
    print_info "When everything works, enable it with:"
    echo ""
    echo "  systemctl enable lumenyx"
    echo "  systemctl start lumenyx"
    echo ""
}

# STEP 7: START
step_start() {
    echo ""
    echo -e "${BLUE}═══ STEP 6: START NODE ═══${NC}"
    echo ""
    
    if ask_yes_no "Start node now?"; then
        echo ""
        print_info "Starting LUMENYX node..."
        print_info "Press Ctrl+C to stop"
        echo ""
        eval "$FULL_CMD"
    else
        echo ""
        print_info "To start later: $LUMENYX_DIR/start.sh"
    fi
}

# MAIN
main() {
    step_welcome
    step_prechecks
    step_check_system
    step_install
    step_mode
    step_wallet
    step_create_scripts
    step_start
}

main


