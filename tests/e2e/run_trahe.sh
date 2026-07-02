#!/usr/bin/env bash
# Wrapper for the Trahe ACME full demo (now chapter-based).
#
# Usage:
#   tests/e2e/run_trahe.sh                       # all 5 chapters
#   tests/e2e/run_trahe.sh --chapters 3          # only chapter 3
#   tests/e2e/run_trahe.sh --chapters 1,3-4      # selected subset
#   tests/e2e/run_trahe.sh --headless --fail-fast
#
# Per-chapter outputs land in tmp/e2e-trahe-out/chapters/<NN>-<slug>/.
# Each chapter emits its own webm + mp4 + .srt captions + .md narration
# script + screenshots/ folder, so chapters can be reviewed and
# AI-narrated independently. Use scripts/stitch-demo.sh to concat the
# per-chapter mp4s into a single demo-full.mp4 once you're happy with
# each chapter individually.

set -euo pipefail
VENV="$HOME/Documents/develeop/rust/geodb-rs/crates/geodb-py/.env_py312/bin/activate"
if [[ ! -f "$VENV" ]]; then
  echo "⚠ Playwright venv not found at $VENV" >&2
  exit 2
fi
# shellcheck disable=SC1090
source "$VENV"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
exec python "$REPO_ROOT/tests/e2e/demo_trahe_acme.py" "$@"
