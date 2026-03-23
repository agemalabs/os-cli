#!/bin/bash
set -e

REPO="agemalabs/os-cli"
BINARY_NAME="os-aarch64-apple-darwin"
INSTALL_DIR="$HOME/.local/bin"

echo "Installing OS CLI..."
echo ""

# Check architecture
ARCH=$(uname -m)
if [ "$ARCH" != "arm64" ]; then
    echo "Error: OS CLI currently only supports Apple Silicon (arm64)."
    echo "Your architecture: $ARCH"
    exit 1
fi

# Check for macOS
if [ "$(uname -s)" != "Darwin" ]; then
    echo "Error: OS CLI currently only supports macOS."
    exit 1
fi

# Fetch latest release
echo "Fetching latest release..."
RELEASE_JSON=$(curl -sf \
    -H "Accept: application/vnd.github+json" \
    -H "User-Agent: os-cli-installer" \
    "https://api.github.com/repos/$REPO/releases/latest") || {
    echo "Error: Failed to fetch release."
    exit 1
}

TAG=$(echo "$RELEASE_JSON" | grep -o '"tag_name": *"[^"]*"' | head -1 | sed 's/.*: *"\(.*\)"/\1/')
echo "Latest version: $TAG"

# Download binary directly from release assets
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$TAG/$BINARY_NAME"
CHECKSUM_URL="https://github.com/$REPO/releases/download/$TAG/checksums.txt"

# Download checksum file
echo "Downloading checksum..."
CHECKSUMS=$(curl -sfL "$CHECKSUM_URL") || {
    echo "Error: Failed to download checksums."
    exit 1
}

# Download binary
echo "Downloading binary..."
TMPFILE=$(mktemp)
curl -sfL "$DOWNLOAD_URL" -o "$TMPFILE" || {
    echo "Error: Failed to download binary."
    rm -f "$TMPFILE"
    exit 1
}

# Verify checksum
echo "Verifying checksum..."
EXPECTED_HASH=$(echo "$CHECKSUMS" | grep "$BINARY_NAME" | awk '{print $1}')
ACTUAL_HASH=$(shasum -a 256 "$TMPFILE" | awk '{print $1}')

if [ "$EXPECTED_HASH" != "$ACTUAL_HASH" ]; then
    echo "Error: Checksum mismatch!"
    echo "  Expected: $EXPECTED_HASH"
    echo "  Actual:   $ACTUAL_HASH"
    rm -f "$TMPFILE"
    exit 1
fi

# Install binary
mkdir -p "$INSTALL_DIR"
chmod +x "$TMPFILE"
mv "$TMPFILE" "$INSTALL_DIR/os"

echo "Installed to $INSTALL_DIR/os"

# Remove old binary if it exists at the previous location
if [ -f "/usr/local/bin/os" ]; then
    echo "Removing old binary at /usr/local/bin/os..."
    sudo rm -f "/usr/local/bin/os" 2>/dev/null || true
fi

# Ensure ~/.local/bin is in PATH
if ! echo "$PATH" | tr ':' '\n' | grep -q "^$HOME/.local/bin$"; then
    SHELL_RC="$HOME/.zshrc"
    if [ -f "$SHELL_RC" ]; then
        echo '' >> "$SHELL_RC"
        echo '# OS CLI' >> "$SHELL_RC"
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$SHELL_RC"
        echo "Added ~/.local/bin to PATH in .zshrc"
    fi
fi

echo ""
echo "OS CLI $TAG installed successfully."
echo "  Run: os"
echo "  First run will authenticate via Google or GitHub."
echo "  Update later with: os upgrade"
