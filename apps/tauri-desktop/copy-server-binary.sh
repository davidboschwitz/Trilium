#!/bin/bash
# Script to copy the Rust server binary to Tauri's binaries directory

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_SERVER_DIR="$SCRIPT_DIR/../rust-server"
TAURI_BINARIES_DIR="$SCRIPT_DIR/src-tauri/binaries"

# Ensure binaries directory exists
mkdir -p "$TAURI_BINARIES_DIR"

# Determine binary name based on OS
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    BINARY_NAME="trilium-rust.exe"
else
    BINARY_NAME="trilium-rust"
fi

echo "Building Rust server..."
cd "$RUST_SERVER_DIR"
cargo build --release

echo "Copying binary to Tauri binaries directory..."
cp "target/release/$BINARY_NAME" "$TAURI_BINARIES_DIR/"
chmod +x "$TAURI_BINARIES_DIR/$BINARY_NAME"

echo "✓ Binary copied successfully: $TAURI_BINARIES_DIR/$BINARY_NAME"
ls -lh "$TAURI_BINARIES_DIR/$BINARY_NAME"
