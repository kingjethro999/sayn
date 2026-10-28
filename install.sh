#!/usr/bin/env bash
# ==============================================================================
# Sayn Official Installer & Environment Setup
# Supports: Linux (Ubuntu/Debian, Fedora, Arch) and macOS (Intel & Apple Silicon)
# ==============================================================================
set -euo pipefail

BOLD='\033[1m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${CYAN}${BOLD}✦ Sayn Installer & Environment Setup${NC}\n"

# 1. System Detection
OS="$(uname -s)"
ARCH="$(uname -m)"

echo -e "Detected System: ${BOLD}${OS} (${ARCH})${NC}"

case "${OS}" in
    Linux)
        PLATFORM="linux"
        ;;
    Darwin)
        PLATFORM="macos"
        ;;
    *)
        echo -e "${RED}✖ Unsupported operating system: ${OS}.${NC}"
        echo "For Windows, please use the PowerShell installer: install.ps1"
        exit 1
        ;;
esac

# 2. Dependency Checks & Automated Setup
echo -e "\n${BOLD}[1/4] Checking prerequisites...${NC}"

# Check/Install Git
if ! command -v git &>/dev/null; then
    echo -e "${YELLOW}Git not found. Installing Git...${NC}"
    if [ "${PLATFORM}" = "linux" ]; then
        if command -v apt-get &>/dev/null; then
            sudo apt-get update && sudo apt-get install -y git
        elif command -v dnf &>/dev/null; then
            sudo dnf install -y git
        elif command -v pacman &>/dev/null; then
            sudo pacman -Sy --noconfirm git
        else
            echo -e "${RED}Please install git using your package manager.${NC}"
            exit 1
        fi
    elif [ "${PLATFORM}" = "macos" ]; then
        if command -v brew &>/dev/null; then
            brew install git
        else
            xcode-select --install || true
        fi
    fi
fi
echo -e "  ${GREEN}✔${NC} Git is available: $(git --version)"

# Check/Install Rust & Cargo
if ! command -v cargo &>/dev/null && [ ! -f "$HOME/.cargo/bin/cargo" ]; then
    echo -e "${YELLOW}Rust toolchain not found. Installing Rust via rustup...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    # Source cargo env immediately
    if [ -f "$HOME/.cargo/env" ]; then
        . "$HOME/.cargo/env"
    fi
fi

if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi

if command -v cargo &>/dev/null; then
    echo -e "  ${GREEN}✔${NC} Rust toolchain is available: $(rustc --version)"
else
    echo -e "${RED}✖ Failed to initialize Rust toolchain.${NC}"
    exit 1
fi

# Check/Install Node.js & npm
if ! command -v node &>/dev/null; then
    echo -e "${YELLOW}Node.js not found. Installing Node.js LTS...${NC}"
    if [ "${PLATFORM}" = "linux" ]; then
        if command -v apt-get &>/dev/null; then
            curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
            sudo apt-get install -y nodejs
        else
            echo -e "${YELLOW}Please install Node.js 18+ from https://nodejs.org${NC}"
        fi
    elif [ "${PLATFORM}" = "macos" ]; then
        if command -v brew &>/dev/null; then
            brew install node
        fi
    fi
fi

if command -v node &>/dev/null; then
    echo -e "  ${GREEN}✔${NC} Node.js is available: $(node --version)"
    echo -e "  ${GREEN}✔${NC} Package manager (npm): $(npm --version)"
else
    echo -e "${YELLOW}Warning: Node.js is recommended to run Next.js generated projects.${NC}"
fi

# 3. Build or Install Sayn Binary
echo -e "\n${BOLD}[2/4] Installing Sayn CLI...${NC}"
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "${INSTALL_DIR}"
mkdir -p "$HOME/.cargo/bin"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -f "${SCRIPT_DIR}/Cargo.toml" ]; then
    echo "Building Sayn from source repository..."
    cargo build --release --bin sayn --manifest-path "${SCRIPT_DIR}/Cargo.toml"
    cp "${SCRIPT_DIR}/target/release/sayn" "${INSTALL_DIR}/sayn"
    cp "${SCRIPT_DIR}/target/release/sayn" "$HOME/.cargo/bin/sayn"
else
    echo "Downloading latest release binary for ${PLATFORM}-${ARCH}..."
    ASSET_NAME="sayn-${PLATFORM}-${ARCH}.tar.gz"
    DOWNLOAD_URL="https://github.com/kingjethro999/sayn/releases/latest/download/${ASSET_NAME}"
    TMP_DIR="$(mktemp -d)"
    if curl -sSL -f "${DOWNLOAD_URL}" -o "${TMP_DIR}/${ASSET_NAME}"; then
        tar -xzf "${TMP_DIR}/${ASSET_NAME}" -C "${INSTALL_DIR}"
    else
        echo -e "${RED}Could not download pre-built release from ${DOWNLOAD_URL}.${NC}"
        echo "Falling back to installing via Cargo git..."
        cargo install --git https://github.com/kingjethro999/sayn.git sayn-cli --bin sayn
        cp "$HOME/.cargo/bin/sayn" "${INSTALL_DIR}/sayn"
    fi
    rm -rf "${TMP_DIR}"
fi

chmod +x "${INSTALL_DIR}/sayn" "$HOME/.cargo/bin/sayn" 2>/dev/null || true
echo -e "  ${GREEN}✔${NC} Installed sayn binary to ${INSTALL_DIR}/sayn"

# 4. PATH Configuration
echo -e "\n${BOLD}[3/4] Configuring shell PATH...${NC}"

ensure_path_in_file() {
    local file="$1"
    local line='export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"'
    if [ -f "${file}" ]; then
        if ! grep -q '.local/bin' "${file}" 2>/dev/null; then
            echo "" >> "${file}"
            echo "# Sayn and Cargo PATH" >> "${file}"
            echo "${line}" >> "${file}"
            echo -e "  ${GREEN}✔${NC} Added Sayn to ${file}"
        fi
    fi
}

ensure_path_in_file "$HOME/.bashrc"
ensure_path_in_file "$HOME/.zshrc"
ensure_path_in_file "$HOME/.profile"

export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"

# 5. Verification
echo -e "\n${BOLD}[4/4] Verifying installation...${NC}"
if command -v sayn &>/dev/null; then
    VERSION_OUTPUT="$(sayn --version)"
    echo -e "${GREEN}${BOLD}✔ Sayn installed successfully! (${VERSION_OUTPUT})${NC}\n"
    echo "To get started, restart your terminal or run:"
    echo -e "  ${CYAN}source ~/.zshrc${NC}  (or source ~/.bashrc)"
    echo -e "  ${CYAN}sayn --help${NC}"
    echo -e "  ${CYAN}sayn create --web my-project${NC}"
else
    echo -e "${YELLOW}Sayn installed to ${INSTALL_DIR}/sayn.${NC}"
    echo "Please add $HOME/.local/bin to your PATH or run: source ~/.zshrc"
fi