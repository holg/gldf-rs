#!/usr/bin/env bash
# Generate AI voice-over + burn-in subtitles for one chapter of the
# Trahe ACME demo.
#
# Inputs (all read from a chapter directory):
#   <chapter>/<slug>.mp4   — the slow-mo'd recording
#   <chapter>/<slug>.srt   — SRT captions (one cue per Act)
#   <chapter>/<slug>.txt   — voice-over script: one paragraph per cue,
#                            paragraphs separated by ONE blank line,
#                            in the same order as the SRT.
#
# Output:
#   <chapter>/<slug>.narrated.mp4
#
# How it works:
#   1. Split <slug>.txt into one .txt per cue ("para-NN.txt").
#   2. For each paragraph: macOS `say -o para-NN.aiff` → TTS audio.
#   3. Compute target duration from the SRT cue length, prepend the
#      cue's start-time silence, pad each para-aiff to its cue length
#      with `apad`, concat into one full-length audio track.
#   4. ffmpeg: burn SRT into the video AND mux the audio.
#
# Usage:
#   scripts/narrate-chapter.sh tmp/e2e-trahe-out/chapters/01-build-gldf
#   scripts/narrate-chapter.sh tmp/e2e-trahe-out/chapters/01-build-gldf --voice Samantha
#
# A starter `<slug>.txt` is auto-generated from the chapter's `.md`
# narration script if the .txt is missing — strips markdown bullets
# and uses the first paragraph after each bullet as the spoken line.
# Edit the .txt to polish the prose, then re-run.

set -euo pipefail

CHAPTER_DIR="${1:-}"
shift || true
# Default voice + rate. TTS engine selection order:
#   1. `edge-tts` (Microsoft Neural voices via Edge read-aloud) if it
#      resolves on PATH. Studio quality, free, requires network.
#   2. macOS `say` Premium/Enhanced voice if installed.
#   3. macOS `say` Daniel (en_GB, smoothest of the stock voices).
#
# Override either with `--voice NAME` (the value is passed to whichever
# engine ends up being chosen).
ENGINE=""               # "edge" or "say" — picked below
VOICE=""
RATE=150                # words per minute for `say`. edge-tts uses a
SHIFT=0                 # seconds to subtract from every cue start so the
                        # narration begins earlier and stays in sync. Use
                        # this when there's a long lead-in silence (first
                        # cue starts at e.g. 8s) or when cumulative drift
                        # has pushed the tail past the video end.
                        # percentage `--rate` string; we translate.
# Per-style key=value pairs. Joined with `\,` at filter-build time —
# the literal backslash-comma sequence is what libavfilter wants
# inside force_style so commas don't terminate the outer filter
# parameter list.
SUB_STYLE_PAIRS=(
  "FontName=Helvetica"
  "FontSize=18"
  "PrimaryColour=&Hffffff&"
  "OutlineColour=&H000000&"
  "Outline=2"
  "Shadow=1"
  "Alignment=2"
  "MarginV=40"
)

while (($#)); do
  case "$1" in
    --voice)   VOICE="$2"; shift 2 ;;
    --rate)    RATE="$2"; shift 2 ;;
    --engine)  ENGINE="$2"; shift 2 ;;
    --shift)   SHIFT="$2"; shift 2 ;;
    *) echo "⚠ unknown arg: $1" >&2; exit 2 ;;
  esac
done

# Pick a TTS engine if not forced via --engine.
if [[ -z "$ENGINE" ]]; then
  if command -v edge-tts >/dev/null 2>&1; then
    ENGINE="edge"
  else
    ENGINE="say"
  fi
fi

# Pick a default voice for the chosen engine.
if [[ -z "$VOICE" ]]; then
  case "$ENGINE" in
    edge)
      # Microsoft's English neural voices. Andrew = warm male, Ava =
      # expressive female, Brian = casual male, Aria = confident news
      # female. Andrew Multilingual handles abbreviations + product
      # names cleanly, which the GLDF demo throws around a lot.
      VOICE="en-US-AndrewMultilingualNeural"
      ;;
    say)
      pick="$(say -v '?' | awk '/en_/ && (/Premium/ || /Enhanced/) { for (i=1; i<NF; i++) { if ($i ~ /en_/) { print prev; exit } prev=prev " " $i } prev=$1 }')"
      if [[ -n "$pick" ]]; then
        VOICE="$pick"
      else
        VOICE="Daniel"
      fi
      ;;
  esac
fi
echo "  · engine: $ENGINE  ·  voice: $VOICE  ·  rate: ${RATE} wpm"

if [[ -z "$CHAPTER_DIR" || ! -d "$CHAPTER_DIR" ]]; then
  echo "Usage: $0 <chapter_dir> [--voice NAME] [--rate WPM]" >&2
  exit 2
