#!/usr/bin/env bash
# Run the standalone Bevy camera probe in the shared Playwright venv.
# Usage: tests/e2e/camera_probe.sh [--headless] [--gldf <path>]

set -euo pipefail

VENV="$HOME/Documents/develeop/rust/geodb-rs/crates/geodb-py/.env_py312/bin/activate"
if [[ ! -f "$VENV" ]]; then
  echo "⚠ Playwright venv not found at $VENV" >&2
  exit 2
fi
# shellcheck disable=SC1090
source "$VENV"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
exec python "$REPO_ROOT/tests/e2e/camera_probe.py" "$@"
