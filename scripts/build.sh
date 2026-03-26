#!/bin/bash
# Manual build script for Hermes
# Usage: ./scripts/build.sh [destination]
# Default destination: ~/.local/share/nvim/hermes

set -e

DEST="${1:-$HOME/.local/share/nvim/hermes}"
REPO_URL="https://github.com/Ruddickmg/hermes.nvim.git"
BUILD_DIR="$DEST/build"

echo "Building Hermes from source..."
echo "Destination: $DEST"

# Create directories
mkdir -p "$DEST"
mkdir -p "$BUILD_DIR"

# Clone repository
echo "Cloning repository..."
if [ -d "$BUILD_DIR/.git" ]; then
    cd "$BUILD_DIR"
    git pull origin main
else
    git clone --depth 1 --branch main "$REPO_URL" "$BUILD_DIR"
    cd "$BUILD_DIR"
fi

# Build with cargo
echo "Building with cargo (this may take a few minutes)..."
cargo build --release

# Determine library extension
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    EXT="so"
    PLATFORM="linux"
    ARCH=$(uname -m)
    # Normalize architecture name
    if [ "$ARCH" == "x86_64" ] || [ "$ARCH" == "amd64" ]; then
        ARCH="x86_64"
    elif [ "$ARCH" == "aarch64" ] || [ "$ARCH" == "arm64" ]; then
        ARCH="aarch64"
    fi
elif [[ "$OSTYPE" == "darwin"* ]]; then
    EXT="dylib"
    PLATFORM="macos"
    ARCH=$(uname -m)
    if [ "$ARCH" == "x86_64" ]; then
        ARCH="x86_64"
    elif [ "$ARCH" == "arm64" ]; then
        ARCH="aarch64"
    fi
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "win32" ]]; then
    EXT="dll"
    PLATFORM="windows"
    ARCH=$(uname -m)
else
    echo "Unsupported platform: $OSTYPE"
    exit 1
fi

# Copy built library
SOURCE="$BUILD_DIR/target/release/libhermes.$EXT"
DEST_FILE="$DEST/libhermes-$PLATFORM-$ARCH.$EXT"

echo "Copying library to: $DEST_FILE"
cp "$SOURCE" "$DEST_FILE"

# Create version file
echo "built" > "$DEST/version.txt"

# Clean up build directory (optional)
echo "Cleaning up build directory..."
rm -rf "$BUILD_DIR"

echo "Build complete!"
echo "Binary location: $DEST_FILE"
echo ""
echo "You can now use Hermes in Neovim:"
echo "  require('hermes').setup()"
