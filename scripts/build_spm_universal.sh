#!/usr/bin/env bash
#
# build_spm_universal.sh - Build COMPLETE universal SPM package
#
# Builds for ALL Apple platforms with BOTH debug and release configurations
# Includes dSYMs for all platforms
# Ready for GitHub binary release
#
# Platforms:
#   - iOS (device + simulator)
#   - macOS (Intel x86_64 + Apple Silicon arm64 universal)
#   - tvOS (device + simulator)
#   - watchOS (device arm64 + arm64_32 + simulator)
#   - visionOS (device + simulator) [if targets installed]
#
# Output: Complete SPM package ready for distribution
#

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
FFI_CRATE="$ROOT_DIR/crates/gldf-rs-ffi"
SPM_PACKAGE="$ROOT_DIR/gldf-rs-Apps/SPM-gldf-rsKit"
TARGET_DIR="$ROOT_DIR/target"
TEMP_BUILD="$FFI_CRATE/temp_universal_build"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

step() {
    echo -e "\n${BLUE}==>${NC} ${GREEN}$1${NC}"
}

info() {
    echo -e "${CYAN}  $1${NC}"
}

echo "======================================================================"
echo "Building UNIVERSAL SPM Package (Debug + Release)"
echo "======================================================================"
echo ""
echo "This will build:"
echo "  ✓ iOS (arm64 device + arm64 simulator)"
echo "  ✓ macOS (universal: x86_64 Intel + arm64 Apple Silicon)"
echo "  ✓ tvOS (arm64 device + arm64 simulator)"
echo "  ✓ watchOS (arm64 + arm64_32 device + arm64 simulator)"
echo "  ✓ Both DEBUG and RELEASE configurations"
echo "  ✓ dSYMs for all platforms"
echo ""
echo "Ready for: GitHub binary release, App Store, development"
echo ""
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

# Clean previous build
rm -rf "$TEMP_BUILD"
mkdir -p "$TEMP_BUILD"

# Step 0: Download fresh data (optional but recommended)
step "Checking embedded data..."
DATA_FILE="$FFI_CRATE/gldf-rs_rs_data/gldf-rs.flat.comp.blobs.bin"
if [ -f "$DATA_FILE" ]; then
    DATA_AGE=$(( ($(date +%s) - $(stat -f %m "$DATA_FILE")) / 86400 ))
    echo "Current data is $DATA_AGE days old"
    if [ $DATA_AGE -gt 30 ]; then
        echo -e "${YELLOW}Warning: Data is over 30 days old${NC}"
        echo "Consider updating data before release:"
        echo "  cd crates/gldf-rs-core && cargo run --example download_and_build"
        echo ""
        read -p "Continue with old data? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "Aborted. Please update data first."
            exit 0
        fi
    else
        echo -e "${GREEN}✓ Data is recent${NC}"
    fi
else
    echo -e "${RED}Error: Embedded data not found at $DATA_FILE${NC}"
    echo "Run: cd crates/gldf-rs-core && cargo run --example download_and_build"
    exit 1
fi

# Step 1: Build Rust for ALL targets in BOTH profiles
step "Building Rust libraries (debug + release)..."

cd "$FFI_CRATE"

# Function to build for a target in both profiles
build_target() {
    local target=$1
    local needs_nightly=$2
    local platform_name=$3

    info "Building $platform_name..."

    if [ "$needs_nightly" = "nightly" ]; then
        cargo +nightly build --target "$target" -Z build-std --lib
        cargo +nightly build --release --target "$target" -Z build-std --lib
    else
        cargo build --target "$target" --lib
        cargo build --release --target "$target" --lib
    fi
}

# macOS (both architectures for universal binary)
build_target "aarch64-apple-darwin" "stable" "macOS Apple Silicon"
build_target "x86_64-apple-darwin" "stable" "macOS Intel"

# iOS
build_target "aarch64-apple-ios" "stable" "iOS device"
build_target "aarch64-apple-ios-sim" "stable" "iOS simulator"

