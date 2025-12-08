#!/usr/bin/env bash
#
# create_manual_release.sh - Create GitHub release ZIP from local build
#
# Use this to manually create a release when CI can't build (e.g., nightly unavailable)
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SPM_DIR="$ROOT_DIR/gldf-rs-Apps/SPM-gldf-rsKit"
RELEASES_DIR="$ROOT_DIR/releases"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

if [ $# -lt 1 ]; then
    echo -e "${YELLOW}Usage: $0 VERSION${NC}"
    echo ""
    echo "Example:"
    echo "  $0 v0.1.4"
    echo ""
    exit 1
fi

VERSION="$1"

echo "======================================================================"
echo "Create Manual Release for $VERSION"
echo "======================================================================"
echo ""

# Verify XCFrameworks exist
if [ ! -d "$SPM_DIR/gldf-rsFfi.xcframework" ]; then
    echo -e "${YELLOW}Error: gldf-rsFfi.xcframework not found${NC}"
    echo "Run: ./scripts/build_spm_universal.sh"
    exit 1
fi

# Create releases directory
mkdir -p "$RELEASES_DIR"

# Package name
RELEASE_ZIP="$RELEASES_DIR/gldf-rsKit-$VERSION-universal.zip"

echo "Creating release package..."
echo "  Source: $SPM_DIR"
echo "  Output: $RELEASE_ZIP"
echo ""

# Create temporary staging area
TEMP_STAGE="/tmp/gldf-rskit-release-$$"
rm -rf "$TEMP_STAGE"
mkdir -p "$TEMP_STAGE/gldf-rsKit"

# Copy essential files (RELEASE ONLY - no debug)
echo "Copying files..."
cp -R "$SPM_DIR/gldf-rsFfi.xcframework" "$TEMP_STAGE/gldf-rsKit/"
cp -R "$SPM_DIR/Sources" "$TEMP_STAGE/gldf-rsKit/"
cp "$SPM_DIR/Package.swift" "$TEMP_STAGE/gldf-rsKit/"
cp "$SPM_DIR/README.md" "$TEMP_STAGE/gldf-rsKit/"

echo "Note: Debug XCFramework NOT included (saves 1.1GB)"

echo "Creating ZIP..."
cd "$TEMP_STAGE"
zip -r "$RELEASE_ZIP" gldf-rsKit -q

cd "$ROOT_DIR"
rm -rf "$TEMP_STAGE"

# Calculate checksum
echo ""
echo "Calculating checksum..."
cd "$RELEASES_DIR"
shasum -a 256 "$(basename "$RELEASE_ZIP")" > checksums.txt
CHECKSUM=$(cat checksums.txt)

echo ""
echo "======================================================================"
echo -e "${GREEN}✅ Release Package Created${NC}"
echo "======================================================================"
echo ""
echo "Package: $RELEASE_ZIP"
echo "Size:    $(du -h "$RELEASE_ZIP" | cut -f1)"
echo ""
echo "Checksum:"
echo "$CHECKSUM"
echo ""
echo "Next steps:"
echo ""
echo "1. Create GitHub release:"
echo "   ${BLUE}gh release create $VERSION \\
     --title \"gldf-rsKit $VERSION\" \\
     --notes \"Universal SPM package for all Apple platforms\"${NC}"
echo ""
echo "2. Upload the release ZIP:"
echo "   ${BLUE}gh release upload $VERSION $RELEASE_ZIP $RELEASES_DIR/checksums.txt${NC}"
echo ""
echo "3. Update Package.swift:"
echo "   ${BLUE}./scripts/update_package_swift_for_release.sh $VERSION <checksum>${NC}"
echo ""
echo "Or use the all-in-one command:"
echo ""
echo "  ${BLUE}./scripts/publish_manual_release.sh $VERSION${NC}"
echo ""
