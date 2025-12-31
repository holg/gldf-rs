#!/usr/bin/env bash
#
# update_version.sh - Update version across all crates and configs
#
# Usage: ./scripts/update_version.sh 0.1.4
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
    echo -e "${RED}Error: Version number required${NC}"
    echo ""
    echo "Usage: $0 VERSION"
    echo ""
    echo "Example:"
    echo "  $0 0.1.4"
    echo ""
    exit 1
fi

NEW_VERSION="$1"

echo "======================================================================"
echo "Update Version to $NEW_VERSION"
echo "======================================================================"
echo ""

# Validate version format (semantic versioning)
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}Error: Invalid version format${NC}"
    echo "Version must be in format: X.Y.Z (e.g., 0.1.4)"
    exit 1
fi

echo "Updating to version: ${BLUE}$NEW_VERSION${NC}"
echo ""

# Function to update Cargo.toml version
update_cargo_toml() {
    local file="$1"
    if [ -f "$file" ]; then
        echo "  Updating: $file"
        sed -i.bak "s/^version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"$NEW_VERSION\"/" "$file"
        rm -f "$file.bak"
    fi
}

# Update workspace Cargo.toml
echo -e "${GREEN}►${NC} Workspace Cargo.toml"
update_cargo_toml "$ROOT_DIR/Cargo.toml"

# Update all crate Cargo.toml files
echo ""
echo -e "${GREEN}►${NC} Crate Cargo.toml files"
for crate_dir in "$ROOT_DIR"/crates/*; do
    if [ -d "$crate_dir" ]; then
        update_cargo_toml "$crate_dir/Cargo.toml"
    fi
done

# Update Flutter pubspec.yaml
echo ""
echo -e "${GREEN}►${NC} Flutter pubspec.yaml"
FLUTTER_PUBSPEC="$ROOT_DIR/gldf-rs-Apps/gldf-rs_flutter/pubspec.yaml"
if [ -f "$FLUTTER_PUBSPEC" ]; then
    echo "  Updating: $FLUTTER_PUBSPEC"
    sed -i.bak "s/^version: [0-9]*\.[0-9]*\.[0-9]*/version: $NEW_VERSION/" "$FLUTTER_PUBSPEC"
    rm -f "$FLUTTER_PUBSPEC.bak"
fi

# Update Flutter example pubspec.yaml
FLUTTER_EXAMPLE_PUBSPEC="$ROOT_DIR/gldf-rs-Apps/gldf-rs_flutter/example/pubspec.yaml"
if [ -f "$FLUTTER_EXAMPLE_PUBSPEC" ]; then
    echo "  Updating: $FLUTTER_EXAMPLE_PUBSPEC (dependency reference)"
    # Update the dependency reference if it exists
    if grep -q "gldf-rs_flutter:" "$FLUTTER_EXAMPLE_PUBSPEC"; then
        echo "  (Example references parent package, no version update needed)"
    fi
fi

# Note about Python (gets version from Cargo.toml automatically via maturin)
echo ""
echo -e "${GREEN}►${NC} Python (gldf-rs-py)"
echo "  ℹ  Version automatically synced from crates/gldf-rs-py/Cargo.toml"

# Note about SPM
echo ""
echo -e "${GREEN}►${NC} Swift Package Manager"
echo "  ℹ  SPM version is set by git tag (v$NEW_VERSION)"
echo "  ℹ  No file needs updating for SPM"

echo ""
echo "======================================================================"
echo -e "${GREEN}✅ Version Updated to $NEW_VERSION${NC}"
echo "======================================================================"
echo ""
echo "Files updated:"
echo "  • Cargo.toml (workspace)"
echo "  • crates/*/Cargo.toml (all crates)"
echo "  • gldf-rs-Apps/gldf-rs_flutter/pubspec.yaml"
echo ""
echo "Next steps:"
echo ""
echo "1. Verify the changes:"
echo "   ${BLUE}git diff${NC}"
echo ""
echo "2. Run tests:"
echo "   ${BLUE}cargo test --workspace${NC}"
echo ""
echo "3. Commit the version bump:"
echo "   ${BLUE}git add -A${NC}"
echo "   ${BLUE}git commit -m \"Bump version to $NEW_VERSION\"${NC}"
echo "   ${BLUE}git push origin main${NC}"
echo ""
echo "4. Create release tag:"
echo "   ${BLUE}git tag v$NEW_VERSION${NC}"
echo "   ${BLUE}git push origin v$NEW_VERSION${NC}"
echo ""
echo "5. GitHub Actions will build and release automatically!"
echo ""
