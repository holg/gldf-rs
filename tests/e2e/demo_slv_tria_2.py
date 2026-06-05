"""Playwright e2e demo: build SLV TRIA 2 from scratch via the wizard, paint, walk 3D.

Acts:
  1. Welcome → "+ Create new GLDF" → 7-step wizard with middle.ldt +
     example_007.l3d → land on Overview.
  2. Sidebar → Variants → flip Edit Mode → duplicate variant 0 twice →
     end with 3 variants.
  3. Sidebar → Marketing → for each variant: scope-switch, add housing
     color, pick RAL (6018 / 1023 / 9003).
  4. Sidebar → 🎬 3D variants → click each variant chip in turn so the
     Bevy bundle re-renders with the new housing colour.

Records an MP4 of the run; ffmpeg slows it down for review.

Run from repo root (dev server must be live on http://localhost:8052):

    source ~/Documents/develeop/rust/geodb-rs/crates/geodb-py/.env_py312/bin/activate
    python tests/e2e/demo_slv_tria_2.py              # headed, slo-mo input
    python tests/e2e/demo_slv_tria_2.py --headless   # CI mode (no GPU → blank canvas)
    python tests/e2e/demo_slv_tria_2.py --slowdown 4 # 4× video slo-mo

Output lands in `tmp/e2e-out/`:
  - screenshots: 01-…, 02-…, etc.
  - video:       run.webm     (raw, real-time, from Playwright)
  - slo-mo MP4:  run-slow.mp4 (ffmpeg, --slowdown speed)
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

from playwright.sync_api import Page, expect, sync_playwright

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
ASSETS = REPO_ROOT / "tests" / "test-create"
OUT_DIR = REPO_ROOT / "tmp" / "e2e-out"
URL = "http://localhost:8052/"

# Variant plan: index, RAL paint, English RAL name, LDT filename it gets
# bound to, the CCT (K) for that variant's LightSource, and the Emitter
# id we'll create + bind into the variant's LEO0 slot.
#
# Variant 0 inherits the wizard's middle.ldt + lightsource_1 (whose CCT
# we'll set to 4000 K) + a new emitter we create. Variants 1 and 2 each
# get their own LDT (added on Files page), LightSource (added on Light
# Sources page), and Emitter (added on Emitters page), then we rebind
# their variant Geometry to point at the new emitter.
VARIANTS = [
    {
        "idx": 0,
        "ral": 6018,
        "ral_name": "yellow green",
        "ldt": "middle.ldt",
        "cct": 4000,
        "emitter_id": "emitter_2",   # emitter_1 is auto-created by Add
                                       # because the form prefills; we
                                       # actually add fresh as emitter_2
    },
    {
        "idx": 1,
        "ral": 1023,
        "ral_name": "traffic yellow",
        "ldt": "narrow.ldt",
        "cct": 3000,
        "emitter_id": "emitter_3",
    },
    {
        "idx": 2,
        "ral": 9003,
        "ral_name": "signal white",
        "ldt": "wide.ldt",
        "cct": 5000,
        "emitter_id": "emitter_4",
    },
]


def tid(page: Page, testid: str):
    """Locate by data-testid — every click in this script goes through here."""
    return page.locator(f'[data-testid="{testid}"]')


def screenshot(page: Page, name: str) -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    path = OUT_DIR / f"{name}.png"
    page.screenshot(path=str(path), full_page=False)
    print(f"  📸 {path.relative_to(REPO_ROOT)}")


def wait_idle(page: Page, ms: int = 400) -> None:
    """Give the Leptos reactor a beat to flush after a state-changing click."""
    page.wait_for_timeout(ms)


# ---------------------------------------------------------------------------
# Acts
# ---------------------------------------------------------------------------


def act_1_wizard(page: Page) -> None:
    """Welcome → New GLDF wizard → 7 steps with middle.ldt + example_007.l3d → Create."""
    print("\n[Act 1] Wizard — build the seed GLDF")
    page.goto(URL)
    expect(tid(page, "welcome-open-wizard")).to_be_visible(timeout=15_000)
    screenshot(page, "01-welcome")

    tid(page, "welcome-open-wizard").click()
    wait_idle(page)
    expect(tid(page, "wizard-header-manufacturer")).to_be_visible()

    # Step 1 — Header
    tid(page, "wizard-header-author").fill("Holger Trahe")
    tid(page, "wizard-header-manufacturer").fill("SLV")
    screenshot(page, "02-wizard-header")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 2 — Photometry (middle.ldt = the mid beam)
    tid(page, "wizard-photometry-input").set_input_files(str(ASSETS / "middle.ldt"))
    wait_idle(page, 700)
    screenshot(page, "03-wizard-photometry")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 3 — Light source (defaults)
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 4 — Emitter (defaults)
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 5 — Geometry (example_007.l3d)
    tid(page, "wizard-geometry-input").set_input_files(str(ASSETS / "example_007.l3d"))
    wait_idle(page, 700)
    screenshot(page, "04-wizard-geometry")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 6 — Variant (defaults)
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 7 — Review → Create
    screenshot(page, "05-wizard-review")
    tid(page, "wizard-create").click()
    wait_idle(page, 1200)

    expect(tid(page, "nav-overview")).to_be_visible()
    screenshot(page, "06-overview-after-create")


def act_2_duplicate_variants(page: Page) -> None:
    """Sidebar → Variants → flip Edit Mode → duplicate variant 0 twice."""
    print("\n[Act 2] Duplicate variants (need 3 for the housing-colour demo)")

    # Edit affordances (Duplicate/Delete/Move) only render in Edit Mode.
    # Flipping here keeps Marketing in Edit Mode too — single flip serves
    # the rest of the script.
    tid(page, "toolbar-toggle-mode").click()
    wait_idle(page)

    tid(page, "nav-variants").click()
    wait_idle(page)
    screenshot(page, "07-variants-before")

    # Always click variant 0's duplicate — copies append after, so the
    # source's index stays put across iterations.
    for n in (1, 2):
        button = tid(page, "variant-duplicate-0")
        if button.count() == 0:
            print(f"  · variant-duplicate-0 missing — stopping at variant {n - 1}")
            break
        button.click()
        wait_idle(page, 600)
        print(f"  · duplicated → variant {n}")
    screenshot(page, "08-variants-after-dup")


def act_3_extra_ldts(page: Page) -> None:
    """Files page → drop narrow.ldt + wide.ldt so we have 3 Photometries."""
    print("\n[Act 3] Drop extra LDTs on Files page")
    tid(page, "nav-files").click()
    wait_idle(page)

    # Files page in editor mode (we flipped in Act 2).
    extras = ["narrow.ldt", "wide.ldt"]
    paths = [str(ASSETS / name) for name in extras]
    tid(page, "files-add-input").set_input_files(paths)
    wait_idle(page, 1500)
    print(f"  · dropped {', '.join(extras)}")
    screenshot(page, "09-files-after-drop")


def act_4_light_sources(page: Page) -> None:
    """Light Sources page → set CCT on lightsource_1 + add 2 more Fixed."""
    print("\n[Act 4] Light Sources — set CCT per variant")
    tid(page, "nav-light-sources").click()
    wait_idle(page)
    screenshot(page, "10-lightsources-before")

    # Each variant configures one LightSource with its CCT. Variant 0
    # tunes the wizard's pre-existing lightsource_1; variants 1+2 create
    # fresh sources (which will land as lightsource_2 and lightsource_3
    # in declaration order — the editor auto-numbers).
    for i, v in enumerate(VARIANTS):
        if i == 0:
            edit_button = tid(page, "ls-edit-lightsource_1")
        else:
            tid(page, "ls-add-fixed").click()
            wait_idle(page, 800)
            # The new card lands in the list with id lightsource_<i+1>.
            edit_button = tid(page, f"ls-edit-lightsource_{i + 1}")
        if edit_button.count() == 0:
            print(f"  · ls-edit for variant {v['idx']} missing — skipping")
            continue
        edit_button.click()
        wait_idle(page, 500)
        cct_field = tid(page, "ls-edit-cct")
        cct_field.fill(str(v["cct"]))
        wait_idle(page, 200)
        tid(page, "ls-edit-save").click()
        wait_idle(page, 600)
        print(f"  · lightsource_{i + 1} → CCT {v['cct']} K")
    screenshot(page, "11-lightsources-after")


def act_5_emitters(page: Page) -> None:
    """Emitters page → create 3 emitters binding (photometry_N ↔ lightsource_N)."""
    print("\n[Act 5] Emitters — one per variant (photometry ↔ lightsource)")
    tid(page, "nav-emitters").click()
    wait_idle(page)
    screenshot(page, "12-emitters-before")

    # The wizard creates photometry_1 (middle.ldt). When the user drops
    # narrow.ldt and wide.ldt on the Files page (Act 3), each LDT drop
    # routes through AddPhotometryFile (auto-creates a <Photometry>),
    # yielding photometry_2 and photometry_3. The dropdown's option
    # values are these ids verbatim.
    photometry_ids = ["photometry_1", "photometry_2", "photometry_3"]
    light_source_ids = ["lightsource_1", "lightsource_2", "lightsource_3"]

    for i, v in enumerate(VARIANTS):
        tid(page, "emitter-add").click()
        wait_idle(page, 400)
        # Type defaults to "Fixed" so we don't need to set it.
        try:
            tid(page, "emitter-form-photometry").select_option(value=photometry_ids[i])
            tid(page, "emitter-form-lightsource").select_option(value=light_source_ids[i])
        except Exception as e:
            print(f"  · failed to wire emitter for variant {v['idx']}: {e}")
            continue
        wait_idle(page, 200)
        tid(page, "emitter-form-submit").click()
        wait_idle(page, 600)
        print(f"  · {v['emitter_id']} = {photometry_ids[i]} ↔ {light_source_ids[i]}")
    screenshot(page, "13-emitters-after")


def act_6_bind_variant_emitters(page: Page) -> None:
    """Variants page → for each variant: Edit → Model Geometry mode (auto-binds
    first emitter) → repoint that binding at the variant's own emitter."""
    print("\n[Act 6] Bind each variant to its emitter")
    tid(page, "nav-variants").click()
    wait_idle(page)
    screenshot(page, "14-variants-before-bind")

    for v in VARIANTS:
        idx = v["idx"]
        edit_button = tid(page, f"variant-edit-{idx}")
        if edit_button.count() == 0:
            print(f"  · variant-edit-{idx} missing — skipping")
            continue
        edit_button.click()
        wait_idle(page, 500)
        screenshot(page, f"14b-variant{idx}-edit-modal")

        # Re-fire Model Geometry mode. The reducer auto-adds a binding
        # pointing at the first available emitter, even if mode was
        # already Model — so we always end up with at least binding 0
        # to repoint.
        tid(page, f"variant-geometry-mode-{idx}-model").click()
        wait_idle(page, 900)
        screenshot(page, f"14c-variant{idx}-after-model-click")

        # Repoint binding 0 at THIS variant's emitter. Short timeout so
        # we fail fast (the 30s default makes Act 6 take 90s on miss).
        picker = tid(page, f"variant-binding-emitter-{idx}-0")
        try:
            picker.wait_for(state="attached", timeout=3_000)
            picker.select_option(value=v["emitter_id"])
            wait_idle(page, 400)
            print(f"  · variant {idx} → {v['emitter_id']}")
        except Exception as e:
            print(f"  · failed to pick emitter for variant {idx}: {e}")

        # Close the modal so the next variant's button is clickable.
        page.keyboard.press("Escape")
        wait_idle(page, 400)
    screenshot(page, "15-variants-after-bind")


