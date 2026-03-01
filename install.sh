#!/usr/bin/env bash
# install.sh - One-command Smash installer for Linux
# Usage: curl -sSL https://raw.githubusercontent.com/Jalpan04/smash/master/install.sh | bash

set -e

REPO="https://github.com/Jalpan04/smash"
RELEASES="https://api.github.com/repos/Jalpan04/smash/releases/latest"
INSTALL_DIR="$HOME/.local/bin"
MODEL_DIR="$HOME/.smash/model"
BINARY_NAME="smash-linux-x86_64"

echo "Installing Smash shell..."
echo ""

# Create install dirs
mkdir -p "$INSTALL_DIR"
mkdir -p "$MODEL_DIR"

# Try to grab the latest pre-built binary from GitHub Releases first
ASSET_URL=$(curl -sSL "$RELEASES" 2>/dev/null \
  | grep "browser_download_url" \
  | grep "$BINARY_NAME" \
  | cut -d '"' -f 4 \
  | head -n 1)

if [ -n "$ASSET_URL" ]; then
    echo "Downloading pre-built binary from Releases..."
    curl -sSL "$ASSET_URL" -o "$INSTALL_DIR/smash"
    chmod +x "$INSTALL_DIR/smash"
    echo "Binary installed to $INSTALL_DIR/smash"
else
    # Fall back to building from source
    echo "No pre-built binary found. Building from source..."

    if ! command -v cargo &>/dev/null; then
        echo "Rust not found. Installing rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
        source "$HOME/.cargo/env"
    fi

    TMP=$(mktemp -d)
    trap "rm -rf $TMP" EXIT

    git clone --depth=1 "$REPO" "$TMP/smash"
    cd "$TMP/smash"
    cargo build --release
    cp target/release/smash "$INSTALL_DIR/smash"
    cp -r output/onnx/. "$MODEL_DIR/"
    echo "Built and installed to $INSTALL_DIR/smash"
fi

# If we installed from binary, we still need the ONNX model
# Clone just the model files using git sparse-checkout
if [ ! -f "$MODEL_DIR/encoder_model.onnx" ]; then
    echo "Downloading AI model files..."
    TMP2=$(mktemp -d)
    trap "rm -rf $TMP2" EXIT
    git clone --depth=1 --filter=blob:none --sparse "$REPO" "$TMP2/smash_model"
    cd "$TMP2/smash_model"
    git sparse-checkout set output/onnx
    cp -r output/onnx/. "$MODEL_DIR/"
    echo "Model installed to $MODEL_DIR"
fi

# Set SMASH_MODEL_DIR so the shell can find the model
RC_FILE="$HOME/.bashrc"
[ -f "$HOME/.zshrc" ] && RC_FILE="$HOME/.zshrc"

EXPORT_LINE="export SMASH_MODEL_DIR=\"$MODEL_DIR\""
if ! grep -qF "SMASH_MODEL_DIR" "$RC_FILE" 2>/dev/null; then
    echo "" >> "$RC_FILE"
    echo "# Smash shell model directory" >> "$RC_FILE"
    echo "$EXPORT_LINE" >> "$RC_FILE"
    echo "Added SMASH_MODEL_DIR to $RC_FILE"
fi

# Add ~/.local/bin to PATH if not already there
if ! echo "$PATH" | grep -q "$HOME/.local/bin"; then
    echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >> "$RC_FILE"
    echo "Added $INSTALL_DIR to PATH in $RC_FILE"
fi

echo ""
echo "Installation complete."
echo ""
echo "  Run now:   $INSTALL_DIR/smash"
echo "  Or after:  source $RC_FILE && smash"
echo ""
echo "  Create ~/.smashrc to configure aliases on startup."
echo "  See example: https://github.com/Jalpan04/smash/blob/master/example.smashrc"
