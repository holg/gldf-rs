#!/bin/bash
# Embed a WASM plugin into a GLDF file
#
# Usage: embed_plugin.sh <input.gldf> <plugin_dir> <output.gldf>
#
# The plugin directory should contain:
#   - manifest.json
#   - *.js (JavaScript bindings)
#   - *.wasm (WebAssembly module)
#
# The plugin will be added to: other/viewer/<plugin_id>/

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ $# -lt 3 ]; then
    echo "Usage: $0 <input.gldf> <plugin_dir> <output.gldf>"
    echo ""
    echo "Example:"
    echo "  $0 luminaire.gldf ../eulumdat-rs/crates/eulumdat-plugin/dist enriched.gldf"
    exit 1
fi

INPUT_GLDF="$1"
PLUGIN_DIR="$2"
OUTPUT_GLDF="$3"

# Check input exists
if [ ! -f "$INPUT_GLDF" ]; then
    echo "Error: Input GLDF not found: $INPUT_GLDF"
    exit 1
fi

# Check plugin directory
if [ ! -d "$PLUGIN_DIR" ]; then
    echo "Error: Plugin directory not found: $PLUGIN_DIR"
    exit 1
fi

# Check manifest exists
if [ ! -f "$PLUGIN_DIR/manifest.json" ]; then
    echo "Error: manifest.json not found in $PLUGIN_DIR"
    exit 1
fi

# Get plugin ID from manifest
PLUGIN_ID=$(python3 -c "import json; print(json.load(open('$PLUGIN_DIR/manifest.json'))['id'])")
echo "Plugin ID: $PLUGIN_ID"

# Create temp directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo "Extracting GLDF..."
unzip -q "$INPUT_GLDF" -d "$TEMP_DIR"

# Create plugin directory
PLUGIN_DEST="$TEMP_DIR/other/viewer/$PLUGIN_ID"
mkdir -p "$PLUGIN_DEST"

# Copy plugin files
echo "Copying plugin files..."
cp "$PLUGIN_DIR"/*.json "$PLUGIN_DEST/" 2>/dev/null || true
cp "$PLUGIN_DIR"/*.js "$PLUGIN_DEST/" 2>/dev/null || true
cp "$PLUGIN_DIR"/*.wasm "$PLUGIN_DEST/" 2>/dev/null || true

# Count files
JS_COUNT=$(ls "$PLUGIN_DEST"/*.js 2>/dev/null | wc -l | tr -d ' ')
WASM_COUNT=$(ls "$PLUGIN_DEST"/*.wasm 2>/dev/null | wc -l | tr -d ' ')
echo "  Added: $JS_COUNT JS files, $WASM_COUNT WASM files"

# Show file sizes
echo "  Files:"
ls -lh "$PLUGIN_DEST"/ | grep -v "^total" | awk '{print "    " $9 ": " $5}'

# Get absolute output path
if [[ "$OUTPUT_GLDF" != /* ]]; then
    OUTPUT_GLDF="$(pwd)/$OUTPUT_GLDF"
fi

# Repack GLDF
echo "Creating output GLDF..."
cd "$TEMP_DIR"
rm -f "$OUTPUT_GLDF"  # Remove if exists
zip -r -q "$OUTPUT_GLDF" .
cd - > /dev/null

# Show result
OUTPUT_SIZE=$(ls -lh "$OUTPUT_GLDF" | awk '{print $5}')
echo ""
echo "Created: $OUTPUT_GLDF ($OUTPUT_SIZE)"
echo ""
echo "Embedded plugins:"
unzip -l "$OUTPUT_GLDF" | grep "other/viewer/" | head -20
