#!/usr/bin/env bash
# Stitch per-chapter Trahe ACME demo MP4s into one walkthrough.
#
# Reads `tmp/e2e-trahe-out/chapters/<NN>-<slug>/<slug>.mp4` in numeric
# order and concatenates them with ffmpeg's concat demuxer (no
# re-encoding — fast, lossless, assumes identical codec settings,
# which the chapter recorder enforces).
#
# Output: tmp/e2e-trahe-out/demo-full.mp4
#
# Usage:
#   scripts/stitch-demo.sh              # all chapters present on disk
#   scripts/stitch-demo.sh 1 2 3        # only specific chapter numbers

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHAPTERS_DIR="$REPO_ROOT/tmp/e2e-trahe-out/chapters"
OUT="$REPO_ROOT/tmp/e2e-trahe-out/demo-full.mp4"

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "⚠ ffmpeg not on PATH" >&2
  exit 2
fi

if [[ ! -d "$CHAPTERS_DIR" ]]; then
  echo "⚠ no chapter recordings found at $CHAPTERS_DIR" >&2
  echo "  Run tests/e2e/run_trahe.sh first." >&2
  exit 2
fi

# Build the concat list. Filter chapters if numeric args were passed.
LIST="$(mktemp)"
trap 'rm -f "$LIST"' EXIT

found=0
for dir in "$CHAPTERS_DIR"/*/; do
  [[ -d "$dir" ]] || continue
  base="$(basename "$dir")"             # e.g. "01-build-gldf"
  num="${base%%-*}"                     # "01"
  num_int="$((10#$num))"                # strip leading zero
  slug="${base#*-}"                     # "build-gldf"
  mp4="$dir$slug.mp4"
  [[ -f "$mp4" ]] || { echo "⚠ skipping $base — no $slug.mp4" >&2; continue; }

  if (($# > 0)); then
    keep=0
    for n in "$@"; do
      [[ "$n" == "$num_int" ]] && keep=1
    done
    (( keep )) || continue
  fi

  # Escape single quotes per ffmpeg concat demuxer syntax.
  esc="${mp4//\'/\'\\\'\'}"
  printf "file '%s'\n" "$esc" >> "$LIST"
  echo "  + $base/$slug.mp4"
  found=1
done

if (( ! found )); then
  echo "⚠ no chapter mp4s matched" >&2
  exit 2
fi

echo
echo "🎞  stitching → $OUT"
ffmpeg -y -hide_banner -loglevel error \
  -f concat -safe 0 -i "$LIST" \
  -c copy "$OUT"

kb=$(( $(stat -f%z "$OUT" 2>/dev/null || stat -c%s "$OUT") / 1024 ))
echo "✅ $OUT (${kb} KB)"
