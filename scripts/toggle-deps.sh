#!/usr/bin/env bash
# Toggle between local path dependencies (dev) and crates.io versions (release)
#
# Usage:
#   ./scripts/toggle-deps.sh dev      # Switch to local paths
#   ./scripts/toggle-deps.sh release  # Switch to crates.io versions
#   ./scripts/toggle-deps.sh status   # Show current state

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CARGO_TOML="$ROOT_DIR/Cargo.toml"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Dependency definitions
# Format: "name|crates_version|local_path|extra_attrs"
# extra_attrs: additional attributes to preserve (e.g., "default-features = false")
# Paths are relative to the workspace root (gldf-rs/)
# Note: eulumdat-rs and l3d-rs are workspaces, so we point to their crate subdirectories
DEPS=(
    "eulumdat|0.3.0|../eulumdat-rs/crates/eulumdat|"
    "l3d_rs|0.2.3|../l3d-rs/crates/l3d_rs|"
    "eulumdat-bevy|0.3.0|../eulumdat-rs/crates/eulumdat-bevy|default-features = false"
)

show_status() {
    echo -e "${BLUE}=== Dependency Status ===${NC}\n"

    for dep_def in "${DEPS[@]}"; do
        IFS='|' read -r name version path extra <<< "$dep_def"

        # Check if local path is present
        if grep -qE "^${name}\s*=.*path\s*=" "$CARGO_TOML" 2>/dev/null; then
            echo -e "  ${name}: ${YELLOW}LOCAL${NC} (path = \"$path\")"
        else
            echo -e "  ${name}: ${GREEN}CRATES.IO${NC} (version = \"$version\")"
        fi
    done
    echo ""
}

switch_to_dev() {
    echo -e "${BLUE}=== Switching to LOCAL dependencies ===${NC}\n"

    for dep_def in "${DEPS[@]}"; do
        IFS='|' read -r name version path extra <<< "$dep_def"

        # Check if already local
        if grep -qE "^${name}\s*=.*path\s*=" "$CARGO_TOML" 2>/dev/null; then
            echo -e "  ${name}: ${YELLOW}Already local${NC}"
            continue
        fi

        # Build the new dependency line
        if [ -n "$extra" ]; then
            new_dep="${name} = { version = \"${version}\", path = \"${path}\", ${extra} }"
        else
            new_dep="${name} = { version = \"${version}\", path = \"${path}\" }"
        fi

        # Match any line starting with the dep name
        if grep -qE "^${name}\s*=" "$CARGO_TOML"; then
            # Use perl for more reliable in-place editing
            perl -i -pe "s|^${name}\s*=.*|${new_dep}|" "$CARGO_TOML"
            echo -e "  ${name}: ${GREEN}Switched to local${NC}"
        else
            echo -e "  ${name}: ${RED}Not found in Cargo.toml${NC}"
        fi
    done

    echo -e "\n${GREEN}Done!${NC} Run 'cargo check' to verify."
}

switch_to_release() {
    echo -e "${BLUE}=== Switching to CRATES.IO dependencies ===${NC}\n"

    for dep_def in "${DEPS[@]}"; do
        IFS='|' read -r name version path extra <<< "$dep_def"

        # Check if already crates.io (no path)
        if ! grep -qE "^${name}\s*=.*path\s*=" "$CARGO_TOML" 2>/dev/null; then
            echo -e "  ${name}: ${YELLOW}Already crates.io${NC}"
            continue
        fi

        # Build the new dependency line (without path)
        if [ -n "$extra" ]; then
            new_dep="${name} = { version = \"${version}\", ${extra} }"
        else
            new_dep="${name} = \"${version}\""
        fi

        # Match any line starting with the dep name
        if grep -qE "^${name}\s*=" "$CARGO_TOML"; then
            # Use perl for more reliable in-place editing
            perl -i -pe "s|^${name}\s*=.*|${new_dep}|" "$CARGO_TOML"
            echo -e "  ${name}: ${GREEN}Switched to crates.io${NC}"
        else
            echo -e "  ${name}: ${RED}Not found in Cargo.toml${NC}"
        fi
    done

    echo -e "\n${GREEN}Done!${NC} Run 'cargo check' to verify."
}

usage() {
    echo "Usage: $0 {dev|release|status}"
    echo ""
    echo "Commands:"
    echo "  dev      Switch to local path dependencies for development"
    echo "  release  Switch to crates.io versions for publishing"
    echo "  status   Show current dependency state"
    exit 1
}

# Main
cd "$ROOT_DIR"

case "${1:-status}" in
    dev|local)
        switch_to_dev
        ;;
    release|crates|publish)
        switch_to_release
        ;;
    status|show)
        show_status
        ;;
    *)
        usage
        ;;
esac
