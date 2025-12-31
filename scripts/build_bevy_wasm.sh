#!/bin/bash
# Build gldf-bevy for WASM and copy to gldf-rs-wasm dist folder
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Building gldf-bevy for WASM..."

# Build for WASM with wasm-sync feature (enables web-sys for localStorage)
# Build the binary (gldf-bevy-viewer) not the library
cd "$PROJECT_DIR"
cargo build --release --target wasm32-unknown-unknown -p gldf-bevy --bin gldf-bevy-viewer --features standalone

# Generate JS bindings with wasm-bindgen
echo "Generating JS bindings..."
WASM_FILE="$PROJECT_DIR/target/wasm32-unknown-unknown/release/gldf-bevy-viewer.wasm"

# Create output directory
BEVY_OUT_DIR="$PROJECT_DIR/crates/gldf-rs-wasm/dist/bevy"
mkdir -p "$BEVY_OUT_DIR"

# Clean old files
rm -f "$BEVY_OUT_DIR"/*.js "$BEVY_OUT_DIR"/*.wasm

# Run wasm-bindgen
wasm-bindgen "$WASM_FILE" \
    --out-dir "$BEVY_OUT_DIR" \
    --target web \
    --no-typescript

# Optimize WASM (optional, requires wasm-opt from binaryen)
if command -v wasm-opt &> /dev/null; then
    echo "Optimizing WASM..."
    wasm-opt -O3 "$BEVY_OUT_DIR/gldf-bevy-viewer_bg.wasm" -o "$BEVY_OUT_DIR/gldf-bevy-viewer_bg.wasm"
fi

# Generate hash from WASM content (first 8 chars of md5)
HASH=$(md5 -q "$BEVY_OUT_DIR/gldf-bevy-viewer_bg.wasm" | head -c 8)
echo "Content hash: $HASH"

# Rename files with hash
mv "$BEVY_OUT_DIR/gldf-bevy-viewer.js" "$BEVY_OUT_DIR/gldf-bevy-viewer-${HASH}.js"
mv "$BEVY_OUT_DIR/gldf-bevy-viewer_bg.wasm" "$BEVY_OUT_DIR/gldf-bevy-viewer-${HASH}_bg.wasm"

# Fix the import path in the JS file
sed -i '' "s/gldf-bevy-viewer_bg.wasm/gldf-bevy-viewer-${HASH}_bg.wasm/g" "$BEVY_OUT_DIR/gldf-bevy-viewer-${HASH}.js"

# Create manifest file
echo "{\"hash\":\"${HASH}\",\"js\":\"gldf-bevy-viewer-${HASH}.js\",\"wasm\":\"gldf-bevy-viewer-${HASH}_bg.wasm\"}" > "$BEVY_OUT_DIR/manifest.json"

echo "Bevy WASM built successfully!"
echo "Output: $BEVY_OUT_DIR/"
ls -la "$BEVY_OUT_DIR/"
