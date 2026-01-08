#!/bin/bash

# ═══════════════════════════════════════════════════════════════════════════════
# LUMENYX SETUP SCRIPT v1.0
# ═══════════════════════════════════════════════════════════════════════════════
# Step-by-step guide to install and run a LUMENYX node
# ═══════════════════════════════════════════════════════════════════════════════

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
LUMENYX_DIR="$HOME/lumenyx"
BINARY_NAME="lumenyx-node"
BINARY_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v1.0.0/lumenyx-node"
CHECKSUM_URL="https://github.com/lumenyx-chain/lumenyx/releases/download/v1.0.0/sha256sum.txt"
BOOTNODE="/ip4/89.147.111.102/tcp/30333/p2p/12D3KooWB4tgfwi4fmkL7dK1xbdQh1AdaENYNeYSvCdnTXNZwQ9F"
GITHUB_REPO="https://github.com/lumenyx-chain/lumenyx.git"

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

# STEP 1: WELCOME
step_welcome() {
    clear
    print_banner
    echo "Welcome to LUMENYX setup!"
    echo ""
    echo "This script will:"
    echo "  1. Check your system"
    echo "  2. Download LUMENYX"
    echo "  3. Create your wallet"
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
    print_ok "Binary ready: $LUMENYX_DIR/$BINARY_NAME"
    wait_enter
}

# STEP 4: WALLET
step_wallet() {
    echo ""
    echo -e "${BLUE}═══ STEP 3: WALLET CREATION ═══${NC}"
    echo ""
    
    echo -e "${RED}╔═══════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}║  CRITICAL: Write down the 12-word seed phrase on paper!          ║${NC}"
    echo -e "${RED}║  If you lose it, your funds are LOST FOREVER.                    ║${NC}"
    echo -e "${RED}╚═══════════════════════════════════════════════════════════════════╝${NC}"
    
    wait_enter
    
    WALLET_OUTPUT=$("$LUMENYX_DIR/$BINARY_NAME" key generate --words 12)
    
    SEED_PHRASE=$(echo "$WALLET_OUTPUT" | grep "Secret phrase:" | sed 's/.*Secret phrase:[[:space:]]*//')
    SS58_ADDRESS=$(echo "$WALLET_OUTPUT" | grep "SS58 Address:" | sed 's/.*SS58 Address:[[:space:]]*//')
    
    echo ""
    echo -e "${YELLOW}YOUR SEED PHRASE (12 words):${NC}"
    echo ""
    echo -e "${GREEN}  $SEED_PHRASE${NC}"
    echo ""
    echo -e "Your LUMENYX address:"
    echo -e "${GREEN}  $SS58_ADDRESS${NC}"
    echo ""
    
    echo "SS58 Address: $SS58_ADDRESS" > "$LUMENYX_DIR/wallet.txt"
    echo "SEED PHRASE NOT SAVED - WRITE IT DOWN!" >> "$LUMENYX_DIR/wallet.txt"
    
    while true; do
        echo -ne "${RED}Have you written down your seed phrase? Type 'YES': ${NC}"
        read -r confirm
        if [[ "$confirm" == "YES" ]]; then
            break
        fi
    done
    
    print_ok "Wallet created!"
    wait_enter
}

# STEP 5: NODE MODE
step_mode() {
    echo ""
    echo -e "${BLUE}═══ STEP 4: NODE MODE ═══${NC}"
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

# STEP 6: START
step_start() {
    echo ""
    echo -e "${BLUE}═══ STEP 5: START NODE ═══${NC}"
    echo ""
    
    if [[ "$NODE_MODE" == "mining" ]]; then
        FULL_CMD="$LUMENYX_DIR/$BINARY_NAME --chain mainnet --name \"$NODE_NAME\" --validator --rpc-cors all --bootnodes $BOOTNODE"
    else
        FULL_CMD="$LUMENYX_DIR/$BINARY_NAME --chain mainnet --name \"$NODE_NAME\" --rpc-cors all --bootnodes $BOOTNODE"
    fi
    
    echo "#!/bin/bash" > "$LUMENYX_DIR/start.sh"
    echo "$FULL_CMD" >> "$LUMENYX_DIR/start.sh"
    chmod +x "$LUMENYX_DIR/start.sh"
    
    print_ok "Start script: $LUMENYX_DIR/start.sh"
    echo ""
    
    echo -ne "${YELLOW}Start node now? [y/n]: ${NC}"
    read -r start_now
    
    if [[ "$start_now" == "y" || "$start_now" == "Y" ]]; then
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
    step_check_system
    step_install
    step_wallet
    step_mode
    step_start
}

main
