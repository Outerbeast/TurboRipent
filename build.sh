#!/bin/sh
set -e

# Check for Rust installation
if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Build the project in release mode
echo "Building project..."
cargo build --release

# Copy binary to project root
BIN_NAME="TurboRipent"
DEST="$(dirname "$0")"

cp "target/release/$BIN_NAME" "$DEST"
cp "$BIN_NAME.desktop" "$DEST"

sha256sum "$DEST/$BIN_NAME" > "$DEST/$BIN_NAME.sha256.txt"
sha256sum "$DEST/$BIN_NAME.desktop" > "$DEST/$BIN_NAME.desktop.sha256.txt"

echo "Build complete. The executable is located at $DEST/$BIN_NAME"
echo "Desktop entry is located at $DEST/$BIN_NAME.desktop"
echo "Register it with: cp $BIN_NAME.desktop ~/.local/share/applications/"
cat "$DEST/$BIN_NAME.sha256.txt"
