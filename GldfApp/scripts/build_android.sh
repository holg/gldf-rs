#!/usr/bin/env bash
set -euo pipefail

echo "======================================================================"
echo "Building GLDF Android Library"
echo "======================================================================"

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
FFI_CRATE="$ROOT_DIR/gldf-rs-ffi"
ANDROID_APP="$ROOT_DIR/GldfApp/android-app"
TARGET_DIR="$ROOT_DIR/target"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

step() {
    echo -e "\n${BLUE}==>${NC} ${GREEN}$1${NC}"
}

# Check for Android NDK
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
    echo -e "${YELLOW}Warning: ANDROID_NDK_HOME not set${NC}"
    echo "Please set ANDROID_NDK_HOME to your Android NDK installation path"
    echo "Example: export ANDROID_NDK_HOME=\$HOME/Library/Android/sdk/ndk/26.1.10909125"
    exit 1
fi

# Check for cargo-ndk
if ! command -v cargo-ndk &> /dev/null; then
    echo -e "${YELLOW}cargo-ndk not found. Installing...${NC}"
    cargo install cargo-ndk
fi

# Add Android targets
step "Adding Android targets..."
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android

# Build for Android targets
step "Building Rust library for Android targets..."
cd "$FFI_CRATE"

echo "  • Building for arm64-v8a (aarch64-linux-android)..."
cargo ndk -t arm64-v8a build --release

echo "  • Building for armeabi-v7a (armv7-linux-androideabi)..."
cargo ndk -t armeabi-v7a build --release

echo "  • Building for x86_64 (x86_64-linux-android)..."
cargo ndk -t x86_64 build --release

echo "  • Building for x86 (i686-linux-android)..."
cargo ndk -t x86 build --release

echo -e "${GREEN}  ✓ All Android targets built${NC}"

# Generate Kotlin bindings
step "Generating Kotlin bindings..."
cd "$FFI_CRATE"

if [ ! -d "generated" ]; then
    mkdir -p generated
fi

# First build for host to generate bindings
cargo build --release

cargo run --bin uniffi-bindgen generate \
    --library "$TARGET_DIR/release/libgldf_ffi.dylib" \
    --language kotlin \
    --out-dir generated

if [ ! -f "generated/uniffi/gldf_ffi/gldf_ffi.kt" ]; then
    echo -e "${YELLOW}  Error: Kotlin bindings generation failed${NC}"
    exit 1
fi

echo -e "${GREEN}  ✓ Kotlin bindings generated${NC}"

# Copy libraries to Android jniLibs
step "Copying native libraries to Android project..."

JNILIBS="$ANDROID_APP/app/src/main/jniLibs"
rm -rf "$JNILIBS"
mkdir -p "$JNILIBS/arm64-v8a"
mkdir -p "$JNILIBS/armeabi-v7a"
mkdir -p "$JNILIBS/x86_64"
mkdir -p "$JNILIBS/x86"

cp "$TARGET_DIR/aarch64-linux-android/release/libgldf_ffi.so" "$JNILIBS/arm64-v8a/"
cp "$TARGET_DIR/armv7-linux-androideabi/release/libgldf_ffi.so" "$JNILIBS/armeabi-v7a/"
cp "$TARGET_DIR/x86_64-linux-android/release/libgldf_ffi.so" "$JNILIBS/x86_64/"
cp "$TARGET_DIR/i686-linux-android/release/libgldf_ffi.so" "$JNILIBS/x86/"

echo -e "${GREEN}  ✓ Native libraries copied${NC}"

# Copy Kotlin bindings
step "Copying Kotlin bindings..."
KOTLIN_DIR="$ANDROID_APP/app/src/main/java/uniffi/gldf_ffi"
mkdir -p "$KOTLIN_DIR"
cp "$FFI_CRATE/generated/uniffi/gldf_ffi/gldf_ffi.kt" "$KOTLIN_DIR/"

echo -e "${GREEN}  ✓ Kotlin bindings copied${NC}"

echo ""
echo "======================================================================"
echo -e "${GREEN}✅ Android Build Complete!${NC}"
echo "======================================================================"
echo ""
echo "📦 Native Libraries:"
echo "   $JNILIBS/arm64-v8a/libgldf_ffi.so"
echo "   $JNILIBS/armeabi-v7a/libgldf_ffi.so"
echo "   $JNILIBS/x86_64/libgldf_ffi.so"
echo "   $JNILIBS/x86/libgldf_ffi.so"
echo ""
echo "📚 Kotlin Bindings:"
echo "   $KOTLIN_DIR/gldf_ffi.kt"
echo ""
echo "🚀 Build APK:"
echo "   cd $ANDROID_APP"
echo "   ./gradlew assembleDebug"
echo ""
