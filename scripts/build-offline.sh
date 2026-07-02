#!/bin/bash
#
# Build the two offline-demo artifacts:
#
#   tmp/gldf-editor-offline.html        — ONE single HTML file with WASM + JS +
#                                          CSS + 4 sample GLDFs base64-inlined.
#                                          Drop it anywhere (USB stick, email,
#                                          file:///tmp/) and double-click to
#                                          prove gldf.icu runs without a server.
#                                          Now also inlines Bevy 3D viewer
#                                          (~26 MB) and the AcadLisp REPL (lives
#                                          inside the Leptos editor crate).
#                                          ~80 MB output.
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

# Bevy 3D viewer assets — lazy-loaded in dist/, inlined here so the single
# HTML can drive 3D rendering AND the Lisp REPL (which calls into Bevy via
# the `gldf_bevy_*` window bridges). The Lisp interpreter itself lives in
# the Leptos crate's WASM, but it's useless without Bevy to drive.
BEVY_JS_PATH=$(find "$DIST_DIR/bevy" -maxdepth 1 -name 'gldf_bevy-*.js' ! -name '*.br' 2>/dev/null | head -1)
BEVY_WASM_PATH=$(find "$DIST_DIR/bevy" -maxdepth 1 -name 'gldf_bevy-*_bg.wasm' ! -name '*.br' 2>/dev/null | head -1)
HAVE_BEVY=false
if [[ -n "$BEVY_JS_PATH" && -f "$BEVY_JS_PATH" && -n "$BEVY_WASM_PATH" && -f "$BEVY_WASM_PATH" ]]; then
    HAVE_BEVY=true
    BEVY_JS_NAME=$(basename "$BEVY_JS_PATH")
    BEVY_WASM_NAME=$(basename "$BEVY_WASM_PATH")
fi

echo "→ source:        $DIST_DIR"
echo "  JS:            $JS_NAME ($(du -h "$JS_PATH" | cut -f1))"
echo "  WASM:          $WASM_NAME ($(du -h "$WASM_PATH" | cut -f1))"
echo "  CSS:           $CSS_NAME ($(du -h "$CSS_PATH" | cut -f1))"
if [[ "$HAVE_BEVY" == "true" ]]; then
    echo "  Bevy JS:       $BEVY_JS_NAME ($(du -h "$BEVY_JS_PATH" | cut -f1))"
    echo "  Bevy WASM:     $BEVY_WASM_NAME ($(du -h "$BEVY_WASM_PATH" | cut -f1))"
else
    echo "  Bevy:          (not found in $DIST_DIR/bevy/ — 3D viewer will be stubbed)"
fi

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

echo "  encoding Leptos WASM → base64 ($(du -h "$WASM_PATH" | cut -f1) source)"
WASM_B64=$(b64_encode "$WASM_PATH")

BEVY_JS_CONTENT=""
BEVY_WASM_B64=""
if [[ "$HAVE_BEVY" == "true" ]]; then
    echo "  encoding Bevy WASM → base64 ($(du -h "$BEVY_WASM_PATH" | cut -f1) source)"
    BEVY_WASM_B64=$(b64_encode "$BEVY_WASM_PATH")
    BEVY_JS_CONTENT=$(cat "$BEVY_JS_PATH")
fi

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

// Bevy lazy-loader: replace the dist version (which fetches manifest.json +
// dynamic-imports the bevy bundle from the network) with an in-memory shim.
// When `__OFFLINE_BEVY_AVAILABLE__ === true` we load the inlined Bevy JS
// shim + WASM bytes; otherwise we stub the API so the editor still runs
// without 3D. This needs to override the bevy-loader.js that gets inlined
// later in <head> — that file also assigns window.loadBevyViewer, so we
// re-override AFTER it runs by reassigning on DOMContentLoaded.

let __offlineBevyLoaded = false;
let __offlineBevyLoading = false;
let __offlineBevyPromise = null;

