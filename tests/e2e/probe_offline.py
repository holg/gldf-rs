"""Smoke-test the offline builds (single HTML + folder) and assert the
editor's welcome screen actually renders.

Two artifacts, two access methods:
  * Single HTML — opened via file://. Should "just work" in every
    browser including Chromium because WASM is inlined as base64 and
    decoded into a Uint8Array (no fetch() needed).
  * Folder build — opened via a local HTTP server. Chromium blocks
    file:// → file:// fetch(), so the folder build's WASM init only
    works under http://. (Firefox handles file:// fetch fine, but the
    probe stays Chromium for portability.) The probe spins up a
    short-lived http.server on a random port instead of relying on a
    pre-existing server.

Run from repo root:
  python tests/e2e/probe_offline.py [--headed]
"""

import argparse
import http.server
import socketserver
import sys
import threading
from contextlib import contextmanager
from pathlib import Path

from playwright.sync_api import sync_playwright, expect


REPO_ROOT = Path(__file__).resolve().parents[2]
TMP = REPO_ROOT / "tmp"
SINGLE = TMP / "gldf-editor-offline.html"
FOLDER_DIR = TMP / "gldf-editor-offline-folder"
FOLDER_INDEX = FOLDER_DIR / "index.html"


@contextmanager
def temp_http_server(directory: Path):
    """Spin up a tiny HTTP server on a random port, yield the port, and
    shut it down on exit. Used to serve the folder build to a browser
    in a way Chromium's CORS rules accept."""
    class Handler(http.server.SimpleHTTPRequestHandler):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, directory=str(directory), **kwargs)
        def log_message(self, format, *args):  # quiet
            pass
    # Bind on port 0 → kernel picks a free one.
    with socketserver.TCPServer(("127.0.0.1", 0), Handler) as srv:
        port = srv.server_address[1]
        t = threading.Thread(target=srv.serve_forever, daemon=True)
        t.start()
        try:
            yield port
        finally:
            srv.shutdown()


def probe(playwright, url: str, label: str, headed: bool) -> bool:
    """Open `url` and assert the welcome screen rendered.

    Returns True on success. Prints the failure reason on failure so the
    main loop can keep going through the other artifact.
    """
    print(f"\n→ Probing {label}")
    print(f"   url: {url}")

    browser = playwright.chromium.launch(
        headless=not headed,
        args=[
            # Chromium blocks file:// → file:// fetch by default; the
            # folder build relies on it for relative WASM/JS loading.
            "--allow-file-access-from-files",
        ],
    )
    try:
        ctx = browser.new_context()
        page = ctx.new_page()

        # Surface console errors so a silent WASM init failure is visible.
        errors: list[str] = []
        page.on("pageerror", lambda e: errors.append(f"page error: {e}"))
        page.on("console", lambda m: m.type == "error" and errors.append(f"console error: {m.text}"))

        page.goto(url, wait_until="domcontentloaded", timeout=30_000)

        # The Leptos welcome screen renders a button with data-testid
        # `welcome-open-wizard`. If WASM init completed and Leptos
        # hydrated the page, this element appears in the DOM.
        try:
            expect(page.locator('[data-testid="welcome-open-wizard"]')).to_be_visible(timeout=20_000)
        except Exception as e:
            print(f"   ✗ welcome screen didn't render: {type(e).__name__}: {e}")
            for err in errors[-10:]:
                print(f"     {err}")
            return False

        print(f"   ✓ editor booted, welcome screen visible")
        if errors:
            print(f"   ⚠ {len(errors)} non-fatal console errors (sampling 3):")
            for err in errors[:3]:
                print(f"     {err}")
        return True
    finally:
        browser.close()


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--headed", action="store_true",
                   help="Open the browser window visibly (for manual inspection)")
    args = p.parse_args()

    if not SINGLE.exists() or not FOLDER_INDEX.exists():
        print("ERROR: offline artifacts missing. Run scripts/build-offline.sh first.",
              file=sys.stderr)
        return 2

    results = {}
    with sync_playwright() as pw:
        # The single HTML is fully self-contained — file:// is the demo.
        results["single-HTML (file://)"] = probe(
            pw, f"file://{SINGLE}", "single-HTML via file://", args.headed,
        )
        # The folder build needs HTTP for Chromium; spin up a tiny server.
        with temp_http_server(FOLDER_DIR) as port:
            results["folder (http://)"] = probe(
                pw, f"http://127.0.0.1:{port}/", "folder build via http://", args.headed,
            )

    print("\nSummary:")
    for label, ok in results.items():
        print(f"  {'✓' if ok else '✗'} {label}")
    return 0 if all(results.values()) else 1


if __name__ == "__main__":
    sys.exit(main())
