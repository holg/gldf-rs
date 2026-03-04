#!/bin/bash
# Universal WASM split bundle builder with Brotli pre-compression
#
# Reads configuration from build-config.toml in the same directory.
# Can be used across multiple projects (eulumdat-rs, gldf-rs, acadlisp, etc.)
#
# Usage:
#   ./build-wasm-split.sh          # Build only
#   ./build-wasm-split.sh deploy   # Build and deploy via rsync
#   ./build-wasm-split.sh serve    # Build and serve locally
#   ./build-wasm-split.sh --help   # Show help
#
# The split architecture ensures fast initial page load while still
# providing full 3D visualization and PDF export capabilities when needed.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CONFIG_FILE="$SCRIPT_DIR/build-config.toml"

# =============================================================================
# TOML Parser (simple key=value extraction)
# =============================================================================

# Read a value from TOML config
# Usage: toml_get "section.key" [default_value]
toml_get() {
    local key="$1"
    local default="$2"
    local section=""
    local field=""

    # Split key into section and field (e.g., "project.name" -> "project" "name")
    if [[ "$key" == *.* ]]; then
        section="${key%.*}"
        field="${key##*.}"
    else
        field="$key"
    fi

    local in_section=false
    local current_section=""

    while IFS= read -r line || [[ -n "$line" ]]; do
        # Skip comments and empty lines
        [[ "$line" =~ ^[[:space:]]*# ]] && continue
        [[ -z "${line// }" ]] && continue

        # Check for section header [section] or [section.subsection]
        if [[ "$line" =~ ^\[([a-zA-Z0-9._-]+)\] ]]; then
            current_section="${BASH_REMATCH[1]}"
            if [[ -z "$section" ]] || [[ "$current_section" == "$section" ]] || [[ "$current_section" == "$section."* ]]; then
                in_section=true
            else
                in_section=false
            fi
            continue
        fi

        # If we're in the right section (or no section specified), look for the field
        if [[ "$in_section" == true ]] || [[ -z "$section" && -z "$current_section" ]]; then
            # Match key = value or key = "value"
            if [[ "$line" =~ ^[[:space:]]*${field}[[:space:]]*=[[:space:]]*\"([^\"]*)\" ]]; then
                echo "${BASH_REMATCH[1]}"
                return 0
            elif [[ "$line" =~ ^[[:space:]]*${field}[[:space:]]*=[[:space:]]*([^[:space:]#]+) ]]; then
                echo "${BASH_REMATCH[1]}"
                return 0
            fi
        fi
    done < "$CONFIG_FILE"

    echo "$default"
}

# Read array from TOML (simple format: key = ["a", "b"])
toml_get_array() {
    local key="$1"
    local section="${key%.*}"
    local field="${key##*.}"

    local in_section=false
    local current_section=""

    while IFS= read -r line || [[ -n "$line" ]]; do
        [[ "$line" =~ ^[[:space:]]*# ]] && continue
        [[ -z "${line// }" ]] && continue

        if [[ "$line" =~ ^\[([a-zA-Z0-9._-]+)\] ]]; then
            current_section="${BASH_REMATCH[1]}"
            [[ "$current_section" == "$section" ]] && in_section=true || in_section=false
            continue
        fi

        if [[ "$in_section" == true ]]; then
            if [[ "$line" =~ ^[[:space:]]*${field}[[:space:]]*=[[:space:]]*\[(.+)\] ]]; then
                # Extract array elements
                local array_content="${BASH_REMATCH[1]}"
                # Remove quotes and split by comma
                echo "$array_content" | tr ',' '\n' | sed 's/[" ]//g'
                return 0
            fi
        fi
    done < "$CONFIG_FILE"
}

# =============================================================================
# Load Configuration
# =============================================================================

if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "ERROR: Configuration file not found: $CONFIG_FILE"
    echo ""
    echo "Create a build-config.toml file with your project settings."
    echo "See the eulumdat-rs repository for an example."
    exit 1
fi

# Project settings
PROJECT_NAME=$(toml_get "project.name" "app")
PROJECT_DISPLAY=$(toml_get "project.display_name" "$PROJECT_NAME")

# Paths (relative to ROOT_DIR)
WASM_CRATE=$(toml_get "paths.wasm_crate" "crates/wasm")
BEVY_CRATE=$(toml_get "paths.bevy_crate" "crates/bevy")
BEVY_OUTPUT_REL=$(toml_get "paths.bevy_output" "target/wasm32-unknown-unknown/web-release")
TYPST_SOURCE_REL=$(toml_get "paths.typst_source" "")
DIST_OUTPUT_REL=$(toml_get "paths.dist_output" "$WASM_CRATE/dist")
ASSETS_REL=$(toml_get "paths.assets" "assets")
CORE_LIB_REL=$(toml_get "paths.core_lib" "")  # Optional core library path to watch for changes

# Absolute paths
WASM_DIR="$ROOT_DIR/$WASM_CRATE"
BEVY_DIR="$ROOT_DIR/$BEVY_CRATE"
BEVY_OUTPUT="$ROOT_DIR/$BEVY_OUTPUT_REL"
TYPST_SOURCE="$ROOT_DIR/$TYPST_SOURCE_REL"
DIST_DIR="$ROOT_DIR/$DIST_OUTPUT_REL"
ASSETS_DIR="$ROOT_DIR/$ASSETS_REL"
CORE_LIB_DIR=""
[[ -n "$CORE_LIB_REL" ]] && CORE_LIB_DIR="$ROOT_DIR/$CORE_LIB_REL"

# Bevy settings
# For WASM, we build the library (cdylib) not the binary
BEVY_LIBRARY=$(toml_get "bevy.library_name" "${PROJECT_NAME}_bevy")
BEVY_BINARY=$(toml_get "bevy.binary_name" "${PROJECT_NAME}-3d")
BEVY_FEATURES=$(toml_get_array "bevy.features" | tr '\n' ',' | sed 's/,$//')

# Bundle flags
BUILD_LEPTOS=$(toml_get "bundles.leptos" "true")
BUILD_BEVY=$(toml_get "bundles.bevy" "true")
BUILD_TYPST=$(toml_get "bundles.typst" "false")
BUILD_GMAPS=$(toml_get "bundles.gmaps" "false")

# Deploy settings
DEPLOY_TARGET=$(toml_get "deploy.target" "")
RSYNC_FLAGS=$(toml_get "deploy.rsync_flags" "-avz")
LOCAL_PORT=$(toml_get "deploy.local.port" "8042")
SERVER_CRATE=$(toml_get "deploy.local.server_crate" "")

# Pages
SECRET_PAGE=$(toml_get "pages.secret_export_page" "")
CUSTOM_404_SVG=$(toml_get "pages.custom_404_svg" "")

# Env vars
GMAPS_ENV_KEY=$(toml_get "env.google_maps_key" "GOOGLE_MAPS_API")

# =============================================================================
# Check for tools
# =============================================================================

HAVE_BROTLI=false
if command -v brotli &> /dev/null; then
    HAVE_BROTLI=true
fi

# =============================================================================
# Hash caching for incremental builds
# =============================================================================

CACHE_FILE="$ROOT_DIR/target/.wasm-build-cache"
FORCE_REBUILD=false

# Calculate hash of source files for a crate
# Usage: calculate_source_hash <crate_dir>
calculate_source_hash() {
    local crate_dir="$1"
    if [[ ! -d "$crate_dir" ]]; then
        echo "0"
        return
    fi
    # Hash all .rs files and Cargo.toml
    find "$crate_dir/src" -name "*.rs" -type f 2>/dev/null | sort | xargs cat 2>/dev/null | \
        cat - "$crate_dir/Cargo.toml" 2>/dev/null | \
        if command -v md5sum &> /dev/null; then md5sum | cut -c1-16; else md5 -q | cut -c1-16; fi
}

# Get cached hash for a component
# Usage: get_cached_hash <component_name>
get_cached_hash() {
    local component="$1"
    if [[ -f "$CACHE_FILE" ]]; then
        grep "^${component}=" "$CACHE_FILE" 2>/dev/null | cut -d'=' -f2
    fi
}

# Save hash to cache
# Usage: save_hash <component_name> <hash>
save_hash() {
    local component="$1"
    local hash="$2"
    mkdir -p "$(dirname "$CACHE_FILE")"
    # Remove old entry and add new one
    if [[ -f "$CACHE_FILE" ]]; then
        grep -v "^${component}=" "$CACHE_FILE" > "${CACHE_FILE}.tmp" 2>/dev/null || true
        mv "${CACHE_FILE}.tmp" "$CACHE_FILE"
    fi
    echo "${component}=${hash}" >> "$CACHE_FILE"
}

# Check if rebuild is needed
# Usage: needs_rebuild <component_name> <crate_dir>
# Returns 0 (true) if rebuild needed, 1 (false) if cached
needs_rebuild() {
    local component="$1"
    local crate_dir="$2"

    if [[ "$FORCE_REBUILD" == "true" ]]; then
        return 0
    fi

    local current_hash=$(calculate_source_hash "$crate_dir")
    local cached_hash=$(get_cached_hash "$component")

    if [[ "$current_hash" == "$cached_hash" ]] && [[ -n "$cached_hash" ]]; then
        return 1  # No rebuild needed
    fi
    return 0  # Rebuild needed
}

# =============================================================================
# Command line handling
# =============================================================================

ACTION="build"
if [[ "$1" == "deploy" ]]; then
    ACTION="deploy"
elif [[ "$1" == "serve" ]]; then
    ACTION="serve"
elif [[ "$1" == "force" ]]; then
    FORCE_REBUILD=true
    ACTION="build"
elif [[ "$1" == "clean" ]]; then
    echo "Cleaning build cache..."
    rm -f "$CACHE_FILE"
    rm -rf "$DIST_DIR"
    echo "Done."
    exit 0
elif [[ "$1" == "--help" ]] || [[ "$1" == "-h" ]]; then
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  (none)    Build WASM bundles (incremental - skips unchanged)"
    echo "  force     Force rebuild all bundles (ignore cache)"
    echo "  deploy    Build and deploy via rsync to configured target"
    echo "  serve     Build and start local development server"
    echo "  clean     Remove build cache and dist directory"
    echo "  --help    Show this help"
    echo ""
    echo "Configuration: $CONFIG_FILE"
    echo ""
    echo "Project: $PROJECT_DISPLAY"
    echo "Output:  $DIST_DIR"
    if [[ -n "$DEPLOY_TARGET" ]]; then
        echo "Deploy:  $DEPLOY_TARGET"
    fi
    exit 0
fi

# =============================================================================
# Build Process
# =============================================================================

echo "=== Building $PROJECT_DISPLAY Split WASM ==="
echo ""
if [[ "$BUILD_LEPTOS" == "true" ]]; then
    echo "  Bundle 1: Leptos editor (loads immediately)"
fi
if [[ "$BUILD_BEVY" == "true" ]]; then
    echo "  Bundle 2: Bevy 3D viewer (loads on demand)"
fi
if [[ "$BUILD_TYPST" == "true" ]]; then
    echo "  Bundle 3: Typst PDF compiler (loads on demand)"
fi
if [[ "$HAVE_BROTLI" == "true" ]]; then
    echo ""
    echo "  Brotli pre-compression: enabled"
fi
echo ""

STEP=1
TOTAL_STEPS=8

# Track what was built (for later steps)
BEVY_BUILT=false
LEPTOS_BUILT=false

# -----------------------------------------------------------------------------
# Step 1: Build Bevy 3D viewer
# -----------------------------------------------------------------------------
if [[ "$BUILD_BEVY" == "true" ]]; then
    # Also check core lib if configured (e.g., eulumdat for eulumdat-rs, gldf-rs-lib for gldf-rs)
    BEVY_NEEDS_BUILD=false
    if needs_rebuild "bevy" "$BEVY_DIR"; then
        BEVY_NEEDS_BUILD=true
    elif [[ -n "$CORE_LIB_DIR" ]] && needs_rebuild "core-lib" "$CORE_LIB_DIR"; then
        BEVY_NEEDS_BUILD=true
    elif [[ ! -f "$BEVY_OUTPUT/${BEVY_LIBRARY}.js" ]]; then
        BEVY_NEEDS_BUILD=true
    fi

    if [[ "$BEVY_NEEDS_BUILD" == "true" ]]; then
        echo "[$STEP/$TOTAL_STEPS] Building Bevy 3D viewer..."
        cd "$BEVY_DIR"

        FEATURE_FLAG=""
        if [[ -n "$BEVY_FEATURES" ]]; then
            FEATURE_FLAG="--features $BEVY_FEATURES"
        fi

        # For WASM, we build the library (cdylib) not the binary
        # The library exports wasm_start() via wasm-bindgen
        echo "  Building library: $BEVY_LIBRARY"
        cargo build --lib --release $FEATURE_FLAG --target wasm32-unknown-unknown
        mkdir -p "$BEVY_OUTPUT"
        wasm-bindgen --out-dir "$BEVY_OUTPUT" --target web \
            "$ROOT_DIR/target/wasm32-unknown-unknown/release/${BEVY_LIBRARY}.wasm"

        if command -v wasm-opt &> /dev/null && [[ -f "$BEVY_OUTPUT/${BEVY_LIBRARY}_bg.wasm" ]]; then
            echo "  Running wasm-opt..."
            wasm-opt -Oz -o "$BEVY_OUTPUT/${BEVY_LIBRARY}_bg_opt.wasm" "$BEVY_OUTPUT/${BEVY_LIBRARY}_bg.wasm"
            mv "$BEVY_OUTPUT/${BEVY_LIBRARY}_bg_opt.wasm" "$BEVY_OUTPUT/${BEVY_LIBRARY}_bg.wasm"
        fi

        # Save hashes after successful build
        save_hash "bevy" "$(calculate_source_hash "$BEVY_DIR")"
        [[ -n "$CORE_LIB_DIR" ]] && save_hash "core-lib" "$(calculate_source_hash "$CORE_LIB_DIR")"
        BEVY_BUILT=true
        echo ""
    else
        echo "[$STEP/$TOTAL_STEPS] Bevy 3D viewer: unchanged, skipping build"
    fi
fi
((STEP++))

# -----------------------------------------------------------------------------
# Step 2: Build Leptos editor
# -----------------------------------------------------------------------------
if [[ "$BUILD_LEPTOS" == "true" ]]; then
    LEPTOS_NEEDS_BUILD=false
    if needs_rebuild "leptos" "$WASM_DIR"; then
        LEPTOS_NEEDS_BUILD=true
    elif [[ -n "$CORE_LIB_DIR" ]] && needs_rebuild "core-lib" "$CORE_LIB_DIR"; then
        LEPTOS_NEEDS_BUILD=true
    elif [[ ! -d "$DIST_DIR" ]] || [[ -z "$(ls -A "$DIST_DIR"/*.wasm 2>/dev/null)" ]]; then
        LEPTOS_NEEDS_BUILD=true
    fi

    if [[ "$LEPTOS_NEEDS_BUILD" == "true" ]]; then
        echo "[$STEP/$TOTAL_STEPS] Building Leptos editor with trunk..."
        cd "$WASM_DIR"
        trunk build --release

        # Save hash after successful build
        save_hash "leptos" "$(calculate_source_hash "$WASM_DIR")"
        LEPTOS_BUILT=true
        echo ""
    else
        echo "[$STEP/$TOTAL_STEPS] Leptos editor: unchanged, skipping build"
    fi
fi
((STEP++))

# -----------------------------------------------------------------------------
# Step 3: Add content hashes to Bevy files
# -----------------------------------------------------------------------------
if [[ "$BUILD_BEVY" == "true" ]]; then
    # Only process if Bevy was rebuilt or dist/bevy is missing
    if [[ "$BEVY_BUILT" == "true" ]] || [[ -z "$(ls -A "$DIST_DIR/bevy/"*.wasm 2>/dev/null)" ]]; then
        echo "[$STEP/$TOTAL_STEPS] Adding content hashes to Bevy files..."
        mkdir -p "$DIST_DIR/bevy"

        rm -f "$DIST_DIR/bevy/"*.js "$DIST_DIR/bevy/"*.wasm "$DIST_DIR/bevy/"*.br

        # Use md5sum on Linux, md5 on macOS
        if command -v md5sum &> /dev/null; then
            JS_HASH=$(md5sum "$BEVY_OUTPUT/${BEVY_LIBRARY}.js" | cut -c1-16)
            WASM_HASH=$(md5sum "$BEVY_OUTPUT/${BEVY_LIBRARY}_bg.wasm" | cut -c1-16)
        else
            JS_HASH=$(md5 -q "$BEVY_OUTPUT/${BEVY_LIBRARY}.js" | cut -c1-16)
            WASM_HASH=$(md5 -q "$BEVY_OUTPUT/${BEVY_LIBRARY}_bg.wasm" | cut -c1-16)
        fi

        cp "$BEVY_OUTPUT/${BEVY_LIBRARY}.js" "$DIST_DIR/bevy/${BEVY_LIBRARY}-${JS_HASH}.js"
        cp "$BEVY_OUTPUT/${BEVY_LIBRARY}_bg.wasm" "$DIST_DIR/bevy/${BEVY_LIBRARY}-${WASM_HASH}_bg.wasm"

        # Update JS to reference hashed WASM
        if [[ "$(uname)" == "Darwin" ]]; then
            sed -i '' "s/${BEVY_LIBRARY}_bg.wasm/${BEVY_LIBRARY}-${WASM_HASH}_bg.wasm/g" "$DIST_DIR/bevy/${BEVY_LIBRARY}-${JS_HASH}.js"
        else
            sed -i "s/${BEVY_LIBRARY}_bg.wasm/${BEVY_LIBRARY}-${WASM_HASH}_bg.wasm/g" "$DIST_DIR/bevy/${BEVY_LIBRARY}-${JS_HASH}.js"
        fi

        # Generate manifest.json for the inlined bevy-loader.js
        cat > "$DIST_DIR/bevy/manifest.json" << MANIFESTEOF
{"js":"${BEVY_LIBRARY}-${JS_HASH}.js","wasm":"${BEVY_LIBRARY}-${WASM_HASH}_bg.wasm"}
MANIFESTEOF
        echo "  Generated bevy/manifest.json"
        echo ""
    else
        echo "[$STEP/$TOTAL_STEPS] Bevy files: unchanged, using cached hashes"
        # Extract existing hashes from dist/bevy filenames
        JS_HASH=$(ls "$DIST_DIR/bevy/${BEVY_LIBRARY}"-*.js 2>/dev/null | head -1 | sed "s/.*${BEVY_LIBRARY}-\([^.]*\)\.js/\1/")
        WASM_HASH=$(ls "$DIST_DIR/bevy/${BEVY_LIBRARY}"-*_bg.wasm 2>/dev/null | head -1 | sed "s/.*${BEVY_LIBRARY}-\([^_]*\)_bg\.wasm/\1/")

        # Ensure manifest.json exists even for cached builds
        if [[ ! -f "$DIST_DIR/bevy/manifest.json" ]]; then
            cat > "$DIST_DIR/bevy/manifest.json" << MANIFESTEOF
{"js":"${BEVY_LIBRARY}-${JS_HASH}.js","wasm":"${BEVY_LIBRARY}-${WASM_HASH}_bg.wasm"}
MANIFESTEOF
            echo "  Generated bevy/manifest.json (was missing)"
        fi
    fi
fi
((STEP++))

# -----------------------------------------------------------------------------
# Step 4: Optimize and hash Typst files
# -----------------------------------------------------------------------------
if [[ "$BUILD_TYPST" == "true" ]] && [[ -d "$TYPST_SOURCE" ]]; then
    echo "[$STEP/$TOTAL_STEPS] Optimizing Typst WASM with wasm-opt..."
    mkdir -p "$DIST_DIR/typst"

    rm -f "$DIST_DIR/typst/"*.js "$DIST_DIR/typst/"*.wasm "$DIST_DIR/typst/"*.br

    TYPST_WASM_OPTIMIZED="$DIST_DIR/typst/typst_wasm_optimized.wasm"
    if command -v wasm-opt &> /dev/null; then
        echo "  Running wasm-opt -Oz..."
        ORIG_SIZE=$(ls -lh "$TYPST_SOURCE/typst_wasm_bg.wasm" | awk '{print $5}')
        wasm-opt -Oz -o "$TYPST_WASM_OPTIMIZED" "$TYPST_SOURCE/typst_wasm_bg.wasm"
        OPT_SIZE=$(ls -lh "$TYPST_WASM_OPTIMIZED" | awk '{print $5}')
        echo "  Size: $ORIG_SIZE -> $OPT_SIZE"
    else
        echo "  wasm-opt not found, copying unoptimized..."
        cp "$TYPST_SOURCE/typst_wasm_bg.wasm" "$TYPST_WASM_OPTIMIZED"
    fi

    if command -v md5sum &> /dev/null; then
        TYPST_JS_HASH=$(md5sum "$TYPST_SOURCE/typst_wasm.js" | cut -c1-16)
        TYPST_WASM_HASH=$(md5sum "$TYPST_WASM_OPTIMIZED" | cut -c1-16)
    else
        TYPST_JS_HASH=$(md5 -q "$TYPST_SOURCE/typst_wasm.js" | cut -c1-16)
        TYPST_WASM_HASH=$(md5 -q "$TYPST_WASM_OPTIMIZED" | cut -c1-16)
    fi

    cp "$TYPST_SOURCE/typst_wasm.js" "$DIST_DIR/typst/typst_wasm-${TYPST_JS_HASH}.js"
    mv "$TYPST_WASM_OPTIMIZED" "$DIST_DIR/typst/typst_wasm-${TYPST_WASM_HASH}_bg.wasm"

    if [[ "$(uname)" == "Darwin" ]]; then
        sed -i '' "s/typst_wasm_bg.wasm/typst_wasm-${TYPST_WASM_HASH}_bg.wasm/g" "$DIST_DIR/typst/typst_wasm-${TYPST_JS_HASH}.js"
    else
        sed -i "s/typst_wasm_bg.wasm/typst_wasm-${TYPST_WASM_HASH}_bg.wasm/g" "$DIST_DIR/typst/typst_wasm-${TYPST_JS_HASH}.js"
    fi
    echo ""
else
    echo "[$STEP/$TOTAL_STEPS] Skipping Typst (not configured or source not found)..."
    echo ""
fi
((STEP++))

# -----------------------------------------------------------------------------
# Step 5: Generate bevy-loader.js
# -----------------------------------------------------------------------------
if [[ "$BUILD_BEVY" == "true" ]]; then
    # Check if loader already exists with correct hash reference
    EXISTING_LOADER=$(ls "$DIST_DIR/bevy-loader-"*.js 2>/dev/null | head -1)
    if [[ -n "$EXISTING_LOADER" ]] && grep -q "${BEVY_LIBRARY}-${JS_HASH}.js" "$EXISTING_LOADER" 2>/dev/null; then
        echo "[$STEP/$TOTAL_STEPS] bevy-loader.js: unchanged, skipping"
        BEVY_LOADER_HASH=$(echo "$EXISTING_LOADER" | sed 's/.*bevy-loader-\([^.]*\)\.js/\1/')
    else
        echo "[$STEP/$TOTAL_STEPS] Generating bevy-loader.js..."
        # Remove old loaders
        rm -f "$DIST_DIR/bevy-loader-"*.js "$DIST_DIR/bevy-loader-"*.js.br

        cat > "$DIST_DIR/bevy-loader-temp.js" << 'JSEOF'
// Lazy loader for Bevy 3D Scene Viewer
// Auto-generated with content hashes for cache busting

let bevyLoaded = false;
let bevyLoading = false;
let bevyLoadPromise = null;

// Storage keys for L3D/LDT data (must match Rust constants)
const L3D_STORAGE_KEY = 'gldf_current_l3d';
const LDT_STORAGE_KEY = 'gldf_current_ldt';
const EMITTER_CONFIG_KEY = 'gldf_emitter_config';
const MOUNTING_CONFIG_KEY = 'gldf_mounting_config';
const GLDF_TIMESTAMP_KEY = 'gldf_timestamp';

// Storage keys for IFC geometry (must match Rust constants in ifc_loader.rs)
const IFC_GEOMETRY_KEY = 'ifc_geometry';
const IFC_TIMESTAMP_KEY = 'ifc_timestamp';

/**
 * Save L3D data to localStorage for Bevy viewer
 * @param {Uint8Array} l3dData - L3D file bytes
 * @param {string|null} ldtData - LDT file content (optional)
 * @param {string|null} emitterConfig - JSON string of emitter configurations (optional)
 * @param {string|null} mountingConfig - JSON string of mounting configuration (optional)
 */
function saveL3dForBevy(l3dData, ldtData, emitterConfig, mountingConfig) {
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
            console.log('[Bevy] Emitter config stored');
        } else {
            localStorage.removeItem(EMITTER_CONFIG_KEY);
        }

        // Store mounting config for luminaire positioning
        if (mountingConfig) {
            localStorage.setItem(MOUNTING_CONFIG_KEY, mountingConfig);
            console.log('[Bevy] Mounting config stored');
        } else {
            localStorage.removeItem(MOUNTING_CONFIG_KEY);
        }

        // Update timestamp to trigger Bevy reload
        const ts = Date.now().toString();
        localStorage.setItem(GLDF_TIMESTAMP_KEY, ts);
        console.log('[Bevy] All data saved to localStorage, timestamp:', ts);
    } catch (e) {
        console.error('[Bevy] Failed to save L3D data:', e);
    }
}

/**
 * Save IFC geometry data to localStorage for Bevy viewer
 * @param {string} geometryJson - JSON string containing {vertices: [(x,y,z)...], triangles: [(i,j,k)...]}
 * @param {string|null} variantName - Name of the variant being displayed (optional)
 */
function saveIfcGeometryForBevy(geometryJson, variantName) {
    console.log('[Bevy IFC] saveIfcGeometryForBevy called');
    try {
        localStorage.setItem(IFC_GEOMETRY_KEY, geometryJson);
        console.log('[Bevy IFC] Geometry stored, length:', geometryJson.length);

        // Update timestamp to trigger Bevy reload
        const ts = Date.now().toString();
        localStorage.setItem(IFC_TIMESTAMP_KEY, ts);
        console.log('[Bevy IFC] Timestamp updated:', ts);

        if (variantName) {
            console.log('[Bevy IFC] Variant:', variantName);
        }
    } catch (e) {
        console.error('[Bevy IFC] Failed to save geometry:', e);
    }
}

/**
 * Clear IFC geometry from localStorage
 */
function clearIfcGeometry() {
    localStorage.removeItem(IFC_GEOMETRY_KEY);
    localStorage.removeItem(IFC_TIMESTAMP_KEY);
    console.log('[Bevy IFC] Geometry cleared');
}

JSEOF
        # Now append the dynamic part with variable substitution
        cat >> "$DIST_DIR/bevy-loader-temp.js" << EOF
async function loadBevyViewer() {
    if (bevyLoaded) {
        console.log("[Bevy] Already loaded");
        return;
    }
    if (bevyLoading && bevyLoadPromise) {
        console.log("[Bevy] Loading in progress, waiting...");
        return bevyLoadPromise;
    }

    bevyLoading = true;
    console.log("[Bevy] Loading 3D viewer...");

    bevyLoadPromise = (async () => {
        try {
            const bevy = await import('./bevy/${BEVY_LIBRARY}-${JS_HASH}.js');
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
            bevyLoadPromise = null;
            throw error;
        }
    })();

    return bevyLoadPromise;
}

function isBevyLoaded() { return bevyLoaded; }
function isBevyLoading() { return bevyLoading; }

window.loadBevyViewer = loadBevyViewer;
window.isBevyLoaded = isBevyLoaded;
window.isBevyLoading = isBevyLoading;
window.saveL3dForBevy = saveL3dForBevy;
window.saveIfcGeometryForBevy = saveIfcGeometryForBevy;
window.clearIfcGeometry = clearIfcGeometry;

console.log("[Bevy] Loader ready (JS: ${JS_HASH}, WASM: ${WASM_HASH})");
EOF

        if command -v md5sum &> /dev/null; then
            BEVY_LOADER_HASH=$(md5sum "$DIST_DIR/bevy-loader-temp.js" | cut -c1-16)
        else
            BEVY_LOADER_HASH=$(md5 -q "$DIST_DIR/bevy-loader-temp.js" | cut -c1-16)
        fi
        mv "$DIST_DIR/bevy-loader-temp.js" "$DIST_DIR/bevy-loader-${BEVY_LOADER_HASH}.js"
        echo "  bevy-loader-${BEVY_LOADER_HASH}.js"
    fi
fi
((STEP++))

# -----------------------------------------------------------------------------
# Step 6: Generate typst-loader.js
# -----------------------------------------------------------------------------
if [[ "$BUILD_TYPST" == "true" ]] && [[ -d "$TYPST_SOURCE" ]]; then
    echo "[$STEP/$TOTAL_STEPS] Generating typst-loader.js..."

    cat > "$DIST_DIR/typst-loader-temp.js" << EOF
// Typst WASM loader for PDF compilation
// Auto-generated with content hashes for cache busting

let typstModule = null;
let typstInitPromise = null;

async function initTypst() {
    if (typstModule) return typstModule;
    if (typstInitPromise) return typstInitPromise;

    typstInitPromise = (async () => {
        try {
            console.log('[Typst] Loading PDF compiler...');
            const module = await import('./typst/typst_wasm-${TYPST_JS_HASH}.js');
            await module.default();
            typstModule = module;
            console.log('[Typst] PDF compiler loaded successfully');
            return module;
        } catch (e) {
            console.error('[Typst] Failed to load:', e);
            typstInitPromise = null;
            throw e;
        }
    })();

    return typstInitPromise;
}

window.compileTypstToPdf = async function(typstSource) {
    const module = await initTypst();
    try {
        const pdfBytes = module.compile_to_pdf(typstSource);
        return pdfBytes;
    } catch (e) {
        console.error('[Typst] Compilation error:', e);
        throw new Error('Typst compilation failed: ' + e);
    }
};

window.isTypstLoaded = function() { return typstModule !== null; };
window.preloadTypst = async function() { await initTypst(); };

console.log("[Typst] Loader ready (JS: ${TYPST_JS_HASH}, WASM: ${TYPST_WASM_HASH})");
EOF

    if command -v md5sum &> /dev/null; then
        TYPST_LOADER_HASH=$(md5sum "$DIST_DIR/typst-loader-temp.js" | cut -c1-16)
    else
        TYPST_LOADER_HASH=$(md5 -q "$DIST_DIR/typst-loader-temp.js" | cut -c1-16)
    fi
    mv "$DIST_DIR/typst-loader-temp.js" "$DIST_DIR/typst-loader-${TYPST_LOADER_HASH}.js"
    echo "  typst-loader-${TYPST_LOADER_HASH}.js"
else
    echo "[$STEP/$TOTAL_STEPS] Skipping typst-loader.js (not configured)..."
fi
((STEP++))

# -----------------------------------------------------------------------------
# Step 6.5: Generate gmaps-loader.js (if configured)
# -----------------------------------------------------------------------------
if [[ "$BUILD_GMAPS" == "true" ]] && [[ -f "$WASM_DIR/src/static/gmaps-loader.js" ]]; then
    echo "[6.5/$TOTAL_STEPS] Generating gmaps-loader.js with API key..."

    GMAPS_API_KEY=""
    if [[ -f "$ROOT_DIR/.env" ]]; then
        GMAPS_API_KEY=$(grep "^${GMAPS_ENV_KEY}=" "$ROOT_DIR/.env" | cut -d'=' -f2)
    fi

    if [[ -n "$GMAPS_API_KEY" ]]; then
        sed "s/__GMAPS_API_KEY__/${GMAPS_API_KEY}/g" "$WASM_DIR/src/static/gmaps-loader.js" > "$DIST_DIR/gmaps-loader-temp.js"
        echo "  API key injected from .env"
    else
        cp "$WASM_DIR/src/static/gmaps-loader.js" "$DIST_DIR/gmaps-loader-temp.js"
        echo "  WARNING: No $GMAPS_ENV_KEY key found in .env"
    fi

    if command -v md5sum &> /dev/null; then
        GMAPS_LOADER_HASH=$(md5sum "$DIST_DIR/gmaps-loader-temp.js" | cut -c1-16)
    else
        GMAPS_LOADER_HASH=$(md5 -q "$DIST_DIR/gmaps-loader-temp.js" | cut -c1-16)
    fi
    mv "$DIST_DIR/gmaps-loader-temp.js" "$DIST_DIR/gmaps-loader-${GMAPS_LOADER_HASH}.js"
    echo "  gmaps-loader-${GMAPS_LOADER_HASH}.js"
    echo ""
fi

# -----------------------------------------------------------------------------
# Step 6.6: Update index.html with hashed loader filenames
# -----------------------------------------------------------------------------
echo "[6.6/$TOTAL_STEPS] Updating index.html with hashed loader filenames..."

SED_INPLACE=(-i '')
[[ "$(uname)" != "Darwin" ]] && SED_INPLACE=(-i)

if [[ "$BUILD_BEVY" == "true" ]] && [[ -n "$BEVY_LOADER_HASH" ]]; then
    sed "${SED_INPLACE[@]}" "s|bevy-loader.js\"|bevy-loader-${BEVY_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
    sed "${SED_INPLACE[@]}" "s|bevy-loader.js?v=[0-9]*\"|bevy-loader-${BEVY_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
fi

if [[ "$BUILD_TYPST" == "true" ]] && [[ -n "$TYPST_LOADER_HASH" ]]; then
    sed "${SED_INPLACE[@]}" "s|typst-loader.js\"|typst-loader-${TYPST_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
    sed "${SED_INPLACE[@]}" "s|typst-loader.js?v=[0-9]*\"|typst-loader-${TYPST_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
fi

if [[ "$BUILD_GMAPS" == "true" ]] && [[ -n "$GMAPS_LOADER_HASH" ]]; then
    sed "${SED_INPLACE[@]}" "s|gmaps-loader.js\"|gmaps-loader-${GMAPS_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
    sed "${SED_INPLACE[@]}" "s|gmaps-loader.js?v=[0-9]*\"|gmaps-loader-${GMAPS_LOADER_HASH}.js\"|g" "$DIST_DIR/index.html"
fi

echo "  Updated loader references in index.html"
echo ""

# -----------------------------------------------------------------------------
# Step 7: Pre-compress with Brotli
# -----------------------------------------------------------------------------
echo "[$((STEP++))]/$TOTAL_STEPS] Pre-compressing with Brotli..."

if [[ "$HAVE_BROTLI" == "true" ]]; then
    if command -v nproc &> /dev/null; then
        NCPU=$(nproc)
    elif command -v sysctl &> /dev/null; then
        NCPU=$(sysctl -n hw.ncpu 2>/dev/null || echo 4)
    else
        NCPU=4
    fi
    echo "  Using $NCPU parallel jobs..."

    FILES_TO_COMPRESS=()

    for f in "$DIST_DIR/"*.wasm "$DIST_DIR/bevy/"*.wasm "$DIST_DIR/typst/"*.wasm; do
        [[ -f "$f" ]] && FILES_TO_COMPRESS+=("$f")
    done

    for f in "$DIST_DIR/"*.js "$DIST_DIR/bevy/"*.js "$DIST_DIR/typst/"*.js; do
        [[ -f "$f" ]] && FILES_TO_COMPRESS+=("$f")
    done

    for f in "$DIST_DIR/"*.css; do
        [[ -f "$f" ]] && FILES_TO_COMPRESS+=("$f")
    done

    echo "  Compressing ${#FILES_TO_COMPRESS[@]} files in parallel..."
    printf '%s\n' "${FILES_TO_COMPRESS[@]}" | xargs -P "$NCPU" -I {} brotli -f -q 11 {}

    echo "  Done!"
else
    echo "  brotli not found, skipping pre-compression."
    echo "  Install with: brew install brotli"
fi
echo ""

# -----------------------------------------------------------------------------
# Step 8: Create additional pages
# -----------------------------------------------------------------------------
echo "[$TOTAL_STEPS/$TOTAL_STEPS] Creating additional pages for static deployment..."

if [[ -n "$SECRET_PAGE" ]]; then
    cp "$DIST_DIR/index.html" "$DIST_DIR/$SECRET_PAGE"
    echo "  Created $SECRET_PAGE (enables PDF/Typst export)"
fi

if [[ -n "$CUSTOM_404_SVG" ]] && [[ -f "$ASSETS_DIR/$CUSTOM_404_SVG" ]]; then
    cat > "$DIST_DIR/404.html" << 'HTMLEOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>404 - Page Not Found</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            background: #070810;
            font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
            color: #c7f8ff;
            padding: 2rem;
        }
        .container { max-width: 1400px; width: 100%; text-align: center; }
        .svg-container { width: 100%; max-width: 1000px; margin: 0 auto 2rem; }
        .svg-container svg { width: 100%; height: auto; }
        .message { opacity: 0.8; margin-bottom: 2rem; }
        .home-link {
            display: inline-block;
            padding: 0.75rem 2rem;
            background: linear-gradient(135deg, #22d8ff 0%, #9ff7ff 100%);
            color: #070810;
            text-decoration: none;
            border-radius: 8px;
            font-weight: 600;
            transition: transform 0.2s, box-shadow 0.2s;
        }
        .home-link:hover {
            transform: translateY(-2px);
            box-shadow: 0 4px 20px rgba(34, 216, 255, 0.4);
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="svg-container">
HTMLEOF
    cat "$ASSETS_DIR/$CUSTOM_404_SVG" >> "$DIST_DIR/404.html"
    cat >> "$DIST_DIR/404.html" << 'HTMLEOF'
        </div>
        <p class="message">The page you're looking for seems to be in the dark.</p>
        <a href="/" class="home-link">← Back to Editor</a>
    </div>
</body>
</html>
HTMLEOF
    echo "  Created 404.html with custom SVG"
fi
echo ""

# =============================================================================
# Summary
# =============================================================================

echo "=== Build Complete ==="
echo ""

# Show what was rebuilt vs cached
echo "Build status:"
if [[ "$BEVY_BUILT" == "true" ]]; then
    echo "  Bevy 3D viewer:     REBUILT"
elif [[ "$BUILD_BEVY" == "true" ]]; then
    echo "  Bevy 3D viewer:     cached (unchanged)"
fi
if [[ "$LEPTOS_BUILT" == "true" ]]; then
    echo "  Leptos editor:      REBUILT"
elif [[ "$BUILD_LEPTOS" == "true" ]]; then
    echo "  Leptos editor:      cached (unchanged)"
fi
echo ""

LEPTOS_WASM=$(ls "$DIST_DIR/"*_bg.wasm 2>/dev/null | head -1)
BEVY_WASM_FILE=$(ls "$DIST_DIR/bevy/"*_bg.wasm 2>/dev/null | head -1)
TYPST_WASM_FILE=$(ls "$DIST_DIR/typst/"*_bg.wasm 2>/dev/null | head -1)

LEPTOS_SIZE=$(ls -lh "$LEPTOS_WASM" 2>/dev/null | awk '{print $5}')
BEVY_SIZE=$(ls -lh "$BEVY_WASM_FILE" 2>/dev/null | awk '{print $5}')
TYPST_SIZE=$(ls -lh "$TYPST_WASM_FILE" 2>/dev/null | awk '{print $5}')

echo "Bundle sizes (raw / compressed):"
if [[ "$HAVE_BROTLI" == "true" ]]; then
    LEPTOS_BR=$(ls -lh "${LEPTOS_WASM}.br" 2>/dev/null | awk '{print $5}')
    BEVY_BR=$(ls -lh "${BEVY_WASM_FILE}.br" 2>/dev/null | awk '{print $5}')
    TYPST_BR=$(ls -lh "${TYPST_WASM_FILE}.br" 2>/dev/null | awk '{print $5}')

    [[ -n "$LEPTOS_SIZE" ]] && echo "  Leptos editor:      $LEPTOS_SIZE -> $LEPTOS_BR (loads immediately)"
    [[ -n "$BEVY_SIZE" ]] && echo "  Bevy 3D viewer:     $BEVY_SIZE -> $BEVY_BR (loads on demand)"
    [[ -n "$TYPST_SIZE" ]] && echo "  Typst PDF compiler: $TYPST_SIZE -> $TYPST_BR (loads on demand)"
else
    [[ -n "$LEPTOS_SIZE" ]] && echo "  Leptos editor:      $LEPTOS_SIZE (loads immediately)"
    [[ -n "$BEVY_SIZE" ]] && echo "  Bevy 3D viewer:     $BEVY_SIZE (loads on demand)"
    [[ -n "$TYPST_SIZE" ]] && echo "  Typst PDF compiler: $TYPST_SIZE (loads on demand)"
fi
echo ""

if [[ "$BUILD_BEVY" == "true" ]]; then
    echo "Hashed filenames:"
    echo "  Bevy:  ${BEVY_LIBRARY}-${JS_HASH}.js / ${BEVY_LIBRARY}-${WASM_HASH}_bg.wasm"
fi
if [[ "$BUILD_TYPST" == "true" ]] && [[ -n "$TYPST_JS_HASH" ]]; then
    echo "  Typst: typst_wasm-${TYPST_JS_HASH}.js / typst_wasm-${TYPST_WASM_HASH}_bg.wasm"
fi
echo ""
echo "Output: $DIST_DIR"
echo ""

# =============================================================================
# Deploy / Serve
# =============================================================================

if [[ "$ACTION" == "deploy" ]]; then
    echo ""
    if [[ -z "$DEPLOY_TARGET" ]]; then
        echo "ERROR: No deploy target configured in build-config.toml"
        echo "Add [deploy] section with target = \"user@host:/path/\""
        exit 1
    fi

    echo "=== Deploying to $DEPLOY_TARGET ==="
    echo "Running: rsync $RSYNC_FLAGS $DIST_DIR/ $DEPLOY_TARGET"
    echo ""
    rsync $RSYNC_FLAGS "$DIST_DIR/" "$DEPLOY_TARGET"
    echo ""
    echo "Deploy complete!"

elif [[ "$ACTION" == "serve" ]]; then
    echo ""
    echo "=== Starting local server on port $LOCAL_PORT ==="
    echo ""

    if [[ -n "$SERVER_CRATE" ]]; then
        echo "Running: cargo run -p $SERVER_CRATE -- -p $LOCAL_PORT --dist $DIST_DIR"
        echo "Open: http://localhost:$LOCAL_PORT"
        echo ""
        cargo run -p "$SERVER_CRATE" -- -p "$LOCAL_PORT" --dist "$DIST_DIR"
    else
        echo "Running: python3 -m http.server $LOCAL_PORT -d $DIST_DIR"
        echo "Open: http://localhost:$LOCAL_PORT"
        echo ""
        python3 -m http.server "$LOCAL_PORT" -d "$DIST_DIR"
    fi

else
    # Just show instructions
    if [[ -n "$DEPLOY_TARGET" ]]; then
        echo "To deploy to $DEPLOY_TARGET:"
        echo "  $0 deploy"
        echo ""
    fi
    echo "To serve locally:"
    if [[ -n "$SERVER_CRATE" ]]; then
        echo "  $0 serve"
        echo "  # or: cargo run -p $SERVER_CRATE -- -p $LOCAL_PORT --dist $DIST_DIR"
    else
        echo "  $0 serve"
        echo "  # or: python3 -m http.server $LOCAL_PORT -d $DIST_DIR"
    fi
    echo "  open http://localhost:$LOCAL_PORT"
fi
