#!/usr/bin/env bash
# Synthesize edge-tts narration for the Trahe ACME demo chapters and
# mux it onto each chapter mp4.
#
# Inputs (per chapter):
#   tests/e2e/narration/<NN>-<slug>.json   — hand-written voice-over script
#                                            keyed by cue title.
#   tmp/e2e-trahe-out/chapters/<NN>-<slug>/<slug>.srt   — cue timings
#   tmp/e2e-trahe-out/chapters/<NN>-<slug>/<slug>.mp4   — silent video
#
# Outputs (per chapter):
#   tmp/e2e-trahe-out/chapters/<NN>-<slug>/<slug>-narrated.mp4
#   tmp/e2e-trahe-out/chapters/<NN>-<slug>/audio/cue-NN.mp3   (per-cue tts)
#   tmp/e2e-trahe-out/chapters/<NN>-<slug>/audio/track.mp3    (full track)
#
# Each cue is synthesized as its own mp3, then padded with silence to
# fill the cue's SRT window. The first cue gets an `intro_pad_seconds`
# of silence at the head so the title-cards aren't talked over.
# Cues whose narration runs LONGER than the SRT window are time-
# compressed via ffmpeg's `atempo` (max 1.5x — beyond that the voice
# sounds chipmunky). Cues with no narration text get straight silence
# so the audio track stays continuous and the embedded chapter timing
# of the final concat stays accurate.
#
# Usage:
#   scripts/narrate-chapters.sh                 # all chapters
#   scripts/narrate-chapters.sh 1 2 3           # only these chapter numbers
#
# Requires: edge-tts (in the venv used by tests/e2e/run_trahe.sh),
# ffmpeg, ffprobe, jq.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHAPTERS_DIR="$REPO_ROOT/tmp/e2e-trahe-out/chapters"
NARRATION_DIR="$REPO_ROOT/tests/e2e/narration"

# Activate the same Playwright venv the demo runner uses — that's
# where edge-tts is installed.
VENV="$HOME/Documents/develeop/rust/geodb-rs/crates/geodb-py/.env_py312/bin/activate"
if [[ -f "$VENV" ]]; then
  # shellcheck disable=SC1090
  source "$VENV"
fi

for tool in edge-tts ffmpeg ffprobe jq python3; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "⚠ $tool not on PATH" >&2
    exit 2
  fi
done

# Parse an SRT timecode (HH:MM:SS,mmm) into integer milliseconds.
srt_to_ms() {
  local ts="$1"
  python3 -c "
import sys
ts = sys.argv[1]
h, m, rest = ts.split(':')
s, ms = rest.split(',')
print(int(h)*3600000 + int(m)*60000 + int(s)*1000 + int(ms))
" "$ts"
}

# Get the duration of an audio/video file in milliseconds.
probe_ms() {
  ffprobe -v error -show_entries format=duration -of csv=p=0 "$1" \
    | python3 -c "import sys; print(int(float(sys.stdin.read().strip()) * 1000))"
}

# Generate `dur_ms` of silence as an mp3 at `out`.
gen_silence() {
  local dur_ms="$1" out="$2"
  local dur_s
  dur_s=$(python3 -c "print(${dur_ms}/1000)")
  ffmpeg -y -hide_banner -loglevel error \
    -f lavfi -i "anullsrc=r=24000:cl=mono" \
    -t "$dur_s" -q:a 9 -acodec libmp3lame "$out"
}

# Pad `src` with trailing silence so the result is at least `target_ms`
# long. Never speeds up — earlier versions atempo'd overruns down to
# fit the cue window, which produced chipmunk audio whenever the
# writer over-scripted a short cue. The new contract: audio is played
# at natural speed, and overruns are tolerated; the trailing audio
# spills into the next cue's window (or into post-last-cue silence).
# This requires the writer to keep cue narration close to the SRT
# duration (~2.5 words per second of cue), but the result sounds
# right — and post-cue silence is unavoidable footage anyway.
pad_to_at_least() {
  local src="$1" target_ms="$2" out="$3"
  local src_ms
  src_ms=$(probe_ms "$src")
  if (( src_ms >= target_ms )); then
    # Already long enough — pass through (re-encode for codec
    # consistency in the concat step).
    ffmpeg -y -hide_banner -loglevel error \
      -i "$src" -acodec libmp3lame -q:a 4 "$out"
    return
  fi
  local pad_ms=$(( target_ms - src_ms ))
  if (( pad_ms <= 50 )); then
    cp "$src" "$out"
    return
  fi
  local pad_s
  pad_s=$(python3 -c "print(${pad_ms}/1000)")
  ffmpeg -y -hide_banner -loglevel error \
    -i "$src" -af "apad=pad_dur=${pad_s}" \
    -acodec libmp3lame -q:a 4 "$out"
}

