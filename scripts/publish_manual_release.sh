  #!/usr/bin/env bash
#
# publish_manual_release.sh - Complete manual release workflow
#
# Creates package, uploads to GitHub, updates Package.swift
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

if [ $# -lt 1 ]; then
    echo -e "${RED}Error: Version required${NC}"
    echo ""
    echo "Usage: $0 VERSION"
    echo ""
    echo "Example:"
    echo "  $0 v0.1.4"
    echo ""
    exit 1
fi

VERSION="$1"

echo "======================================================================"
echo "Complete Manual Release for $VERSION"
echo "======================================================================"
echo ""

# Step 1: Create release package
echo -e "${BLUE}Step 1/4:${NC} Creating release package..."
"$SCRIPT_DIR/create_manual_release.sh" "$VERSION"

RELEASE_ZIP="$ROOT_DIR/releases/gldf-rsKit-$VERSION-universal.zip"
CHECKSUMS="$ROOT_DIR/releases/checksums.txt"

# Step 2: Create GitHub release
echo ""
echo -e "${BLUE}Step 2/4:${NC} Creating GitHub release..."
if gh release view "$VERSION" &>/dev/null; then
    echo -e "${YELLOW}Release $VERSION already exists, skipping creation${NC}"
else
    gh release create "$VERSION" \
        --title "gldf-rsKit $VERSION" \
        --notes "Universal SPM package for all Apple platforms

## 📦 What's Included

- ✅ iOS 13.0+ (device + simulator)
- ✅ macOS 13.0+ (Intel x86_64 + Apple Silicon arm64 universal)
- ✅ tvOS 13.0+ (device + simulator)
- ✅ watchOS 6.0+ (device + simulator)

## 🎯 Configurations

- Release XCFramework (optimized, smaller)
- Debug XCFramework (full symbols, debugging)
- Complete dSYMs (all platforms)

## 📥 Installation

### Swift Package Manager

Add to your \`Package.swift\`:

\`\`\`swift
dependencies: [
    .package(url: \"https://github.com/YOUR_ORG/gldf-rs-rs.git\", from: \"${VERSION#v}\")
]
\`\`\`

Or in Xcode:
1. File → Add Packages
2. Enter: \`https://github.com/YOUR_ORG/gldf-rs-rs.git\`
3. Select version: \`${VERSION#v}\`

### 📖 Documentation

See [README](https://github.com/YOUR_ORG/gldf-rs-rs/blob/main/gldf-rs-Apps/SPM-gldf-rsKit/README.md) for usage examples.

### 🔐 Checksums

See \`checksums.txt\` for SHA-256 verification."

    echo -e "${GREEN}✓ Release created${NC}"
fi

# Step 3: Upload files
echo ""
echo -e "${BLUE}Step 3/4:${NC} Uploading release files..."
gh release upload "$VERSION" "$RELEASE_ZIP" "$CHECKSUMS" --clobber
echo -e "${GREEN}✓ Files uploaded${NC}"

# Step 4: Update Package.swift
echo ""
echo -e "${BLUE}Step 4/4:${NC} Updating Package.swift..."
CHECKSUM=$(grep "$(basename "$RELEASE_ZIP")" "$CHECKSUMS" | awk '{print $1}')

if [ -z "$CHECKSUM" ]; then
    echo -e "${RED}Error: Could not extract checksum${NC}"
    exit 1
fi

"$SCRIPT_DIR/update_package_swift_for_release.sh" "$VERSION" "$CHECKSUM"
echo -e "${GREEN}✓ Package.swift updated${NC}"

echo ""
echo "======================================================================"
echo -e "${GREEN}✅ Release $VERSION Published Successfully!${NC}"
echo "======================================================================"
echo ""
echo "View release:"
echo "  ${BLUE}https://github.com/YOUR_ORG/gldf-rs-rs/releases/tag/$VERSION${NC}"
echo ""
echo "Checksum: $CHECKSUM"
echo ""
echo "Next: Commit Package.swift update"
echo "  ${BLUE}git add gldf-rs-Apps/SPM-gldf-rsKit/Package.swift${NC}"
echo "  ${BLUE}git commit -m \"Update Package.swift for $VERSION release\"${NC}"
echo "  ${BLUE}git push origin main${NC}"
echo ""
