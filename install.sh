#!/bin/bash
set -e

echo "Installing OS CLI..."
echo ""

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "Rust is required but not installed."
    echo "Install it from https://rustup.rs"
    exit 1
fi

# Clone or pull latest
if [ -d "$HOME/.os-cli" ]; then
    echo "Updating existing installation..."
    cd "$HOME/.os-cli" && git pull
else
    echo "Cloning os-cli..."
    git clone https://github.com/agemalabs/os-cli "$HOME/.os-cli"
    cd "$HOME/.os-cli"
fi

# Build release binary
echo ""
echo "Building (this may take a few minutes on first install)..."
cargo build --release

# Install to /usr/local/bin
sudo cp target/release/os /usr/local/bin/os

echo ""
echo "OS CLI installed successfully."
echo "  Run: os"
echo "  First run will authenticate via Google or GitHub."
