#!/usr/bin/env bash
set -euo pipefail

echo "======================================================================"
echo "Building Standalone GldfKit SPM Package"
echo "======================================================================"

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
FFI_CRATE="$ROOT_DIR/gldf-rs-ffi"
SPM_PACKAGE="$ROOT_DIR/GldfApp/spm"
TARGET_DIR="$ROOT_DIR/target"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

step() {
    echo -e "\n${BLUE}==>${NC} ${GREEN}$1${NC}"
}

# Step 1: Build the library first so we can extract bindings from it
step "Building Rust library for macOS (arm64 + x86_64)..."
cd "$FFI_CRATE"

# Ensure both macOS targets are installed
echo "  • Ensuring Rust targets are installed..."
rustup target add aarch64-apple-darwin x86_64-apple-darwin 2>/dev/null || true

echo "  • Building macOS arm64..."
cargo build --release --target aarch64-apple-darwin --lib

echo "  • Building macOS x86_64..."
cargo build --release --target x86_64-apple-darwin --lib

# Create universal macOS binary using lipo
echo "  • Creating universal macOS binary..."
mkdir -p "$TARGET_DIR/universal-apple-darwin/release"
lipo -create \
    "$TARGET_DIR/aarch64-apple-darwin/release/libgldf_ffi.dylib" \
    "$TARGET_DIR/x86_64-apple-darwin/release/libgldf_ffi.dylib" \
    -output "$TARGET_DIR/universal-apple-darwin/release/libgldf_ffi.dylib"

# Fix install name for macOS universal binary
echo "  • Fixing install names for macOS..."
install_name_tool -id "@rpath/GldfFfi.framework/Versions/A/GldfFfi" \
    "$TARGET_DIR/universal-apple-darwin/release/libgldf_ffi.dylib"

# Generate dSYM for macOS universal binary
echo "  • Generating dSYM for macOS..."
dsymutil "$TARGET_DIR/universal-apple-darwin/release/libgldf_ffi.dylib" \
    -o "$TARGET_DIR/universal-apple-darwin/release/GldfFfi.framework.dSYM" 2>/dev/null || true

echo -e "${GREEN}  ✓ Universal macOS binary created${NC}"

# Step 2: Generate Swift bindings from the COMPILED library (not UDL)
step "Generating fresh Swift bindings from compiled library..."
cd "$FFI_CRATE"

if [ ! -d "generated" ]; then
    mkdir -p generated
fi

echo "  • Running uniffi-bindgen from library..."
cargo run --bin uniffi-bindgen generate \
    --library "$TARGET_DIR/aarch64-apple-darwin/release/libgldf_ffi.dylib" \
    --language swift \
    --out-dir generated

if [ ! -f "generated/gldf_ffi.swift" ]; then
    echo -e "${YELLOW}  Error: Swift bindings generation failed${NC}"
    exit 1
fi

echo -e "${GREEN}  ✓ Swift bindings generated from library${NC}"

# Step 3: Build Rust library for remaining platforms
step "Building Rust libraries for iOS platforms..."
cd "$FFI_CRATE"

# Ensure iOS targets are installed
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios 2>/dev/null || true

echo "  • Building for iOS device (aarch64-apple-ios)..."
cargo build --release --target aarch64-apple-ios --lib

echo "  • Building for iOS simulator arm64 (aarch64-apple-ios-sim)..."
cargo build --release --target aarch64-apple-ios-sim --lib

echo "  • Building for iOS simulator x86_64 (x86_64-apple-ios)..."
cargo build --release --target x86_64-apple-ios --lib

# Create universal iOS simulator binary
echo "  • Creating universal iOS simulator binary..."
mkdir -p "$TARGET_DIR/universal-apple-ios-sim/release"
lipo -create \
    "$TARGET_DIR/aarch64-apple-ios-sim/release/libgldf_ffi.dylib" \
    "$TARGET_DIR/x86_64-apple-ios/release/libgldf_ffi.dylib" \
    -output "$TARGET_DIR/universal-apple-ios-sim/release/libgldf_ffi.dylib"

# Fix install names for iOS simulator
echo "  • Fixing install names for iOS simulator..."
install_name_tool -id "@rpath/GldfFfi.framework/GldfFfi" \
    "$TARGET_DIR/universal-apple-ios-sim/release/libgldf_ffi.dylib"

# Fix install name for iOS device
echo "  • Fixing install names for iOS device..."
install_name_tool -id "@rpath/GldfFfi.framework/GldfFfi" \
    "$TARGET_DIR/aarch64-apple-ios/release/libgldf_ffi.dylib"

# Generate dSYMs for iOS
echo "  • Generating dSYMs for iOS..."
dsymutil "$TARGET_DIR/aarch64-apple-ios/release/libgldf_ffi.dylib" \
    -o "$TARGET_DIR/aarch64-apple-ios/release/GldfFfi.framework.dSYM" 2>/dev/null || true
dsymutil "$TARGET_DIR/universal-apple-ios-sim/release/libgldf_ffi.dylib" \
    -o "$TARGET_DIR/universal-apple-ios-sim/release/GldfFfi.framework.dSYM" 2>/dev/null || true

