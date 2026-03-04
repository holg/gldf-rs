#!/usr/bin/env bash
# Run the same checks as .github/workflows/ci.yml locally.
# Usage: scripts/test-ci-locally.sh [--quick]
#   --quick  Skip WASM build, docs, and audit (just fmt + clippy + build + test)

set -euo pipefail

QUICK=false
for arg in "$@"; do
  case "$arg" in
    --quick) QUICK=true ;;
  esac
done

BOLD="\033[1m"
GREEN="\033[32m"
RED="\033[31m"
RESET="\033[0m"

pass() { echo -e "${GREEN}${BOLD}✓ $1${RESET}"; }
fail() { echo -e "${RED}${BOLD}✗ $1${RESET}"; exit 1; }
step() { echo -e "\n${BOLD}── $1 ──${RESET}"; }

cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

# 1. Formatting
step "rustfmt"
cargo fmt --all -- --check && pass "rustfmt" || fail "rustfmt"

# 2. Clippy
step "clippy"
cargo clippy --workspace --all-targets -- -D warnings && pass "clippy" || fail "clippy"

# 3. Build
step "cargo build"
cargo build --workspace --release && pass "build" || fail "build"

# 4. Tests
step "cargo test"
cargo test --workspace --release && pass "test" || fail "test"

if [ "$QUICK" = true ]; then
  echo ""
  pass "Quick CI passed (skipped WASM, docs, audit)"
  exit 0
fi

# 5. WASM build
step "WASM check"
cargo check -p gldf-rs-wasm --target wasm32-unknown-unknown && pass "WASM check" || fail "WASM check"

# 6. Documentation
step "cargo doc"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps && pass "docs" || fail "docs"

# 7. Security audit (if cargo-audit is installed)
step "cargo audit"
if command -v cargo-audit &>/dev/null; then
  cargo audit && pass "audit" || fail "audit"
else
  echo "  cargo-audit not installed, skipping (install with: cargo install cargo-audit)"
fi

echo ""
pass "All CI checks passed"
