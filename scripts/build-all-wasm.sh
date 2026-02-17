#!/bin/bash
# Build plugin WASMs and package them into GLDF files
#
# This script builds plugin WASMs that get bundled inside GLDF files.
# These are loaded via the plugin system at runtime, NOT deployed directly.
#
# For building the main viewer app, use: ./scripts/build_all.sh
set -e

echo "=== Building Plugin WASMs ==="
echo ""
echo "Note: These WASMs are bundled inside GLDF files, not deployed directly."
echo ""

# 1. Build gldf-starsky-wasm plugin
echo "📦 Building gldf-starsky-wasm..."
cd crates/gldf-starsky-wasm
wasm-pack build --target web --release
cd ../..
echo "✅ gldf-starsky-wasm built -> crates/gldf-starsky-wasm/pkg/"

# 2. Build stadium plugin and package into GLDF
echo ""
echo "📦 Building gldf-stadium-plugin WASM..."
./scripts/build_stadium_wasm.sh gldf
echo "✅ gldf-stadium-plugin built -> tests/data/wip_stadium.gldf"

echo ""
echo "=== Plugin Builds Complete ==="
echo ""
echo "Output locations:"
echo "  - gldf-starsky-wasm: crates/gldf-starsky-wasm/pkg/"
echo "  - stadium plugin:    tests/data/wip_stadium.gldf"
echo ""
echo "These GLDF files can be opened in the main viewer app."
echo "To build the viewer: ./scripts/build_all.sh local"
