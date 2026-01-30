#!/bin/sh
# SmartScribe installer for Linux and macOS
# Usage: curl -sSL https://raw.githubusercontent.com/ThilinaTLM/smart-scribe/main/scripts/install.sh | bash
#
# Environment variables:
#   INSTALL_DIR - Override installation directory (default: ~/.local/bin)
#   VERSION     - Install specific version (default: latest)

set -e

# Colors (disabled if not a terminal)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    NC='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    BOLD=''
    NC=''
fi

REPO="ThilinaTLM/smart-scribe"
BINARY_NAME="smart-scribe"

info() {
    printf "${BLUE}==>${NC} ${BOLD}%s${NC}\n" "$1"
}

success() {
    printf "${GREEN}==>${NC} ${BOLD}%s${NC}\n" "$1"
}

warn() {
    printf "${YELLOW}Warning:${NC} %s\n" "$1"
}

error() {
    printf "${RED}Error:${NC} %s\n" "$1" >&2
    exit 1
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "darwin" ;;
        *)       error "Unsupported operating system: $(uname -s)" ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)             error "Unsupported architecture: $(uname -m)" ;;
    esac
}

# Get the latest release version from GitHub API
get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"

    if command -v curl > /dev/null 2>&1; then
        curl -sS "$url" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    elif command -v wget > /dev/null 2>&1; then
        wget -qO- "$url" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Download file
download() {
    local url="$1"
    local output="$2"

    if command -v curl > /dev/null 2>&1; then
        curl -fsSL "$url" -o "$output"
    elif command -v wget > /dev/null 2>&1; then
        wget -q "$url" -O "$output"
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Check for recommended dependencies (warn only, don't fail)
check_dependencies() {
    local os="$1"
    local missing=""

    if [ "$os" = "linux" ]; then
        info "Checking optional dependencies..."

        # Keystroke support (any one is sufficient)
        if ! command -v ydotool > /dev/null 2>&1 && \
           ! command -v wtype > /dev/null 2>&1 && \
           ! command -v xdotool > /dev/null 2>&1; then
            missing="${missing}\n  - xdotool, wtype, or ydotool (for --keystroke feature)"
        fi

        if [ -n "$missing" ]; then
            echo ""
            warn "The following optional dependencies are not installed:"
            printf "%b\n" "$missing"
            echo ""
            echo "These are only needed for specific features. SmartScribe will work without them."
            echo ""
        else
            success "All optional dependencies found"
        fi
    fi
}

# Main installation
main() {
    echo ""
    printf "${BOLD}SmartScribe Installer${NC}\n"
    echo "=============================="
    echo ""

    # Detect platform
    OS=$(detect_os)
    ARCH=$(detect_arch)
    info "Detected platform: ${OS}-${ARCH}"

    # Get version
    if [ -n "${VERSION:-}" ]; then
        VERSION_TAG="$VERSION"
    else
        info "Fetching latest release..."
        VERSION_TAG=$(get_latest_version)
    fi

    if [ -z "$VERSION_TAG" ]; then
        error "Could not determine version to install"
    fi

    info "Installing version: ${VERSION_TAG}"

    # Construct artifact name
    ARTIFACT="${BINARY_NAME}-${OS}-${ARCH}"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION_TAG}/${ARTIFACT}"

    # Determine install directory
    if [ -n "${INSTALL_DIR:-}" ]; then
        INSTALL_PATH="$INSTALL_DIR"
    elif [ -w "/usr/local/bin" ]; then
        INSTALL_PATH="/usr/local/bin"
    else
        INSTALL_PATH="${HOME}/.local/bin"
    fi

    # Create install directory if needed
    if [ ! -d "$INSTALL_PATH" ]; then
        info "Creating directory: ${INSTALL_PATH}"
        mkdir -p "$INSTALL_PATH"
    fi

    # Download binary
    TEMP_FILE=$(mktemp)
    info "Downloading ${ARTIFACT}..."

    if ! download "$DOWNLOAD_URL" "$TEMP_FILE"; then
        rm -f "$TEMP_FILE"
        error "Failed to download binary. Check if the release exists for your platform."
    fi

    # Install binary
    info "Installing to ${INSTALL_PATH}/${BINARY_NAME}"
    mv "$TEMP_FILE" "${INSTALL_PATH}/${BINARY_NAME}"
    chmod +x "${INSTALL_PATH}/${BINARY_NAME}"

    # Verify installation
    if [ -x "${INSTALL_PATH}/${BINARY_NAME}" ]; then
        INSTALLED_VERSION=$("${INSTALL_PATH}/${BINARY_NAME}" --version 2>/dev/null || echo "unknown")
        success "Installed: ${INSTALLED_VERSION}"
    else
        error "Installation failed - binary not executable"
    fi

    # Check if install path is in PATH
    case ":${PATH}:" in
        *":${INSTALL_PATH}:"*)
            ;;
        *)
            echo ""
            warn "${INSTALL_PATH} is not in your PATH"
            echo ""
            echo "Add it to your shell profile:"
            echo ""
            echo "  # For bash (~/.bashrc):"
            echo "  export PATH=\"\$PATH:${INSTALL_PATH}\""
            echo ""
            echo "  # For zsh (~/.zshrc):"
            echo "  export PATH=\"\$PATH:${INSTALL_PATH}\""
            echo ""
            echo "  # For fish (~/.config/fish/config.fish):"
            echo "  fish_add_path ${INSTALL_PATH}"
            echo ""
            ;;
    esac

    # Check dependencies
    check_dependencies "$OS"

    # Print next steps
    echo ""
    success "Installation complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Set your Gemini API key:"
    echo "     smart-scribe config set api_key YOUR_API_KEY"
    echo ""
    echo "  2. Test it:"
    echo "     smart-scribe --help"
    echo ""
    echo "Get your API key at: https://aistudio.google.com/apikey"
    echo ""
}

main "$@"
