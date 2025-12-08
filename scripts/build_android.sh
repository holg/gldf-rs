#!/usr/bin/env bash
#
# build_android.sh - Build Android application
#
# Usage: ./build_android.sh [dev|release]
#
# Arguments:
#   dev     - Development build (debug APK)
#   release - Release build (signed APK/AAB for Play Store)
#
# Output: GeoDB-Apps/android-app/app/build/outputs/
#

set -euo pipefail

# Parse arguments
BUILD_MODE="${1:-release}"
if [[ "$BUILD_MODE" != "dev" && "$BUILD_MODE" != "release" ]]; then
    echo "❌ Error: Build mode must be 'dev' or 'release'"
    echo "Usage: $0 [dev|release]"
    exit 1
fi

# Convert to Gradle task
if [[ "$BUILD_MODE" == "dev" ]]; then
    GRADLE_TASK="assembleDebug"
    BUILD_TYPE="Debug"
else
    GRADLE_TASK="assembleRelease"
    BUILD_TYPE="Release"
fi

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ANDROID_PROJECT="$ROOT_DIR/GeoDB-Apps/android-app"
FFI_CRATE="$ROOT_DIR/crates/geodb-ffi"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "======================================================================"
echo "Building Android App (mode: $BUILD_MODE)"
echo "======================================================================"

# Step 1: Build Rust for Android targets
echo -e "\n${BLUE}==>${NC} ${GREEN}Building Rust for Android...${NC}"
cd "$FFI_CRATE"

# Cargo profile
if [[ "$BUILD_MODE" == "dev" ]]; then
    CARGO_FLAG=""
else
    CARGO_FLAG="--release"
fi

echo "  • Building for arm64-v8a (aarch64)..."
cargo ndk --target aarch64-linux-android --platform 21 build $CARGO_FLAG

echo "  • Building for armeabi-v7a (armv7)..."
cargo ndk --target armv7-linux-androideabi --platform 21 build $CARGO_FLAG

echo "  • Building for x86_64 (emulator)..."
cargo ndk --target x86_64-linux-android --platform 21 build $CARGO_FLAG

echo "  • Building for x86 (emulator, legacy)..."
cargo ndk --target i686-linux-android --platform 21 build $CARGO_FLAG

echo -e "${GREEN}✓ Rust libraries built for all Android ABIs${NC}"

# Step 2: Copy native libraries to Android project
echo -e "\n${BLUE}==>${NC} ${GREEN}Copying native libraries...${NC}"

TARGET_DIR="$ROOT_DIR/target"
JNI_LIBS="$ANDROID_PROJECT/app/src/main/jniLibs"

# Determine Cargo profile directory
if [[ "$BUILD_MODE" == "dev" ]]; then
    PROFILE_DIR="debug"
else
    PROFILE_DIR="release"
fi

mkdir -p "$JNI_LIBS"/{arm64-v8a,armeabi-v7a,x86_64,x86}

cp "$TARGET_DIR/aarch64-linux-android/$PROFILE_DIR/libgeodb_ffi.so" "$JNI_LIBS/arm64-v8a/"
cp "$TARGET_DIR/armv7-linux-androideabi/$PROFILE_DIR/libgeodb_ffi.so" "$JNI_LIBS/armeabi-v7a/"
cp "$TARGET_DIR/x86_64-linux-android/$PROFILE_DIR/libgeodb_ffi.so" "$JNI_LIBS/x86_64/"
cp "$TARGET_DIR/i686-linux-android/$PROFILE_DIR/libgeodb_ffi.so" "$JNI_LIBS/x86/"

echo -e "${GREEN}✓ Native libraries copied to jniLibs${NC}"

# Step 3: Build Android APK/AAB
echo -e "\n${BLUE}==>${NC} ${GREEN}Building Android $BUILD_TYPE APK...${NC}"
cd "$ANDROID_PROJECT"

# Ensure gradlew is executable
chmod +x ./gradlew

./gradlew clean
./gradlew $GRADLE_TASK

echo ""
echo "======================================================================"
echo -e "${GREEN}✅ Android Build Complete!${NC}"
echo "======================================================================"
echo ""
echo "Build Type: $BUILD_TYPE"
echo "Output Location:"
if [[ "$BUILD_MODE" == "dev" ]]; then
    APK_PATH="$ANDROID_PROJECT/app/build/outputs/apk/debug/app-debug.apk"
    echo "  $APK_PATH"
    echo ""
    echo "Install on device/emulator:"
    echo "  adb install $APK_PATH"
    echo ""
    echo "Or run directly:"
    echo "  ./gradlew installDebug"
else
    APK_PATH="$ANDROID_PROJECT/app/build/outputs/apk/release/app-release.apk"
    AAB_PATH="$ANDROID_PROJECT/app/build/outputs/bundle/release/app-release.aab"
    echo "  APK: $APK_PATH"
    echo "  AAB: $AAB_PATH"
    echo ""
    echo -e "${YELLOW}Note: Release builds require signing!${NC}"
    echo ""
    echo "To sign and align:"
    echo "  1. Configure signing in app/build.gradle.kts"
    echo "  2. Or use: ./gradlew bundleRelease"
    echo "  3. Upload AAB to Play Console"
fi
echo ""
