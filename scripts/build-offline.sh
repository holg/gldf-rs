#!/bin/bash
#
# Build the two offline-demo artifacts:
#
#   tmp/gldf-editor-offline.html        — ONE single HTML file with WASM + JS +
#                                          CSS + 4 sample GLDFs base64-inlined.
#                                          Drop it anywhere (USB stick, email,
#                                          file:///tmp/) and double-click to
#                                          prove gldf.icu runs without a server.
#                                          Bevy 3D viewer is omitted (~26 MB);
#                                          Leptos editor only. ~40 MB output.
#
#   tmp/gldf-editor-offline-folder/     — Path-rewritten copy of dist/. Open
#                                          index.html via file:// — the
#                                          browser fetches WASM / JS / CSS
#                                          from the local filesystem so the
#                                          page loads fast AND the Bevy 3D
#                                          viewer also works.
#
# Inputs:
#   - crates/gldf-rs-wasm/dist/         (run `scripts/build-wasm-split.sh` first)
#
# Run:
#   bash scripts/build-offline.sh
#
# The two artifacts answer different framings of "it works offline":
#   * Single HTML  — the demo of incredulity. ONE file, no folder, no server.
#   * Folder       — practical day-to-day "host on a USB stick / NAS share".

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
DIST_DIR="$ROOT_DIR/crates/gldf-rs-wasm/dist"
OUT_DIR="$ROOT_DIR/tmp"
FOLDER_OUT="$OUT_DIR/gldf-editor-offline-folder"
HTML_OUT="$OUT_DIR/gldf-editor-offline.html"

# Sample GLDFs ride along in both builds. Order matters — first one becomes
# the welcome page's primary sample, the others are the smaller cards.
SAMPLES=(
    "slv_tria_2.gldf"
    "biolux_hcl.gldf"
    "FLOODLIGHT_MAX_1200W_757_SYM_10_WAL_enriched.gldf"
    "LEDVANCE_FLOODLIGHT_MAX.gldf"
)

# ──────────────────────────────────────────────────────────────────────────
# Pre-flight

if [[ ! -d "$DIST_DIR" ]]; then
    echo "ERROR: $DIST_DIR doesn't exist." >&2
    echo "Run scripts/build-wasm-split.sh first to produce the regular bundle." >&2
    exit 2
fi

# Find hashed filenames. Trunk bumps these on every rebuild, so we discover
# them at script time instead of hardcoding.
INDEX_HTML="$DIST_DIR/index.html"
[[ -f "$INDEX_HTML" ]] || { echo "ERROR: $INDEX_HTML missing" >&2; exit 2; }

# Hash patterns: trunk emits `<basename>-<hex>.<ext>` (or `<basename>-<hex>_bg.wasm`).
JS_PATH=$(find "$DIST_DIR" -maxdepth 1 -name 'gldf_rs_wasm-*.js' ! -name '*.br' | head -1)
WASM_PATH=$(find "$DIST_DIR" -maxdepth 1 -name 'gldf_rs_wasm-*_bg.wasm' ! -name '*.br' | head -1)
CSS_PATH=$(find "$DIST_DIR" -maxdepth 1 -name 'styles-*.css' ! -name '*.br' | head -1)

for f in "$JS_PATH" "$WASM_PATH" "$CSS_PATH"; do
    [[ -n "$f" && -f "$f" ]] || { echo "ERROR: missing dist asset (js/wasm/css)" >&2; exit 2; }
done

JS_NAME=$(basename "$JS_PATH")
WASM_NAME=$(basename "$WASM_PATH")
CSS_NAME=$(basename "$CSS_PATH")

echo "→ source:        $DIST_DIR"
echo "  JS:            $JS_NAME ($(du -h "$JS_PATH" | cut -f1))"
echo "  WASM:          $WASM_NAME ($(du -h "$WASM_PATH" | cut -f1))"
echo "  CSS:           $CSS_NAME ($(du -h "$CSS_PATH" | cut -f1))"

mkdir -p "$OUT_DIR"

