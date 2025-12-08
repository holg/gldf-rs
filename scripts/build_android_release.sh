#!/usr/bin/env bash
#
# build_android_release.sh - Build Android APK/AAB for release
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ANDROID_APP="$ROOT_DIR/GeoDB-Apps/android-app"
FFI_CRATE="$ROOT_DIR/crates/gldf-rs-ffi"
RELEASES_DIR="$ROOT_DIR/releases/android"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "======================================================================"
echo "Build Android Release APK/AAB"
echo "======================================================================"
echo ""

# Check for cargo-ndk
if ! command -v cargo-ndk &> /dev/null; then
    echo -e "${RED}Error: cargo-ndk not found${NC}"
    echo "Install with: cargo install cargo-ndk"
    exit 1
fi

# Check for Android SDK
if [ -z "${ANDROID_SDK_ROOT:-}" ] && [ -z "${ANDROID_HOME:-}" ]; then
    echo -e "${YELLOW}Warning: ANDROID_SDK_ROOT or ANDROID_HOME not set${NC}"
    echo "Trying default location..."
    if [ -d "$HOME/Library/Android/sdk" ]; then
        export ANDROID_SDK_ROOT="$HOME/Library/Android/sdk"
        export ANDROID_HOME="$ANDROID_SDK_ROOT"
    elif [ -d "$HOME/Android/Sdk" ]; then
        export ANDROID_SDK_ROOT="$HOME/Android/Sdk"
        export ANDROID_HOME="$ANDROID_SDK_ROOT"
    else
        echo -e "${RED}Error: Android SDK not found${NC}"
        echo "Install Android Studio or set ANDROID_SDK_ROOT"
        exit 1
    fi
fi

# Set ANDROID_SDK_ROOT if only ANDROID_HOME is set
if [ -z "${ANDROID_SDK_ROOT:-}" ] && [ -n "${ANDROID_HOME:-}" ]; then
    export ANDROID_SDK_ROOT="$ANDROID_HOME"
fi

# Set ANDROID_HOME if only ANDROID_SDK_ROOT is set
if [ -z "${ANDROID_HOME:-}" ] && [ -n "${ANDROID_SDK_ROOT:-}" ]; then
    export ANDROID_HOME="$ANDROID_SDK_ROOT"
fi

echo "Android SDK: ${ANDROID_SDK_ROOT:-Not set}"
echo ""

# Step 1: Build Rust libraries for all Android ABIs
echo -e "${BLUE}Step 1/3:${NC} Building Rust native libraries..."
echo ""

cd "$FFI_CRATE"

TARGETS=(
    "aarch64-linux-android"   # arm64-v8a
    "armv7-linux-androideabi" # armeabi-v7a
    "x86_64-linux-android"    # x86_64
    "i686-linux-android"      # x86
)

for target in "${TARGETS[@]}"; do
    echo "Building for $target..."
    cargo ndk --target "$target" build --release
done

echo ""
echo -e "${GREEN}✓ Rust libraries built${NC}"
echo ""

# Step 2: Copy libraries to Android app
echo -e "${BLUE}Step 2/3:${NC} Copying native libraries to Android app..."
echo ""

JNI_LIBS="$ANDROID_APP/app/src/main/jniLibs"
rm -rf "$JNI_LIBS"
mkdir -p "$JNI_LIBS"/{arm64-v8a,armeabi-v7a,x86_64,x86}

# Copy libraries
cp "$ROOT_DIR/target/aarch64-linux-android/release/libgeodb_ffi.so" \
   "$JNI_LIBS/arm64-v8a/"
cp "$ROOT_DIR/target/armv7-linux-androideabi/release/libgeodb_ffi.so" \
   "$JNI_LIBS/armeabi-v7a/"
cp "$ROOT_DIR/target/x86_64-linux-android/release/libgeodb_ffi.so" \
   "$JNI_LIBS/x86_64/"
cp "$ROOT_DIR/target/i686-linux-android/release/libgeodb_ffi.so" \
   "$JNI_LIBS/x86/"

echo "Native libraries:"
du -sh "$JNI_LIBS"/*/*.so

echo ""
echo -e "${GREEN}✓ Libraries copied${NC}"
echo ""

# Step 3: Build Android APK/AAB
echo -e "${BLUE}Step 3/3:${NC} Building Android release..."
echo ""

cd "$ANDROID_APP"

# Check for signing configuration
if [ -f "app/release.keystore" ] || [ -n "${KEYSTORE_FILE:-}" ]; then
    echo "Building signed release..."
    BUILD_TYPE="bundleRelease"
    OUTPUT_TYPE="AAB"
else
    echo -e "${YELLOW}No keystore found, building unsigned APK${NC}"
    echo "For signed release, create keystore:"
    echo "  keytool -genkey -v -keystore app/release.keystore \\"
    echo "    -alias geodb -keyalg RSA -keysize 2048 -validity 10000"
    BUILD_TYPE="assembleRelease"
    OUTPUT_TYPE="APK"
fi

./gradlew clean
./gradlew "$BUILD_TYPE"

echo ""
echo -e "${GREEN}✓ Android build complete${NC}"
echo ""

# Copy outputs to releases directory
echo "Copying to releases directory..."
mkdir -p "$RELEASES_DIR"

if [ "$OUTPUT_TYPE" = "AAB" ]; then
    # Find and copy AAB
    find "$ANDROID_APP/app/build/outputs/bundle" -name "*.aab" -exec cp {} "$RELEASES_DIR/" \;

    # Also build universal APK from AAB if bundletool is available
    if command -v bundletool-all.jar &> /dev/null; then
        echo "Generating universal APK from AAB..."
        AAB_FILE=$(find "$RELEASES_DIR" -name "*.aab" | head -1)
        bundletool-all.jar build-apks \
            --bundle="$AAB_FILE" \
            --output="$RELEASES_DIR/geodb-universal.apks" \
            --mode=universal
    fi
else
    # Copy APKs
    find "$ANDROID_APP/app/build/outputs/apk" -name "*.apk" -exec cp {} "$RELEASES_DIR/" \;
fi

echo ""
echo "======================================================================"
echo -e "${GREEN}✅ Android Release Built Successfully!${NC}"
echo "======================================================================"
echo ""
echo "Output files in: $RELEASES_DIR"
ls -lh "$RELEASES_DIR"
echo ""

# Show APK details
if [ -f "$RELEASES_DIR/app-release.apk" ]; then
    echo "APK Info:"
    if command -v aapt &> /dev/null; then
        aapt dump badging "$RELEASES_DIR/app-release.apk" | grep -E "package|version|sdkVersion|native-code"
    fi
    echo ""
    echo "APK Size: $(du -h "$RELEASES_DIR/app-release.apk" | cut -f1)"
fi

echo ""
echo "Next steps:"
echo ""
if [ "$OUTPUT_TYPE" = "AAB" ]; then
    echo "1. Upload AAB to Google Play Console:"
    echo "   ${BLUE}$RELEASES_DIR/*.aab${NC}"
else
    echo "1. Sign the APK for release:"
    echo "   ${BLUE}jarsigner -verbose -sigalg SHA256withRSA -digestalg SHA-256 \\
     -keystore app/release.keystore \\
     $RELEASES_DIR/app-release.apk geodb${NC}"
    echo ""
    echo "2. Zipalign the signed APK:"
    echo "   ${BLUE}zipalign -v 4 app-release.apk app-release-aligned.apk${NC}"
fi
echo ""
