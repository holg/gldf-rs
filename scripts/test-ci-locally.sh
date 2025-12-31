#!/usr/bin/env bash
# File: /Users/htr/Documents/develeop/rust/gldf-rs-rs/scripts/test-ci-locally.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Testing CI Workflow Locally ===${NC}\n"

# Run cargo fmt check (only for this workspace, not external path deps)
echo -e "${YELLOW}Step 1: Running cargo fmt check...${NC}"
# Only check packages in this workspace, exclude external path dependencies
WORKSPACE_PACKAGES=$(cargo metadata --no-deps --format-version 1 2>/dev/null | \
    python3 -c "import sys,json; d=json.load(sys.stdin); print(' '.join(['-p '+p['name'] for p in d['packages']]))" 2>/dev/null || echo "")
if [ -n "$WORKSPACE_PACKAGES" ]; then
    if cargo fmt $WORKSPACE_PACKAGES -- --check; then
        echo -e "${GREEN}✓ cargo fmt passed${NC}\n"
    else
        echo -e "${RED}✗ cargo fmt failed${NC}"
        echo -e "${YELLOW}Run 'cargo fmt $WORKSPACE_PACKAGES' to fix formatting issues${NC}\n"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ Could not determine workspace packages, skipping fmt check${NC}\n"
fi

# Run clippy (only for workspace packages)
echo -e "${YELLOW}Step 2: Running clippy...${NC}"
if cargo clippy --workspace --all-targets -- -D warnings; then
    echo -e "${GREEN}✓ clippy passed${NC}\n"
else
    echo -e "${RED}✗ clippy failed${NC}\n"
    exit 1
fi

# Run cargo build
echo -e "${YELLOW}Step 3: Running cargo build...${NC}"
# Exclude gldf-rs-python as it requires maturin to build
if cargo build --workspace --exclude gldf-rs-python; then
    echo -e "${GREEN}✓ build passed${NC}\n"
else
    echo -e "${RED}✗ build failed${NC}\n"
    exit 1
fi

# Build gldf-rs-python separately with maturin
echo -e "${YELLOW}Step 3b: Building gldf-rs-python with maturin...${NC}"
if command -v maturin &> /dev/null; then
#    if (source crates/gldf-rs-python/.env_py312/bin/activate && maturin build --locked); then
    if (cd crates/gldf-rs-python && source ~/Documents/develeop/rust/geodb-rs/crates/geodb-py/.env_py312/bin/activate && maturin build); then
        echo -e "${GREEN}✓ gldf-rs-python build passed${NC}\n"
    else
        echo -e "${RED}✗ gldf-rs-python build failed${NC}\n"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ maturin not installed, skipping gldf-rs-python build${NC}"
    echo -e "${YELLOW}Install with: pip install maturin${NC}\n"
fi

# Run cargo doc
echo -e "${YELLOW}Step 4: Running cargo doc...${NC}"
if RUSTDOCFLAGS="-D warnings" cargo doc --workspace --document-private-items --no-deps; then
    echo -e "${GREEN}✓ doc generation passed${NC}\n"
else
    echo -e "${RED}✗ doc generation failed${NC}\n"
    exit 1
fi

# Additional checks
echo -e "${YELLOW}Step 5: Running additional checks...${NC}"

# Check cargo-sort FIRST (it should run before taplo)
if command -v cargo-sort &> /dev/null; then
    echo "Running cargo-sort check..."
    if cargo-sort -cwg; then
        echo -e "${GREEN}✓ cargo-sort passed${NC}"
    else
        echo -e "${RED}✗ cargo-sort failed${NC}"
        echo -e "${YELLOW}Run 'cargo-sort -wg' to fix Cargo.toml sorting${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ cargo-sort not installed, skipping Cargo.toml sort check${NC}"
    echo -e "${YELLOW}Install with: cargo install cargo-sort${NC}"
fi