# ──────────────────────────────────────────────────────────────────────────
# 1 — Folder build (faster, smaller, includes Bevy 3D viewer)
#
# This is the "host on a USB stick" variant. Trunk emits absolute "/" paths
# (intended for a real web server's docroot); we rewrite them to relative
# paths so the browser can open index.html via file:// and find every asset
# next to it. Drop the .br files (Brotli pre-compressed alternates) since
# file:// never negotiates Accept-Encoding.

echo
echo "→ Folder build → $FOLDER_OUT"
rm -rf "$FOLDER_OUT"
mkdir -p "$FOLDER_OUT"

# rsync to preserve the bevy/ subdir; exclude .br files.
rsync -a --exclude='*.br' "$DIST_DIR/" "$FOLDER_OUT/"

# Rewrite the 4 absolute "/" references in index.html to relative paths.
# We use sed -i with a temp suffix for macOS compatibility (BSD sed needs an
# explicit empty argument after -i; we use ".bak" then delete to be portable).
INDEX_FOLDER="$FOLDER_OUT/index.html"
sed -i.bak \
    -e "s|href=\"/$CSS_NAME\"|href=\"./$CSS_NAME\"|g" \
    -e "s|module_or_path: '/$WASM_NAME'|module_or_path: './$WASM_NAME'|g" \
    -e "s|href=\"/$JS_NAME\"|href=\"./$JS_NAME\"|g" \
    -e "s|href=\"/$WASM_NAME\"|href=\"./$WASM_NAME\"|g" \
    -e "s|import init, \\* as bindings from '/$JS_NAME'|import init, * as bindings from './$JS_NAME'|g" \
    "$INDEX_FOLDER"
rm "$INDEX_FOLDER.bak"

FOLDER_SIZE=$(du -sh "$FOLDER_OUT" | cut -f1)
echo "  ✓ $FOLDER_OUT ($FOLDER_SIZE)"
echo "  open via: file://$FOLDER_OUT/index.html"

# ──────────────────────────────────────────────────────────────────────────
# 2 — Single HTML (Leptos editor only — Bevy 3D viewer dropped)
#
# Strategy: take dist/index.html and replace every external reference with
# inlined content:
#
#   <link href="/styles-…">       → <style>…</style>
#   import * from '/gldf_rs_wasm-….js' → inline the JS as a module
#   init({ module_or_path: '/…_bg.wasm' }) → init({ Uint8Array.from(base64) })
#   <link modulepreload/preload>  → drop (the inline init doesn't need them)
#   Welcome's fetch_bytes(name)   → intercept fetch() to return base64 samples
#
# Bevy 3D viewer is omitted: its loader.js fetches ./bevy/manifest.json and
# dynamically imports a second WASM bundle. Inlining 26 MB more WASM PLUS
# rewiring the dynamic import would balloon the file to ~80 MB. The single-
# HTML build's purpose is the offline-proof demo of the editor, which doesn't
# need 3D rendering.

echo
echo "→ Single HTML build → $HTML_OUT"

# Encode binaries to base64. Use macOS-compatible flags ("-b 0" forces a
# single line, no wrapping). On Linux, base64 wraps at 76 chars by default,
# which produces a valid string but adds newlines we'd then have to strip;
# detect the platform and pick the right invocation.
b64_encode() {
    # macOS BSD base64 takes input via -i, doesn't recognise -w; line
    # wrapping is controlled by -b N (0 = no wrapping). GNU base64
    # takes a positional filename and -w N for wrapping. Detect by
    # asking the binary.
    if base64 --help 2>&1 | grep -q '\-b num'; then
        base64 -b 0 -i "$1"
    else
        base64 -w 0 "$1"
    fi
}

echo "  encoding WASM → base64 ($(du -h "$WASM_PATH" | cut -f1) source)"
WASM_B64=$(b64_encode "$WASM_PATH")

# Build the JS module that:
#   1. Inlines the JS shim from dist/.
#   2. Wraps init() in a small bootstrap that decodes base64 → Uint8Array.
#   3. Overrides fetch() so the welcome screen's sample-load buttons
#      resolve in-memory instead of hitting the network.
# We read the original JS as-is, then prepend a fetch() shim that serves
# the inlined sample bytes for any request whose path ends in one of the
# four sample filenames.

JS_CONTENT=$(cat "$JS_PATH")
CSS_CONTENT=$(cat "$CSS_PATH")

