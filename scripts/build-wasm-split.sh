#!/bin/bash
# Build script for split WASM bundles with Brotli pre-compression
#
# Builds TWO separate WASM bundles:
#   1. Leptos editor (~5MB compressed) - loads immediately on page load
#   2. Bevy 3D viewer (~4MB compressed) - loads on demand when user clicks "3D Scene" tab
#
# Usage:
#   ./build-wasm-split.sh [local|deploy|force]
#
# The split architecture ensures fast initial page load while still
# providing full 3D visualization capabilities when needed.
#
# Pre-compressed .br files are generated for servers that support Content-Encoding: br
# (nginx, Cloudflare, Vercel, Netlify, etc.)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
WASM_DIR="$ROOT_DIR/crates/gldf-rs-wasm"
BEVY_DIR="$ROOT_DIR/crates/gldf-bevy"
BEVY_OUTPUT="$ROOT_DIR/target/wasm32-unknown-unknown/web-release"

# Check for brotli
HAVE_BROTLI=false
if command -v brotli &> /dev/null; then
    HAVE_BROTLI=true
fi

# Help
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo "Usage: $0 [local|deploy|force]"
    echo ""
    echo "Builds two WASM bundles with Brotli pre-compression:"
    echo "  - Leptos editor:  ~5MB raw -> ~500KB compressed (loads immediately)"
    echo "  - Bevy 3D viewer: ~22MB raw -> ~4MB compressed (loads on demand)"
    echo ""
    echo "Options:"
    echo "  local   Build (if needed) and start local server on port 8052"
    echo "  deploy  Build (if needed) and deploy to gldf.icu"
    echo "  force   Force rebuild everything"
    echo "  (none)  Build only if needed, show next steps"
    echo ""
    echo "The script automatically detects if sources have changed and skips"
    echo "unnecessary rebuilds. Use 'force' to rebuild everything."
    echo ""
    echo "Output: $WASM_DIR/dist/"
    exit 0
fi

# Check if force rebuild requested
FORCE_BUILD=false
if [ "$1" = "force" ]; then
    FORCE_BUILD=true
    shift  # Remove 'force' from args so local/deploy still work
fi

# Function to get newest mtime in a directory (recursively for .rs files)
newest_source_time() {
    find "$1" -name "*.rs" -o -name "Cargo.toml" 2>/dev/null | xargs stat -f "%m" 2>/dev/null | sort -rn | head -1
}

# Function to get mtime of a file
file_mtime() {
    stat -f "%m" "$1" 2>/dev/null || echo "0"
}

# Check if Bevy needs rebuild
BEVY_WASM_FILE="$BEVY_OUTPUT/gldf-bevy-viewer_bg.wasm"
BEVY_NEEDS_BUILD=true
if [ "$FORCE_BUILD" = false ] && [ -f "$BEVY_WASM_FILE" ]; then
    BEVY_SRC_TIME=$(newest_source_time "$BEVY_DIR/src")
    BEVY_WASM_TIME=$(file_mtime "$BEVY_WASM_FILE")
    if [ "$BEVY_WASM_TIME" -gt "$BEVY_SRC_TIME" ] 2>/dev/null; then
        BEVY_NEEDS_BUILD=false
    fi
fi

# Check if Leptos needs rebuild
LEPTOS_WASM_FILE=$(ls "$WASM_DIR/dist/"*_bg.wasm 2>/dev/null | head -1)
LEPTOS_NEEDS_BUILD=true
if [ "$FORCE_BUILD" = false ] && [ -n "$LEPTOS_WASM_FILE" ] && [ -f "$LEPTOS_WASM_FILE" ]; then
    LEPTOS_SRC_TIME=$(newest_source_time "$WASM_DIR/src")
    LEPTOS_WASM_TIME=$(file_mtime "$LEPTOS_WASM_FILE")
    if [ "$LEPTOS_WASM_TIME" -gt "$LEPTOS_SRC_TIME" ] 2>/dev/null; then
        LEPTOS_NEEDS_BUILD=false
    fi
fi

# Check if bevy files are in dist
BEVY_IN_DIST=false
if ls "$WASM_DIR/dist/bevy/"*.wasm &>/dev/null; then
    BEVY_IN_DIST=true
fi

echo "=== GLDF Viewer WASM Build ==="
echo ""

if [ "$FORCE_BUILD" = true ]; then
    echo "  Force rebuild requested"