fi

# Resolve <slug> from the chapter dir name's "NN-<slug>" prefix.
base="$(basename "$CHAPTER_DIR")"
slug="${base#*-}"
MP4="$CHAPTER_DIR/$slug.mp4"
SRT="$CHAPTER_DIR/$slug.srt"
MD="$CHAPTER_DIR/$slug.md"
TXT="$CHAPTER_DIR/$slug.txt"
OUT="$CHAPTER_DIR/$slug.narrated.mp4"

for f in "$MP4" "$SRT"; do
  [[ -f "$f" ]] || { echo "⚠ missing $f" >&2; exit 2; }
done

# ──────────────────────────────────────────────────────────────────────
# Step 0: if no .txt voice-over script exists, seed one from the .md so
# the user can polish it. The seed strips markdown bullets and uses the
# Act docstring directly — flat but speakable.
if [[ ! -f "$TXT" ]]; then
  if [[ ! -f "$MD" ]]; then
    echo "⚠ no $TXT and no $MD to seed from" >&2
    exit 2
  fi
  echo "📝 seeding $TXT from $MD (edit, then re-run)"
  # Pull bullet lines beginning with "- **`...`** — body" and emit the
  # body as one paragraph each, separated by blank lines.
  awk '
    /^- \*\*`/ {
      sub(/^- \*\*`[^`]*`\*\* — /, "")
      if (NR > 1 && prev_emitted) print ""
      print
      prev_emitted = 1
    }
  ' "$MD" > "$TXT"
fi

# Sanity: how many cues does the SRT have?
cue_count=$(awk '/^[0-9]+$/{c=$0} END{print c}' "$SRT")
para_count=$(awk 'BEGIN{p=1} /^[[:space:]]*$/{p++; next} END{print p}' "$TXT")
echo "  · SRT cues: $cue_count  ·  TXT paragraphs: $para_count"
if [[ "$cue_count" -ne "$para_count" ]]; then
  echo "⚠ cue count ($cue_count) != paragraph count ($para_count)" >&2
  echo "  Edit $TXT so the number of paragraphs matches. Each paragraph" >&2
  echo "  is one Act's voice-over and aligns to one SRT cue." >&2
  exit 2
fi

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

# ──────────────────────────────────────────────────────────────────────
# Step 1: split the .txt into one paragraph per file (para-01.txt …).
awk -v dir="$WORK" '
  BEGIN { n = 1; first = 1; out = sprintf("%s/para-%02d.txt", dir, n) }
  /^[[:space:]]*$/ {
    if (!first) { close(out); n++; out = sprintf("%s/para-%02d.txt", dir, n); first = 1 }
    next
  }
  {
    if (first) { first = 0 } else { print "" > out }
    printf "%s\n", $0 > out
  }
' "$TXT"

# ──────────────────────────────────────────────────────────────────────
# Step 2: parse SRT cue start times (seconds) into an array.
#   "00:00:31,861" → 31.861
mapfile -t cue_starts < <(
  awk -F' --> ' -v shift="$SHIFT" '
    /-->/ {
      split($1, t, /[:,]/)
      s = t[1]*3600 + t[2]*60 + t[3] + t[4]/1000 - shift
      if (s < 0) s = 0
      printf "%.3f\n", s
    }
  ' "$SRT"
)
total_video=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$MP4")
cue_starts+=("$total_video")  # sentinel for "end of last cue"
if [[ "$SHIFT" != "0" ]]; then
  echo "  · shifted cue starts earlier by ${SHIFT}s"
fi

# ──────────────────────────────────────────────────────────────────────
# Step 3: TTS each paragraph + pad to fit its cue interval.
echo "🎙  rendering $para_count paragraphs via $ENGINE ($VOICE, ${RATE} wpm)…"
concat_list="$WORK/concat.txt"
: > "$concat_list"

# Translate `RATE` (words-per-minute) to edge-tts's percentage delta
# from its default 175 wpm baseline (negative = slower, positive =
# faster). Edge expects an explicit sign prefix.
if [[ "$ENGINE" == "edge" ]]; then
  delta=$(( (RATE - 175) * 100 / 175 ))
  if (( delta >= 0 )); then
    edge_rate="+${delta}%"
  else
    edge_rate="${delta}%"
  fi
fi

# Anchor the global audio timeline. `audio_pos` is where the
# accumulated voice track currently ends (seconds since t=0). For each
# paragraph we compute the lead-in silence as `cue_start - audio_pos`,
# clamped to a minimum of 0.10 s so paragraphs never overlap.
#
# This guarantees:
#   - audio NEVER gets truncated mid-word (no `atrim`),
#   - long paragraphs simply run past their cue boundary instead of
#     being cut, pushing subsequent paragraphs slightly later,
#   - short paragraphs get silence to fill the gap to the next cue.
#
# The drift this introduces is bounded: each subsequent cue gets a
# smaller lead-in if we're behind, so we catch up over a few cues.
audio_pos=0
MIN_GAP=0.20   # always leave at least 200 ms breathing room

