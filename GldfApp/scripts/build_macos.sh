#!/usr/bin/env bash
set -euo pipefail

echo "======================================================================"
echo "Building GLDF macOS Application"
echo "======================================================================"

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
FFI_CRATE="$ROOT_DIR/gldf-rs-ffi"
SPM_DIR="$ROOT_DIR/GldfApp/spm"
MACOS_APP="$ROOT_DIR/GldfApp/macos-app"
TARGET_DIR="$ROOT_DIR/target"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

step() {
    echo -e "\n${BLUE}==>${NC} ${GREEN}$1${NC}"
}

warn() {
    echo -e "${YELLOW}Warning: $1${NC}"
}

# Step 1: Build XCFramework (if needed)
step "Checking XCFramework..."
if [ ! -d "$SPM_DIR/GldfFfi.xcframework" ]; then
    echo "XCFramework not found. Building SPM package first..."
    "$SCRIPT_DIR/build_spm_package.sh"
fi

# Verify XCFramework exists
if [ ! -d "$SPM_DIR/GldfFfi.xcframework" ]; then
    echo -e "${YELLOW}Error: XCFramework build failed${NC}"
    exit 1
fi

echo -e "${GREEN}  ✓ XCFramework ready${NC}"

# Step 2: Build macOS App
step "Building macOS application..."
cd "$MACOS_APP"

# Build the app
xcodebuild \
    -project GldfViewer.xcodeproj \
    -scheme GldfViewer \
    -configuration Release \
    -derivedDataPath build \
    build \
    CODE_SIGN_IDENTITY="" \
    CODE_SIGNING_REQUIRED=NO \
    CODE_SIGNING_ALLOWED=NO \
    2>&1 | grep -E "^(Build|Compiling|Linking|error:|warning:)" || true

if [ $? -eq 0 ]; then
    echo -e "${GREEN}  ✓ macOS app built successfully${NC}"
else
    echo -e "${YELLOW}  Build may have warnings - check output above${NC}"
fi

# Find the built app
APP_PATH="$MACOS_APP/build/Build/Products/Release/GldfViewer.app"
if [ -d "$APP_PATH" ]; then
    echo ""
    echo "======================================================================"
    echo -e "${GREEN}✅ Build Complete!${NC}"
    echo "======================================================================"
    echo ""
    echo "📦 Application:"
    echo "   $APP_PATH"
    echo ""
    echo "🚀 Run the app:"
    echo "   open \"$APP_PATH\""
    echo ""
    echo "📋 Or open in Xcode:"
    echo "   open \"$MACOS_APP/GldfViewer.xcodeproj\""
    echo ""
else
    echo ""
    echo -e "${YELLOW}Build completed but app not found at expected location.${NC}"
    echo "Open the project in Xcode to build manually:"
    echo "   open \"$MACOS_APP/GldfViewer.xcodeproj\""
fi
