#!/bin/bash
set -e # Stop on error

# Configuration
CORE_PATH="crates/gldf-rs-core"
WASM_PATH="crates/gldf-rs-wasm"
DIST_ROOT="dist"

# Ensure we start fresh
rm -rf $DIST_ROOT
mkdir -p $DIST_ROOT

echo "🧹 Cleaned previous builds."

# ==============================================================================
# Helper Function to Build a Variant
# ==============================================================================
build_variant() {
    local NAME=$1
    local CLI_FEATURES=$2
    local WASM_FEATURES=$3
    local OUT_DIR="$DIST_ROOT/$NAME"

    echo "----------------------------------------------------------------"
    echo "📦 Building Variant: $NAME"
    echo "   CLI Features:  $CLI_FEATURES"
    echo "   WASM Features: $WASM_FEATURES"
    echo "----------------------------------------------------------------"

    # 1. Clean Data (Force Builder to regenerate)
    rm -f $CORE_PATH/data/*.bin

    # 2. Build Data (Using CLI)
    # We use --release for speed
    echo "   🔨 Baking Data... $CLI_FEATURES"
    cargo run --quiet --release -p gldf-rs-cli --no-default-features --features "$CLI_FEATURES" -- build

    # 3. Build WASM
    echo "   🦀 Compiling WASM... $WASM_PATH --out-dir \"../../$OUT_DIR\" --features \"$WASM_FEATURES\""
    wasm-pack build $WASM_PATH \
        --target web \
        --out-dir "../../$OUT_DIR" \
        --release \
        --quiet \
        --no-default-features \
        --features "$WASM_FEATURES"

    echo "   ✅ Finished: $OUT_DIR"
}

# ==============================================================================
# THE MATRIX
# ==============================================================================

# 1. Standard (Flat, Compact) - Smallest Size
build_variant "flat" \
    "builder,compact" \
    "compact,json"

# 2. Turbo (Flat, Compact, Search Blobs) - Fastest Search, Larger Memory
build_variant "flat-blobs" \
    "builder,compact,search_blobs" \
    "compact,search_blobs,json"

# 3. Legacy (Nested, Compact) - Educational / Compatibility
build_variant "nested" \
    "legacy_model,builder,compact" \
    "legacy_model,compact,json"

# 4. Legacy Turbo (Nested, Compact, Search Blobs)
build_variant "nested-blobs" \
    "legacy_model,builder,compact,search_blobs" \
    "legacy_model,compact,search_blobs,json"

# ==============================================================================
# Summary
# ==============================================================================
echo "----------------------------------------------------------------"
echo "🎉 All builds complete!"
echo "----------------------------------------------------------------"
echo "🎉 Copy js/cs/html from wasm-pack-helper"
cp wasm-pack-helper/* dist/
ls -lh $DIST_ROOT