echo -e "${GREEN}  ✓ iOS platforms built${NC}"

# Step 4: Create XCFramework with proper framework structure
step "Creating XCFramework with framework wrappers..."

TEMP_BUILD="$FFI_CRATE/temp_framework_build"
rm -rf "$TEMP_BUILD"
mkdir -p "$TEMP_BUILD"

# Function to create a .framework from a .dylib or .a
# macOS requires versioned (deep) bundle structure, iOS uses shallow structure
create_framework() {
    local platform=$1
    local lib_path=$2
    local headers_dir=$3
    local output_dir=$4
    local min_os="${5:-13.0}"

    echo "  • Creating framework for $platform..."

    mkdir -p "$output_dir"
    local fw_dir="$output_dir/GldfFfi.framework"
    rm -rf "$fw_dir"

    # Determine platform name for Info.plist
    local platform_name="iPhoneOS"
    local is_macos=false
    case "$platform" in
        ios-simulator) platform_name="iPhoneSimulator" ;;
        macos) platform_name="MacOSX"; is_macos=true ;;
        watchos-device) platform_name="WatchOS" ;;
        watchos-simulator) platform_name="WatchSimulator" ;;
    esac

    if [ "$is_macos" = true ]; then
        # macOS: Create versioned (deep) bundle structure
        # GldfFfi.framework/
        #   Versions/
        #     A/
        #       GldfFfi (binary)
        #       Resources/Info.plist
        #       Headers/*.h
        #       Modules/module.modulemap
        #     Current -> A
        #   GldfFfi -> Versions/Current/GldfFfi
        #   Headers -> Versions/Current/Headers
        #   Modules -> Versions/Current/Modules
        #   Resources -> Versions/Current/Resources

        mkdir -p "$fw_dir/Versions/A/Headers"
        mkdir -p "$fw_dir/Versions/A/Modules"
        mkdir -p "$fw_dir/Versions/A/Resources"

        # Copy binary
        cp "$lib_path" "$fw_dir/Versions/A/GldfFfi"
        chmod +x "$fw_dir/Versions/A/GldfFfi"

        # Copy headers
        cp "$headers_dir"/*.h "$fw_dir/Versions/A/Headers/" 2>/dev/null || true

        # Create module.modulemap
        cat > "$fw_dir/Versions/A/Modules/module.modulemap" <<EOF
framework module GldfFfi {
    umbrella header "gldf_ffiFFI.h"
    export *
}
EOF

        # Create Info.plist in Resources
        cat > "$fw_dir/Versions/A/Resources/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>GldfFfi</string>
    <key>CFBundleIdentifier</key>
    <string>com.trahe.GldfFfi</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>GldfFfi</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleShortVersionString</key>
    <string>0.3.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>${platform_name}</string>
    </array>
    <key>MinimumOSVersion</key>
    <string>${min_os}</string>
</dict>
</plist>
EOF

        # Create symlinks for versioned structure
        cd "$fw_dir/Versions"
        ln -sf A Current
        cd "$fw_dir"
        ln -sf Versions/Current/GldfFfi GldfFfi
        ln -sf Versions/Current/Headers Headers
        ln -sf Versions/Current/Modules Modules
        ln -sf Versions/Current/Resources Resources
        cd - > /dev/null
    else
        # iOS/other: Create shallow bundle structure
        mkdir -p "$fw_dir/Headers"
        mkdir -p "$fw_dir/Modules"

        # Copy binary
        cp "$lib_path" "$fw_dir/GldfFfi"
        chmod +x "$fw_dir/GldfFfi"

        # Copy headers
        cp "$headers_dir"/*.h "$fw_dir/Headers/" 2>/dev/null || true

        # Create module.modulemap
        cat > "$fw_dir/Modules/module.modulemap" <<EOF
framework module GldfFfi {
    umbrella header "gldf_ffiFFI.h"
    export *
}
EOF

        # Create Info.plist
        cat > "$fw_dir/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>GldfFfi</string>
    <key>CFBundleIdentifier</key>
    <string>com.trahe.GldfFfi</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>GldfFfi</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleShortVersionString</key>
    <string>0.3.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>${platform_name}</string>
    </array>
    <key>MinimumOSVersion</key>
    <string>${min_os}</string>
</dict>
</plist>
EOF
    fi
}

# Create frameworks for each platform
create_framework "ios-device" \
    "$TARGET_DIR/aarch64-apple-ios/release/libgldf_ffi.dylib" \
    "$FFI_CRATE/generated" \
    "$TEMP_BUILD/ios-arm64" \
    "13.0"

create_framework "ios-simulator" \
    "$TARGET_DIR/universal-apple-ios-sim/release/libgldf_ffi.dylib" \
    "$FFI_CRATE/generated" \
    "$TEMP_BUILD/ios-universal-simulator" \
    "13.0"

create_framework "macos" \
    "$TARGET_DIR/universal-apple-darwin/release/libgldf_ffi.dylib" \
    "$FFI_CRATE/generated" \
    "$TEMP_BUILD/macos-universal" \
    "13.0"

# Build XCFramework with debug symbols
step "Building XCFramework with debug symbols..."
XCFRAMEWORK_PATH="$TEMP_BUILD/GldfFfi.xcframework"
rm -rf "$XCFRAMEWORK_PATH"

# Check if dSYMs exist and include them
DSYM_ARGS=""
if [ -d "$TARGET_DIR/aarch64-apple-ios/release/GldfFfi.framework.dSYM" ]; then
    DSYM_ARGS="$DSYM_ARGS -debug-symbols $TARGET_DIR/aarch64-apple-ios/release/GldfFfi.framework.dSYM"
fi
if [ -d "$TARGET_DIR/universal-apple-ios-sim/release/GldfFfi.framework.dSYM" ]; then
    DSYM_ARGS="$DSYM_ARGS -debug-symbols $TARGET_DIR/universal-apple-ios-sim/release/GldfFfi.framework.dSYM"
fi
if [ -d "$TARGET_DIR/universal-apple-darwin/release/GldfFfi.framework.dSYM" ]; then
    DSYM_ARGS="$DSYM_ARGS -debug-symbols $TARGET_DIR/universal-apple-darwin/release/GldfFfi.framework.dSYM"
fi

xcodebuild -create-xcframework \
    -framework "$TEMP_BUILD/ios-arm64/GldfFfi.framework" \
    -debug-symbols "$TARGET_DIR/aarch64-apple-ios/release/GldfFfi.framework.dSYM" \
    -framework "$TEMP_BUILD/ios-universal-simulator/GldfFfi.framework" \
    -debug-symbols "$TARGET_DIR/universal-apple-ios-sim/release/GldfFfi.framework.dSYM" \
    -framework "$TEMP_BUILD/macos-universal/GldfFfi.framework" \
    -debug-symbols "$TARGET_DIR/universal-apple-darwin/release/GldfFfi.framework.dSYM" \
    -output "$XCFRAMEWORK_PATH" 2>/dev/null || \
xcodebuild -create-xcframework \
    -framework "$TEMP_BUILD/ios-arm64/GldfFfi.framework" \
    -framework "$TEMP_BUILD/ios-universal-simulator/GldfFfi.framework" \
    -framework "$TEMP_BUILD/macos-universal/GldfFfi.framework" \
    -output "$XCFRAMEWORK_PATH"

echo -e "${GREEN}  ✓ XCFramework created${NC}"

# Step 5: Copy to SPM package
step "Syncing to SPM package..."

# Create SPM package structure if it doesn't exist
mkdir -p "$SPM_PACKAGE/Sources/GldfKit"
mkdir -p "$SPM_PACKAGE/Tests/GldfKitTests"

# Copy XCFramework
echo "  • Copying XCFramework..."
rm -rf "$SPM_PACKAGE/GldfFfi.xcframework"
cp -R "$XCFRAMEWORK_PATH" "$SPM_PACKAGE/"

# Copy Swift bindings with proper import handling
echo "  • Copying and updating Swift bindings..."
awk '
BEGIN { in_import_section = 0; import_section_done = 0 }
/^#if canImport\(gldf_ffiFFI\)/ {
    if (!import_section_done) {
        print "#if canImport(GldfFfi)"
        print "import GldfFfi"
        print "#elseif canImport(gldf_ffiFFI)"
        print "import gldf_ffiFFI"
        print "#endif"
        in_import_section = 1
        import_section_done = 1
    }
    next
}
/^import gldf_ffiFFI/ && in_import_section { next }
/^#endif/ && in_import_section { in_import_section = 0; next }
{ print }
' "$FFI_CRATE/generated/gldf_ffi.swift" > "$SPM_PACKAGE/Sources/GldfKit/gldf_ffi.swift"

# Ensure tests exist
if [ ! -f "$SPM_PACKAGE/Tests/GldfKitTests/GldfKitTests.swift" ]; then
    echo "  • Creating test file..."
    cat > "$SPM_PACKAGE/Tests/GldfKitTests/GldfKitTests.swift" <<'EOF'
import XCTest
@testable import GldfKit

final class GldfKitTests: XCTestCase {
    func testLibraryVersion() throws {
        let version = gldfLibraryVersion()
        print("GLDF Library version: \(version)")
        XCTAssertFalse(version.isEmpty, "Version should not be empty")
    }
}
EOF
fi

# Cleanup
step "Cleaning up..."
rm -rf "$TEMP_BUILD"

echo ""
echo "======================================================================"
echo -e "${GREEN}✅ SPM Package Build Complete!${NC}"
echo "======================================================================"
echo ""
echo "📦 Package Location:"
echo "   $SPM_PACKAGE"
echo ""
echo "📚 Package Contents:"
echo "   • GldfFfi.xcframework    (Rust dylibs - macOS universal arm64+x86_64)"
echo "   • Sources/GldfKit/       (Swift bindings)"
echo "   • Tests/                 (Unit tests)"
echo "   • Package.swift          (SPM manifest)"
echo ""
echo "🚀 Usage in Other Projects:"
echo "   1. Drag GldfApp/spm folder to Xcode project"
echo "   2. File → Add Packages → Add Local"
echo "   3. import GldfKit"
echo ""
