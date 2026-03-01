#!/usr/bin/env bash
# install.sh - One-command installer for Smash (Smart Bash)
# Usage: curl -sSf https://raw.githubusercontent.com/Jalpan04/smash/master/install.sh | bash

set -e

REPO="https://github.com/Jalpan04/smash.git"
INSTALL_DIR="$HOME/.smash"

echo "=========================================="
echo "  Smash - Smart Bash Shell Installer"
echo "=========================================="
echo ""

# Check for git
if ! command -v git &>/dev/null; then
    echo "Installing git..."
    sudo apt-get install -y git || sudo yum install -y git || sudo pacman -S --noconfirm git
fi

# Check for Rust - install if missing
if ! command -v cargo &>/dev/null; then
    echo "Rust not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    source "$HOME/.cargo/env"
    export PATH="$HOME/.cargo/bin:$PATH"
else
    echo "Rust found: $(rustc --version)"
fi

# Install git-lfs (needed to pull the ONNX model files)
if ! command -v git-lfs &>/dev/null; then
    echo "Installing git-lfs..."
    sudo apt-get install -y git-lfs 2>/dev/null \
        || sudo yum install -y git-lfs 2>/dev/null \
        || sudo pacman -S --noconfirm git-lfs 2>/dev/null \
        || (curl -s https://packagecloud.io/install/repositories/github/git-lfs/script.deb.sh | sudo bash && sudo apt-get install -y git-lfs)
fi

git lfs install

# Clone or update the repository
if [ -d "$INSTALL_DIR/.git" ]; then
    echo "Updating existing install..."
    git -C "$INSTALL_DIR" pull
else
    echo "Cloning Smash..."
    git clone "$REPO" "$INSTALL_DIR"
fi

# Build the shell
echo ""
echo "Building Smash (this may take a few minutes on the first run)..."
cd "$INSTALL_DIR"
"$HOME/.cargo/bin/cargo" build --release

# Create a symlink in /usr/local/bin for easy access
BINARY="$INSTALL_DIR/target/release/smash"
if [ -f "$BINARY" ]; then
    sudo ln -sf "$BINARY" /usr/local/bin/smash
    echo ""
    echo "=========================================="
    echo "  Smash successfully installed!"
    echo "  Run: smash"
    echo "  AI commands: smash list all files"
    echo "               smash show free disk space"
    echo "=========================================="
else
    echo "Build failed. Please check the output above."
    exit 1
fi