# Check taplo AFTER cargo-sort (taplo formats what cargo-sort organized)
if command -v taplo &> /dev/null; then
    echo "Running taplo format check..."
    if taplo format --check; then
        echo -e "${GREEN}✓ taplo passed${NC}"
    else
        echo -e "${RED}✗ taplo failed${NC}"
        echo -e "${YELLOW}Run 'taplo format' to fix TOML formatting${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ taplo not installed, skipping TOML format check${NC}"
    echo -e "${YELLOW}Install with: cargo install taplo-cli${NC}"
fi

# Check cargo-deny
if command -v cargo-deny &> /dev/null; then
    echo "Running cargo-deny check..."
    if cargo-deny check bans licenses sources --hide-inclusion-graph --show-stats; then
        echo -e "${GREEN}✓ cargo-deny passed${NC}"
    else
        echo -e "${RED}✗ cargo-deny failed${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ cargo-deny not installed, skipping dependency check${NC}"
    echo -e "${YELLOW}Install with: cargo install cargo-deny${NC}"
fi

# Run Rust tests (native)
echo -e "${YELLOW}Step 6: Running Rust tests (native targets)...${NC}"
if cargo test --workspace --exclude gldf-rs-wasm --exclude gldf-rs-python -- --test-threads=1; then
    echo -e "${GREEN}✓ native tests passed${NC}\n"
else
    echo -e "${RED}✗ native tests failed${NC}\n"
    exit 1
fi

# Run WASM tests for gldf-rs-wasm if tooling is available
echo -e "${YELLOW}Step 6b: Running gldf-rs-wasm tests (wasm32, Node)...${NC}"
if command -v wasm-pack &> /dev/null; then
    if (cd crates/gldf-rs-wasm && wasm-pack test --node); then
        echo -e "${GREEN}✓ gldf-rs-wasm tests passed (node)${NC}\n"
    else
        echo -e "${RED}✗ gldf-rs-wasm tests failed${NC}\n"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ wasm-pack not installed, skipping gldf-rs-wasm tests${NC}"
    echo -e "${YELLOW}Install with: cargo install wasm-pack${NC}"
fi

# Build WASM target (demo app)
echo -e "${YELLOW}Step 7: Building WASM demo (Trunk)...${NC}"
if command -v trunk &> /dev/null && command -v wasm-bindgen &> /dev/null; then
    echo "Building gldf-rs-wasm with Trunk..."
    # Use homebrew's wasm-opt if available (newer version handles WASM features better)
    WASM_OPT_PATH=""
    if [[ -x "/opt/homebrew/opt/binaryen/bin/wasm-opt" ]]; then
        WASM_OPT_PATH="/opt/homebrew/opt/binaryen/bin"
    fi
    if (cd crates/gldf-rs-wasm && PATH="${WASM_OPT_PATH:-}:$PATH" trunk build --release); then
        echo -e "${GREEN}✓ WASM build passed${NC}\n"
    else
        echo -e "${RED}✗ WASM build failed${NC}\n"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ trunk or wasm-bindgen-cli not installed, skipping WASM build${NC}"
    echo -e "${YELLOW}Install with:${NC}"
    echo -e "${YELLOW}  cargo install trunk${NC}"
    echo -e "${YELLOW}  cargo install wasm-bindgen-cli${NC}"
    echo -e "${YELLOW}  rustup target add wasm32-unknown-unknown${NC}"
fi

