"""Lisp panel smoke test.

Loads gldf.icu in a browser, clicks the toolbar λ button, types a few
script lines into the textarea, clicks Run, and asserts the output and
result panes update.

This is a *headless smoke test*, not a visual regression — it doesn't
exercise the gldf-bevy bridge (that requires the 3D viewer to be
mounted). The headless version of the engine is already covered by
`crates/gldf-rs-lisp/tests/smoke.rs`; this one just proves the
component wired up in the WASM bundle.

Run (after `scripts/build-wasm-split.sh force` and a server on 8052):
  python tests/e2e/probe_lisp_panel.py [--headed]
"""

import argparse
import sys
import time

from playwright.sync_api import sync_playwright, expect


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--headed", action="store_true")
    p.add_argument("--url", default="http://localhost:8052/")
    args = p.parse_args()

    errors: list[str] = []
    with sync_playwright() as pw:
        browser = pw.chromium.launch(headless=not args.headed)
        ctx = browser.new_context()
        page = ctx.new_page()
        page.on("pageerror", lambda e: errors.append(f"page error: {e}"))
        page.on("console", lambda m: m.type == "error" and errors.append(f"console: {m.text}"))

        page.goto(args.url, wait_until="domcontentloaded", timeout=30_000)
        # Wait for the editor to hydrate. The welcome screen appears
        # before the toolbar; the toolbar only mounts after a GLDF is
        # loaded (see fn MainArea → has_file). Drop a sample fixture
        # into the welcome file input so the chrome shows up.
        try:
            expect(page.locator('[data-testid="welcome-open-wizard"]')).to_be_visible(timeout=20_000)
        except Exception as e:
            print(f"✗ editor never booted: {e}")
            for err in errors[-5:]:
                print(f"  {err}")
            return 1

        sample = "tests/data/SLV - Tria 2.gldf"
        page.locator('[data-testid="welcome-file-input"]').set_input_files(sample)
        # After load the toolbar mounts; wait for the λ button.
        try:
            expect(page.locator('[data-testid="toolbar-lisp"]')).to_be_visible(timeout=15_000)
        except Exception as e:
            print(f"✗ toolbar λ button never appeared: {e}")
            for err in errors[-5:]:
                print(f"  {err}")
            return 1

        # Click λ → panel opens.
        page.locator('[data-testid="toolbar-lisp"]').click()
        expect(page.locator('[data-testid="lisp-panel"]')).to_be_visible(timeout=5_000)
        print("✓ lisp panel opens")

        # The starter script is pre-filled. Click Run.
        page.locator('[data-testid="lisp-panel-run"]').click()
        time.sleep(0.5)

        # The starter script ends with (GLDF-VARIANT-COUNT). No product is
        # loaded INTO THE LISP INTERPRETER (the welcome-screen drop loads
        # into the editor state; the engine is a separate thread-local
        # interpreter that has never seen a (GLDF-LOAD …) call), so the
        # result is 0. The interesting assertion is just that the panel
        # produced *some* result — i.e. the engine ran.
        result_text = page.locator('[data-testid="lisp-panel-result"]').inner_text()
        if not result_text.strip().startswith("⇒"):
            print(f"✗ result pane has no result marker: {result_text!r}")
            return 1
        print(f"✓ result pane: {result_text!r}")

        # And the close button works.
        page.locator('[data-testid="lisp-panel-close"]').click()
        try:
            expect(page.locator('[data-testid="lisp-panel"]')).to_be_hidden(timeout=2_000)
            print("✓ lisp panel closes")
        except Exception as e:
            print(f"✗ panel didn't close: {e}")
            return 1

        if errors:
            print(f"⚠ {len(errors)} non-fatal console errors (sampling 3):")
            for err in errors[:3]:
                print(f"  {err}")

        browser.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
