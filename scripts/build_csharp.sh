#!/usr/bin/env bash
#
# build_csharp.sh - Build C# / .NET bindings for gldf-rs
#
# Usage: ./build_csharp.sh [dev|release]
#
# Prerequisites:
#   - Rust toolchain
#   - uniffi-bindgen-cs (cargo install uniffi-bindgen-cs --git https://github.com/NordSecurity/uniffi-bindgen-cs --tag v0.9.0+v0.28.3)
#   - .NET 8 SDK (brew install dotnet@8)
#
# Output: crates/gldf-rs-csharp/bin/Release/net8.0/Gldf.Net.dll
#

set -euo pipefail

# Parse arguments
BUILD_MODE="${1:-release}"
if [[ "$BUILD_MODE" != "dev" && "$BUILD_MODE" != "release" ]]; then
    echo "Error: Build mode must be 'dev' or 'release'"
    echo "Usage: $0 [dev|release]"
    exit 1
fi

if [[ "$BUILD_MODE" == "dev" ]]; then
    CARGO_PROFILE="debug"
    CARGO_FLAG=""
    DOTNET_CONFIG="Debug"
else
    CARGO_PROFILE="release"
    CARGO_FLAG="--release"
    DOTNET_CONFIG="Release"
fi

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
FFI_CRATE="$ROOT_DIR/crates/gldf-rs-ffi"
CSHARP_DIR="$ROOT_DIR/crates/gldf-rs-csharp"
TARGET_DIR="$ROOT_DIR/target"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

step() {
    echo -e "\n${BLUE}==>${NC} ${GREEN}$1${NC}"
}

echo "======================================================================"
echo "Building C# / .NET Bindings (mode: $BUILD_MODE)"
echo "======================================================================"

# Step 1: Build Rust FFI library
step "Building Rust FFI library..."
cd "$ROOT_DIR"
cargo build $CARGO_FLAG -p gldf-rs-ffi

# Determine library filename based on OS
case "$(uname -s)" in
    Darwin)
        LIB_EXT="dylib"
        LIB_PREFIX="lib"
        RUNTIME_DIR="osx-arm64"
        ;;
    Linux)
        LIB_EXT="so"
        LIB_PREFIX="lib"
        RUNTIME_DIR="linux-x64"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        LIB_EXT="dll"
        LIB_PREFIX=""
        RUNTIME_DIR="win-x64"
        ;;
    *)
        echo "Unsupported OS: $(uname -s)"
        exit 1
        ;;
esac

LIB_FILE="$TARGET_DIR/$CARGO_PROFILE/${LIB_PREFIX}gldf_ffi.$LIB_EXT"

if [ ! -f "$LIB_FILE" ]; then
    echo "Error: Library not found at $LIB_FILE"
    exit 1
fi
echo "Library: $LIB_FILE"

# Step 2: Generate C# bindings
step "Generating C# bindings..."
mkdir -p "$CSHARP_DIR/src/Generated"

if ! command -v uniffi-bindgen-cs &>/dev/null; then
    echo "uniffi-bindgen-cs not found. Install with:"
    echo "  cargo install uniffi-bindgen-cs --git https://github.com/NordSecurity/uniffi-bindgen-cs --tag v0.9.0+v0.28.3"
    exit 1
fi

uniffi-bindgen-cs \
    --library \
    --no-format \
    --out-dir "$CSHARP_DIR/src/Generated/" \
    "$LIB_FILE"

# Make public types accessible (generator defaults to internal)
sed -i '' \
    -e 's/^internal record /public record /g' \
    -e 's/^internal class GldfEngine/public class GldfEngine/g' \
    -e 's/^internal interface IGldfEngine/public interface IGldfEngine/g' \
    -e 's/^internal static class GldfFfiMethods/public static class GldfFfiMethods/g' \
    -e 's/^internal class GldfException/public class GldfException/g' \
    -e 's/^internal class UniffiException/public class UniffiException/g' \
    "$CSHARP_DIR/src/Generated/gldf_ffi.cs"

echo "Generated: $CSHARP_DIR/src/Generated/gldf_ffi.cs"

# Step 3: Copy native library to runtimes
step "Copying native library to runtimes..."
mkdir -p "$CSHARP_DIR/runtimes/$RUNTIME_DIR/native"
cp "$LIB_FILE" "$CSHARP_DIR/runtimes/$RUNTIME_DIR/native/"
echo "Copied to: runtimes/$RUNTIME_DIR/native/"

# Step 4: Build .NET project
step "Building .NET project..."
if ! command -v dotnet &>/dev/null; then
    # Try homebrew path
    export PATH="/opt/homebrew/opt/dotnet@8/bin:$PATH"
fi

if ! command -v dotnet &>/dev/null; then
    echo "dotnet not found. Install with: brew install dotnet@8"
    exit 1
fi

cd "$CSHARP_DIR"
dotnet build -c "$DOTNET_CONFIG"

# Step 5: Create NuGet package (release only)
if [[ "$BUILD_MODE" == "release" ]]; then
    step "Creating NuGet package..."
    dotnet pack -c Release --no-build
fi

echo ""
echo "======================================================================"
echo -e "${GREEN}C# / .NET Build Complete!${NC}"
echo "======================================================================"
echo ""
echo "Build Mode: $BUILD_MODE"
echo "Library: $LIB_FILE"
echo "Assembly: $CSHARP_DIR/bin/$DOTNET_CONFIG/net8.0/Gldf.Net.dll"
echo ""
echo "Usage in C#:"
echo '  using Gldf.Net;'
echo '  var doc = GldfDocument.FromBytes(File.ReadAllBytes("luminaire.gldf"));'
echo '  Console.WriteLine(doc.Header.manufacturer);'
echo '  var ifc = doc.ExportToIfc();'
echo ""
