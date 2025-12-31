#!/usr/bin/env bash
#
# update_embedded_data.sh - Download and build fresh database for FFI
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CORE_CRATE="$ROOT_DIR/crates/gldf-rs-core"
FFI_DATA_DIR="$ROOT_DIR/crates/gldf-rs-ffi/gldf-rs_rs_data"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "======================================================================"
echo "Update Embedded Database"
echo "======================================================================"
echo ""

# Check current data
if [ -f "$FFI_DATA_DIR/gldf-rs.flat.comp.blobs.bin" ]; then
    CURRENT_SIZE=$(du -h "$FFI_DATA_DIR/gldf-rs.flat.comp.blobs.bin" | cut -f1)
    CURRENT_DATE=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M" "$FFI_DATA_DIR/gldf-rs.flat.comp.blobs.bin")
    echo "Current data: $CURRENT_SIZE (from $CURRENT_DATE)"
else
    echo "No embedded data found"
fi

echo ""
echo -e "${BLUE}Downloading latest data from GitHub...${NC}"
echo ""

# Download URL (direct link to raw file)
DATA_URL="https://github.com/dr5hn/countries-states-cities-database/raw/master/json/countries%2Bstates%2Bcities.json.gz"
DATA_DIR="$CORE_CRATE/data"
DATA_FILE="$DATA_DIR/countries+states+cities.json.gz"

mkdir -p "$DATA_DIR"

# Download with curl
curl -L -o "$DATA_FILE" "$DATA_URL"

# Check if successful
if [ ! -f "$DATA_FILE" ]; then
    echo -e "${YELLOW}Error: Download failed${NC}"
    exit 1
fi

DOWNLOAD_SIZE=$(du -h "$DATA_FILE" | cut -f1)
echo ""
echo -e "${GREEN}✓ Download complete${NC} ($DOWNLOAD_SIZE)"
echo ""

# Build optimized binary cache
echo -e "${BLUE}Building optimized binary cache...${NC}"
echo ""

# Use gldf-rs-cli to load and build the cache
# The load process with builder feature will automatically create the binary cache
cd "$ROOT_DIR"
cargo run --release -p gldf-rs-cli -- stats

# The binary cache will be created in crates/gldf-rs-core/data/
CACHE_FILE=$(find "$DATA_DIR" -name "*.flat.comp.blobs.bin" 2>/dev/null | head -1)

if [ -z "$CACHE_FILE" ]; then
    echo -e "${YELLOW}Error: Binary cache not found${NC}"
    echo "Expected: data/countries+states+cities.json.*.flat.comp.blobs.bin"
    exit 1
fi

echo "Generated cache: $CACHE_FILE"
NEW_SIZE=$(du -h "$CACHE_FILE" | cut -f1)
echo "Size: $NEW_SIZE"

# Copy to FFI data directory
echo ""
echo -e "${BLUE}Copying to FFI crate...${NC}"
mkdir -p "$FFI_DATA_DIR"
cp "$CACHE_FILE" "$FFI_DATA_DIR/gldf-rs.flat.comp.blobs.bin"

echo -e "${GREEN}✓ Embedded data updated${NC}"
echo ""

# Show summary
echo "======================================================================"
echo -e "${GREEN}✅ Database Updated Successfully${NC}"
echo "======================================================================"
echo ""
echo "Location: $FFI_DATA_DIR/gldf-rs.flat.comp.blobs.bin"
echo "Size:     $NEW_SIZE"
echo "Source:   https://github.com/dr5hn/countries-states-cities-database"
echo ""
echo "Next steps:"
echo ""
echo "1. Rebuild the SPM package:"
echo "   ${BLUE}./scripts/build_spm_universal.sh${NC}"
echo ""
echo "2. Create release:"
echo "   ${BLUE}./scripts/publish_manual_release.sh v0.1.4${NC}"
echo ""