elif [ "$BEVY_NEEDS_BUILD" = false ] && [ "$LEPTOS_NEEDS_BUILD" = false ] && [ "$BEVY_IN_DIST" = true ]; then
    echo "  All builds up to date!"
    echo ""

    # Still need to show sizes and handle local/deploy
    LEPTOS_WASM=$(ls "$WASM_DIR/dist/"*_bg.wasm 2>/dev/null | head -1)
    BEVY_WASM=$(ls "$WASM_DIR/dist/bevy/"*_bg.wasm 2>/dev/null | head -1)
    LEPTOS_SIZE=$(ls -lh "$LEPTOS_WASM" 2>/dev/null | awk '{print $5}')
    BEVY_SIZE=$(ls -lh "$BEVY_WASM" 2>/dev/null | awk '{print $5}')

    echo "Bundle sizes:"
    if [ "$HAVE_BROTLI" = true ]; then
        LEPTOS_BR=$(ls -lh "${LEPTOS_WASM}.br" 2>/dev/null | awk '{print $5}')
        BEVY_BR=$(ls -lh "${BEVY_WASM}.br" 2>/dev/null | awk '{print $5}')
        echo "  Leptos editor:  $LEPTOS_SIZE -> $LEPTOS_BR"
        echo "  Bevy 3D viewer: $BEVY_SIZE -> $BEVY_BR"
    else
        echo "  Leptos editor:  $LEPTOS_SIZE"
        echo "  Bevy 3D viewer: $BEVY_SIZE"
    fi
    echo ""
    echo "Output: $WASM_DIR/dist/"
    echo ""

    # Handle post-build action
    case "$1" in
        local|serve)
            echo "Starting local server on port 8052..."
            echo "Open http://localhost:8052"
            echo ""
            python3 -m http.server 8052 -d "$WASM_DIR/dist"
            ;;
        deploy)
            echo "Deploying to gldf.icu..."
            rsync -avz  "$WASM_DIR/dist/" trahe.eu:/var/www/gldf.icu/html/
            echo ""
            echo "Deployed to https://gldf.icu/"
            ;;
        *)
            echo "To serve locally:  $0 local"
            echo "To deploy:         $0 deploy"
            echo "To force rebuild:  $0 force"
            ;;
    esac
    exit 0
else
    # Bevy files missing from dist - need to copy even if build is up to date
    if [ "$BEVY_IN_DIST" = false ] && [ "$BEVY_NEEDS_BUILD" = false ]; then
        echo "  Bevy files missing from dist, will copy..."
    fi
fi

echo "  Bundle 1: Leptos editor (loads immediately)"
echo "  Bundle 2: Bevy 3D viewer (loads on demand)"
if [ "$HAVE_BROTLI" = true ]; then
    echo "  Brotli pre-compression: enabled"
fi
echo ""

# Step 1: Build Bevy 3D viewer (if needed)
if [ "$BEVY_NEEDS_BUILD" = true ]; then
    echo "[1/5] Building Bevy 3D viewer with bevy-cli..."
    cd "$BEVY_DIR"
    bevy build --release --features standalone web
else
    echo "[1/5] Bevy 3D viewer: up to date (skipped)"
fi
echo ""

# Step 2: Build Leptos editor (if needed)
if [ "$LEPTOS_NEEDS_BUILD" = true ]; then
    echo "[2/5] Building Leptos editor with trunk..."
    cd "$WASM_DIR"
    # Trunk.toml sets no_wasm_opt = true to avoid compatibility issues
    trunk build --release
else
    echo "[2/5] Leptos editor: up to date (skipped)"
fi
echo ""

# Step 3: Add content hashes to Bevy files for cache busting
echo "[3/5] Adding content hashes to Bevy files..."
mkdir -p "$WASM_DIR/dist/bevy"

# Clean old bevy files
rm -f "$WASM_DIR/dist/bevy/"*.js "$WASM_DIR/dist/bevy/"*.wasm "$WASM_DIR/dist/bevy/"*.br

# bevy-cli outputs with binary name: gldf-bevy-viewer.js and gldf-bevy-viewer_bg.wasm
BEVY_JS="$BEVY_OUTPUT/gldf-bevy-viewer.js"
BEVY_WASM="$BEVY_OUTPUT/gldf-bevy-viewer_bg.wasm"

# Calculate short hash (first 16 chars of md5)
JS_HASH=$(md5 -q "$BEVY_JS" | cut -c1-16)
WASM_HASH=$(md5 -q "$BEVY_WASM" | cut -c1-16)

# Copy with hashed names
cp "$BEVY_JS" "$WASM_DIR/dist/bevy/gldf-bevy-${JS_HASH}.js"
cp "$BEVY_WASM" "$WASM_DIR/dist/bevy/gldf-bevy-${WASM_HASH}_bg.wasm"