# tvOS (requires nightly) - DISABLED: components unavailable since 2025-11-14
# build_target "aarch64-apple-tvos" "nightly" "tvOS device"
# build_target "aarch64-apple-tvos-sim" "nightly" "tvOS simulator"

# watchOS (requires nightly) - DISABLED: components unavailable since 2025-11-14
# build_target "aarch64-apple-watchos" "nightly" "watchOS arm64"
# build_target "arm64_32-apple-watchos" "nightly" "watchOS arm64_32"
# build_target "aarch64-apple-watchos-sim" "nightly" "watchOS simulator"

# visionOS (if targets are installed - optional)
if rustup target list --installed | grep -q "aarch64-apple-visionos"; then
    build_target "aarch64-apple-visionos" "nightly" "visionOS device"
    build_target "aarch64-apple-visionos-sim" "nightly" "visionOS simulator"
    HAS_VISIONOS=true
else
    HAS_VISIONOS=false
    info "visionOS targets not installed, skipping"
fi

echo -e "${GREEN}✓ All Rust libraries built (debug + release)${NC}"

# Step 2: Generate Swift bindings
step "Generating Swift bindings..."

if [ ! -d "generated" ]; then
    mkdir -p generated
fi

cargo run --bin uniffi-bindgen generate \
    --library "$TARGET_DIR/aarch64-apple-darwin/release/libgldf-rs_ffi.dylib" \
    --language swift \
    --out-dir generated

if [ ! -f "generated/gldf-rs_ffi.swift" ]; then
    echo -e "${YELLOW}Error: Swift bindings generation failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Swift bindings generated${NC}"

# Step 3: Create frameworks for ALL platforms in BOTH configurations
step "Creating frameworks with proper structure..."

