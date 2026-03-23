#!/bin/bash
set -e

REPO="agemalabs/os-cli"
BINARY_NAME="os-aarch64-apple-darwin"
INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/os"
CONFIG_FILE="$CONFIG_DIR/config.toml"

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

# Check for python3 (used for JSON parsing)
if ! command -v python3 &> /dev/null; then
    echo "Error: python3 is required for installation."
    echo "It should be pre-installed on macOS. Check your system."
    exit 1
fi

# Get GitHub token
GITHUB_TOKEN=""
if [ -f "$CONFIG_FILE" ]; then
    GITHUB_TOKEN=$(grep -E '^github_token' "$CONFIG_FILE" 2>/dev/null | sed 's/.*= *"\(.*\)"/\1/' || true)
fi

if [ -z "$GITHUB_TOKEN" ]; then
    echo "A GitHub Personal Access Token is required to download from the private repo."
    echo "Create one at: https://github.com/settings/tokens"
    echo "Required scope: repo"
    echo ""
    printf "GitHub token: "
    read -r GITHUB_TOKEN

    if [ -z "$GITHUB_TOKEN" ]; then
        echo "Error: No token provided."
        exit 1
    fi
fi

# Fetch latest release
echo "Fetching latest release..."
RELEASE_JSON=$(curl -sf \
    -H "Authorization: token $GITHUB_TOKEN" \
    -H "Accept: application/vnd.github+json" \
    -H "User-Agent: os-cli-installer" \
    "https://api.github.com/repos/$REPO/releases/latest") || {
    echo "Error: Failed to fetch release. Check your GitHub token."
    exit 1
}

TAG=$(echo "$RELEASE_JSON" | grep -o '"tag_name": *"[^"]*"' | head -1 | sed 's/.*: *"\(.*\)"/\1/')
echo "Latest version: $TAG"

# Get asset download URLs (use API URLs, not browser URLs)
BINARY_ASSET_URL=$(echo "$RELEASE_JSON" | python3 -c "
import sys, json
data = json.load(sys.stdin)
for asset in data.get('assets', []):
    if asset['name'] == '$BINARY_NAME':
        print(asset['url'])
        break
")

CHECKSUM_ASSET_URL=$(echo "$RELEASE_JSON" | python3 -c "
import sys, json
data = json.load(sys.stdin)
for asset in data.get('assets', []):
    if asset['name'] == 'checksums.txt':
        print(asset['url'])
        break
")

if [ -z "$BINARY_ASSET_URL" ] || [ -z "$CHECKSUM_ASSET_URL" ]; then
    echo "Error: Could not find binary or checksum assets in release."
    exit 1
fi

# Download checksum file
echo "Downloading checksum..."
CHECKSUMS=$(curl -sfL \
    -H "Authorization: token $GITHUB_TOKEN" \
    -H "Accept: application/octet-stream" \
    -H "User-Agent: os-cli-installer" \
    "$CHECKSUM_ASSET_URL") || {
    echo "Error: Failed to download checksums."
    exit 1
}

# Download binary
echo "Downloading binary..."
TMPFILE=$(mktemp)
curl -sfL \
    -H "Authorization: token $GITHUB_TOKEN" \
    -H "Accept: application/octet-stream" \
    -H "User-Agent: os-cli-installer" \
    "$BINARY_ASSET_URL" \
    -o "$TMPFILE" || {
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

# Save GitHub token to config
mkdir -p "$CONFIG_DIR"
if [ -f "$CONFIG_FILE" ]; then
    # Add github_token if not present
    if ! grep -q 'github_token' "$CONFIG_FILE"; then
        echo "github_token = \"$GITHUB_TOKEN\"" >> "$CONFIG_FILE"
    fi
else
    cat > "$CONFIG_FILE" <<CONF
api_url = "https://api.os.agemalabs.com"
token = ""
default_org = "agema-labs"
github_token = "$GITHUB_TOKEN"
CONF
fi
chmod 600 "$CONFIG_FILE"

echo ""
echo "OS CLI $TAG installed successfully."
echo "  Run: os"
echo "  First run will authenticate via Google or GitHub."
echo "  Update later with: os upgrade"
