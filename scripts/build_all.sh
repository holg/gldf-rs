#!/bin/bash
# Build main WASM app and optionally start local server
#
# Note: Plugin WASMs (gldf-starsky-wasm, gldf-stadium-plugin) are NOT built here.
# They are bundled inside GLDF files and loaded via the plugin system at runtime.
# Use scripts/build_stadium_wasm.sh or scripts/build-all-wasm.sh to build plugins separately.
#
# Usage:
#   ./scripts/build_all.sh local   # Build and start local server on port 8052
#   ./scripts/build_all.sh deploy  # Build for deployment (no server)
#   ./scripts/build_all.sh         # Same as 'local'
#   ./scripts/build_all.sh force   # Force rebuild everything

set -e

MODE="${1:-local}"
PORT=8052
DIST_DIR="crates/gldf-rs-wasm/dist"
CACHE_DIR=".build_cache"

echo "=== GLDF-RS Build ==="
echo "Mode: $MODE"
echo ""

# Create cache directory
mkdir -p "$CACHE_DIR"

# Function to compute hash of source files for a crate
compute_source_hash() {
    local crate_path="$1"
    # Hash all .rs files, Cargo.toml, and relevant config files
    find "$crate_path/src" -name "*.rs" -type f 2>/dev/null | sort | xargs cat 2>/dev/null | md5 | head -c 16
    cat "$crate_path/Cargo.toml" 2>/dev/null | md5 | head -c 8
}

# Function to check if rebuild is needed
needs_rebuild() {
    local crate_name="$1"
    local cache_file="$CACHE_DIR/${crate_name}.hash"
    local current_hash="$2"
    local output_check="$3"  # Optional: file or directory to check exists

    if [ "$MODE" = "force" ]; then
        return 0  # Force rebuild
    fi

    # Check if output files exist (if specified)
    if [ -n "$output_check" ] && [ ! -e "$output_check" ]; then
        echo "   Output missing: $output_check"
        return 0  # Output doesn't exist, needs rebuild
    fi

    if [ ! -f "$cache_file" ]; then
        return 0  # No cache, needs rebuild
    fi

    local cached_hash=$(cat "$cache_file")
    if [ "$cached_hash" != "$current_hash" ]; then
        return 0  # Hash changed, needs rebuild
    fi

    return 1  # No rebuild needed
}

# Function to save hash to cache
save_hash() {
    local crate_name="$1"
    local hash="$2"
    echo "$hash" > "$CACHE_DIR/${crate_name}.hash"
}

# ========================================
# Build gldf-rs-wasm (main app)
# ========================================

# Compute hash for gldf-rs-wasm and its dependencies
WASM_HASH=$(
    (
        compute_source_hash "crates/gldf-rs-wasm"
        compute_source_hash "crates/gldf-rs-lib"
        cat "crates/gldf-rs-wasm/index.html" 2>/dev/null | md5 | head -c 8
        cat "crates/gldf-rs-wasm/Trunk.toml" 2>/dev/null | md5 | head -c 8
    ) | md5 | head -c 16
)

if needs_rebuild "gldf-rs-wasm" "$WASM_HASH" "$DIST_DIR/index.html"; then
    echo "📦 Building gldf-rs-wasm (sources changed or output missing)..."
    cd crates/gldf-rs-wasm
    # Temporarily hide wasm-opt to skip optimization (version 116 incompatible with new WASM features)
    WASM_OPT_PATH=$(which wasm-opt 2>/dev/null || true)
    if [ -n "$WASM_OPT_PATH" ] && [ -f "$WASM_OPT_PATH" ]; then
        mv "$WASM_OPT_PATH" "${WASM_OPT_PATH}.bak" 2>/dev/null || true
    fi
    trunk build --release --skip-version-check
    # Restore wasm-opt
    if [ -f "${WASM_OPT_PATH}.bak" ]; then
        mv "${WASM_OPT_PATH}.bak" "$WASM_OPT_PATH" 2>/dev/null || true
    fi
    cd ../..
    save_hash "gldf-rs-wasm" "$WASM_HASH"
    echo "✅ gldf-rs-wasm built"
