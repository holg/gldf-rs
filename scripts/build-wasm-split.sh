#!/bin/bash
# Build script for split WASM bundles
# - Leptos editor: ~5MB (loads immediately)
# - Bevy 3D viewer: ~22MB (loads on demand)
# All files get content hashes in filenames for cache busting

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
WASM_DIR="$ROOT_DIR/crates/gldf-rs-leptos"
BEVY_DIR="$ROOT_DIR/crates/gldf-bevy"

echo "=== Building GLDF Viewer WASM (Split Bundles) ==="
echo ""

# Step 1: Build Bevy 3D viewer with bevy-cli
echo "[1/4] Building Bevy 3D viewer with bevy-cli..."
cd "$BEVY_DIR"
bevy build --release web

# The output is in target/wasm32-unknown-unknown/web-release/
BEVY_OUTPUT="$ROOT_DIR/target/wasm32-unknown-unknown/web-release"

echo ""
echo "[2/4] Building Leptos editor with trunk..."
cd "$WASM_DIR"
trunk build --release

echo ""
echo "[3/4] Adding content hashes to Bevy files..."
mkdir -p "$WASM_DIR/dist/bevy"

# Clean old bevy files
rm -f "$WASM_DIR/dist/bevy/"*.js "$WASM_DIR/dist/bevy/"*.wasm

# Calculate short hash (first 16 chars of md5)
JS_HASH=$(md5 -q "$BEVY_OUTPUT/gldf-bevy-viewer.js" | cut -c1-16)
WASM_HASH=$(md5 -q "$BEVY_OUTPUT/gldf-bevy-viewer_bg.wasm" | cut -c1-16)

# Copy with hashed names
cp "$BEVY_OUTPUT/gldf-bevy-viewer.js" "$WASM_DIR/dist/bevy/gldf-bevy-${JS_HASH}.js"
cp "$BEVY_OUTPUT/gldf-bevy-viewer_bg.wasm" "$WASM_DIR/dist/bevy/gldf-bevy-${WASM_HASH}_bg.wasm"

# Update the JS file to reference the hashed WASM filename
sed -i '' "s/gldf-bevy-viewer_bg.wasm/gldf-bevy-${WASM_HASH}_bg.wasm/g" "$WASM_DIR/dist/bevy/gldf-bevy-${JS_HASH}.js"

echo ""
echo "[4/4] Updating bevy-loader.js with hashed filenames..."

# Create bevy-loader with correct hashed filename
cat > "$WASM_DIR/dist/bevy-loader.js" << EOF
// Lazy loader for GLDF Bevy 3D Scene Viewer
// Auto-generated with content hashes for cache busting

let bevyLoaded = false;
let bevyLoading = false;
let loadPromise = null;

async function loadBevyViewer() {
    if (bevyLoaded) {
        console.log("[GLDF Bevy] Already loaded");
        return;
    }
    if (bevyLoading && loadPromise) {
        console.log("[GLDF Bevy] Loading in progress, waiting...");
        return loadPromise;
    }

    bevyLoading = true;
    console.log("[GLDF Bevy] Loading 3D viewer (~22MB)...");

    loadPromise = (async () => {
        try {
            const bevy = await import('./bevy/gldf-bevy-${JS_HASH}.js');
            await bevy.default();
            bevyLoaded = true;
            bevyLoading = false;
            console.log("[GLDF Bevy] 3D viewer loaded successfully");
        } catch (error) {
            const errorStr = error.toString();
            if (errorStr.includes("Using exceptions for control flow") ||
                errorStr.includes("don't mind me")) {
                console.log("[GLDF Bevy] Ignoring control flow exception (not a real error)");
                bevyLoaded = true;
                bevyLoading = false;
                return;
            }
            console.error("[GLDF Bevy] Failed to load 3D viewer:", error);
            bevyLoading = false;
            loadPromise = null;
            throw error;
        }
    })();

    return loadPromise;
}

function isBevyLoaded() { return bevyLoaded; }
function isBevyLoading() { return bevyLoading; }

window.loadBevyViewer = loadBevyViewer;
window.isBevyLoaded = isBevyLoaded;
window.isBevyLoading = isBevyLoading;

console.log("[GLDF Bevy] Loader ready (JS: ${JS_HASH}, WASM: ${WASM_HASH})");
EOF

# Check sizes
echo ""
echo "=== Build Complete ==="
echo ""
echo "Bundle sizes:"
LEPTOS_SIZE=$(ls -lh "$WASM_DIR/dist/"*_bg.wasm 2>/dev/null | awk '{print $5}' | head -1)
BEVY_SIZE=$(ls -lh "$WASM_DIR/dist/bevy/"*_bg.wasm 2>/dev/null | awk '{print $5}')
echo "  Leptos editor:  $LEPTOS_SIZE (loads immediately)"
echo "  Bevy 3D viewer: $BEVY_SIZE (loads on demand)"
echo ""
echo "Hashed filenames:"
echo "  gldf-bevy-${JS_HASH}.js"
echo "  gldf-bevy-${WASM_HASH}_bg.wasm"
echo ""
echo "Output directory: $WASM_DIR/dist/"
echo ""
echo "To serve locally:"
echo "  python3 -m http.server 8052 -d $WASM_DIR/dist"
echo "  Open http://localhost:8052"
