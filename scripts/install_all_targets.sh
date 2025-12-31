#!/usr/bin/env bash
#
# install_all_targets.sh - Install ALL required Rust targets for universal build
#
# This installs targets for:
#   - iOS, macOS, tvOS, watchOS, visionOS
#   - Both stable and nightly toolchains
#

set -euo pipefail

echo "======================================================================"
echo "Installing ALL Rust Targets for Universal SPM Build"
echo "======================================================================"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

step() {
    echo -e "\n${BLUE}==>${NC} ${GREEN}$1${NC}"
}

# Ensure nightly is installed (use known-good version with all targets)
step "Installing nightly toolchain..."
NIGHTLY_VERSION="nightly-2024-08-01"
echo "  Using $NIGHTLY_VERSION (known to have all Apple targets)"
rustup install $NIGHTLY_VERSION
echo -e "${GREEN}✓ Nightly $NIGHTLY_VERSION installed${NC}"

# Stable targets
step "Installing stable targets..."

echo "  • macOS targets..."
rustup target add aarch64-apple-darwin    # Apple Silicon
rustup target add x86_64-apple-darwin     # Intel

echo "  • iOS targets..."
rustup target add aarch64-apple-ios       # iPhone/iPad device
rustup target add aarch64-apple-ios-sim   # iOS Simulator (Apple Silicon Mac)

echo -e "${GREEN}✓ Stable targets installed${NC}"

# Nightly targets
step "Installing nightly targets..."

echo "  • tvOS targets..."
rustup target add --toolchain $NIGHTLY_VERSION aarch64-apple-tvos       # Apple TV device
rustup target add --toolchain $NIGHTLY_VERSION aarch64-apple-tvos-sim   # tvOS Simulator

echo "  • watchOS targets..."
rustup target add --toolchain $NIGHTLY_VERSION aarch64-apple-watchos      # Apple Watch (64-bit)
rustup target add --toolchain $NIGHTLY_VERSION arm64_32-apple-watchos     # Apple Watch (32-bit)
rustup target add --toolchain $NIGHTLY_VERSION aarch64-apple-watchos-sim  # watchOS Simulator

echo "  • visionOS targets (optional)..."
rustup target add --toolchain $NIGHTLY_VERSION aarch64-apple-visionos || echo -e "${YELLOW}  visionOS device target not available yet${NC}"
rustup target add --toolchain $NIGHTLY_VERSION aarch64-apple-visionos-sim || echo -e "${YELLOW}  visionOS simulator target not available yet${NC}"

echo -e "${GREEN}✓ Nightly targets installed${NC}"

# Verification
step "Verifying installation..."

echo ""
echo "Stable targets:"
rustup target list --installed | grep apple-darwin
rustup target list --installed | grep apple-ios

echo ""
echo "Nightly targets ($NIGHTLY_VERSION):"
rustup target list --toolchain $NIGHTLY_VERSION --installed | grep apple-tvos
rustup target list --toolchain $NIGHTLY_VERSION --installed | grep apple-watchos
rustup target list --toolchain $NIGHTLY_VERSION --installed | grep apple-visionos || echo "(visionOS not available)"

echo ""
echo "======================================================================"
echo -e "${GREEN}✅ All Targets Installed!${NC}"
echo "======================================================================"
echo ""
echo "Ready to build universal SPM package:"
echo "  ./scripts/build_spm_universal.sh"
echo ""
