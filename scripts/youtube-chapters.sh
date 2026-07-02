#!/usr/bin/env bash
# Stitch per-chapter Trahe ACME demo MP4s into one YouTube-ready
# walkthrough with embedded chapter metadata.
#
# Output:
#   tmp/e2e-trahe-out/demo-full.mp4          — stitched video with chapter markers
#   tmp/e2e-trahe-out/demo-chapters.txt      — paste into YouTube description
#   tmp/e2e-trahe-out/demo-chapters.ffmeta   — intermediate ffmetadata file
#
# Reads tmp/e2e-trahe-out/chapters/<NN>-<slug>/<slug>.mp4 in numeric
# order, looks up each duration via `ffprobe`, builds a cumulative
# chapter timeline, and produces:
#   1) a plain-text description block ("00:00 Chapter 1 — Build the GLDF" …)
#   2) an ffmetadata file with [CHAPTER] blocks (TIMEBASE=1/1000)
#   3) a concat of the chapter mp4s with `-map_metadata` so the final
#      mp4 carries the chapter markers (VLC / mpv / iOS QuickTime
#      Player honour them; YouTube reads them OR the description
#      timestamps, whichever you prefer).
#
# Usage:
#   scripts/youtube-chapters.sh                # stitch every chapter on disk
#   scripts/youtube-chapters.sh 1 2 3          # only these chapter numbers
#   scripts/youtube-chapters.sh --narrated     # prefer <slug>-narrated.mp4
#                                              # when present (from
#                                              # scripts/narrate-chapters.sh)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHAPTERS_DIR="$REPO_ROOT/tmp/e2e-trahe-out/chapters"
OUT_DIR="$REPO_ROOT/tmp/e2e-trahe-out"

# Strip --narrated from positional args; remember the choice.
USE_NARRATED=0
filtered_args=()
for a in "$@"; do
  if [[ "$a" == "--narrated" ]]; then
    USE_NARRATED=1
  else
    filtered_args+=("$a")
  fi
done
set -- "${filtered_args[@]}"

if (( USE_NARRATED )); then
  OUT_MP4="$OUT_DIR/demo-full-narrated.mp4"
  OUT_TXT="$OUT_DIR/demo-chapters-narrated.txt"
  OUT_META="$OUT_DIR/demo-chapters-narrated.ffmeta"
else
  OUT_MP4="$OUT_DIR/demo-full.mp4"
  OUT_TXT="$OUT_DIR/demo-chapters.txt"
  OUT_META="$OUT_DIR/demo-chapters.ffmeta"
fi

for tool in ffmpeg ffprobe; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "⚠ $tool not on PATH" >&2
    exit 2
  fi
done

if [[ ! -d "$CHAPTERS_DIR" ]]; then
  echo "⚠ no chapter recordings found at $CHAPTERS_DIR" >&2
  echo "  Run tests/e2e/run_trahe.sh first." >&2
  exit 2
fi

# Map chapter slug → display title. Keep in sync with CHAPTER_MANIFEST
# in tests/e2e/chapters.py — this script doesn't import the Python so
# the titles live here once.
title_for_slug() {
  case "$1" in
    build-gldf)       echo "Build the GLDF" ;;
    metadata)         echo "Fill the metadata panes" ;;
    3d-variants)      echo "3D variants — edit + static showcase" ;;
    export-reimport)  echo "Export, validate & re-import" ;;
    tm33)             echo "TM-33 round-trip & bootstrap" ;;
    lisp-bonus)       echo "Bonus — Lisp REPL choreography" ;;
    *)                echo "$1" ;;
  esac
}

# Round a floating-point seconds value to integer milliseconds.
# Uses awk because bash can't do floating-point arithmetic.
to_ms() {
  awk -v t="$1" 'BEGIN { printf "%.0f", t * 1000 }'
}

# Format integer seconds as M:SS or H:MM:SS — what YouTube wants in
# the description block.
fmt_yt() {
  local s=$1
  local h=$(( s / 3600 ))
  local m=$(( (s % 3600) / 60 ))
  local r=$(( s % 60 ))
  if (( h > 0 )); then
    printf "%d:%02d:%02d" "$h" "$m" "$r"
  else
    printf "%d:%02d" "$m" "$r"
  fi
}

LIST="$(mktemp)"
trap 'rm -f "$LIST"' EXIT

# Pass 1: enumerate chapters, collect durations, write the YouTube
# description block + ffmetadata stub.
#
# YouTube auto-detects chapters from the description text only — it
# does NOT read MP4 chapter atoms. The chapter block must follow
# strict rules: first timestamp must be EXACTLY `0:00`, ≥3 chapters
# total, each chapter ≥10 s long, one space between timestamp and
# title (not two), and the timestamp block must start at the very
# top of the description (no prose lines above it). We emit the
# timestamps first; any descriptive prose can be appended manually
# below them on YouTube.
: > "$OUT_TXT"