# Run Python tests for gldf-rs-python if tooling is available
echo -e "${YELLOW}Step 7b: Running Python tests for gldf-rs-python...${NC}"
if command -v python3 &> /dev/null && command -v maturin &> /dev/null; then
    TMP_VENV=".venv_gldf-rs_test"
    python3 -m venv "$TMP_VENV"
    # shellcheck disable=SC1090
    source "$TMP_VENV/bin/activate"
    python -m pip install --upgrade pip >/dev/null 2>&1
    if python -c "import pytest" 2>/dev/null; then
        echo "pytest already available in venv"
    else
        python -m pip install pytest >/dev/null 2>&1 || true
    fi

    echo "Building and installing gldf-rs-python into venv (maturin develop)..."
    if maturin develop -m crates/gldf-rs-python/Cargo.toml --release >/dev/null; then
        echo -e "${GREEN}✓ gldf-rs-python built and installed${NC}"
        echo "Running pytest..."
        # pytest returns exit code 5 when no tests are collected, treat as success
        cd crates/gldf-rs-python
        pytest -q
        pytest_exit=$?
        cd - > /dev/null
        if [[ $pytest_exit -eq 0 || $pytest_exit -eq 5 ]]; then
            if [[ $pytest_exit -eq 5 ]]; then
                echo -e "${YELLOW}⚠ No Python tests found${NC}\n"
            else
                echo -e "${GREEN}✓ gldf-rs-python tests passed${NC}\n"
            fi
        else
            echo -e "${RED}✗ gldf-rs-python tests failed${NC}\n"
            deactivate || true
            rm -rf "$TMP_VENV"
            exit 1
        fi
    else
        echo -e "${RED}✗ maturin develop failed for gldf-rs-python${NC}"
        deactivate || true
        rm -rf "$TMP_VENV"
        exit 1
    fi

    deactivate || true
    rm -rf "$TMP_VENV"
else
    echo -e "${YELLOW}⚠ python3 or maturin not installed, skipping gldf-rs-python tests${NC}"
    echo -e "${YELLOW}Install with:${NC}"
    echo -e "${YELLOW}  pipx install maturin  (or: pip install maturin)${NC}"
    echo -e "${YELLOW}  pip install pytest${NC}"
fi

# Pre-publish checks
echo -e "\n${BLUE}=== Pre-publish Checks for crates.io ===${NC}\n"

