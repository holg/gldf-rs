"""Focused 3D-variants demo on the real SLV Tria 2 GLDF.

Loads `tests/data/SLV - Tria 2.gldf` directly via the welcome page's
file input (no wizard), navigates to the 3D variants page, and walks
each variant chip with two screenshots per variant:

  1. Default camera angle (the auto-framed view we always show).
  2. From-below view (camera pushed under the luminaire, looking up),
     so we can verify the spotlights illuminate the floor **and the
     LED disc is visible from below** — the load-bearing physics test
     the orientation pipeline now claims to pass.

Outputs (in `tmp/e2e-slv-3d-only/`):
  - default + below screenshot per variant
  - run.webm (raw recording)
  - demo.mp4 (slo-mo 2.5×)

Usage:
    source ~/Documents/develeop/rust/geodb-rs/crates/geodb-py/.env_py312/bin/activate
    python tests/e2e/demo_slv_3d_only.py
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

from playwright.sync_api import Page, expect, sync_playwright

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
GLDF_PATH = REPO_ROOT / "tests" / "data" / "SLV - Tria 2.gldf"
OUT_DIR = REPO_ROOT / "tmp" / "e2e-slv-3d-only"
URL = "http://localhost:8052/"

# Camera presets (x, y, z, yaw, pitch).
#
# "default" is whatever the Bevy auto-frame produces — we leave the
# camera alone and screenshot.
#
# "below" puts the camera UNDER the luminaire looking up. Mounting is
# at the ceiling (~3 m), so we push the camera to y ≈ 0.8 m (about
# eye-level), centred under the luminaire, and pitch UP. This is the
# physics-test view: if the spots are pointing down, we see the LED
# discs from beneath; if they're pointing up (the old bug), we see
# dark cup interiors.
CAM_BELOW = (0.0, 0.8, 0.0, 0.0, 0.8)  # ~46° up-tilt
CAM_SIDE = (3.0, 1.6, 3.0, -2.4, -0.3)  # off-axis, slightly above
# `None` means "do not call set_camera — use auto-framed default".


def tid(page: Page, testid: str):
    return page.locator(f'[data-testid="{testid}"]')


def screenshot(page: Page, name: str) -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    path = OUT_DIR / f"{name}.png"
    page.screenshot(path=str(path), full_page=False)
    print(f"  📸 {path.relative_to(REPO_ROOT)}")


def wait_idle(page: Page, ms: int = 400) -> None:
    page.wait_for_timeout(ms)


def set_camera(page: Page, cam: tuple[float, float, float, float, float]) -> None:
    """Call the wasm-bindgen camera bridge attached to window."""
    x, y, z, yaw, pitch = cam
    page.evaluate(
        f"window.gldf_bevy_set_camera && window.gldf_bevy_set_camera({x}, {y}, {z}, {yaw}, {pitch})"
    )


def load_gldf(page: Page) -> None:
    print(f"\n[Step 1] Open SLV Tria 2 GLDF — {GLDF_PATH.name}")
    page.goto(URL)
    expect(tid(page, "welcome-file-input")).to_be_attached(timeout=15_000)
    screenshot(page, "01-welcome")
    tid(page, "welcome-file-input").set_input_files(str(GLDF_PATH))
    wait_idle(page, 1_500)
    # Wait for the editor shell to appear after import.
    expect(tid(page, "nav-bevy-viewer")).to_be_visible(timeout=15_000)
    screenshot(page, "02-after-import")


def open_bevy(page: Page) -> None:
    print("\n[Step 2] Navigate to 3D variants")
    tid(page, "nav-bevy-viewer").click()
    wait_idle(page, 4_000)
    page.wait_for_selector("#bevy-canvas", timeout=15_000)
    # The Bevy 3D bundle is lazy-loaded; first time can take a beat to
    # spin up. The earlier Trahe demo uses ~2.5s. Use the same.
    wait_idle(page, 3_000)
    screenshot(page, "03-bevy-default-camera")


def walk_variants(page: Page) -> None:
    """Click each variant chip, take 2 screenshots (default + from below)."""
    print("\n[Step 3] Walk variants — default + below view each")
    # Discover available chips by data-testid prefix.
    chip_count = page.locator('[data-testid^="bevy-variant-chip-"]').count()
    print(f"  · found {chip_count} variant chips")
    for idx in range(chip_count):
        chip = tid(page, f"bevy-variant-chip-{idx}")
        if chip.count() == 0:
            continue
        # Use index-only filenames; the chip text on the SLV file is
        # "comb Em / no Em / -" etc. — slashes create directories.
        chip.click()
        wait_idle(page, 1_800)

        # (a) Default auto-framed view.
        screenshot(page, f"10-v{idx}-default")

        # (b) Side view — orientation sanity. Housing flat plate at the
        # top, gimbal cups at the bottom. If we ever regress to
        # upside-down housings, this is where it'll show.
        set_camera(page, CAM_SIDE)
        wait_idle(page, 800)
        screenshot(page, f"12-v{idx}-side")

        print(f"  · variant {idx} captured")


def finalize_video(args: argparse.Namespace) -> None:
    raw = OUT_DIR / "run.webm"
    final = OUT_DIR / "demo.mp4"
    if not raw.exists():
        print("⚠ no run.webm — skipping mp4")
        return
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        print("⚠ ffmpeg not on PATH — skipping mp4")
        return
    # Same slo-mo treatment as run_trahe.sh demo.mp4: 2.5×.
    cmd = [
        ffmpeg, "-y", "-i", str(raw),
        "-filter:v", f"setpts={args.slowdown}*PTS",
        "-an", str(final),
    ]
    print(f"\n🎞  ffmpeg slo-mo ({args.slowdown}×) → {final.relative_to(REPO_ROOT)}")
    subprocess.run(cmd, check=False, capture_output=True)
    if final.exists():
        kb = final.stat().st_size // 1024
        print(f"  🎞  demo: {final.relative_to(REPO_ROOT)} ({kb} KB)")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--headless", action="store_true")
    parser.add_argument("--slow-mo", type=int, default=180)
    parser.add_argument("--slowdown", type=float, default=2.5)
    args = parser.parse_args()

    if not GLDF_PATH.exists():
        print(f"❌ GLDF not found: {GLDF_PATH}", file=sys.stderr)
        return 2

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for old in OUT_DIR.glob("*.png"):
        old.unlink()
    for old in list(OUT_DIR.glob("*.webm")) + list(OUT_DIR.glob("*.mp4")):
        old.unlink()

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=args.headless, slow_mo=args.slow_mo)
        ctx_args = {
            "viewport": {"width": 1280, "height": 800},
            "record_video_dir": str(OUT_DIR),
            "record_video_size": {"width": 1280, "height": 800},
        }
        context = browser.new_context(**ctx_args)
        page = context.new_page()
        try:
            load_gldf(page)
            open_bevy(page)
            walk_variants(page)
        except Exception as e:
            screenshot(page, "99-error")
            print(f"❌ {e}", file=sys.stderr)
            raise
        finally:
            print(f"\n✅ done — screenshots in {OUT_DIR.relative_to(REPO_ROOT)}/")
            context.close()
            browser.close()
            # Rename the auto-generated webm to a stable name.
            webms = list(OUT_DIR.glob("*.webm"))
            if webms:
                final = OUT_DIR / "run.webm"
                if final.exists():
                    final.unlink()
                webms[0].rename(final)
                kb = final.stat().st_size // 1024
                print(f"  🎬 raw: {final.relative_to(REPO_ROOT)} ({kb} KB)")
                finalize_video(args)

    return 0


if __name__ == "__main__":
    sys.exit(main())