# Update the JS file to reference the hashed WASM filename
sed -i '' "s/gldf-bevy-viewer_bg.wasm/gldf-bevy-${WASM_HASH}_bg.wasm/g" "$WASM_DIR/dist/bevy/gldf-bevy-${JS_HASH}.js"
echo ""

# Step 4: Generate gldf-bevy-loader.js with hashed filenames
echo "[4/5] Generating gldf-bevy-loader.js..."

cat > "$WASM_DIR/dist/gldf-bevy-loader.js" << EOF
// Lazy loader for GLDF Bevy 3D Scene Viewer
// Auto-generated with content hashes for cache busting
//
// The 3D viewer (~4MB compressed) is NOT loaded until the user clicks "3D Scene" tab.
// This keeps the initial page load fast (~500KB for the editor only).

let bevyLoaded = false;
let bevyLoading = false;
let loadPromise = null;

// Storage keys for L3D/LDT data
const L3D_STORAGE_KEY = 'gldf_current_l3d';
const LDT_STORAGE_KEY = 'gldf_current_ldt';
const EMITTER_CONFIG_KEY = 'gldf_emitter_config';
const GLDF_TIMESTAMP_KEY = 'gldf_timestamp';
const STAR_SKY_STORAGE_KEY = 'gldf_star_sky_json';

/**
 * Save L3D data to localStorage for Bevy viewer
 * @param {Uint8Array} l3dData - L3D file bytes
 * @param {string|null} ldtData - LDT file content (optional)
 * @param {string|null} emitterConfig - JSON string of emitter configurations (optional)
 */
function saveL3dForBevy(l3dData, ldtData, emitterConfig) {
    console.log('[Bevy] saveL3dForBevy called with:', l3dData?.length, 'bytes L3D');
    try {
        // Convert to base64 for storage (handle large arrays properly)
        let binary = '';
        const bytes = new Uint8Array(l3dData);
        const chunkSize = 0x8000; // Process in chunks to avoid stack overflow
        for (let i = 0; i < bytes.length; i += chunkSize) {
            const chunk = bytes.subarray(i, Math.min(i + chunkSize, bytes.length));
            binary += String.fromCharCode.apply(null, chunk);
        }
        const base64 = btoa(binary);
        console.log('[Bevy] Base64 length:', base64.length);
        localStorage.setItem(L3D_STORAGE_KEY, base64);

        if (ldtData) {
            localStorage.setItem(LDT_STORAGE_KEY, ldtData);
            console.log('[Bevy] LDT stored, length:', ldtData.length);
        } else {
            localStorage.removeItem(LDT_STORAGE_KEY);
        }

        // Store emitter config for per-emitter rendering
        if (emitterConfig) {
            localStorage.setItem(EMITTER_CONFIG_KEY, emitterConfig);
            console.log('[Bevy] Emitter config stored:', emitterConfig);
        } else {
            localStorage.removeItem(EMITTER_CONFIG_KEY);
        }

        // Update timestamp to trigger Bevy reload
        const ts = Date.now().toString();
        localStorage.setItem(GLDF_TIMESTAMP_KEY, ts);
        console.log('[Bevy] All data saved to localStorage, timestamp:', ts);
    } catch (e) {
        console.error('[Bevy] Failed to save L3D data:', e);
    }
}

async function loadBevyViewer() {
    if (bevyLoaded) {
        console.log("[Bevy] Already loaded");
        return;
    }
    if (bevyLoading && loadPromise) {
        console.log("[Bevy] Loading in progress, waiting...");
        return loadPromise;
    }

    bevyLoading = true;
    console.log("[Bevy] Loading 3D viewer (~4MB compressed)...");

    loadPromise = (async () => {
        try {
            const bevy = await import('./bevy/gldf-bevy-${JS_HASH}.js');
            await bevy.default();
            bevy.run_on_canvas("#bevy-canvas");
            bevyLoaded = true;
            bevyLoading = false;
            console.log("[Bevy] 3D viewer loaded successfully");
        } catch (error) {
            const errorStr = error.toString();
            if (errorStr.includes("Using exceptions for control flow") ||
                errorStr.includes("don't mind me")) {
                console.log("[Bevy] Ignoring control flow exception (not a real error)");
                bevyLoaded = true;
                bevyLoading = false;
                return;
            }
            console.error("[Bevy] Failed to load 3D viewer:", error);
            bevyLoading = false;
            loadPromise = null;
            throw error;
        }
    })();

    return loadPromise;
}

function isBevyLoaded() { return bevyLoaded; }
function isBevyLoading() { return bevyLoading; }

/**
 * Save star sky JSON data to localStorage for Bevy viewer
 * @param {string} jsonData - Star sky JSON string
 */
