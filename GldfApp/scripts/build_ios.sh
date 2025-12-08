#!/usr/bin/env bash
set -euo pipefail

echo "======================================================================"
echo "Building GLDF iOS Application"
echo "======================================================================"

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
FFI_CRATE="$ROOT_DIR/gldf-rs-ffi"
SPM_DIR="$ROOT_DIR/GldfApp/spm"
APP_DIR="$ROOT_DIR/GldfApp/mac_ios"
TARGET_DIR="$ROOT_DIR/target"

# Default to simulator, use "device" argument for real device
BUILD_FOR="${1:-simulator}"

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

# Step 2: Build iOS App
cd "$APP_DIR"

if [ "$BUILD_FOR" = "device" ]; then
    step "Building iOS application for device..."
    SDK="iphoneos"
    DESTINATION="generic/platform=iOS"
else
    step "Building iOS application for simulator..."
    SDK="iphonesimulator"
    DESTINATION="generic/platform=iOS Simulator"
fi

# Build the app
xcodebuild \
    -project GldfViewer.xcodeproj \
    -scheme "GLDF Viewer" \
    -configuration Release \
    -sdk "$SDK" \
    -destination "$DESTINATION" \
    -derivedDataPath build \
    build \
    CODE_SIGN_IDENTITY="" \
    CODE_SIGNING_REQUIRED=NO \
    CODE_SIGNING_ALLOWED=NO \
    2>&1 | grep -E "^(Build|Compiling|Linking|error:|warning:)" || true

if [ $? -eq 0 ]; then
    echo -e "${GREEN}  ✓ iOS app built successfully${NC}"
else
    echo -e "${YELLOW}  Build may have warnings - check output above${NC}"
fi

# Find the built app
if [ "$BUILD_FOR" = "device" ]; then
    BUILT_APP="$APP_DIR/build/Build/Products/Release-iphoneos/GLDF Viewer.app"
else
    BUILT_APP="$APP_DIR/build/Build/Products/Release-iphonesimulator/GLDF Viewer.app"
fi

if [ -d "$BUILT_APP" ]; then
    echo ""
    echo "======================================================================"
    echo -e "${GREEN}✅ Build Complete!${NC}"
    echo "======================================================================"
    echo ""
    echo "📦 Application:"
    echo "   $BUILT_APP"
    echo ""

    if [ "$BUILD_FOR" = "simulator" ]; then
        echo "🚀 Install to simulator:"
        echo "   xcrun simctl install booted \"$BUILT_APP\""
        echo ""
        echo "   Or boot a simulator first:"
        echo "   xcrun simctl boot \"iPhone 15 Pro\""
        echo ""
    else
        echo "🚀 Install to device via Xcode or:"
        echo "   ios-deploy --bundle \"$BUILT_APP\""
        echo ""
    fi

    echo "📋 Or open in Xcode:"
    echo "   open \"$APP_DIR/GldfViewer.xcodeproj\""
    echo ""
else
    echo ""
    echo -e "${YELLOW}Build completed but app not found at expected location.${NC}"
    echo "Open the project in Xcode to build manually:"
    echo "   open \"$APP_DIR/GldfViewer.xcodeproj\""
fi

echo ""
echo "Usage:"
echo "  $0           # Build for simulator (default)"
echo "  $0 simulator # Build for simulator"
echo "  $0 device    # Build for real device"