cat > "$OUT_META" <<'EOF'
;FFMETADATA1
title=Trahe ACME — gldf.icu walkthrough
encoder=gldf-rs e2e chapter recorder

EOF

cumulative_ms=0
chapter_num=0
found=0

for dir in "$CHAPTERS_DIR"/*/; do
  [[ -d "$dir" ]] || continue
  base="$(basename "$dir")"          # "01-build-gldf"
  num="${base%%-*}"                  # "01"
  num_int="$((10#$num))"             # 1
  slug="${base#*-}"                  # "build-gldf"
  if (( USE_NARRATED )) && [[ -f "$dir${slug}-narrated.mp4" ]]; then
    mp4="$dir${slug}-narrated.mp4"
  else
    mp4="$dir$slug.mp4"
  fi
  [[ -f "$mp4" ]] || { echo "⚠ skipping $base — no $slug.mp4" >&2; continue; }

  if (($# > 0)); then
    keep=0
    for n in "$@"; do
      [[ "$n" == "$num_int" ]] && keep=1
    done
    (( keep )) || continue
  fi

  duration_s=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$mp4")
  duration_ms=$(to_ms "$duration_s")
  start_ms=$cumulative_ms
  end_ms=$(( cumulative_ms + duration_ms ))
  cumulative_ms=$end_ms

  title=$(title_for_slug "$slug")
  start_s=$(( start_ms / 1000 ))
  yt_ts=$(fmt_yt "$start_s")

  # YouTube requires EXACTLY one space between the timestamp and the
  # chapter title. Two spaces breaks the auto-detection silently —
  # the chapter player just doesn't show up. Same goes for any
  # leading whitespace.
  printf "%s Chapter %d — %s\n" "$yt_ts" "$num_int" "$title" >> "$OUT_TXT"

  {
    echo "[CHAPTER]"
    echo "TIMEBASE=1/1000"
    echo "START=$start_ms"
    echo "END=$end_ms"
    echo "title=Chapter $num_int — $title"
    echo ""
  } >> "$OUT_META"

  esc="${mp4//\'/\'\\\'\'}"
  printf "file '%s'\n" "$esc" >> "$LIST"
  mp4_basename="$(basename "$mp4")"
  echo "  + $base/$mp4_basename (${duration_s%.*}s → ${yt_ts})"
  chapter_num=$(( chapter_num + 1 ))
  found=1
done

if (( ! found )); then
  echo "⚠ no chapter mp4s matched" >&2
  exit 2
fi

# YouTube needs the first chapter timestamp to be exactly 0:00 — already
# the case because we start at cumulative_ms=0. It also wants at least
# three chapters and each chapter at least 10 s long for the player to
# pick them up.
if (( chapter_num < 3 )); then
  echo "ℹ note: YouTube requires at least 3 chapters in the description for the chapter player to activate. Got $chapter_num." >&2
fi

echo ""
echo "🎞  stitching → $OUT_MP4"
# `-i $LIST` is the concat demuxer; `-i $OUT_META` is the ffmetadata
# source. `-map 0` keeps all streams from the concat; `-map_metadata 1`
# pulls chapter markers from the ffmeta file.
ffmpeg -y -hide_banner -loglevel error \
  -f concat -safe 0 -i "$LIST" \
  -i "$OUT_META" \
  -map_metadata 1 \
  -c copy \
  -movflags +faststart \
  "$OUT_MP4"

# Append the descriptive prose AFTER the chapter block, separated by
# a blank line. YouTube's chapter-block detector reads from the top
# of the description down and stops at the first non-matching line,
# so anything below the blank line is safe to be normal prose.
{
  echo ""
  echo "🎬 Trahe ACME — a fully in-browser walkthrough of gldf.icu,"
  echo "the open-source GLDF editor: 11-step wizard, four variants,"
  echo "XSD-validated export, TM-33 round-trip, and a Lisp REPL"
  echo "scripting layer on the Bevy 3D viewer."
  echo ""
  echo "Source: https://github.com/holg/gldf-rs"
  echo "Live editor: https://gldf.icu"
} >> "$OUT_TXT"

kb=$(( $(stat -f%z "$OUT_MP4" 2>/dev/null || stat -c%s "$OUT_MP4") / 1024 ))
echo "✅ $OUT_MP4 (${kb} KB, $chapter_num chapters)"
echo "📋 $OUT_TXT — paste into YouTube description"
echo "🎯 $OUT_META — ffmetadata source (embedded in mp4)"