else
    echo "⏭️  gldf-rs-wasm unchanged, skipping build"
fi

# ========================================
# Build gldf-bevy 3D viewer
# ========================================

# Compute hash for gldf-bevy and dependencies
BEVY_HASH=$(
    (
        compute_source_hash "crates/gldf-bevy"
        compute_source_hash "crates/gldf-rs-lib"
    ) | md5 | head -c 16
)

BEVY_OUT_DIR="$DIST_DIR/bevy"

if needs_rebuild "gldf-bevy" "$BEVY_HASH" "$BEVY_OUT_DIR/manifest.json"; then
    echo ""
    echo "📦 Building gldf-bevy 3D viewer (sources changed or output missing)..."
    cargo build --release --target wasm32-unknown-unknown -p gldf-bevy --lib

    # Generate JS bindings with wasm-bindgen
    WASM_FILE="target/wasm32-unknown-unknown/release/gldf_bevy.wasm"
    mkdir -p "$BEVY_OUT_DIR"
    rm -f "$BEVY_OUT_DIR"/*.js "$BEVY_OUT_DIR"/*.wasm "$BEVY_OUT_DIR"/manifest.json

    wasm-bindgen "$WASM_FILE" \
        --out-dir "$BEVY_OUT_DIR" \
        --target web \
        --no-typescript

    # Generate hash from WASM content for cache busting
    HASH=$(md5 -q "$BEVY_OUT_DIR/gldf_bevy_bg.wasm" | head -c 8)

    # Rename files with hash
    mv "$BEVY_OUT_DIR/gldf_bevy.js" "$BEVY_OUT_DIR/gldf-bevy-viewer-${HASH}.js"
    mv "$BEVY_OUT_DIR/gldf_bevy_bg.wasm" "$BEVY_OUT_DIR/gldf-bevy-viewer-${HASH}_bg.wasm"

    # Fix the import path in the JS file
    sed -i '' "s/gldf_bevy_bg.wasm/gldf-bevy-viewer-${HASH}_bg.wasm/g" "$BEVY_OUT_DIR/gldf-bevy-viewer-${HASH}.js"

    # Create manifest file for bevy-loader.js to discover
    echo "{\"hash\":\"${HASH}\",\"js\":\"gldf-bevy-viewer-${HASH}.js\",\"wasm\":\"gldf-bevy-viewer-${HASH}_bg.wasm\"}" > "$BEVY_OUT_DIR/manifest.json"

    save_hash "gldf-bevy" "$BEVY_HASH"
    echo "✅ gldf-bevy built (hash: $HASH)"
else
    echo ""
    echo "⏭️  gldf-bevy unchanged, skipping build"
    # Show existing hash from manifest
    if [ -f "$BEVY_OUT_DIR/manifest.json" ]; then
        HASH=$(cat "$BEVY_OUT_DIR/manifest.json" | grep -o '"hash":"[^"]*"' | cut -d'"' -f4)
        echo "   Using cached build (hash: $HASH)"
    fi
fi

# Copy static assets
echo ""
echo "📋 Copying static assets..."
if [ -d "crates/gldf-rs-wasm/src/static" ]; then
    cp -r crates/gldf-rs-wasm/src/static/* "$DIST_DIR/" 2>/dev/null || true
fi

# Copy test GLDF files for demo (these contain bundled plugins)
mkdir -p "$DIST_DIR/samples"
cp tests/data/*.gldf "$DIST_DIR/samples/" 2>/dev/null || true

echo ""
echo "=== Build Complete ==="
echo ""
echo "Output: $DIST_DIR/"
ls -lh "$DIST_DIR/" | head -20

# Start local server if mode is 'local'
if [ "$MODE" = "local" ]; then
    echo ""
    echo "=== Starting Local Server ==="
    echo "URL: http://localhost:$PORT"
    echo "Press Ctrl+C to stop"
    echo ""

    # Kill any existing server on this port
    lsof -ti:$PORT | xargs kill -9 2>/dev/null || true

    # Start Python HTTP server
    cd "$DIST_DIR"
    python3 -m http.server $PORT
fi