process_chapter() {
  local ch_dir="$1"
  local base
  base="$(basename "$ch_dir")"             # "01-build-gldf"
  local num_int="$((10#${base%%-*}))"
  local slug="${base#*-}"                  # "build-gldf"
  local srt="$ch_dir/$slug.srt"
  local mp4="$ch_dir/$slug.mp4"
  local narration_json="$NARRATION_DIR/$base.json"
  local out_mp4="$ch_dir/${slug}-narrated.mp4"

  echo
  echo "── chapter $num_int: $slug ────────────────────────"

  if [[ ! -f "$srt" || ! -f "$mp4" ]]; then
    echo "  ⚠ missing srt or mp4, skipping"
    return
  fi
  if [[ ! -f "$narration_json" ]]; then
    echo "  ⚠ no narration script at $narration_json, skipping"
    return
  fi

  local audio_dir="$ch_dir/audio"
  rm -rf "$audio_dir"
  mkdir -p "$audio_dir"

  local voice intro_pad
  voice=$(jq -r '.voice // "en-US-AriaNeural"' "$narration_json")
  intro_pad=$(jq -r '.intro_pad_seconds // 0' "$narration_json")
  local intro_pad_ms
  intro_pad_ms=$(python3 -c "print(int(${intro_pad}*1000))")
  echo "  voice: $voice  intro_pad: ${intro_pad}s"

  # Parse the SRT into a python helper that emits one line per cue:
  #   <start_ms>\t<end_ms>\t<title>
  local cues_tsv
  cues_tsv=$(python3 - "$srt" <<'PY'
import sys, re
path = sys.argv[1]
with open(path, encoding="utf-8") as f:
    blocks = re.split(r"\n\s*\n", f.read().strip())
for b in blocks:
    lines = [l for l in b.splitlines() if l.strip()]
    if len(lines) < 3:
        continue
    ts = lines[1]
    m = re.match(r"(\d+):(\d+):(\d+),(\d+)\s*-->\s*(\d+):(\d+):(\d+),(\d+)", ts)
    if not m:
        continue
    h1, m1, s1, ms1, h2, m2, s2, ms2 = map(int, m.groups())
    start = h1*3600000 + m1*60000 + s1*1000 + ms1
    end   = h2*3600000 + m2*60000 + s2*1000 + ms2
    title = " ".join(lines[2:])
    print(f"{start}\t{end}\t{title}")
PY
  )

  # Build the audio track: optional intro silence, then per-cue mp3s.
  local concat_list="$audio_dir/concat.txt"
  : > "$concat_list"

  if (( intro_pad_ms > 0 )); then
    local intro_mp3="$audio_dir/intro-silence.mp3"
    gen_silence "$intro_pad_ms" "$intro_mp3"
    printf "file '%s'\n" "$intro_mp3" >> "$concat_list"
  fi

  local prev_end_ms=0
  local cue_idx=0
  local total_pre_pad=0

  # We need to fill any gap BETWEEN the chapter start and the first
  # cue's start — the SRT's first cue rarely starts at 0:00, so the
  # video's title-card window stays silent (with optional intro pad
  # ahead of that).
  local first_start_ms
  first_start_ms=$(printf '%s\n' "$cues_tsv" | head -n 1 | awk -F'\t' '{print $1}')
  if [[ -n "$first_start_ms" && "$first_start_ms" =~ ^[0-9]+$ ]]; then
    local gap_ms=$(( first_start_ms - intro_pad_ms ))
    if (( gap_ms > 50 )); then
      local lead_silence="$audio_dir/lead-silence.mp3"
      gen_silence "$gap_ms" "$lead_silence"
      printf "file '%s'\n" "$lead_silence" >> "$concat_list"
      total_pre_pad=$gap_ms
    fi
  fi

  while IFS=$'\t' read -r start_ms end_ms title; do
    [[ -z "$start_ms" ]] && continue
    cue_idx=$(( cue_idx + 1 ))
    local cue_dur_ms=$(( end_ms - start_ms ))
    if (( cue_dur_ms <= 0 )); then
      continue
    fi
    # Inter-cue gap (almost always 0 since SRTs are contiguous, but
    # guards against edited cue files).
    if (( start_ms > prev_end_ms && prev_end_ms > 0 )); then
      local gap_ms=$(( start_ms - prev_end_ms ))
      if (( gap_ms > 50 )); then
        local gap_mp3="$audio_dir/gap-${cue_idx}.mp3"
        gen_silence "$gap_ms" "$gap_mp3"
        printf "file '%s'\n" "$gap_mp3" >> "$concat_list"
      fi
    fi
    prev_end_ms=$end_ms

    # Look up the narration text for this cue title in the JSON.
    local text
    text=$(jq -r --arg t "$title" '
      .cues[]? | select(.cue == $t) | .text
    ' "$narration_json")

    local cue_audio="$audio_dir/cue-$(printf "%02d" "$cue_idx").mp3"

    if [[ -z "$text" || "$text" == "null" ]]; then
      echo "  · cue $cue_idx [$title] no text → silent"
      gen_silence "$cue_dur_ms" "$cue_audio"
    else
      local raw="$audio_dir/cue-$(printf "%02d" "$cue_idx")-raw.mp3"
      # Edge-TTS emits mp3 to stdout when given --write-media as a path.
      edge-tts --voice "$voice" --text "$text" --write-media "$raw" >/dev/null 2>&1
      if [[ ! -s "$raw" ]]; then
        echo "  ⚠ edge-tts produced empty file for cue $cue_idx"
        gen_silence "$cue_dur_ms" "$cue_audio"
      else
        pad_to_at_least "$raw" "$cue_dur_ms" "$cue_audio"
        local actual_ms tts_ms over
        tts_ms=$(probe_ms "$raw")
        actual_ms=$(probe_ms "$cue_audio")
        if (( actual_ms > cue_dur_ms )); then
          over=$(( actual_ms - cue_dur_ms ))
          echo "  · cue $cue_idx [$title] tts=${tts_ms}ms target=${cue_dur_ms}ms actual=${actual_ms}ms (+${over}ms overrun)"
        else
          echo "  · cue $cue_idx [$title] tts=${tts_ms}ms target=${cue_dur_ms}ms actual=${actual_ms}ms"
        fi
      fi
    fi
    printf "file '%s'\n" "$cue_audio" >> "$concat_list"
  done <<< "$cues_tsv"

  if [[ ! -s "$concat_list" ]]; then
    echo "  ⚠ no cues, skipping mux"
    return
  fi

  local track_mp3="$audio_dir/track.mp3"
  ffmpeg -y -hide_banner -loglevel error \
    -f concat -safe 0 -i "$concat_list" \
    -acodec libmp3lame -q:a 4 "$track_mp3"

  # Mux: keep the original video stream, add the synthesized audio
  # and the SRT as a soft sub stream. -shortest in case the audio
  # accidentally ran past the video end (it won't, since cue durations
  # are derived from the SRT which matches the video).
  ffmpeg -y -hide_banner -loglevel error \
    -i "$mp4" -i "$track_mp3" -i "$srt" \
    -map 0:v -map 1:a -map 2:s \
    -c:v copy -c:a aac -b:a 128k -c:s mov_text \
    -metadata:s:s:0 language=eng -metadata:s:s:0 title="Captions" \
    -movflags +faststart \
    "$out_mp4"

  local kb
  kb=$(( $(stat -f%z "$out_mp4" 2>/dev/null || stat -c%s "$out_mp4") / 1024 ))
  echo "  ✅ $out_mp4 (${kb} KB)"
}

# Iterate chapters, optionally filtered by numeric args.
for dir in "$CHAPTERS_DIR"/*/; do
  [[ -d "$dir" ]] || continue
  base="$(basename "$dir")"
  num_int="$((10#${base%%-*}))"
  if (($# > 0)); then
    keep=0
    for n in "$@"; do
      [[ "$n" == "$num_int" ]] && keep=1
    done
    (( keep )) || continue
  fi
  process_chapter "$dir"
done

echo
echo "🎙  narration complete. Each chapter now has <slug>-narrated.mp4 alongside <slug>.mp4."
echo "   Stitch the narrated chapters with:"
echo "     scripts/youtube-chapters.sh --narrated"