function saveStarSkyForBevy(jsonData) {
    console.log('[Bevy] saveStarSkyForBevy called with:', jsonData?.length, 'chars');
    try {
        localStorage.setItem(STAR_SKY_STORAGE_KEY, jsonData);
        const ts = Date.now().toString();
        localStorage.setItem(GLDF_TIMESTAMP_KEY, ts);
        console.log('[Bevy] Star sky data saved, timestamp:', ts);
    } catch (e) {
        console.error('[Bevy] Failed to save star sky data:', e);
    }
}

/**
 * Clear star sky data from localStorage
 */
function clearStarSkyData() {
    localStorage.removeItem(STAR_SKY_STORAGE_KEY);
    console.log('[Bevy] Star sky data cleared');
}

// Expose to window for WASM to call
window.loadBevyViewer = loadBevyViewer;
window.isBevyLoaded = isBevyLoaded;
window.isBevyLoading = isBevyLoading;
window.saveL3dForBevy = saveL3dForBevy;
window.saveStarSkyForBevy = saveStarSkyForBevy;
window.clearStarSkyData = clearStarSkyData;

console.log("[Bevy] Loader ready (JS: ${JS_HASH}, WASM: ${WASM_HASH})");
EOF

# Also create manifest.json for backwards compatibility
cat > "$WASM_DIR/dist/bevy/manifest.json" << EOF
{"hash":"${JS_HASH}","js":"gldf-bevy-${JS_HASH}.js","wasm":"gldf-bevy-${WASM_HASH}_bg.wasm"}
EOF

# Create version.json with all hashes for versioning
# Uses same hash format as gldf_rs::version::compute_hash (first 16 hex chars of MD5)
LEPTOS_JS_FILE=$(ls "$WASM_DIR/dist/"*.js 2>/dev/null | grep -v loader | head -1)
LEPTOS_WASM_FILE=$(ls "$WASM_DIR/dist/"*_bg.wasm 2>/dev/null | head -1)
LEPTOS_JS_HASH=$(md5 -q "$LEPTOS_JS_FILE" 2>/dev/null | cut -c1-16)
LEPTOS_WASM_HASH=$(md5 -q "$LEPTOS_WASM_FILE" 2>/dev/null | cut -c1-16)
BUILD_TIME=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
GIT_HASH=$(git -C "$ROOT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")
LIB_VERSION=$(grep "^version" "$ROOT_DIR/crates/gldf-rs-lib/Cargo.toml" 2>/dev/null | head -1 | sed 's/.*"\(.*\)".*/\1/' || echo "0.0.0")
# If version uses workspace, get from root
if echo "$LIB_VERSION" | grep -q "workspace"; then
    LIB_VERSION=$(grep "^version" "$ROOT_DIR/Cargo.toml" 2>/dev/null | head -1 | sed 's/.*"\(.*\)".*/\1/' || echo "0.0.0")
fi

cat > "$WASM_DIR/dist/version.json" << EOF
{
  "build_time": "${BUILD_TIME}",
  "version": "${LIB_VERSION}",
  "git_hash": "${GIT_HASH}",
  "components": {
    "leptos": {
      "js_hash": "${LEPTOS_JS_HASH}",
      "wasm_hash": "${LEPTOS_WASM_HASH}"
    },
    "bevy": {
      "js_hash": "${JS_HASH}",
      "wasm_hash": "${WASM_HASH}"
    }
  }
}
EOF
echo "  Created version.json (v${LIB_VERSION}, git:${GIT_HASH})"
echo ""

# Step 5: Pre-compress with Brotli (parallel)
echo "[5/5] Pre-compressing with Brotli..."

if [ "$HAVE_BROTLI" = true ]; then
    # Get number of CPU cores for parallel compression
    if command -v nproc &> /dev/null; then
        NCPU=$(nproc)
    elif command -v sysctl &> /dev/null; then
        NCPU=$(sysctl -n hw.ncpu 2>/dev/null || echo 4)
    else
        NCPU=4
    fi
    echo "  Using $NCPU parallel jobs..."

    # Collect all files to compress
    FILES_TO_COMPRESS=()

    # WASM files (these are the big ones)
    for f in "$WASM_DIR/dist/"*.wasm "$WASM_DIR/dist/bevy/"*.wasm; do
        [ -f "$f" ] && FILES_TO_COMPRESS+=("$f")
    done

    # JS files
    for f in "$WASM_DIR/dist/"*.js "$WASM_DIR/dist/bevy/"*.js; do
        [ -f "$f" ] && FILES_TO_COMPRESS+=("$f")
    done

    # CSS files
    for f in "$WASM_DIR/dist/"*.css; do
        [ -f "$f" ] && FILES_TO_COMPRESS+=("$f")
    done

    # Compress all files in parallel using xargs
    echo "  Compressing ${#FILES_TO_COMPRESS[@]} files in parallel..."
    printf '%s\n' "${FILES_TO_COMPRESS[@]}" | xargs -P "$NCPU" -I {} brotli -f -q 11 {}

    echo "  Done!"