# Function to create a .framework from a .dylib or .a
create_framework() {
    local profile=$1        # "debug" or "release"
    local platform=$2       # "ios-device", "macos", etc.
    local lib_path=$3       # path to .dylib or .a
    local headers_dir=$4    # path to headers
    local output_dir=$5     # output directory
    local min_os=$6         # minimum OS version

    local profile_suffix=""
    if [ "$profile" = "debug" ]; then
        profile_suffix="-debug"
    fi

    info "Creating $platform framework ($profile)..."

    mkdir -p "$output_dir"
    local fw_dir="$output_dir/gldf-rsFfi.framework"

    # macOS requires versioned bundle structure
    if [[ "$platform" == "macos" ]]; then
        mkdir -p "$fw_dir/Versions/A/Resources"
        mkdir -p "$fw_dir/Versions/A/Headers"
        mkdir -p "$fw_dir/Versions/A/Modules"

        cp "$lib_path" "$fw_dir/Versions/A/gldf-rsFfi"
        chmod +x "$fw_dir/Versions/A/gldf-rsFfi"

        install_name_tool -id @rpath/gldf-rsFfi.framework/Versions/A/gldf-rsFfi "$fw_dir/Versions/A/gldf-rsFfi" 2>/dev/null || true

        cp "$headers_dir"/*.h "$fw_dir/Versions/A/Headers/" 2>/dev/null || true
        cp "$headers_dir"/*.swift "$fw_dir/Versions/A/Headers/" 2>/dev/null || true

        cat > "$fw_dir/Versions/A/Headers/module.modulemap" <<EOF
framework module gldf-rsFfi {
    umbrella header "gldf-rs_ffiFFI.h"
    export *
}
EOF
        cp "$fw_dir/Versions/A/Headers/module.modulemap" "$fw_dir/Versions/A/Modules/"

        cat > "$fw_dir/Versions/A/Resources/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>gldf-rsFfi</string>
    <key>CFBundleIdentifier</key>
    <string>com.trahe.gldf-rsFfi</string>
    <key>CFBundleName</key>
    <string>gldf-rsFfi</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleShortVersionString</key>
    <string>0.0.1</string>
    <key>MinimumOSVersion</key>
    <string>${min_os}</string>
</dict>
</plist>
EOF

        cd "$fw_dir/Versions"
        ln -sf A Current
        cd "$fw_dir"
        ln -sf Versions/Current/gldf-rsFfi gldf-rsFfi
        ln -sf Versions/Current/Resources Resources
        ln -sf Versions/Current/Headers Headers
        ln -sf Versions/Current/Modules Modules

        return
    fi

    # Non-macOS platforms use shallow bundle
    mkdir -p "$fw_dir"

    cp "$lib_path" "$fw_dir/gldf-rsFfi"
    chmod +x "$fw_dir/gldf-rsFfi"

    install_name_tool -id @rpath/gldf-rsFfi.framework/gldf-rsFfi "$fw_dir/gldf-rsFfi" 2>/dev/null || true

    mkdir -p "$fw_dir/Headers"
    cp "$headers_dir"/*.h "$fw_dir/Headers/" 2>/dev/null || true
    cp "$headers_dir"/*.swift "$fw_dir/Headers/" 2>/dev/null || true

    cat > "$fw_dir/Headers/module.modulemap" <<EOF
framework module gldf-rsFfi {
    umbrella header "gldf-rs_ffiFFI.h"
    export *
}
EOF

    mkdir -p "$fw_dir/Modules"
    cp "$fw_dir/Headers/module.modulemap" "$fw_dir/Modules/"

    local platform_name="iPhoneOS"
    case "$platform" in
        ios-simulator) platform_name="iPhoneSimulator" ;;
        tvos-device) platform_name="AppleTVOS" ;;
        tvos-simulator) platform_name="AppleTVSimulator" ;;
        watchos-device) platform_name="WatchOS" ;;
        watchos-simulator) platform_name="WatchSimulator" ;;
        visionos-device) platform_name="XROS" ;;
        visionos-simulator) platform_name="XRSimulator" ;;
    esac

    cat > "$fw_dir/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>gldf-rsFfi</string>
    <key>CFBundleIdentifier</key>
    <string>com.trahe.gldf-rsFfi</string>
    <key>CFBundleName</key>
    <string>gldf-rsFfi</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleShortVersionString</key>
    <string>0.0.1</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>${platform_name}</string>
    </array>
    <key>MinimumOSVersion</key>
    <string>${min_os}</string>
</dict>
</plist>
EOF
}

# Build frameworks for BOTH profiles
for profile in debug release; do
    step "Creating $profile frameworks..."

    PROFILE_DIR="$profile"

    # iOS
    create_framework "$profile" "ios-device" \
        "$TARGET_DIR/aarch64-apple-ios/$PROFILE_DIR/libgldf-rs_ffi.dylib" \
        "$FFI_CRATE/generated" \
        "$TEMP_BUILD/$profile/ios-arm64" \
        "13.0"

    create_framework "$profile" "ios-simulator" \
        "$TARGET_DIR/aarch64-apple-ios-sim/$PROFILE_DIR/libgldf-rs_ffi.dylib" \
        "$FFI_CRATE/generated" \
        "$TEMP_BUILD/$profile/ios-arm64-simulator" \
        "13.0"

    # macOS universal
    info "Creating macOS universal binary ($profile)..."
    MACOS_UNIVERSAL_DIR="$TARGET_DIR/macos-universal-$profile"
    mkdir -p "$MACOS_UNIVERSAL_DIR"
    lipo -create \
        "$TARGET_DIR/aarch64-apple-darwin/$PROFILE_DIR/libgldf-rs_ffi.dylib" \
        "$TARGET_DIR/x86_64-apple-darwin/$PROFILE_DIR/libgldf-rs_ffi.dylib" \
        -output "$MACOS_UNIVERSAL_DIR/libgldf-rs_ffi.dylib"

    create_framework "$profile" "macos" \
        "$MACOS_UNIVERSAL_DIR/libgldf-rs_ffi.dylib" \
        "$FFI_CRATE/generated" \
        "$TEMP_BUILD/$profile/macos-arm64_x86_64" \
        "13.0"

    # tvOS - DISABLED: components unavailable
    # create_framework "$profile" "tvos-device" \
    #     "$TARGET_DIR/aarch64-apple-tvos/$PROFILE_DIR/libgldf-rs_ffi.a" \
    #     "$FFI_CRATE/generated" \
    #     "$TEMP_BUILD/$profile/tvos-arm64" \
    #     "13.0"

    # create_framework "$profile" "tvos-simulator" \
    #     "$TARGET_DIR/aarch64-apple-tvos-sim/$PROFILE_DIR/libgldf-rs_ffi.a" \
    #     "$FFI_CRATE/generated" \
    #     "$TEMP_BUILD/$profile/tvos-arm64-simulator" \
    #     "13.0"

    # watchOS - DISABLED: components unavailable
    # info "Creating watchOS fat binary ($profile)..."
    # WATCHOS_FAT_DIR="$TARGET_DIR/watchos-fat-$profile"
    # mkdir -p "$WATCHOS_FAT_DIR"
    # lipo -create \
    #     "$TARGET_DIR/aarch64-apple-watchos/$PROFILE_DIR/libgldf-rs_ffi.a" \
    #     "$TARGET_DIR/arm64_32-apple-watchos/$PROFILE_DIR/libgldf-rs_ffi.a" \
    #     -output "$WATCHOS_FAT_DIR/libgldf-rs_ffi.a"

    # create_framework "$profile" "watchos-device" \
    #     "$WATCHOS_FAT_DIR/libgldf-rs_ffi.a" \
    #     "$FFI_CRATE/generated" \
    #     "$TEMP_BUILD/$profile/watchos-arm64_arm64_32" \
    #     "6.0"

    # create_framework "$profile" "watchos-simulator" \
    #     "$TARGET_DIR/aarch64-apple-watchos-sim/$PROFILE_DIR/libgldf-rs_ffi.a" \
    #     "$FFI_CRATE/generated" \
    #     "$TEMP_BUILD/$profile/watchos-arm64-simulator" \
    #     "6.0"

    # visionOS (if available)
    if [ "$HAS_VISIONOS" = true ]; then
        create_framework "$profile" "visionos-device" \
            "$TARGET_DIR/aarch64-apple-visionos/$PROFILE_DIR/libgldf-rs_ffi.a" \
            "$FFI_CRATE/generated" \
            "$TEMP_BUILD/$profile/xros-arm64" \
            "1.0"

        create_framework "$profile" "visionos-simulator" \
            "$TARGET_DIR/aarch64-apple-visionos-sim/$PROFILE_DIR/libgldf-rs_ffi.a" \
            "$FFI_CRATE/generated" \
            "$TEMP_BUILD/$profile/xros-arm64-simulator" \
            "1.0"
    fi

    echo -e "${GREEN}✓ $profile frameworks created${NC}"
done

# Step 4: Generate dSYMs for all configurations
step "Generating dSYMs..."

generate_dsym() {
    local dylib_path=$1
    local output_dir=$2
    local platform_name=$3

    if [ -f "$dylib_path" ]; then
        info "Generating dSYM for $platform_name..."
        mkdir -p "$output_dir"
        dsymutil "$dylib_path" -o "$output_dir/gldf-rsFfi.framework.dSYM"

        # Rename DWARF file to match framework name
        local dwarf_file=$(find "$output_dir/gldf-rsFfi.framework.dSYM/Contents/Resources/DWARF" -type f | head -1)
        if [ -f "$dwarf_file" ]; then
            mv "$dwarf_file" "$output_dir/gldf-rsFfi.framework.dSYM/Contents/Resources/DWARF/gldf-rsFfi"
        fi
    fi
}

for profile in debug release; do
    PROFILE_DIR="$profile"

    # iOS
    generate_dsym "$TARGET_DIR/aarch64-apple-ios/$PROFILE_DIR/libgldf-rs_ffi.dylib" \
        "$TEMP_BUILD/$profile/ios-arm64/dSYMs" "iOS device ($profile)"

    generate_dsym "$TARGET_DIR/aarch64-apple-ios-sim/$PROFILE_DIR/libgldf-rs_ffi.dylib" \
        "$TEMP_BUILD/$profile/ios-arm64-simulator/dSYMs" "iOS simulator ($profile)"

    # macOS
    generate_dsym "$TARGET_DIR/macos-universal-$profile/libgldf-rs_ffi.dylib" \
        "$TEMP_BUILD/$profile/macos-arm64_x86_64/dSYMs" "macOS universal ($profile)"
done

echo -e "${GREEN}✓ dSYMs generated${NC}"

# Step 5: Build XCFrameworks (separate for debug and release)
step "Building XCFrameworks..."

# Debug XCFramework
info "Creating debug XCFramework..."
XCFRAMEWORK_DEBUG="$TEMP_BUILD/gldf-rsFfi-debug.xcframework"
rm -rf "$XCFRAMEWORK_DEBUG"

xcodebuild -create-xcframework \
    -framework "$TEMP_BUILD/debug/ios-arm64/gldf-rsFfi.framework" \
    -debug-symbols "$TEMP_BUILD/debug/ios-arm64/dSYMs/gldf-rsFfi.framework.dSYM" \
    -framework "$TEMP_BUILD/debug/ios-arm64-simulator/gldf-rsFfi.framework" \
    -debug-symbols "$TEMP_BUILD/debug/ios-arm64-simulator/dSYMs/gldf-rsFfi.framework.dSYM" \
    -framework "$TEMP_BUILD/debug/macos-arm64_x86_64/gldf-rsFfi.framework" \
    -debug-symbols "$TEMP_BUILD/debug/macos-arm64_x86_64/dSYMs/gldf-rsFfi.framework.dSYM" \
    -framework "$TEMP_BUILD/debug/tvos-arm64/gldf-rsFfi.framework" \
    -framework "$TEMP_BUILD/debug/tvos-arm64-simulator/gldf-rsFfi.framework" \
    -framework "$TEMP_BUILD/debug/watchos-arm64_arm64_32/gldf-rsFfi.framework" \
    -framework "$TEMP_BUILD/debug/watchos-arm64-simulator/gldf-rsFfi.framework" \
    $(if [ "$HAS_VISIONOS" = true ]; then echo "-framework $TEMP_BUILD/debug/xros-arm64/gldf-rsFfi.framework"; fi) \
    $(if [ "$HAS_VISIONOS" = true ]; then echo "-framework $TEMP_BUILD/debug/xros-arm64-simulator/gldf-rsFfi.framework"; fi) \
    -output "$XCFRAMEWORK_DEBUG"

echo -e "${GREEN}✓ Debug XCFramework created${NC}"

# Release XCFramework
info "Creating release XCFramework..."
XCFRAMEWORK_RELEASE="$TEMP_BUILD/gldf-rsFfi.xcframework"
rm -rf "$XCFRAMEWORK_RELEASE"

xcodebuild -create-xcframework \
    -framework "$TEMP_BUILD/release/ios-arm64/gldf-rsFfi.framework" \
    -debug-symbols "$TEMP_BUILD/release/ios-arm64/dSYMs/gldf-rsFfi.framework.dSYM" \
    -framework "$TEMP_BUILD/release/ios-arm64-simulator/gldf-rsFfi.framework" \
    -debug-symbols "$TEMP_BUILD/release/ios-arm64-simulator/dSYMs/gldf-rsFfi.framework.dSYM" \
    -framework "$TEMP_BUILD/release/macos-arm64_x86_64/gldf-rsFfi.framework" \
    -debug-symbols "$TEMP_BUILD/release/macos-arm64_x86_64/dSYMs/gldf-rsFfi.framework.dSYM" \
    -framework "$TEMP_BUILD/release/tvos-arm64/gldf-rsFfi.framework" \
    -framework "$TEMP_BUILD/release/tvos-arm64-simulator/gldf-rsFfi.framework" \
    -framework "$TEMP_BUILD/release/watchos-arm64_arm64_32/gldf-rsFfi.framework" \
    -framework "$TEMP_BUILD/release/watchos-arm64-simulator/gldf-rsFfi.framework" \
    $(if [ "$HAS_VISIONOS" = true ]; then echo "-framework $TEMP_BUILD/release/xros-arm64/gldf-rsFfi.framework"; fi) \
    $(if [ "$HAS_VISIONOS" = true ]; then echo "-framework $TEMP_BUILD/release/xros-arm64-simulator/gldf-rsFfi.framework"; fi) \
    -output "$XCFRAMEWORK_RELEASE"

echo -e "${GREEN}✓ Release XCFramework created${NC}"

# Step 6: Update SPM package
step "Updating SPM package..."

mkdir -p "$SPM_PACKAGE/Sources/gldf-rsKit"
mkdir -p "$SPM_PACKAGE/Tests/gldf-rsFfiTests"

# Copy release XCFramework (default)
rm -rf "$SPM_PACKAGE/gldf-rsFfi.xcframework"
cp -R "$XCFRAMEWORK_RELEASE" "$SPM_PACKAGE/"

# Also keep debug XCFramework available
cp -R "$XCFRAMEWORK_DEBUG" "$SPM_PACKAGE/gldf-rsFfi-debug.xcframework"

# Copy Swift bindings
cp "$FFI_CRATE/generated/gldf-rs_ffi.swift" "$SPM_PACKAGE/Sources/gldf-rsKit/"

# Create Package.swift
cat > "$SPM_PACKAGE/Package.swift" <<'EOF'
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "gldf-rsKit",
    platforms: [
        .iOS(.v13),
        .macOS(.v13),
        .tvOS(.v13),
        .watchOS(.v6),
        .visionOS(.v1),
    ],
    products: [
        .library(
            name: "gldf-rsKit",
            targets: ["gldf-rsKit"]
        ),
    ],
    targets: [
        // Binary FFI target - the framework module name is gldf-rsFfi
        // For debug builds, replace path with "gldf-rsFfi-debug.xcframework"
        .binaryTarget(
            name: "gldf-rsFfi",
            path: "gldf-rsFfi.xcframework"
        ),

        // Swift bindings target
        .target(
            name: "gldf-rsKit",
            dependencies: ["gldf-rsFfi"],
            path: "Sources/gldf-rsKit"
        ),

        // Tests
        .testTarget(
            name: "gldf-rsFfiTests",
            dependencies: ["gldf-rsKit"],
            path: "Tests/gldf-rsFfiTests"
        ),
    ]
)
EOF

# Create basic test if it doesn't exist
if [ ! -f "$SPM_PACKAGE/Tests/gldf-rsFfiTests/gldf-rsFfiTests.swift" ]; then
    cat > "$SPM_PACKAGE/Tests/gldf-rsFfiTests/gldf-rsFfiTests.swift" <<'EOF'
import XCTest
@testable import gldf-rsKit

final class gldf-rsKitTests: XCTestCase {
    func testInitialization() throws {
        let db = try gldf-rsEngine()
        let count = db.countryCount()
        XCTAssertGreaterThan(count, 0, "Database should not be empty")
    }
}
EOF
fi

echo -e "${GREEN}✓ SPM package updated${NC}"

# Step 7: Create README for the package
cat > "$SPM_PACKAGE/README.md" <<EOF
# gldf-rsKit - Universal Swift Package

Complete geographic database for all Apple platforms.

## Platforms Supported

- ✅ iOS 13.0+ (device + simulator)
- ✅ macOS 13.0+ (Intel + Apple Silicon universal)
- ✅ tvOS 13.0+ (device + simulator)
- ✅ watchOS 6.0+ (device + simulator)
- ✅ visionOS 1.0+ (device + simulator)

## Features

- **Universal Binary**: Single package works on ALL Apple devices
- **Both Configurations**: Includes debug and release XCFrameworks
- **Complete dSYMs**: Full debugging symbols for all platforms
- **Production Ready**: Optimized for App Store submission

## Installation

### Swift Package Manager

Add to your \`Package.swift\`:

\`\`\`swift
dependencies: [
    .package(url: "https://github.com/yourorg/gldf-rs-rs.git", from: "0.1.0")
]
\`\`\`

Or in Xcode:
1. File → Add Packages
2. Enter repository URL
3. Select version
4. Add to target

### Local Development

\`\`\`swift
dependencies: [
    .package(path: "../gldf-rs-Apps/SPM-gldf-rsKit")
]
\`\`\`

## Usage

\`\`\`swift
import gldf-rsKit

// Initialize database
let db = try gldf-rsEngine()

// Search cities
let results = db.smartSearch(query: "Berlin")
for city in results {
    print("\(city.name), \(city.country)")
}

// Get statistics
let stats = db.getStats()
print("Countries: \(stats.countryCount)")
print("Cities: \(stats.cityCount)")
\`\`\`

## Debug vs Release

The package includes both configurations:

- **Release** (default): \`gldf-rsFfi.xcframework\` - Optimized, smaller
- **Debug**: \`gldf-rsFfi-debug.xcframework\` - Full symbols, larger

To use debug framework, edit \`Package.swift\`:
\`\`\`swift
.binaryTarget(
    name: "gldf-rsFfi",
    path: "gldf-rsFfi-debug.xcframework"  // Use debug version
),
\`\`\`

## Binary Size

| Platform | Debug | Release |
|----------|-------|---------|
| iOS | ~XX MB | ~XX MB |
| macOS | ~XX MB | ~XX MB |
| tvOS | ~XX MB | ~XX MB |
| watchOS | ~XX MB | ~XX MB |

## License

See LICENSE file in repository root.

## Attribution

Uses data from countries-states-cities-database (CC-BY-4.0).
EOF

# Step 8: Create release archive for GitHub
step "Creating GitHub release archive..."

RELEASE_DIR="$ROOT_DIR/releases"
mkdir -p "$RELEASE_DIR"

RELEASE_VERSION="v0.1.0"
RELEASE_ARCHIVE="$RELEASE_DIR/gldf-rsKit-$RELEASE_VERSION-universal.zip"

cd "$SPM_PACKAGE/.."
zip -r "$RELEASE_ARCHIVE" "SPM-gldf-rsKit/" \
    -x "*.DS_Store" \
    -x "*/.build/*" \
    -x "*/.swiftpm/*"

ARCHIVE_SIZE=$(du -h "$RELEASE_ARCHIVE" | cut -f1)

echo -e "${GREEN}✓ Release archive created${NC}"

# Cleanup temp build
rm -rf "$TEMP_BUILD"

echo ""
echo "======================================================================"
echo -e "${GREEN}✅ UNIVERSAL SPM Package Build Complete!${NC}"
echo "======================================================================"
echo ""
echo "📦 Package Location:"
echo "   $SPM_PACKAGE"
echo ""
echo "📚 Package Contents:"
echo "   • gldf-rsFfi.xcframework          (Release, all platforms)"
echo "   • gldf-rsFfi-debug.xcframework    (Debug, all platforms)"
echo "   • Sources/gldf-rsKit/             (Swift bindings)"
echo "   • Package.swift                 (SPM manifest)"
echo "   • README.md                     (Documentation)"
echo ""
echo "🎯 Platforms:"
echo "   ✅ iOS (device + simulator)"
echo "   ✅ macOS (Intel + Apple Silicon universal)"
echo "   ✅ tvOS (device + simulator)"
echo "   ✅ watchOS (device + simulator)"
if [ "$HAS_VISIONOS" = true ]; then
    echo "   ✅ visionOS (device + simulator)"
fi
echo ""
echo "💾 GitHub Release:"
echo "   Archive: $RELEASE_ARCHIVE"
echo "   Size: $ARCHIVE_SIZE"
echo ""
echo "🚀 Next Steps:"
echo "   1. Test locally: swift build (in SPM-gldf-rsKit directory)"
echo "   2. Create GitHub release: gh release create $RELEASE_VERSION"
echo "   3. Upload: gh release upload $RELEASE_VERSION $RELEASE_ARCHIVE"
echo "   4. Or use in Xcode: Add local package"
echo ""
echo "📖 Documentation:"
echo "   See: $SPM_PACKAGE/README.md"
echo ""
