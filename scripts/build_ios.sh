#!/usr/bin/env bash
#
# build_ios.sh - Build iOS application
#
# Usage: ./build_ios.sh [dev|release]
#
# Arguments:
#   dev     - Development build (debug configuration)
#   release - Release build (optimized, for App Store)
#
# Output: gldf-rs-Apps/gldf-rs-mac_watch_tv_ios/build/
#

set -euo pipefail

# Parse arguments
BUILD_MODE="${1:-release}"
if [[ "$BUILD_MODE" != "dev" && "$BUILD_MODE" != "release" ]]; then
    echo "❌ Error: Build mode must be 'dev' or 'release'"
    echo "Usage: $0 [dev|release]"
    exit 1
fi

# Convert to Xcode configuration
if [[ "$BUILD_MODE" == "dev" ]]; then
    XCODE_CONFIG="Debug"
else
    XCODE_CONFIG="Release"
fi

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
XCODE_PROJECT="$ROOT_DIR/gldf-rs-Apps/gldf-rs-mac_watch_tv_ios/gldf-rs.xcodeproj"
ARCHIVE_PATH="$ROOT_DIR/gldf-rs-Apps/gldf-rs-mac_watch_tv_ios/build/gldf-rs-iOS.xcarchive"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "======================================================================"
echo "Building iOS App (mode: $BUILD_MODE)"
echo "======================================================================"

# Step 1: Check if SPM package exists and is up-to-date
SPM_PACKAGE="$ROOT_DIR/gldf-rs-Apps/SPM-gldf-rsKit"
SPM_XCFRAMEWORK="$SPM_PACKAGE/gldf-rsFfi.xcframework"
SPM_SOURCES="$SPM_PACKAGE/Sources/gldf-rsKit/gldf-rs_ffi.swift"

NEEDS_SPM_BUILD=false

if [ ! -d "$SPM_XCFRAMEWORK" ] || [ ! -f "$SPM_SOURCES" ]; then
    echo -e "\n${BLUE}==>${NC} ${GREEN}SPM package not found, building...${NC}"
    NEEDS_SPM_BUILD=true
elif [ ! -f "$SPM_PACKAGE/Package.swift" ]; then
    echo -e "\n${BLUE}==>${NC} ${GREEN}SPM Package.swift missing, rebuilding...${NC}"
    NEEDS_SPM_BUILD=true
else
    echo -e "\n${BLUE}==>${NC} ${GREEN}SPM package exists, checking if rebuild needed...${NC}"
    # Check if Rust FFI source is newer than XCFramework
    FFI_SOURCE="$ROOT_DIR/crates/gldf-rs-ffi/src"
    if [ -d "$FFI_SOURCE" ]; then
        if [ "$FFI_SOURCE" -nt "$SPM_XCFRAMEWORK" ]; then
            echo -e "${YELLOW}  FFI source updated, rebuilding SPM...${NC}"
            NEEDS_SPM_BUILD=true
        else
            echo -e "${GREEN}  ✓ SPM package is up-to-date${NC}"
        fi
    fi
fi

if [ "$NEEDS_SPM_BUILD" = true ]; then
    "$SCRIPT_DIR/build_spm.sh" "$BUILD_MODE"
fi

# Step 2: Clean previous build
echo -e "\n${BLUE}==>${NC} ${GREEN}Cleaning previous build...${NC}"
rm -rf "$ARCHIVE_PATH"

# Step 3: Build iOS archive
echo -e "\n${BLUE}==>${NC} ${GREEN}Building iOS archive...${NC}"

xcodebuild archive \
    -project "$XCODE_PROJECT" \
    -scheme "gldf-rs" \
    -destination "generic/platform=iOS" \
    -archivePath "$ARCHIVE_PATH" \
    -configuration "$XCODE_CONFIG" \
    CODE_SIGN_STYLE=Automatic \
    DEBUG_INFORMATION_FORMAT=dwarf-with-dsym

echo ""
echo "======================================================================"
echo -e "${GREEN}✅ iOS Build Complete!${NC}"
echo "======================================================================"
echo ""
echo "Configuration: $XCODE_CONFIG"
echo "Archive: $ARCHIVE_PATH"
echo ""
if [[ "$BUILD_MODE" == "release" ]]; then
    echo "Next steps for App Store submission:"
    echo "  1. Open Xcode → Window → Organizer"
    echo "  2. Select the archive"
    echo "  3. Click 'Distribute App'"
    echo "  4. Follow App Store submission wizard"
else
    echo "Development build complete. Install on device:"
    echo "  1. Connect iOS device"
    echo "  2. xcodebuild -exportArchive ..."
    echo "  3. Or use Xcode Organizer"
fi
echo ""
