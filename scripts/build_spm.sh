#!/usr/bin/env bash
#
# build_spm.sh - Build Swift Package Manager package for all Apple platforms
#
# Usage: ./build_spm.sh [dev|release]
#
# Arguments:
#   dev     - Development build (faster, with debug symbols, larger binaries)
#   release - Release build (optimized, stripped, smaller binaries)
#
# Builds for: iOS, macOS, tvOS, watchOS, visionOS
# Output: gldf-rs-Apps/SPM gldf-rsKit/
#

set -euo pipefail

# Parse arguments
BUILD_MODE="${1:-release}"
if [[ "$BUILD_MODE" != "dev" && "$BUILD_MODE" != "release" ]]; then
    echo "❌ Error: Build mode must be 'dev' or 'release'"
    echo "Usage: $0 [dev|release]"
    exit 1
fi

# Convert to Cargo profile
if [[ "$BUILD_MODE" == "dev" ]]; then
    CARGO_PROFILE="debug"
    CARGO_FLAG=""
else
    CARGO_PROFILE="release"
    CARGO_FLAG="--release"
fi

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
FFI_CRATE="$ROOT_DIR/crates/gldf-rs-ffi"
SPM_PACKAGE="$ROOT_DIR/gldf-rs-Apps/SPM-gldf-rsKit"
TARGET_DIR="$ROOT_DIR/target"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

step() {
    echo -e "\n${BLUE}==>${NC} ${GREEN}$1${NC}"
}

echo "======================================================================"
echo "Building SPM Package (mode: $BUILD_MODE)"
echo "======================================================================"

# Check if SPM package already exists and is up-to-date
SPM_XCFRAMEWORK="$SPM_PACKAGE/gldf-rsFfi.xcframework"
SPM_SOURCES="$SPM_PACKAGE/Sources/gldf-rsKit/gldf-rs_ffi.swift"

if [ -d "$SPM_XCFRAMEWORK" ] && [ -f "$SPM_SOURCES" ] && [ -f "$SPM_PACKAGE/Package.swift" ]; then
    # Check if FFI source is newer than XCFramework
    if [ "$FFI_CRATE/src" -nt "$SPM_XCFRAMEWORK" ]; then
        echo -e "${YELLOW}FFI source updated since last build, rebuilding...${NC}"
    else
        echo -e "${GREEN}✓ SPM package is up-to-date, skipping rebuild${NC}"
        echo "  XCFramework: $SPM_XCFRAMEWORK"
        echo "  Sources: $SPM_SOURCES"
        echo ""
        echo "To force rebuild: rm -rf \"$SPM_XCFRAMEWORK\""
        exit 0
    fi
fi

# Step 1: Build Rust for macOS first (for binding generation)
step "Building Rust for macOS (binding generation)..."
cd "$FFI_CRATE"
cargo build $CARGO_FLAG --target aarch64-apple-darwin --lib

# Step 2: Generate Swift bindings
step "Generating Swift bindings..."
if [ ! -d "generated" ]; then
    mkdir -p generated
fi

cargo run --bin uniffi-bindgen generate \
    --library "$TARGET_DIR/aarch64-apple-darwin/$CARGO_PROFILE/libgldf-rs_ffi.dylib" \
    --language swift \
    --out-dir generated

if [ ! -f "generated/gldf-rs_ffi.swift" ]; then
    echo -e "${YELLOW}Error: Swift bindings generation failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Swift bindings generated${NC}"

# Step 3: Build for all Apple platforms
step "Building Rust for all Apple platforms..."

echo "  • macOS x86_64 (Intel)..."
cargo build $CARGO_FLAG --target x86_64-apple-darwin --lib

echo "  • iOS device (arm64)..."
cargo build $CARGO_FLAG --target aarch64-apple-ios --lib

echo "  • iOS simulator (arm64)..."
cargo build $CARGO_FLAG --target aarch64-apple-ios-sim --lib

echo "  • tvOS device (arm64)..."
cargo +nightly build $CARGO_FLAG --target aarch64-apple-tvos -Z build-std --lib

echo "  • tvOS simulator (arm64)..."
cargo +nightly build $CARGO_FLAG --target aarch64-apple-tvos-sim -Z build-std --lib

echo "  • watchOS device (arm64)..."
cargo +nightly build $CARGO_FLAG --target aarch64-apple-watchos -Z build-std --lib

echo "  • watchOS device (arm64_32)..."
cargo +nightly build $CARGO_FLAG --target arm64_32-apple-watchos -Z build-std --lib

echo "  • watchOS simulator (arm64)..."
cargo +nightly build $CARGO_FLAG --target aarch64-apple-watchos-sim -Z build-std --lib

echo -e "${GREEN}✓ All platforms built${NC}"

# Step 4: Create XCFramework using comprehensive build script
step "Creating XCFramework..."

# Use the comprehensive SPM build script from gldf-rs-Apps/scripts
SPM_BUILD_SCRIPT="$ROOT_DIR/gldf-rs-Apps/scripts/build_spm_package.sh"

if [ -f "$SPM_BUILD_SCRIPT" ]; then
    echo "  • Using comprehensive SPM build script..."
    echo "  • This will create frameworks, XCFramework, and Package.swift"
    cd "$ROOT_DIR"

    # The comprehensive script handles everything from this point
    # It will use the Rust libraries we just built in target/
    bash "$SPM_BUILD_SCRIPT" || {
        echo -e "${YELLOW}Comprehensive script failed, trying basic setup...${NC}"

        # Fallback: basic setup
        mkdir -p "$SPM_PACKAGE/Sources/gldf-rsKit"
        cp "$FFI_CRATE/generated/gldf-rs_ffi.swift" "$SPM_PACKAGE/Sources/gldf-rsKit/"
        echo -e "${GREEN}✓ Swift bindings copied (basic setup)${NC}"
        echo -e "${YELLOW}⚠ Warning: XCFramework not created. Run comprehensive script manually:${NC}"
        echo "  bash $SPM_BUILD_SCRIPT"
    }
else
    echo -e "${YELLOW}Warning: Comprehensive SPM build script not found${NC}"
    echo "Expected at: $SPM_BUILD_SCRIPT"
    echo ""
    echo "Falling back to basic setup (bindings only, no XCFramework)..."

    # Copy bindings to SPM package
    mkdir -p "$SPM_PACKAGE/Sources/gldf-rsKit"
    cp "$FFI_CRATE/generated/gldf-rs_ffi.swift" "$SPM_PACKAGE/Sources/gldf-rsKit/"
    echo -e "${GREEN}✓ Swift bindings copied to SPM package${NC}"
    echo -e "${YELLOW}⚠ Warning: XCFramework not created${NC}"
    echo ""
    echo "To complete the SPM package, you need to:"
    echo "  1. Create XCFramework from the Rust libraries"
    echo "  2. Copy it to: $SPM_PACKAGE/gldf-rsFfi.xcframework"
    echo "  3. Create Package.swift manifest"
fi

echo ""
echo "======================================================================"
echo -e "${GREEN}✅ SPM Package Build Complete!${NC}"
echo "======================================================================"
echo ""
echo "Build Mode: $BUILD_MODE"
echo "Package Location: $SPM_PACKAGE"
echo ""
echo "Next steps:"
echo "  1. Open Xcode project"
echo "  2. Add 'SPM gldf-rsKit' as local package"
echo "  3. Build and run"
echo ""