def act_7_marketing(page: Page) -> None:
    """Marketing → for each variant: scope-switch, add housing color, pick RAL."""
    print("\n[Act 7] Marketing — paint housing colors")
    tid(page, "nav-marketing").click()
    wait_idle(page)
    screenshot(page, "16-marketing-empty")

    for v in VARIANTS:
        try:
            tid(page, "scope-picker-select").select_option(value=str(v["idx"]))
        except Exception:
            print(f"  · scope-picker has no Variant {v['idx']} — skipping")
            continue
        wait_idle(page, 400)

        tid(page, "marketing-add-housing-color").click()
        wait_idle(page, 400)

        select = tid(page, "marketing-ral-select-0")
        expect(select).to_be_visible()
        select.select_option(value=str(v["ral"]))
        wait_idle(page, 500)
        print(f"  · variant {v['idx']} → RAL {v['ral']} ({v['ral_name']})")
        screenshot(page, f"17-marketing-variant{v['idx']}-ral{v['ral']}")


def act_8_bevy(page: Page) -> None:
    """🎬 3D variants → click each chip, screenshot, watch housing + CCT swap."""
    print("\n[Act 8] 3D variants money shot")
    tid(page, "nav-bevy-viewer").click()
    wait_idle(page, 4_000)
    page.wait_for_selector("#bevy-canvas", timeout=15_000)
    wait_idle(page, 2_500)
    screenshot(page, "18-bevy-default")

    for v in VARIANTS:
        chip = tid(page, f"bevy-variant-chip-{v['idx']}")
        if chip.count() == 0:
            print(f"  · variant chip {v['idx']} missing — skipping")
            continue
        chip.click()
        wait_idle(page, 1500)
        print(f"  · variant {v['idx']} ({v['ral_name']}, RAL {v['ral']}, {v['cct']} K) rendering")
        screenshot(page, f"19-bevy-variant{v['idx']}-ral{v['ral']}")