async function __offlineLoadBevyViewer() {
    if (!window.__OFFLINE_BEVY_AVAILABLE__) {
        console.warn('[offline] 3D viewer not inlined in this build.');
        return null;
    }
    if (__offlineBevyLoaded) return;
    if (__offlineBevyLoading && __offlineBevyPromise) return __offlineBevyPromise;
    __offlineBevyLoading = true;
    console.log('[offline-bevy] Booting inlined 3D viewer…');
    __offlineBevyPromise = (async () => {
        try {
            const shimSource = document.getElementById('offline-bevy-shim').textContent;
            const shimBlob = new Blob([shimSource], { type: 'text/javascript' });
            const shimUrl = URL.createObjectURL(shimBlob);
            const bevy = await import(shimUrl);
            const bevyBytes = __offlineB64ToBytes(window.__OFFLINE_BEVY_WASM_B64__);
            // Free the giant base64 string immediately — we don't need it again.
            window.__OFFLINE_BEVY_WASM_B64__ = null;
            await bevy.default({ module_or_path: bevyBytes });
            // Attach all camera/joint/room bridges to window so the Lisp
            // panel + DevTools console can call them. Must run AFTER
            // bevy.default() (the shim's exports are unreachable until init).
            for (const name of [
                'gldf_bevy_set_camera',
                'gldf_bevy_get_camera',
                'gldf_bevy_luminaire_tilt',
                'gldf_bevy_camera_animate',
                'gldf_bevy_luminaire_animate',
                'gldf_bevy_reset_tilts',
                'gldf_bevy_set_ambient',
                'gldf_bevy_set_room_size',
                'gldf_bevy_set_surface_color',
                'gldf_bevy_set_surface_reflectance',
                'gldf_bevy_set_surface_emissive',
                'gldf_bevy_get_ambient',
                'gldf_bevy_get_room_size',
                'gldf_bevy_get_surface',
            ]) {
                if (typeof bevy[name] === 'function') {
                    window[name] = bevy[name];
                }
            }
            // Suppress browser contextmenu over the canvas (same reason as
            // the dist bevy-loader: without this, right-mouse-look gets
            // stuck because the contextmenu eats the mouseup).
            window.addEventListener('contextmenu', (ev) => {
                if (ev.target && ev.target.id === 'bevy-canvas') ev.preventDefault();
            }, true);
            bevy.run_on_canvas('#bevy-canvas');
            __offlineBevyLoaded = true;
            __offlineBevyLoading = false;
            URL.revokeObjectURL(shimUrl);
            console.log('[offline-bevy] ✅ 3D viewer running');
        } catch (e) {
            const s = String(e);
            if (s.includes('Using exceptions for control flow') || s.includes("don't mind me")) {
                __offlineBevyLoaded = true;
                __offlineBevyLoading = false;
                return;
            }
            console.error('[offline-bevy] failed:', e);
            __offlineBevyLoading = false;
            __offlineBevyPromise = null;
            throw e;
        }
    })();
    return __offlineBevyPromise;
}

// Install on window now AND re-install once the inlined bevy-loader.js
// has run (it overwrites window.loadBevyViewer with its network version,
// so we need to win the last assignment).
window.loadBevyViewer = __offlineLoadBevyViewer;
window.isBevyLoaded = () => __offlineBevyLoaded;
window.isBevyLoading = () => __offlineBevyLoading;
document.addEventListener('DOMContentLoaded', () => {
    window.loadBevyViewer = __offlineLoadBevyViewer;
    window.isBevyLoaded = () => __offlineBevyLoaded;
    window.isBevyLoading = () => __offlineBevyLoading;
});

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
    # The Bevy WASM base64 is HUGE (~36 MB); attach it to window via a
    # separate small script so V8 can free the intermediate string after
    # we decode it. The shim source goes into a non-executable <script>
    # element textContent (same trick as the Leptos shim) so the browser
    # doesn't try to parse it as JS until we Blob+import it on demand.
    echo '    <script>'
    echo "$SAMPLES_JS_BLOCK"
    if [[ "$HAVE_BEVY" == "true" ]]; then
        echo 'window.__OFFLINE_BEVY_AVAILABLE__ = true;'
    else
        echo 'window.__OFFLINE_BEVY_AVAILABLE__ = false;'
    fi
    echo "$BOOTSTRAP_PROLOGUE"
    echo '    </script>'

    if [[ "$HAVE_BEVY" == "true" ]]; then
        # Bevy WASM as base64 (~36 MB string). Separate <script> so the
        # parser can hand it off and free the source range quickly.
        echo '    <script>'
        echo "window.__OFFLINE_BEVY_WASM_B64__ = '$BEVY_WASM_B64';"
        echo '    </script>'
        # Bevy wasm-bindgen shim — same Blob-URL dance as the Leptos shim.
        echo '    <script type="x-offline-bevy-shim" id="offline-bevy-shim">'
        echo "$BEVY_JS_CONTENT"
        echo '    </script>'
    fi

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
if [[ "$HAVE_BEVY" == "true" ]]; then
    echo "      Includes Leptos editor + Bevy 3D viewer + AcadLisp REPL."
else
    echo "      Leptos editor only; 3D viewer omitted (no dist/bevy/ found)."
fi
echo "      Drag-drop any .gldf to load; 4 sample GLDFs inlined."
echo
echo "  ── Folder build ── (practical: lighter, plus the 3D viewer)"
echo "      Includes Bevy 3D viewer. Caveat: Chromium blocks file:// → file://"
echo "      fetch() so WASM init fails when opened by double-click."
echo "      How to demo:"
echo "        Firefox (any version):  open '$FOLDER_OUT/index.html'"
echo "        Anywhere else:          python3 -m http.server 8052 -d '$FOLDER_OUT'"
echo "                                open http://localhost:8052/"
