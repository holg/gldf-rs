"""Standalone Bevy 3D camera probe.

Loads a single known GLDF (the one Act 9 of demo_slv_tria_2.py exports —
tmp/e2e-out/demo-export.gldf) directly via the welcome file-input,
navigates to 🎬 3D variants, captures EVERY browser console message into
tmp/e2e-out/bevy-coords.log, takes one screenshot per variant, and exits.

The whole point is to iterate the camera placement quickly without
re-running the full 10-act demo. After running this once, paste / share
the bevy-coords.log content and the user can tell us exactly where the
camera should go.

Pre-flight: dev server up on 8052 + tmp/e2e-out/demo-export.gldf exists
(run demo_slv_tria_2.py once to produce it).

Run from repo root:

    source ~/Documents/develeop/rust/geodb-rs/crates/geodb-py/.env_py312/bin/activate
    python tests/e2e/camera_probe.py
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from playwright.sync_api import expect, sync_playwright

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
OUT_DIR = REPO_ROOT / "tmp" / "e2e-out"
GLDF = OUT_DIR / "demo-export.gldf"
LOG_FILE = OUT_DIR / "bevy-coords.log"
URL = "http://localhost:8052/"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--headless", action="store_true")
    parser.add_argument(
        "--gldf",
        default=str(GLDF),
        help=f"Path to a GLDF to probe (default {GLDF})",
    )
    parser.add_argument(
        "--slow-mo",
        type=int,
        default=200,
        help="ms between Playwright actions",
    )
    args = parser.parse_args()

    gldf_path = Path(args.gldf)
    if not gldf_path.is_file():
        print(
            f"⚠ {gldf_path} not found.\n"
            f"   Run tests/e2e/demo_slv_tria_2.py first — Act 9 saves it to {GLDF}.",
            file=sys.stderr,
        )
        return 2

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    log_lines: list[str] = []

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=args.headless, slow_mo=args.slow_mo)
        context = browser.new_context(viewport={"width": 1440, "height": 900})
        page = context.new_page()

        def on_console(msg):
            # Capture EVERYTHING — the [CAMERA-PROBE] lines are buried
            # in noise from wgpu / Bevy bootstrap, but we want it all in
            # the log so the user can search.
            log_lines.append(f"[{msg.type}] {msg.text}")

        page.on("console", on_console)

        # Welcome → file-open.
        page.goto(URL)
        expect(page.locator('[data-testid="welcome-file-input"]')).to_be_attached(
            timeout=15_000
        )
        page.locator('[data-testid="welcome-file-input"]').set_input_files(
            str(gldf_path)
        )
        page.wait_for_timeout(2_000)

        # Jump to 3D variants.
        page.locator('[data-testid="nav-bevy-viewer"]').click()
        # Bevy bundle is lazy; give it plenty of room to spin up.
        page.wait_for_timeout(6_000)
        page.wait_for_selector("#bevy-canvas", timeout=15_000)
        page.wait_for_timeout(3_500)
        page.screenshot(path=str(OUT_DIR / "probe-bevy-default.png"))
        print(f"  📸 {(OUT_DIR / 'probe-bevy-default.png').relative_to(REPO_ROOT)}")

        # Click each chip in turn so each variant re-fires the L3D load
        # and produces its own coord dump.
        for idx in (0, 1, 2):
            chip = page.locator(f'[data-testid="bevy-variant-chip-{idx}"]')
            if chip.count() == 0:
                print(f"  · chip {idx} missing — skipping")
                continue
            chip.click()
            page.wait_for_timeout(3_000)
            shot = OUT_DIR / f"probe-bevy-variant{idx}.png"
            page.screenshot(path=str(shot))
            print(f"  📸 {shot.relative_to(REPO_ROOT)}")

        context.close()
        browser.close()

    LOG_FILE.write_text("\n".join(log_lines) + "\n", encoding="utf-8")
    print(f"\n📝 wrote {LOG_FILE.relative_to(REPO_ROOT)} ({len(log_lines)} lines)")

    # Echo just the CAMERA-PROBE lines to stdout so the user sees the
    # critical info inline without having to open the log.
    probe_lines = [line for line in log_lines if "CAMERA-PROBE" in line]
    if probe_lines:
        print("\n── CAMERA-PROBE lines (also in log file) ──")
        for line in probe_lines:
            print(line)
    else:
        print(
            "\n⚠ no [CAMERA-PROBE] lines captured — the frame system may have "
            "early-returned (no L3D parts spawned?). Check the full log.",
            file=sys.stderr,
        )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