# Path the e2e saves the exported GLDF to so Act 10 can re-import it.
EXPORTED_GLDF = OUT_DIR / "demo-export.gldf"


def act_9_export(page: Page) -> Path | None:
    """Click Export GLDF → intercept the browser download → save to tmp/e2e-out/."""
    print("\n[Act 9] Export GLDF")
    # Drive the toolbar from the 3D viewer page (no need to navigate
    # away — the export button is always in the toolbar). Use
    # expect_download as a context manager so we don't miss the event.
    with page.expect_download(timeout=15_000) as dl_info:
        tid(page, "toolbar-export-gldf").click()
    download = dl_info.value
    download.save_as(str(EXPORTED_GLDF))
    size = EXPORTED_GLDF.stat().st_size
    print(f"  · saved {EXPORTED_GLDF.relative_to(REPO_ROOT)} ({size // 1024} KB)")
    screenshot(page, "20-after-export")
    return EXPORTED_GLDF


def act_10_reimport(page: Page, gldf_path: Path) -> None:
    """Clear → drop the saved GLDF back in → click through every page to
    prove the wiring survived round-trip."""
    print("\n[Act 10] Re-import + click-through")

    # Confirm dialog from Clear.
    page.on("dialog", lambda d: d.accept())
    tid(page, "toolbar-clear").click()
    wait_idle(page, 800)
    expect(tid(page, "welcome-open-wizard")).to_be_visible(timeout=10_000)
    screenshot(page, "21-after-clear")

    # File-open the GLDF we just saved.
    tid(page, "welcome-file-input").set_input_files(str(gldf_path))
    wait_idle(page, 2_000)
    expect(tid(page, "nav-overview")).to_be_visible()
    screenshot(page, "22-reimport-overview")
    print("  · re-imported, on Overview")

    # Click through every sidebar page that should show our wiring.
    page_walks = [
        ("nav-files", "23-reimport-files"),
        ("nav-light-sources", "24-reimport-lightsources"),
        ("nav-emitters", "25-reimport-emitters"),
        ("nav-variants", "26-reimport-variants"),
        ("nav-marketing", "27-reimport-marketing"),
        ("nav-linkage", "28-reimport-linkage"),
        ("nav-bevy-viewer", "29-reimport-bevy"),
    ]
    for nav_id, shot_name in page_walks:
        tid(page, nav_id).click()
        wait_idle(page, 1_500)
        # Give Bevy extra time to spin its WebGL context up after a
        # cold-load (the bundle is cached but the scene rebuilds).
        if nav_id == "nav-bevy-viewer":
            wait_idle(page, 3_000)
        screenshot(page, shot_name)
        print(f"  · {nav_id} ✓")

    # Final beat: click each Bevy chip on the re-imported file to
    # verify per-variant state really persisted.
    for v in VARIANTS:
        chip = tid(page, f"bevy-variant-chip-{v['idx']}")
        if chip.count() == 0:
            print(f"  · re-import: chip {v['idx']} missing — skipping")
            continue
        chip.click()
        wait_idle(page, 1_500)
        screenshot(page, f"30-reimport-bevy-variant{v['idx']}-ral{v['ral']}")
        print(f"  · re-import: variant {v['idx']} renders")


