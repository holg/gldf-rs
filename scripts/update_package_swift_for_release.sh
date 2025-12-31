#!/usr/bin/env bash
#
# update_package_swift_for_release.sh - Update Package.swift to use GitHub release binaries
#
# Usage:
#   ./scripts/update_package_swift_for_release.sh v0.1.0 <checksum>
#
# After creating a GitHub release, run this to update Package.swift to reference
# the pre-built binaries instead of local XCFrameworks.
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PACKAGE_SWIFT="$ROOT_DIR/gldf-rs-Apps/SPM-gldf-rsKit/Package.swift"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

if [ $# -lt 1 ]; then
    echo -e "${RED}Error: Missing version argument${NC}"
    echo ""
    echo "Usage: $0 VERSION [CHECKSUM]"
    echo ""
    echo "Examples:"
    echo "  $0 v0.1.0"
    echo "  $0 v0.1.0 abc123def456..."
    echo ""
    echo "If checksum is not provided, the script will show you where to get it."
    exit 1
fi

VERSION="$1"
CHECKSUM="${2:-}"

# Determine GitHub repo URL from git remote
REPO_URL=$(git config --get remote.origin.url | sed 's/\.git$//' | sed 's|^git@github.com:|https://github.com/|')

if [ -z "$REPO_URL" ]; then
    echo -e "${RED}Error: Could not determine GitHub repository URL${NC}"
    echo "Make sure you're in a git repository with a GitHub remote."
    exit 1
fi

echo "======================================================================"
echo "Update Package.swift for GitHub Release"
echo "======================================================================"
echo ""
echo "Version:  $VERSION"
echo "Repo:     $REPO_URL"
echo ""

# If checksum not provided, help user get it
if [ -z "$CHECKSUM" ]; then
    echo -e "${YELLOW}Checksum not provided.${NC}"
    echo ""
    echo "Get the checksum from the GitHub release:"
    echo ""
    echo -e "${BLUE}$REPO_URL/releases/download/$VERSION/checksums.txt${NC}"
    echo ""
    echo "Or run:"
    echo ""
    echo -e "${BLUE}curl -L $REPO_URL/releases/download/$VERSION/checksums.txt${NC}"
    echo ""
    echo "Then run this script again with the checksum:"
    echo ""
    echo -e "${BLUE}$0 $VERSION <checksum>${NC}"
    echo ""
    exit 1
fi

# Backup current Package.swift
cp "$PACKAGE_SWIFT" "$PACKAGE_SWIFT.backup"
echo -e "${GREEN}✓${NC} Backed up Package.swift"

# Build the new binary target configuration
RELEASE_URL="$REPO_URL/releases/download/$VERSION/gldf-rsKit-$VERSION-universal.zip"

echo ""
echo "Creating new Package.swift with:"
echo "  URL:      $RELEASE_URL"
echo "  Checksum: ${CHECKSUM:0:16}..."
echo ""

# Create new Package.swift
cat > "$PACKAGE_SWIFT" <<EOF
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
        // Binary FFI target from GitHub release
        .binaryTarget(
            name: "gldf-rsFfi",
            url: "$RELEASE_URL",
            checksum: "$CHECKSUM"
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

echo -e "${GREEN}✓${NC} Package.swift updated"
echo ""

# Show the diff
echo "Changes:"
echo "--------"
diff "$PACKAGE_SWIFT.backup" "$PACKAGE_SWIFT" || true
echo ""

echo "======================================================================"
echo -e "${GREEN}✅ Package.swift Updated for Release $VERSION${NC}"
echo "======================================================================"
echo ""
echo "Next steps:"
echo ""
echo "1. Review the changes:"
echo "   cat $PACKAGE_SWIFT"
echo ""
echo "2. Test that it works:"
echo "   cd gldf-rs-Apps/SPM-gldf-rsKit"
echo "   swift package resolve"
echo "   swift build"
echo ""
echo "3. Commit the changes:"
echo "   git add gldf-rs-Apps/SPM-gldf-rsKit/Package.swift"
echo "   git commit -m \"Update Package.swift for $VERSION release\""
echo "   git push origin main"
echo ""
echo "4. Tag the Package.swift update (optional):"
echo "   git tag ${VERSION}-pkg"
echo "   git push origin ${VERSION}-pkg"
echo ""
echo "Backup saved at: $PACKAGE_SWIFT.backup"
echo ""
