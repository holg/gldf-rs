#!/usr/bin/env bash
# File: /Users/htr/Documents/develeop/rust/gldf-rs-rs/scripts/deploy-wasm.sh

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== gldf-rs WASM Deployment Script ===${NC}\n"

# Configuration
REMOTE_HOST="trahe.eu"
REMOTE_PATH="/var/www/trahe/html"
LOCAL_DIST="crates/gldf-rs-wasm/dist"
REMOTE_INDEX_NAME="gldf-rs-rs.html"

# Check if we're in the right directory
if [ ! -d "crates/gldf-rs-wasm" ]; then
    echo -e "${RED}Error: Must run from repository root${NC}"
    exit 1
fi

# Check if dist directory exists
if [ ! -d "$LOCAL_DIST" ]; then
    echo -e "${RED}Error: dist directory not found. Run 'trunk build --release' first${NC}"
    exit 1
fi

echo -e "${BLUE}Step 1: Building WASM with Trunk...${NC}"
cd crates/gldf-rs-wasm
trunk build --release
cd ../..
echo -e "${GREEN}✓ Build complete${NC}\n"

echo -e "${BLUE}Step 2: Deploying files to ${REMOTE_HOST}...${NC}"

# Copy all files at once
echo "Deploying gldf-rs-wasm  files..."
scp "$LOCAL_DIST"/*.{wasm,js} "${REMOTE_HOST}:${REMOTE_PATH}/" && \
scp "$LOCAL_DIST/index.html" "${REMOTE_HOST}:${REMOTE_PATH}/$REMOTE_INDEX_NAME"


echo -e "\n${GREEN}✓ Deployment complete!${NC}"
echo -e "${BLUE}View at: https://trahe.eu/$REMOTE_INDEX_NAME${NC}"