# ---------------------------------------------------------------------------
# Video post-process
# ---------------------------------------------------------------------------


def slow_down_video(raw: Path, dst: Path, factor: float) -> bool:
    """ffmpeg-slow the raw WebM into an MP4. Returns False if ffmpeg missing."""
    if not shutil.which("ffmpeg"):
        print("⚠ ffmpeg not on PATH — skipping slo-mo MP4")
        return False
    if not raw.exists():
        print(f"⚠ raw video missing at {raw}")
        return False
    # setpts multiplies the per-frame timestamp, which stretches the
    # timeline without re-encoding frames. -an because there's no audio.
    cmd = [
        "ffmpeg",
        "-y",
        "-i",
        str(raw),
        "-filter:v",
        f"setpts={factor}*PTS",
        "-an",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-crf",
        "23",
        str(dst),
    ]
    print(f"\n🎞  ffmpeg slo-mo ({factor}×) → {dst.relative_to(REPO_ROOT)}")
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(result.stderr[-800:], file=sys.stderr)
        return False
    return True


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--headless", action="store_true")
    parser.add_argument(
        "--slow-mo",
        type=int,
        default=400,
        help="ms between Playwright actions (default 400 — easier to follow on video)",
    )
    parser.add_argument(
        "--slowdown",
        type=float,
        default=3.0,
        help="ffmpeg playback slowdown factor (default 3 = 3× slower MP4)",
    )
    parser.add_argument(
        "--no-video", action="store_true", help="Skip video recording entirely"
    )
    parser.add_argument(
        "--trace", action="store_true", help="Save Playwright trace.zip"
    )
    parser.add_argument("--url", default=URL)
    args = parser.parse_args()

    # Pre-flight: server up + GLDF present.
    import urllib.request

    try:
        with urllib.request.urlopen(args.url, timeout=2) as resp:
            if resp.status != 200:
                print(f"⚠ {args.url} returned HTTP {resp.status}", file=sys.stderr)
                return 2
    except Exception as e:
        print(f"⚠ {args.url} unreachable: {e}", file=sys.stderr)
        return 2

    for fname in ("middle.ldt", "narrow.ldt", "wide.ldt", "example_007.l3d"):
        if not (ASSETS / fname).is_file():
            print(f"⚠ missing {ASSETS / fname}", file=sys.stderr)
            return 2

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    # Wipe stale screenshots/videos so the output dir reflects this run only.
    for old in OUT_DIR.glob("*.png"):
        old.unlink()
    for old in list(OUT_DIR.glob("*.webm")) + list(OUT_DIR.glob("*.mp4")):
        old.unlink()

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=args.headless, slow_mo=args.slow_mo)
        context_args: dict = {"viewport": {"width": 1440, "height": 900}}
        if not args.no_video:
            context_args["record_video_dir"] = str(OUT_DIR)
            context_args["record_video_size"] = {"width": 1440, "height": 900}
        context = browser.new_context(**context_args)
        if args.trace:
            context.tracing.start(screenshots=True, snapshots=True, sources=False)
        page = context.new_page()
        page.on(
            "console",
            lambda msg: (
                print(f"  [browser:{msg.type}] {msg.text}")
                if msg.type in ("error", "warning")
                else None
            ),
        )

        try:
            act_1_wizard(page)
            act_2_duplicate_variants(page)
            act_3_extra_ldts(page)
            act_4_light_sources(page)
            act_5_emitters(page)
            act_6_bind_variant_emitters(page)
            act_7_marketing(page)
            act_8_bevy(page)
            gldf_path = act_9_export(page)
            if gldf_path is not None:
                act_10_reimport(page, gldf_path)
            rc = 0
            print(f"\n✅ done — screenshots in {OUT_DIR.relative_to(REPO_ROOT)}/")
        except Exception as e:
            print(f"\n❌ failed: {e}", file=sys.stderr)
            screenshot(page, "99-error")
            rc = 1
        finally:
            if args.trace:
                context.tracing.stop(path=str(OUT_DIR / "trace.zip"))
                print(f"  📦 trace at {(OUT_DIR / 'trace.zip').relative_to(REPO_ROOT)}")
            video_obj = page.video if not args.no_video else None
            context.close()  # finalises the .webm on disk
            browser.close()

            # Playwright names videos with a random UUID; rename to run.webm.
            if video_obj is not None:
                try:
                    raw_path = Path(video_obj.path())
                    final_raw = OUT_DIR / "run.webm"
                    if raw_path.exists() and raw_path != final_raw:
                        shutil.move(str(raw_path), str(final_raw))
                    if final_raw.exists():
                        print(
                            f"  🎬 raw video: {final_raw.relative_to(REPO_ROOT)} "
                            f"({final_raw.stat().st_size // 1024} KB)"
                        )
                        if args.slowdown > 1.0:
                            slow_down_video(
                                final_raw, OUT_DIR / "run-slow.mp4", args.slowdown
                            )
                except Exception as e:
                    print(f"⚠ video post-process failed: {e}", file=sys.stderr)

        return rc


if __name__ == "__main__":
    raise SystemExit(main())