# Encode each sample to base64. Build a JS object literal mapping filename
# → base64 string. We use heredocs + printf to compose the JS so we don't
# fight bash quoting around the (potentially large) base64 strings.
SAMPLES_JS_BLOCK=$(
    echo "const __OFFLINE_SAMPLES__ = {"
    for name in "${SAMPLES[@]}"; do
        src="$DIST_DIR/$name"
        if [[ ! -f "$src" ]]; then
            echo "// missing: $name (skipped)"
            continue
        fi
        echo "  // $(du -h "$src" | cut -f1) source"
        # JS object key MUST be a quoted string (the filename contains
        # dots / underscores that break bare-identifier syntax). The
        # value is a base64 string — also string-quoted. base64 output
        # is plain ASCII (alnum + / + + + =) with no escapes needed, so
        # straight double-quoting is safe.
        printf '  "%s": "%s",\n' "$name" "$(b64_encode "$src")"
    done
    echo "};"
)

# Compose the bootstrap script. We assemble it via a heredoc so the embedded
# JS reads naturally. The base64 strings get pasted in via shell expansion
# (heredoc-with-quoted-EOF disables interpolation; we want interpolation so
# the WASM bytes land).
echo "  composing single HTML ($(echo -n "$WASM_B64" | wc -c | tr -d ' ') bytes of base64 alone)"

