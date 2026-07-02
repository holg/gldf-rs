#!/bin/bash
# Cross-build the plugin's Rust staticlibs from macOS and refresh
# ThirdParty/{GldfUnreal,GldfSearch}/lib/{Mac,Linux,Win64}.
#
# Targets:
#   Mac    — native build (aarch64; pass --mac-universal yourself if needed)
#   Linux  — x86_64-unknown-linux-gnu via the messense brew toolchain
#            (brew tap messense/macos-cross-toolchains && brew install x86_64-unknown-linux-gnu)
#   Win64  — x86_64-pc-windows-msvc via clang-cl + rustup's rust-lld and an
#            xwin-style sysroot at ~/opt/x86_64-windows (crt/ + sdk/).
#            A staticlib needs no Windows linker, so this works from macOS;
#            only the crates' BINS link (via the rust-lld shim below).
#
# Gotchas encoded here (cost a debugging session):
#   - do NOT put /opt/homebrew/opt/llvm/lib in DYLD_LIBRARY_PATH — rustc then
#     loads brew's libLLVM instead of its own and crashes (SIGSEGV / fake
#     "stack overflow");
#   - brew's lld-link can't load its zstd/LLVM dylibs when invoked by cargo —
#     use rustup's bundled rust-lld with `-flavor link` instead, exported
#     with the toolchain's own lib dir on DYLD_LIBRARY_PATH (the shim).

set -euo pipefail

GLDF_RS="$(cd "$(dirname "$0")/../.." && pwd)"
GLDF_SEARCH="${GLDF_SEARCH_WORKSPACE:-$GLDF_RS/../gldf-search}"
THIRD_PARTY="$GLDF_RS/extra/unreal-plugin-interchange/GldfImporter/ThirdParty"
SYSROOT="$HOME/opt/x86_64-windows"
TOOLCHAIN="$(rustc --print sysroot)"

# ── rust-lld shim (lld-link flavor) ─────────────────────────────────────
SHIM="$GLDF_RS/tmp/cross/lld-link-shim.sh"
mkdir -p "$(dirname "$SHIM")"
cat > "$SHIM" <<EOF
#!/bin/bash
export DYLD_LIBRARY_PATH="$TOOLCHAIN/lib\${DYLD_LIBRARY_PATH:+:\$DYLD_LIBRARY_PATH}"
exec "$TOOLCHAIN/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/rust-lld" -flavor link "\$@"
EOF
chmod +x "$SHIM"

export SCCACHE_DISABLE=1 RUSTC_WRAPPER=""

build_linux() { # $1 = workspace dir, $2 = crate
  (cd "$1" && \
    CC_x86_64_unknown_linux_gnu=x86_64-unknown-linux-gnu-gcc \
    AR_x86_64_unknown_linux_gnu=x86_64-unknown-linux-gnu-ar \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc \
    cargo build -p "$2" --release --target x86_64-unknown-linux-gnu)
}

build_win64() { # $1 = workspace dir, $2 = crate
  (cd "$1" && \
    PATH="/opt/homebrew/opt/llvm/bin:$PATH" \
    CC_x86_64_pc_windows_msvc=clang-cl \
    AR_x86_64_pc_windows_msvc=llvm-lib \
    CFLAGS_x86_64_pc_windows_msvc="--target=x86_64-pc-windows-msvc -I$SYSROOT/crt/include -I$SYSROOT/sdk/include/ucrt -I$SYSROOT/sdk/include/um -I$SYSROOT/sdk/include/shared" \
    CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="$SHIM" \
    CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="-C link-arg=/LIBPATH:$SYSROOT/crt/lib/x64 -C link-arg=/LIBPATH:$SYSROOT/sdk/lib/um/x64 -C link-arg=/LIBPATH:$SYSROOT/sdk/lib/ucrt/x64" \
    cargo build -p "$2" --release --target x86_64-pc-windows-msvc)
}

build_mac() { # $1 = workspace dir, $2 = crate
  (cd "$1" && cargo build -p "$2" --release)
}

echo "=== gldf-unreal (gldf-rs workspace) ==="
build_mac   "$GLDF_RS" gldf-unreal
build_linux "$GLDF_RS" gldf-unreal
build_win64 "$GLDF_RS" gldf-unreal

echo "=== gldf-search-cabi (gldf-search workspace) ==="
build_mac   "$GLDF_SEARCH" gldf-search-cabi
build_linux "$GLDF_SEARCH" gldf-search-cabi
build_win64 "$GLDF_SEARCH" gldf-search-cabi

echo "=== copy into ThirdParty ==="
cp "$GLDF_RS/target/release/libgldf_unreal.a"                                "$THIRD_PARTY/GldfUnreal/lib/Mac/"
cp "$GLDF_RS/target/x86_64-unknown-linux-gnu/release/libgldf_unreal.a"       "$THIRD_PARTY/GldfUnreal/lib/Linux/"
cp "$GLDF_RS/target/x86_64-pc-windows-msvc/release/gldf_unreal.lib"          "$THIRD_PARTY/GldfUnreal/lib/Win64/"
cp "$GLDF_SEARCH/target/release/libgldf_search_cabi.a"                       "$THIRD_PARTY/GldfSearch/lib/Mac/"
cp "$GLDF_SEARCH/target/x86_64-unknown-linux-gnu/release/libgldf_search_cabi.a" "$THIRD_PARTY/GldfSearch/lib/Linux/"
cp "$GLDF_SEARCH/target/x86_64-pc-windows-msvc/release/gldf_search_cabi.lib" "$THIRD_PARTY/GldfSearch/lib/Win64/"

echo "Done:"
ls -la "$THIRD_PARTY"/Gldf*/lib/*/
