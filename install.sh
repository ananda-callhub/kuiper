#!/usr/bin/env bash
set -e

# Kuiper CLI Installer
# MIT License - https://github.com/yourusername/kuiper

# Configuration (can be overridden by environment variables)
REPO="${KUIPER_REPO:-yourusername/kuiper}"
VERSION="${KUIPER_VERSION:-latest}"
INSTALL_DIR="${KUIPER_INSTALL_DIR:-/usr/local/bin}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() { echo -e "${BLUE}$1${NC}"; }
success() { echo -e "${GREEN}$1${NC}"; }
warn() { echo -e "${YELLOW}$1${NC}"; }
error() { echo -e "${RED}$1${NC}"; }

# Detect OS and architecture
OS=$(uname | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
    x86_64)
        ARCH="amd64"
        ;;
    arm64|aarch64)
        ARCH="arm64"
        ;;
    *)
        error "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

case "$OS" in
    darwin)
        OS_NAME="macOS"
        ;;
    linux)
        OS_NAME="Linux"
        ;;
    mingw*|msys*|cygwin*)
        error "Windows detected. Please use the Windows installer:"
        echo "  winget install kuiper"
        echo "  or download from: https://github.com/${REPO}/releases"
        exit 1
        ;;
    *)
        error "Unsupported OS: $OS"
        exit 1
        ;;
esac

# Get the latest version if not specified
if [ "$VERSION" = "latest" ]; then
    info "Fetching latest version..."
    VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "$VERSION" ]; then
        error "Failed to fetch latest version"
        exit 1
    fi
fi

# Construct download URL
RELEASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
BINARY_NAME="kuiper-${OS}-${ARCH}"
URL="${RELEASE_URL}/${BINARY_NAME}"
CHECKSUM_URL="${RELEASE_URL}/SHA256SUMS.txt"

echo ""
echo "╭──────────────────────────────────────╮"
echo "│       Kuiper CLI Installer           │"
echo "╰──────────────────────────────────────╯"
echo ""
info "  OS:      $OS_NAME ($OS)"
info "  Arch:    $ARCH"
info "  Version: $VERSION"
echo ""

# Create temp directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Download binary
info "Downloading Kuiper CLI..."
if command -v curl &> /dev/null; then
    HTTP_CODE=$(curl -fsSL -w "%{http_code}" -o "$TEMP_DIR/kuiper" "$URL")
    if [ "$HTTP_CODE" != "200" ]; then
        error "Download failed (HTTP $HTTP_CODE)"
        error "URL: $URL"
        exit 1
    fi
elif command -v wget &> /dev/null; then
    wget -q -O "$TEMP_DIR/kuiper" "$URL" || {
        error "Download failed"
        error "URL: $URL"
        exit 1
    }
else
    error "Error: curl or wget is required"
    exit 1
fi

# Download and verify checksum (optional but recommended)
info "Verifying checksum..."
if curl -fsSL -o "$TEMP_DIR/SHA256SUMS.txt" "$CHECKSUM_URL" 2>/dev/null; then
    cd "$TEMP_DIR"
    EXPECTED=$(grep "$BINARY_NAME" SHA256SUMS.txt | cut -d' ' -f1)
    if [ -n "$EXPECTED" ]; then
        if command -v sha256sum &> /dev/null; then
            ACTUAL=$(sha256sum kuiper | cut -d' ' -f1)
        elif command -v shasum &> /dev/null; then
            ACTUAL=$(shasum -a 256 kuiper | cut -d' ' -f1)
        fi
        if [ "$EXPECTED" = "$ACTUAL" ]; then
            success "  Checksum verified ✓"
        else
            error "Checksum mismatch!"
            error "  Expected: $EXPECTED"
            error "  Actual:   $ACTUAL"
            exit 1
        fi
    else
        warn "  Checksum not found for $BINARY_NAME, skipping verification"
    fi
    cd - > /dev/null
else
    warn "  Could not download checksums, skipping verification"
fi

chmod +x "$TEMP_DIR/kuiper"

# Install binary
info "Installing to $INSTALL_DIR..."
if [ -w "$INSTALL_DIR" ]; then
    mv "$TEMP_DIR/kuiper" "$INSTALL_DIR/kuiper"
else
    warn "Requires sudo to install to $INSTALL_DIR"
    sudo mv "$TEMP_DIR/kuiper" "$INSTALL_DIR/kuiper"
fi

# Verify installation
if command -v kuiper &> /dev/null; then
    INSTALLED_VERSION=$(kuiper --version 2>/dev/null || echo "unknown")
    echo ""
    success "✓ Kuiper $VERSION installed successfully!"
    echo ""
    echo "Get started:"
    echo "  kuiper do \"hello world\"        # Run a simple task"
    echo "  kuiper do --fast \"quick task\"  # Use fast-tier models"
    echo "  kuiper do --research \"complex\" # Use complex-tier models"
    echo "  kuiper models                   # List available models"
    echo "  kuiper --help                   # Show all commands"
    echo ""
    echo "Set up API keys (at least one required):"
    echo "  export GEMINI_API_KEY=\"your-key\""
    echo "  export ANTHROPIC_API_KEY=\"your-key\""
    echo "  export OPENAI_API_KEY=\"your-key\""
    echo ""
else
    echo ""
    error "✗ Installation may have failed."
    warn "Try adding $INSTALL_DIR to your PATH:"
    echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
    exit 1
fi