# Helper: decode base64 → Uint8Array. Browser-native atob is single-byte; we
# convert to a typed array. This runs at page load before init().
BOOTSTRAP_PROLOGUE=$(cat <<'PROLOGUE'
// === Offline single-HTML bootstrap ===
//
// The Leptos editor is fully inlined below. This bootstrap:
//   (1) Decodes the base64-encoded WASM into a Uint8Array.
//   (2) Decodes the base64-encoded sample GLDFs and registers them on
//       a small in-memory map so the welcome screen's "Load sample"
//       buttons resolve via an intercepted fetch() rather than the
//       network.
//   (3) Disables the 3D variant viewer's loader — that bundle isn't
//       inlined here; users who need 3D should run the regular gldf.icu
//       hosted build or the offline FOLDER build (which ships Bevy too).
//
// Everything runs in the browser. There is no network call; you can
// unplug the cable, open this file from a USB stick, and the editor
// works identically.

function __offlineB64ToBytes(b64) {
    const bin = atob(b64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return bytes;
}

// Intercept fetch() so the welcome screen's load-sample buttons (which
// call `reqwest::get("slv_tria_2.gldf")` etc.) resolve to inlined bytes
// instead of trying to GET the file from a (nonexistent) server.
const __OFFLINE_REAL_FETCH = window.fetch.bind(window);
window.fetch = function offlineFetch(input, init) {
    const url = typeof input === 'string' ? input : (input && input.url) || '';
    for (const name in __OFFLINE_SAMPLES__) {
        if (url.endsWith(name)) {
            const bytes = __offlineB64ToBytes(__OFFLINE_SAMPLES__[name]);
            return Promise.resolve(new Response(bytes, {
                status: 200,
                headers: { 'Content-Type': 'application/octet-stream' },
            }));
        }
    }
    return __OFFLINE_REAL_FETCH(input, init);
};

// Tell the Bevy lazy-loader (which lives in the inlined main page script)
// to short-circuit: the 3D viewer module isn't inlined in single-HTML mode.
window.loadBevyViewer = async () => {
    console.warn('[offline] 3D viewer disabled in single-HTML build. Use the folder build for 3D.');
    return null;
};
window.isBevyLoaded = () => false;
window.isBevyLoading = () => false;

PROLOGUE
)

# Construct the final HTML. The strategy:
#   - Drop the entire <head> section's <link>/<script type="module"> trunk
#     emitted code and replace with our own inline <style>, inline JS shim,
#     and inline WASM init.
#   - Keep the <body> + the bevy-loader.js inline block (it's needed for
#     the page to render even without Bevy; our prologue disables the
#     load).
#
# We anchor on the trunk-emitted markers:
#   <link rel="stylesheet" href="/styles-…"
#   <script type="module">…</script>
#   <link rel="modulepreload" …><link rel="preload" …></head>
# These exact strings exist in the dist index.html.

# Read the original index.html once.
ORIG_HTML=$(cat "$INDEX_HTML")

# Build offline-index.html in-memory via a temp file then move atomic.
TMP_HTML="$HTML_OUT.tmp"
{
    # Header up to (but not including) the trunk stylesheet link. We slice
    # to the <head>'s opening so the title/meta/etc. come through.
    sed -n '1,/<link rel="stylesheet"/p' <<< "$ORIG_HTML" | sed '$d'

    # Inline CSS.
    echo '    <style>'
    echo "$CSS_CONTENT"
    echo '    </style>'

    # Inline our bootstrap script and the sample base64 map.
    echo '    <script>'
    echo "$SAMPLES_JS_BLOCK"
    echo "$BOOTSTRAP_PROLOGUE"
    echo '    </script>'

    # The wasm-bindgen JS shim is an ES module (top-level `export`
    # statements) so we can't paste it into a normal <script>. We
    # serialise it into a same-document Blob and dynamic-import from
    # the resulting blob: URL — the browser treats that exactly like
    # any other module load. No network round-trip; the blob lives in
    # memory.
    #
    # The shim ID is also written into <script id> so the bootstrap can
    # find the source verbatim — `textContent` preserves whitespace and
    # newlines without any HTML-entity escaping (only `</script>` would
    # be an issue, but the wasm-bindgen output doesn't contain it).
    echo "    <script type=\"x-offline-wasm-bindgen-shim\" id=\"offline-shim\">"
    echo "$JS_CONTENT"
    echo '    </script>'

    # Now the bootstrap: read the shim element's textContent, blob it,
    # dynamic-import it, then call init() with the inlined WASM bytes.
    echo '    <script type="module">'
    echo '    const shimSource = document.getElementById("offline-shim").textContent;'
    echo '    const shimBlob = new Blob([shimSource], { type: "text/javascript" });'
    echo '    const shimUrl = URL.createObjectURL(shimBlob);'
    echo '    const shim = await import(shimUrl);'
    echo "    const __wasmBytes = __offlineB64ToBytes('$WASM_B64');"
    echo '    const wasm = await shim.default({ module_or_path: __wasmBytes });'
    echo '    window.wasmBindings = shim;'
    echo '    dispatchEvent(new CustomEvent("TrunkApplicationStarted", { detail: { wasm } }));'
    echo '    URL.revokeObjectURL(shimUrl);'
    echo '    </script>'

    # The trunk-emitted modulepreload/preload links are net useless here
    # (the WASM is inlined). Slurp from the inline bevy-loader block to
    # </html>, then strip any stray <link rel="modulepreload"> or
    # <link rel="preload"> with absolute /-prefixed hrefs that would
    # otherwise cause the browser to hit the network on file://. We do
    # the strip with sed: drop the entire tag on the line that contains
    # it.
    awk '/<!-- Inline bevy-loader.js/,/^<\/html>$/' <<< "$ORIG_HTML" \
        | sed -E 's|<link rel="modulepreload"[^>]*>||g; s|<link rel="preload"[^>]*>||g'
} > "$TMP_HTML"

mv "$TMP_HTML" "$HTML_OUT"

HTML_SIZE=$(du -h "$HTML_OUT" | cut -f1)
echo "  ✓ $HTML_OUT ($HTML_SIZE)"
echo "  open via: file://$HTML_OUT"

# ──────────────────────────────────────────────────────────────────────────

echo
echo "Done. Demo this with no internet:"
echo
echo "  ── Single-HTML build ── (THE incredulity-demo: ONE file, ZERO infra)"
echo "  open '$HTML_OUT'"
echo "      Works in every browser via file://."
echo "      Leptos editor only; 3D viewer omitted."
echo "      Drag-drop any .gldf to load; 4 sample GLDFs inlined."
echo
echo "  ── Folder build ── (practical: lighter, plus the 3D viewer)"
echo "      Includes Bevy 3D viewer. Caveat: Chromium blocks file:// → file://"
echo "      fetch() so WASM init fails when opened by double-click."
echo "      How to demo:"
echo "        Firefox (any version):  open '$FOLDER_OUT/index.html'"
echo "        Anywhere else:          python3 -m http.server 8052 -d '$FOLDER_OUT'"
echo "                                open http://localhost:8052/"