# Check package metadata
echo -e "${YELLOW}Step 8: Validating package metadata...${NC}"
for crate_dir in crates/*/; do
    crate_name=$(basename "$crate_dir")
    echo "Checking $crate_name..."

    if (cd "$crate_dir" && cargo package --list --allow-dirty > /dev/null 2>&1); then
        echo -e "${GREEN}✓ $crate_name package metadata valid${NC}"
    else
        echo -e "${RED}✗ $crate_name package metadata invalid${NC}"
        echo -e "${YELLOW}Run 'cd $crate_dir && cargo package --list' for details${NC}"
        exit 1
    fi
done
echo ""

# Dry-run publish in dependency order
echo -e "${YELLOW}Step 9: Running dry-run publish (in dependency order)...${NC}"

# Define publish order: dependencies first, then dependents
PUBLISH_ORDER=(
    "gldf-rs-core"
    "gldf-rs-wasm"
    "gldf-rs-cli"
    "gldf-rs-python"
)

for crate_name in "${PUBLISH_ORDER[@]}"; do
    crate_dir="crates/$crate_name"

    if [[ ! -d "$crate_dir" ]]; then
        echo -e "${YELLOW}⚠ Skipping $crate_name (directory not found)${NC}"
        continue
    fi

    echo "Validating $crate_name package..."

    # For gldf-rs-core (no dependencies), do full dry-run publish
    if [[ "$crate_name" == "gldf-rs-core" ]]; then
        if (cd "$crate_dir" && cargo publish --dry-run --allow-dirty); then
            echo -e "${GREEN}✓ $crate_name dry-run publish passed${NC}"
        else
            echo -e "${RED}✗ $crate_name dry-run publish failed${NC}"
            exit 1
        fi
    else
        # For dependent crates, just verify package contents
        # (can't do full dry-run until dependencies are on crates.io)
        if (cd "$crate_dir" && cargo package --allow-dirty --list > /dev/null 2>&1); then
            echo -e "${GREEN}✓ $crate_name package validation passed${NC}"
            echo -e "${BLUE}  Note: Full publish validation will happen after gldf-rs-core is published${NC}"
        else
            echo -e "${RED}✗ $crate_name package validation failed${NC}"
            exit 1
        fi
    fi
done
echo ""

# Check for uncommitted changes
echo -e "${YELLOW}Step 10: Checking for uncommitted changes...${NC}"
if [[ -n $(git status --porcelain) ]]; then
    echo -e "${YELLOW}⚠ You have uncommitted changes:${NC}"
    git status --short
    echo -e "${YELLOW}Consider committing or stashing changes before publishing${NC}\n"
else
    echo -e "${GREEN}✓ No uncommitted changes${NC}\n"
fi

# Check if on main branch
echo -e "${YELLOW}Step 11: Checking git branch...${NC}"
current_branch=$(git branch --show-current)
if [[ "$current_branch" != "main" ]]; then
    echo -e "${YELLOW}⚠ You are on branch '$current_branch', not 'main'${NC}"
    echo -e "${YELLOW}Consider switching to main branch before publishing${NC}\n"
else
    echo -e "${GREEN}✓ On main branch${NC}\n"
fi

# Check for version tags
echo -e "${YELLOW}Step 12: Checking version consistency...${NC}"
for crate_name in "${PUBLISH_ORDER[@]}"; do
    crate_dir="crates/$crate_name"

    if [[ ! -d "$crate_dir" ]]; then
        continue
    fi

    version=$(grep -m1 '^version = ' "$crate_dir/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')

    if git tag | grep -q "^${crate_name}-v${version}$"; then
        echo -e "${YELLOW}⚠ Tag ${crate_name}-v${version} already exists${NC}"
        echo -e "${YELLOW}Consider bumping version in $crate_dir/Cargo.toml${NC}"
    else
        echo -e "${GREEN}✓ $crate_name v$version - version tag available${NC}"
    fi
done
echo ""

# Build documentation as it would appear on docs.rs
echo -e "${YELLOW}Step 13: Building documentation for review (as on docs.rs)...${NC}"
echo "Building docs with all features enabled..."

# Clean previous docs
rm -rf target/doc

# Build docs with docs.rs settings
if RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --workspace --all-features --no-deps; then
    echo -e "${GREEN}✓ Documentation built successfully${NC}\n"

    # Find the main crate documentation
    DOC_PATH="target/doc/gldf-rs_core/index.html"

    if [[ -f "$DOC_PATH" ]]; then
        echo -e "${BLUE}Opening documentation in browser...${NC}"

        # Detect OS and open browser accordingly
        if [[ "$OSTYPE" == "darwin"* ]]; then
            open "$DOC_PATH"
        elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
            if command -v xdg-open &> /dev/null; then
                xdg-open "$DOC_PATH"
            else
                echo -e "${YELLOW}Please open: file://$(pwd)/$DOC_PATH${NC}"
            fi
        else
            echo -e "${YELLOW}Please open: file://$(pwd)/$DOC_PATH${NC}"
        fi

        echo -e "${GREEN}✓ Documentation opened for review${NC}"
        echo -e "${BLUE}Review the documentation at: file://$(pwd)/$DOC_PATH${NC}\n"
    else
        echo -e "${YELLOW}⚠ Documentation index not found at expected location${NC}"
        echo -e "${YELLOW}Check target/doc/ directory manually${NC}\n"
    fi
else
    echo -e "${RED}✗ Documentation build failed${NC}"
    echo -e "${YELLOW}Note: This uses nightly Rust with --cfg docsrs flag${NC}"
    echo -e "${YELLOW}Install nightly with: rustup toolchain install nightly${NC}\n"
fi

echo -e "\n${GREEN}=== All CI checks passed! ===${NC}"
echo -e "${GREEN}Your code is ready to be pushed.${NC}"
echo -e "\n${BLUE}Publishing Order (IMPORTANT - follow this sequence):${NC}"
echo -e "  ${YELLOW}1.${NC} Review the documentation that was just opened"
echo -e "  ${YELLOW}2.${NC} Ensure all changes are committed"
echo -e "  ${YELLOW}3.${NC} Publish ${BLUE}gldf-rs-core${NC} first (it has no dependencies):"
echo -e "     ${YELLOW}cd crates/gldf-rs-core && cargo publish${NC}"
echo -e "  ${YELLOW}4.${NC} Wait for gldf-rs-core to be available on crates.io"