else
    echo "  brotli not found, skipping pre-compression."
    echo "  Install with: brew install brotli"
fi
echo ""

# Summary
echo "=== Build Complete ==="
echo ""

# Get sizes
LEPTOS_WASM=$(ls "$WASM_DIR/dist/"*_bg.wasm 2>/dev/null | head -1)
BEVY_WASM=$(ls "$WASM_DIR/dist/bevy/"*_bg.wasm 2>/dev/null | head -1)

LEPTOS_SIZE=$(ls -lh "$LEPTOS_WASM" 2>/dev/null | awk '{print $5}')
BEVY_SIZE=$(ls -lh "$BEVY_WASM" 2>/dev/null | awk '{print $5}')

echo "Bundle sizes (raw / compressed):"
if [ "$HAVE_BROTLI" = true ]; then
    LEPTOS_BR=$(ls -lh "${LEPTOS_WASM}.br" 2>/dev/null | awk '{print $5}')
    BEVY_BR=$(ls -lh "${BEVY_WASM}.br" 2>/dev/null | awk '{print $5}')

    echo "  Leptos editor:  $LEPTOS_SIZE -> $LEPTOS_BR (loads immediately)"
    echo "  Bevy 3D viewer: $BEVY_SIZE -> $BEVY_BR (loads on demand)"
else
    echo "  Leptos editor:  $LEPTOS_SIZE (loads immediately)"
    echo "  Bevy 3D viewer: $BEVY_SIZE (loads on demand)"
fi
echo ""
echo "Hashed filenames:"
echo "  Bevy: gldf-bevy-${JS_HASH}.js / gldf-bevy-${WASM_HASH}_bg.wasm"
echo ""
echo "Output: $WASM_DIR/dist/"
echo ""
if [ "$HAVE_BROTLI" = true ]; then
    echo "Pre-compressed .br files included for servers with static Brotli support."
    echo ""
fi

# Copy/regenerate demo GLDF files
echo "Updating demo GLDF files..."
# Regenerate enriched demo files with latest script
if [ -f "$ROOT_DIR/scripts/enrich_aec_gldf.py" ]; then
    echo "  Regenerating enriched GLDF demos..."
    cd "$ROOT_DIR"
    python3 scripts/enrich_aec_gldf.py > /dev/null 2>&1 || echo "  Warning: Failed to regenerate enriched GLDFs"
fi
# Copy demo files to dist
for gldf in "$ROOT_DIR/tests/data/aec_ga15.gldf" \
            "$ROOT_DIR/tests/data/aec_ga15_enriched.gldf" \
            "$ROOT_DIR/tests/data/aec_ga15_enriched_spectral.gldf" \
            "$ROOT_DIR/tests/data/SLV - Tria 2.gldf"; do
    if [ -f "$gldf" ]; then
        # Normalize filename (replace spaces with underscores, lowercase)
        basename=$(basename "$gldf" | tr ' ' '_' | tr '[:upper:]' '[:lower:]')
        cp "$gldf" "$WASM_DIR/dist/$basename"
        echo "  Copied: $basename"
    fi
done

# Copy astral sky demo files (stars with TM-33 spectral data)
for skyfile in "$ROOT_DIR/tests/data/astral_sky_"*.gldf "$ROOT_DIR/tests/data/astral_sky_"*.json; do
    if [ -f "$skyfile" ]; then
        basename=$(basename "$skyfile")
        cp "$skyfile" "$WASM_DIR/dist/$basename"
        echo "  Copied: $basename"
    fi
done
echo ""

# Handle post-build action
case "$1" in
    local|serve)
        echo "Starting local server on port 8052..."
        echo "Open http://localhost:8052"
        echo ""
        python3 -m http.server 8052 -d "$WASM_DIR/dist"
        ;;
    deploy)
        echo "Deploying to gldf.icu..."
        rsync -avz "$WASM_DIR/dist/" trahe.eu:/var/www/gldf.icu/html/
        echo ""
        echo "Deployed to https://gldf.icu/"
        ;;
    *)
        echo "To serve locally:  $0 local"
        echo "To deploy:         $0 deploy"
        echo "To force rebuild:  $0 force"
        ;;
esac