for ((i = 1; i <= para_count; i++)); do
  para="$WORK/$(printf 'para-%02d.txt' "$i")"
  raw_audio="$WORK/$(printf 'para-%02d.audio' "$i")"
  wav_padded="$WORK/$(printf 'para-%02d.wav' "$i")"

  if [[ "$ENGINE" == "edge" ]]; then
    # `--rate=` not `--rate ` because some argparse builds choke on
    # negative-sign args being split off.
    edge-tts --voice "$VOICE" --rate="$edge_rate" \
             --file "$para" --write-media "$raw_audio" >/dev/null
  else
    say -v "$VOICE" -r "$RATE" -f "$para" -o "$raw_audio"
  fi

  # Measure actual TTS audio length — this is what was missing before.
  tts_len=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$raw_audio")

  cue_start="${cue_starts[$((i - 1))]}"

  # Lead-in = how much silence to insert before this paragraph so it
  # starts at (or as close as possible to) its cue marker. If we're
  # already past the cue marker, clamp to MIN_GAP so we don't try to
  # insert negative silence.
  lead=$(awk -v cs="$cue_start" -v ap="$audio_pos" -v mg="$MIN_GAP" \
    'BEGIN { l = cs - ap; if (l < mg) l = mg; printf "%.3f", l }')

  # Build the per-paragraph WAV: silence[lead] + tts_audio (verbatim,
  # no truncation). Each paragraph is FULL LENGTH; concat handles the
  # join.
  ffmpeg -hide_banner -loglevel error -y \
    -f lavfi -t "$lead" -i "anullsrc=channel_layout=stereo:sample_rate=44100" \
    -i "$raw_audio" \
    -filter_complex "[1:a]aresample=44100[tts];[0:a][tts]concat=n=2:v=0:a=1[a]" \
    -map "[a]" -c:a pcm_s16le "$wav_padded"

  # Advance the timeline cursor by lead + actual TTS length.
  audio_pos=$(awk -v ap="$audio_pos" -v l="$lead" -v t="$tts_len" \
    'BEGIN { printf "%.3f", ap + l + t }')

  # Diagnostic: visible drift so the user can see if a paragraph is
  # running past its cue (which means the script is too long for the
  # footage and the .txt needs trimming).
  drift=$(awk -v ap="$audio_pos" -v ce="${cue_starts[$i]}" \
    'BEGIN { printf "%+.2f", ap - ce }')
  echo "  · cue $i: start@${cue_start}s  tts=${tts_len}s  end@${audio_pos}s  drift=${drift}s"

  echo "file '$wav_padded'" >> "$concat_list"
done

# Concat all padded WAVs into one full-length audio track.
voice_wav="$WORK/voice.wav"
ffmpeg -hide_banner -loglevel error -y \
  -f concat -safe 0 -i "$concat_list" \
  -c copy "$voice_wav"

# ──────────────────────────────────────────────────────────────────────
# Step 4: burn SRT into the video AND mux the audio track.
echo "🎞  burning subtitles + muxing voice → $OUT"

# `subtitles=...` needs single-quoted paths and special-char escaping.
# Use a relative path inside a `cd` to avoid the libass colon-in-path
# problem on macOS.
chapter_abs="$(cd "$CHAPTER_DIR" && pwd)"

# Subtitle strategy: embed the SRT as a soft mov_text track inside
# the MP4. The default brew ffmpeg ships without libass, so the
# `subtitles=...` video filter (hard burn-in) errors out with "No
# option name". A soft track works in every modern player (QuickTime,
# VLC, browsers) and stays editable. If you really need burn-in,
# install an ffmpeg build with libass and run a separate
# `ffmpeg -i narrated.mp4 -vf subtitles=...` pass.

(
  cd "$chapter_abs"
  ffmpeg -hide_banner -loglevel error -y \
    -i "$slug.mp4" \
    -i "$voice_wav" \
    -i "$slug.srt" \
    -map 0:v:0 -map 1:a:0 -map 2:s:0 \
    -c:v copy \
    -c:a aac -b:a 192k \
    -c:s mov_text \
    -metadata:s:s:0 language=eng \
    -metadata:s:s:0 title="Chapter narration" \
    -disposition:s:0 default+forced \
    -shortest \
    "$slug.narrated.mp4"
)

kb=$(( $(stat -f%z "$OUT" 2>/dev/null || stat -c%s "$OUT") / 1024 ))
echo "✅ $OUT (${kb} KB)"
