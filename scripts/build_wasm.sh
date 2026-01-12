#!/bin/bash
# Build GLDF WASM viewer (gldf-rs-wasm + gldf-bevy)
#
# Usage:
#   ./scripts/build_wasm.sh              # Build (debug mode, faster)
#   ./scripts/build_wasm.sh local        # Build and serve locally on port 8052
#   ./scripts/build_wasm.sh deploy       # Build for production (release mode)
#   ./scripts/build_wasm.sh --skip-bevy  # Skip Bevy 3D viewer build (faster)
#   ./scripts/build_wasm.sh --release    # Force release build
#
# Note: Do NOT use 'trunk serve' - it doesn't work with the Bevy 3D loader.
#       Use this script with 'local' argument instead.
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
WASM_DIR="$PROJECT_DIR/crates/gldf-rs-wasm"
DIST_DIR="$WASM_DIR/dist"

MODE="build"
SKIP_BEVY=false
RELEASE=false

# Parse arguments
for arg in "$@"; do
    case $arg in
        local)
            MODE="local"
            ;;
        deploy)
            MODE="deploy"
            RELEASE=true
            ;;
        --skip-bevy)
            SKIP_BEVY=true
            ;;
        --release)
            RELEASE=true
            ;;
    esac
done

echo "=== GLDF WASM Build ==="
echo "Mode: $MODE"
echo "Release: $RELEASE"
echo ""

cd "$PROJECT_DIR"

# Step 1: Build main WASM app with trunk
echo "[1/3] Building gldf-rs-wasm with trunk..."
cd "$WASM_DIR"

if [ "$RELEASE" = true ]; then
    # For release builds, temporarily disable wasm-opt (it fails with certain WASM features)
    # by hiding it from PATH during trunk build
    WASM_OPT_PATH=$(which wasm-opt 2>/dev/null || true)
    if [ -n "$WASM_OPT_PATH" ] && [ -f "$WASM_OPT_PATH" ]; then
        echo "Temporarily disabling wasm-opt for trunk build..."
        mv "$WASM_OPT_PATH" "${WASM_OPT_PATH}.bak"
        trap "mv '${WASM_OPT_PATH}.bak' '$WASM_OPT_PATH' 2>/dev/null || true" EXIT
    fi
    trunk build --release
else
    trunk build
fi

# Step 2: Build Bevy 3D viewer (optional)
if [ "$SKIP_BEVY" = false ]; then
    echo ""
    echo "[2/3] Building gldf-bevy for WASM..."
    cd "$PROJECT_DIR"
    cargo build --release --target wasm32-unknown-unknown -p gldf-bevy --lib

    # Generate JS bindings
    echo "Generating Bevy JS bindings..."
    WASM_FILE="$PROJECT_DIR/target/wasm32-unknown-unknown/release/gldf_bevy.wasm"
    BEVY_OUT_DIR="$DIST_DIR/bevy"
    mkdir -p "$BEVY_OUT_DIR"
    rm -f "$BEVY_OUT_DIR"/*.js "$BEVY_OUT_DIR"/*.wasm

    wasm-bindgen "$WASM_FILE" \
        --out-dir "$BEVY_OUT_DIR" \
        --target web \
        --no-typescript

    # Restore wasm-opt if we hid it
    if [ -f "${WASM_OPT_PATH}.bak" ]; then
        mv "${WASM_OPT_PATH}.bak" "$WASM_OPT_PATH"
        trap - EXIT
    fi

    # Optimize Bevy WASM if wasm-opt available (Bevy WASM works with wasm-opt)
    WASM_OPT_PATH=$(which wasm-opt 2>/dev/null || true)
    if [ -n "$WASM_OPT_PATH" ] && [ -f "$WASM_OPT_PATH" ]; then
        echo "Optimizing Bevy WASM..."
        wasm-opt -O3 "$BEVY_OUT_DIR/gldf_bevy_bg.wasm" -o "$BEVY_OUT_DIR/gldf_bevy_bg.wasm" 2>/dev/null || echo "wasm-opt failed, skipping optimization"
    fi

    # Add content hash
    HASH=$(md5 -q "$BEVY_OUT_DIR/gldf_bevy_bg.wasm" | head -c 8)
    mv "$BEVY_OUT_DIR/gldf_bevy.js" "$BEVY_OUT_DIR/gldf-bevy-viewer-${HASH}.js"
    mv "$BEVY_OUT_DIR/gldf_bevy_bg.wasm" "$BEVY_OUT_DIR/gldf-bevy-viewer-${HASH}_bg.wasm"
    sed -i '' "s/gldf_bevy_bg.wasm/gldf-bevy-viewer-${HASH}_bg.wasm/g" "$BEVY_OUT_DIR/gldf-bevy-viewer-${HASH}.js"
    echo "{\"hash\":\"${HASH}\",\"js\":\"gldf-bevy-viewer-${HASH}.js\",\"wasm\":\"gldf-bevy-viewer-${HASH}_bg.wasm\"}" > "$BEVY_OUT_DIR/manifest.json"
    echo "Bevy hash: $HASH"
else
    echo ""
    echo "[2/3] Skipping Bevy build (--skip-bevy)"
fi

# Step 3: Copy static files (including test GLDFs)
echo ""
echo "[3/3] Copying static files..."
cp -f "$WASM_DIR/src/static/"*.gldf "$DIST_DIR/" 2>/dev/null || true
cp -f "$WASM_DIR/src/static/"*.js "$DIST_DIR/" 2>/dev/null || true

echo ""
echo "=== Build Complete ==="
echo "Output: $DIST_DIR"
du -sh "$DIST_DIR"
echo ""

# List key files
echo "Key files:"
ls -lh "$DIST_DIR"/*.wasm 2>/dev/null | awk '{print "  " $9 ": " $5}'
ls -lh "$DIST_DIR"/*.gldf 2>/dev/null | awk '{print "  " $9 ": " $5}'

# Handle modes
if [ "$MODE" = "local" ]; then
    echo ""
    echo "=== Starting local server ==="
    echo "URL: http://localhost:8052"
    echo "Press Ctrl+C to stop"
    echo ""
    python3 -m http.server 8052 --directory "$DIST_DIR"
elif [ "$MODE" = "deploy" ]; then
    echo ""
    echo "=== Ready for deployment ==="
    echo "Upload contents of: $DIST_DIR"
fi
