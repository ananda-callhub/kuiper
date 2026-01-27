#!/usr/bin/env bash
set -e

# Kuiper CLI Installer
# https://github.com/yourusername/kuiper

VERSION="${KUIPER_VERSION:-latest}"
INSTALL_DIR="${KUIPER_INSTALL_DIR:-/usr/local/bin}"

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
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

case "$OS" in
    darwin|linux)
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

# Construct download URL
if [ "$VERSION" = "latest" ]; then
    RELEASE_URL="https://github.com/yourusername/kuiper/releases/latest/download"
else
    RELEASE_URL="https://github.com/yourusername/kuiper/releases/download/${VERSION}"
fi

BINARY_NAME="kuiper-${OS}-${ARCH}"
URL="${RELEASE_URL}/${BINARY_NAME}"

echo "╭──────────────────────────────────────╮"
echo "│     Kuiper CLI Installer             │"
echo "╰──────────────────────────────────────╯"
echo ""
echo "  OS:      $OS"
echo "  Arch:    $ARCH"
echo "  Version: $VERSION"
echo "  URL:     $URL"
echo ""

# Create temp directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Download binary
echo "Downloading Kuiper CLI..."
if command -v curl &> /dev/null; then
    curl -fsSL -o "$TEMP_DIR/kuiper" "$URL"
elif command -v wget &> /dev/null; then
    wget -q -O "$TEMP_DIR/kuiper" "$URL"
else
    echo "Error: curl or wget is required"
    exit 1
fi

chmod +x "$TEMP_DIR/kuiper"

# Install binary
echo "Installing to $INSTALL_DIR..."
if [ -w "$INSTALL_DIR" ]; then
    mv "$TEMP_DIR/kuiper" "$INSTALL_DIR/kuiper"
else
    echo "Requires sudo to install to $INSTALL_DIR"
    sudo mv "$TEMP_DIR/kuiper" "$INSTALL_DIR/kuiper"
fi

# Verify installation
if command -v kuiper &> /dev/null; then
    echo ""
    echo "✓ Kuiper installed successfully!"
    echo ""
    echo "Get started:"
    echo "  kuiper do \"hello world\"     # Run a simple task"
    echo "  kuiper --help                # Show all commands"
    echo ""
    echo "Set up API keys:"
    echo "  export GEMINI_API_KEY=\"...\""
    echo "  export ANTHROPIC_API_KEY=\"...\""
    echo "  export OPENAI_API_KEY=\"...\""
else
    echo ""
    echo "✗ Installation failed. Please check the error above."
    exit 1
fi
