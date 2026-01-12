#!/bin/bash
# Build gldf-stadium-plugin for WASM
#
# Usage:
#   ./scripts/build_stadium_wasm.sh         # Build only
#   ./scripts/build_stadium_wasm.sh gldf    # Build and package into wip_stadium.gldf
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MODE="${1:-build}"

echo "Building gldf-stadium-plugin for WASM..."

# Build for WASM
cd "$PROJECT_DIR"
cargo build --release --target wasm32-unknown-unknown -p gldf-stadium-plugin --lib

# Generate JS bindings with wasm-bindgen
echo "Generating JS bindings..."
WASM_FILE="$PROJECT_DIR/target/wasm32-unknown-unknown/release/gldf_stadium_plugin.wasm"

# Create output directory
OUT_DIR="$PROJECT_DIR/target/wasm-stadium"
mkdir -p "$OUT_DIR"

# Clean old files
rm -f "$OUT_DIR"/*.js "$OUT_DIR"/*.wasm

# Run wasm-bindgen
wasm-bindgen "$WASM_FILE" \
    --out-dir "$OUT_DIR" \
    --target web \
    --no-typescript

# Optimize WASM (optional, requires wasm-opt from binaryen)
if command -v wasm-opt &> /dev/null; then
    echo "Optimizing WASM..."
    wasm-opt -O3 "$OUT_DIR/gldf_stadium_plugin_bg.wasm" -o "$OUT_DIR/gldf_stadium_plugin_bg.wasm"
fi

echo "Stadium WASM built successfully!"
echo "Output: $OUT_DIR/"
ls -la "$OUT_DIR/"

# Package into GLDF if requested
if [ "$MODE" = "gldf" ]; then
    echo ""
    echo "=== Packaging into wip_stadium.gldf ==="

    GLDF_FILE="$PROJECT_DIR/tests/data/wip_stadium.gldf"
    TEMP_DIR=$(mktemp -d)

    # Extract existing GLDF
    unzip -q "$GLDF_FILE" -d "$TEMP_DIR"

    # Copy WASM files
    mkdir -p "$TEMP_DIR/other/viewer/stadium"
    cp "$OUT_DIR/gldf_stadium_plugin.js" "$TEMP_DIR/other/viewer/stadium/"
    cp "$OUT_DIR/gldf_stadium_plugin_bg.wasm" "$TEMP_DIR/other/viewer/stadium/"

    # Update manifest with correct file names
    cat > "$TEMP_DIR/other/viewer/stadium/manifest.json" << 'EOF'
{
  "type": "stadium",
  "name": "Stadium Floodlight Viewer",
  "version": "0.1.0",
  "description": "3D stadium floodlight visualization using Bevy",
  "js": "gldf_stadium_plugin.js",
  "wasm": "gldf_stadium_plugin_bg.wasm",
  "data_file": "other/stadium_config.json",
  "ldt_file": "ldc/SL9178-480W-5700K.ldt",
  "canvas_id": "stadium-canvas",
  "entry_point": "run_on_canvas",
  "storage_keys": {
    "ldt": "gldf_stadium_ldt",
    "config": "gldf_stadium_config"
  },
  "controls": {
    "keyboard": [
      {"key": "C", "action": "Decrease floodlight tilt"},
      {"key": "V", "action": "Increase floodlight tilt"},
      {"key": "T", "action": "Toggle tower layout (4/6)"},
      {"key": "+", "action": "Increase light intensity"},
      {"key": "-", "action": "Decrease light intensity"},
      {"key": "R", "action": "Reset camera view"},
      {"key": "H", "action": "Toggle shadows"},
      {"key": "WASD", "action": "Move camera"},
      {"key": "Arrows", "action": "Orbit camera"},
      {"key": "QE", "action": "Zoom in/out"}
    ]
  }
}
EOF

    # Repackage GLDF
    rm "$GLDF_FILE"
    cd "$TEMP_DIR"
    zip -rq "$GLDF_FILE" .
    cd "$PROJECT_DIR"

    # Cleanup
    rm -rf "$TEMP_DIR"

    echo "Updated: $GLDF_FILE"
    unzip -l "$GLDF_FILE"
fi
