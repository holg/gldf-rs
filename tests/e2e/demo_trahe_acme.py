"""Playwright e2e: full Trahe ACME 'Lumina Pro' demo.

Builds a richly-populated GLDF from scratch — wizard, every editor pane
filled, 3 variants with distinct housing colours + CCTs, export, re-
import, walk every read view to prove the data round-trips. Auto-saves
`tmp/e2e-trahe-out/demo.mp4` (slo-mo) at the end.

Hybrid selectors:
  - Form inputs prefer existing `id="..."` attrs (electrical, mechanical,
    emergency, header) and `data-testid` where we added them (wizard,
    sidebar nav, marketing, Bevy chips, light-source CCT/name/save,
    variant duplicate/edit/geometry-mode/binding-picker).
  - Action buttons use `get_by_role("button", name=…)` with literal text.
  - Approval marks / design awards: use `input[placeholder='…']` since
    the rows have distinctive placeholder text.
  - Read views: navigate + screenshot — no assertions, just visual.

The visuals reuse the SLV TRIA 2 photometry (middle/narrow/wide LDTs)
and L3D model — only metadata is rebranded as Trahe ACME / Lumina Pro.

Run from repo root (server up on http://localhost:8052/):

    source ~/Documents/develeop/rust/geodb-rs/crates/geodb-py/.env_py312/bin/activate
    python tests/e2e/demo_trahe_acme.py             # headed, slo-mo input
    python tests/e2e/demo_trahe_acme.py --headless  # CI
    python tests/e2e/demo_trahe_acme.py --slowdown 4

Outputs in tmp/e2e-trahe-out/:
  - demo.mp4    (slo-mo, ffmpeg)
  - run.webm    (raw, real-time)
  - 01-*.png … 54-*.png screenshots
  - lumina-pro.gldf (exported file)
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
OUT_DIR = REPO_ROOT / "tmp" / "e2e-trahe-out"
URL = "http://localhost:8052/"
EXPORTED_GLDF = OUT_DIR / "lumina-pro.gldf"

# Variant fixtures.
#
# Each normal variant binds the L3D's TWO physical head LEOs (LEO0 = left,
# LEO1 = right) with the per-variant photometry pair the user specified:
#   * V0 (yellow green): narrow on LEO0, wide on LEO1
#   * V1 (traffic yellow): wide on LEO0, narrow on LEO1
#   * V2 (signal white): middle on both LEOs
# Each variant therefore needs TWO Emitters (one per binding) so each can
# carry its own PhotometryReference; this is the XSD-clean shape.
#
# Variant 3 ("Emergency only") binds the centre <SensorObject partName="Sensor">
# with a third photometry (sensor.ldt) and emergency_behavior=EmergencyOnly
# so Bevy dims the spot to ~60 lm via the existing 0.1× emergency multiplier.
#
# `bindings` shape: list of dicts with
#   `leo_name` — the L3D partName the Bevy loader matches against
#                (LEO0 / LEO1 / Sensor — used as external_name in the
#                Variant geometry binding so emitter.name == config.target_name)
#   `emitter_id` — Emitter created in Act 6
#   `photometry_id` — Photometry resolved by Act 8b id-rename
VARIANTS = [
    {
        "idx": 0,
        "ral": 6018,
        "ral_name": "yellow green",
        # display_name = the on-screen <Variant><Name> shown in the
        # variants list, overview chips, 3D viewer picker, and
        # downstream catalogs. Act 7's DuplicateVariant clones the
        # source name + " (copy)" — Act 8c overwrites all four with the
        # semantic names below via SetVariantName.
        "display_name": "Lumina Pro 4000K Yellow Green",
        "cct": 4000,
        "flux": 1600,
        "weight_kg": 0.379,
        "ls_name": "LED 4000K module",
        "ls_id_semantic": "ls_main_4000k_neutral",
        "ls_power_w": 18.5,
        # The wizard's emitter draft is skipped in Act 1, so Act 6
        # creates fresh emitters from scratch. lightsource_1 is the
        # wizard's; lightsource_{2..4} land in Act 3.
        # Semantic id assigned in Act 3 after the LS rename. Bindings
        # downstream (Act 6 emitters, equipment, etc.) reference THIS
        # id, not the auto-generated `lightsource_N`.
        "lightsource_id": "ls_main_4000k_neutral",
        "emergency_behavior": None,
        "bindings": [
            # leo_name pins the binding to a partName in the L3D
            # (LightEmittingObject "LEO0"/"LEO1" or SensorObject "Sensor").
            # emitter_auto_id is the auto-generated emitter id Act 6 mints
            # via AddEmitterWithChild; Act 8c renames it but bindings
            # cascade so the link survives.
            # photometry_auto_id is the LIVE photometry id at Act 6 time.
            # Act 3 has already renamed `photometry_N → phot_<name>` by
            # then, so we use the SEMANTIC ids here (not the auto-mint
            # `photometry_N`). The lib's RenamePhotometry cascade does
            # NOT retro-fix references that don't exist yet, so Act 6
            # must bind to the live ids at dispatch time.
            #
            # Wizard-first file order:
            #   middle.ldt    → photometry_1 → phot_middle    (wizard Step 2)
            #   emergency.ldt → photometry_2 → phot_emergency (wizard Step 4 — NEW)
            #   narrow.ldt    → photometry_3 → phot_narrow    (Act 2 drop)
            #   wide.ldt      → photometry_4 → phot_wide      (Act 2 drop)
            # sensor.ldt lands as sensorFile_1 (no Photometry minted).
            {"leo_name": "LEO0", "emitter_auto_id": "emitter_1", "photometry_auto_id": "phot_narrow"},
            {"leo_name": "LEO1", "emitter_auto_id": "emitter_2", "photometry_auto_id": "phot_wide"},
        ],
    },
    {
        "idx": 1,
        "ral": 1023,
        "ral_name": "traffic yellow",
        "display_name": "Lumina Pro 3000K Traffic Yellow",
        "cct": 3000,
        "flux": 1800,
        "weight_kg": 0.479,
        "ls_name": "LED 3000K module",
        "ls_id_semantic": "ls_main_3000k_warm",
        "ls_power_w": 18.5,
        "lightsource_id": "ls_main_3000k_warm",
        "emergency_behavior": None,
        "bindings": [
            # V1 swaps the L/R beams vs V0 (wide-left, narrow-right) so
            # the variant viewer shows two distinct distributions.
            {"leo_name": "LEO0", "emitter_auto_id": "emitter_3", "photometry_auto_id": "phot_wide"},
            {"leo_name": "LEO1", "emitter_auto_id": "emitter_4", "photometry_auto_id": "phot_narrow"},
        ],
    },
    {
        "idx": 2,
        "ral": 9003,
        "ral_name": "signal white",
        "display_name": "Lumina Pro 5000K Signal White",
        "cct": 5000,
        "flux": 2200,
        "weight_kg": 0.479,
        "ls_name": "LED 5000K module",
        "ls_id_semantic": "ls_main_5000k_cool",
        "ls_power_w": 18.5,
        "lightsource_id": "ls_main_5000k_cool",
        "emergency_behavior": None,
        "bindings": [
            # Both heads carry the symmetric middle.ldt (38° spread).
            {"leo_name": "LEO0", "emitter_auto_id": "emitter_5", "photometry_auto_id": "phot_middle"},
            {"leo_name": "LEO1", "emitter_auto_id": "emitter_6", "photometry_auto_id": "phot_middle"},
        ],
    },
    {
        "idx": 3,
        "ral": 9005,  # jet black housing for the discreet emergency-only SKU
        "ral_name": "jet black",
        "display_name": "Lumina Pro Emergency Jet Black",
        "cct": 3000,
        # Flux matches emergency.ldt's declared total luminous flux
        # (45 lm — synthesised at ~8% of middle.ldt's 562 lm to match
        # EN 1838 corridor-lighting ratios for 3 h battery-pack
        # downlights). gldf-bevy's 0.1× emergency multiplier on the LED
        # disc gives the visual cue that this is the dimmed mode; the
        # photometric truth lives in the LDT.
        "flux": 45,
        "weight_kg": 0.479,
        "ls_name": "LED Emergency 3000K",
        "ls_id_semantic": "ls_emergency_3000k",
        # Emergency LEDs are small standby modules, typically 2–5 W
        # (not the 18 W of a mains downlight). Three watts is the
        # realistic value for a 600 lm emergency LED on battery backup.
        "ls_power_w": 3.0,
        "lightsource_id": "ls_emergency_3000k",
        "emergency_behavior": "EmergencyOnly",
        "bindings": [
            # Bind the centre LEO to the wizard-minted emergency
            # photometry (45 lm, same 38° beam as middle.ldt). Act 3 has
            # already renamed `photometry_2 → phot_emergency` by Act 6
            # time, so we use the semantic id directly.
            {"leo_name": "Sensor", "emitter_auto_id": "emitter_7", "photometry_auto_id": "phot_emergency"},
        ],
    },
]


# ──────────────────────────────────────────────────────────────────────
# Tiny helpers


def tid(page: Page, testid: str):
    return page.locator(f'[data-testid="{testid}"]')


def by_id(page: Page, html_id: str):
    return page.locator(f"#{html_id}")


def wait_idle(page: Page, ms: int = 350) -> None:
    page.wait_for_timeout(ms)


# Set by the chapter driver in `main()` when a chapter is active; when
# set, `screenshot()` writes into the chapter's `screenshots/` folder
# AND records the filename on the session so the chapter can emit a
# screenshot index on close. None → fall back to the legacy OUT_DIR.
_ACTIVE_SESSION = None


def screenshot(page: Page, name: str) -> None:
    if _ACTIVE_SESSION is not None:
        _ACTIVE_SESSION.screenshot(name)
        return
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    p = OUT_DIR / f"{name}.png"
    page.screenshot(path=str(p), full_page=False)
    print(f"  📸 {p.relative_to(REPO_ROOT)}")


def click_button(page: Page, label: str, timeout: int = 5_000) -> bool:
    try:
        page.get_by_role("button", name=label).first.click(timeout=timeout)
        return True
    except Exception as e:
        print(f"  · button '{label}' not found: {type(e).__name__}")
        return False


def bridge_export_gldf(page: Page, dst: Path, dialux_compat: bool = False) -> bool:
    """Bridging save: silently dump the current editor state to a .gldf
    on disk so the next chapter can load it. Mirrors `act_21_export`
    but without screenshots / narrative prints — it's a no-show
    plumbing step between chapters. Does NOT `page.goto(URL)` — the
    export toolbar is rendered on every editor route, and a goto would
    wipe the in-memory Leptos state we're trying to save.

    Since slice 1 of #211, the toolbar's Export GLDF button opens an
    options modal instead of exporting immediately. We click the
    button → optionally tick the DIALux-compat checkbox → click the
    confirm button. The rolling `_bridge.gldf` always uses the
    canonical (untransformed) path so chapter N+1 sees the same model
    state chapter N produced; per-chapter snapshots set
    `dialux_compat=True` so the artifacts on disk are DIALux-importable
    out of the box."""
    try:
        # Open the modal
        tid(page, "toolbar-export-gldf").click()
        # Tick DIALux-compat if requested
        if dialux_compat:
            tid(page, "export-opt-dialux").check()
        # Confirm + wait for the download
        with page.expect_download(timeout=15_000) as dl:
            tid(page, "export-options-confirm").click()
        download = dl.value
        download.save_as(str(dst))
        tag = " (dialux)" if dialux_compat else ""
        print(f"  💾 bridge-saved {dst.relative_to(REPO_ROOT)}{tag} "
              f"({dst.stat().st_size // 1024} KB)")
        return True
    except Exception as e:
        print(f"⚠ bridge_export_gldf: {e}", file=sys.stderr)
        return False


def bridge_import_gldf(page: Page, src: Path) -> bool:
    """Bridging load: drop a .gldf into the welcome page's file input so
    the editor restores its state before a chapter starts. Skip
    silently if `src` doesn't exist (e.g. when chapter 1 was skipped
    in a partial re-record)."""
    if not src.exists():
        print(f"  · no bridge GLDF at {src.relative_to(REPO_ROOT)} — starting fresh")
        return False
    try:
        page.goto(URL)
        wait_idle(page, 600)
        tid(page, "welcome-file-input").set_input_files(str(src))
        wait_idle(page, 1_500)
        print(f"  📥 bridge-loaded {src.relative_to(REPO_ROOT)}")
        return True
    except Exception as e:
        print(f"⚠ bridge_import_gldf: {e}", file=sys.stderr)
        return False


def ensure_edit_mode(page: Page, *, label: str = "") -> bool:
    """Make sure the editor is in Edit mode. The toolbar toggle reads
    `Edit Mode` while we're viewing and `View Mode` while we're editing,
    so we read the button's text to decide whether to click.

    Welcome-file-input re-imports (`bridge_import_gldf`, Act 22, Act 25)
    always land in View mode — call this right after them. Also call it
    from anywhere you're about to do `+ Add row` / `+ New …` / `Edit`
    on a card, since those affordances vanish in View mode."""
    try:
        toggle = tid(page, "toolbar-toggle-mode")
        text = (toggle.inner_text(timeout=2_000) or "").strip()
        if "Edit Mode" in text:
            toggle.click(timeout=3_000)
            wait_idle(page, 400)
            tag = f" ({label})" if label else ""
            print(f"  ✎ toggled into Edit mode{tag}")
            return True
        return False
    except Exception as e:
        tag = f" ({label})" if label else ""
        print(f"⚠ ensure_edit_mode{tag}: {e}", file=sys.stderr)
        return False


def safe_select(page: Page, locator, value: str) -> bool:
    try:
        locator.select_option(value=value, timeout=2_500)
        return True
    except Exception as e:
        print(f"  · select_option({value}) failed: {type(e).__name__}")
        return False


def safe_fill(page: Page, locator, value: str) -> bool:
    try:
        locator.fill(value, timeout=2_500)
        return True
    except Exception as e:
        print(f"  · fill('{value}') failed: {type(e).__name__}")
        return False


# ──────────────────────────────────────────────────────────────────────
# Acts


def act_1_wizard(page: Page) -> None:
    """Welcome → 11-step wizard. Trahe ACME / Holger Trahe / Lumina Pro /
    middle.ldt + sensor.ldt + emergency.ldt + L3D + Driver.

    The wizard's 11 steps (Header → Photometry → Sensor → Emergency →
    LightSource → Emitter → Geometry → Mounting → Driver → Variant →
    Review) declare the ENTIRE 'rich product' on Finish:
      - one normal-mode <Photometry> from middle.ldt
      - one sensor/sensldt <File> + <Sensor> entry from sensor.ldt
      - one emergency-mode <Photometry> from emergency.ldt
      - one Fixed <LightSource> + bound Emitter
      - L3D geometry + Recessed ceiling mounting
      - one ControlGear + Equipment + ControlGearReference

    The downstream Acts then REFINE this baseline (extra LDTs / extra
    LightSources / per-variant rebind / RAL housing colour / etc.)
    instead of inventing it from scratch. The Emergency wizard step
    means emergency.ldt no longer needs the Act 2 drop + Act 8c
    rename detour — it's a first-class photometry from the very first
    Create."""
    print("\n[Act 1] Wizard — Trahe ACME seed (11 steps)")
    page.goto(URL)
    expect(tid(page, "welcome-open-wizard")).to_be_visible(timeout=15_000)
    screenshot(page, "01-welcome")
    tid(page, "welcome-open-wizard").click()
    wait_idle(page)

    # Step 1 — Header
    tid(page, "wizard-header-author").fill("Holger Trahe")
    tid(page, "wizard-header-manufacturer").fill("Trahe ACME")
    screenshot(page, "02-wizard-1-header")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 2 — Photometry (normal mode beam, middle.ldt)
    tid(page, "wizard-photometry-input").set_input_files(str(ASSETS / "middle.ldt"))
    wait_idle(page, 700)
    screenshot(page, "03-wizard-2-photometry")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 3 — Sensor. Drop sensor.ldt here so apply_wizard lands it as
    # sensor/sensldt + mints a <Sensor> entry. Without this step the
    # Files-page drop pipeline would sniff .ldt as ldc/eulumdat (no way
    # to tell a sensor sheet apart from a photometric LDT by extension
    # alone), and Act 2 would need a follow-up UpdateFile dispatch to
    # retype the content_type. The Sensor wizard step is what lets the
    # Trahe ACME flow stay declarative — drop, click Next, done.
    tid(page, "wizard-sensor-input").set_input_files(str(ASSETS / "sensor.ldt"))
    wait_idle(page, 700)
    screenshot(page, "04-wizard-3-sensor")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 4 — Emergency (NEW). Drop emergency.ldt here so apply_wizard
    # creates a SECOND <Photometry> entry for the dim emergency-mode
    # beam. The wizard dispatches the same AddPhotometryFile action as
    # the normal Photometry step — the auto id is photometryFile_2
    # (since middle.ldt landed first as photometryFile_1). Act 8c then
    # renames it to ph_emergency / ldt_emergency. Without this step
    # the Trahe ACME demo previously dropped emergency.ldt in Act 2
    # alongside narrow / wide; the wizard step eliminates that detour.
    tid(page, "wizard-emergency-input").set_input_files(str(ASSETS / "emergency.ldt"))
    wait_idle(page, 700)
    screenshot(page, "05-wizard-4-emergency")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 5 — LightSource (defaults)
    tid(page, "wizard-next").click()
    wait_idle(page)
    # Step 6 — Emitter (defaults)
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 7 — Geometry (drop the L3D)
    tid(page, "wizard-geometry-input").set_input_files(str(ASSETS / "example_007.l3d"))
    wait_idle(page, 700)
    screenshot(page, "06-wizard-7-geometry")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 8 — Mounting. Pick Ceiling · Recessed with 50mm depth so the
    # Bevy 3D viewer renders the luminaire sunk into the ceiling cavity
    # (the visible payoff of the Mountings end-to-end flow). The
    # wizard's apply_wizard reducer writes Variant.Mountings/Ceiling/
    # Recessed[@recessedDepth=50] which feeds gldf-bevy's
    # MountingConfig.get_y_position with `room_height + 50mm` so the
    # luminaire's emission plane sits above the ceiling line.
    tid(page, "wizard-mounting-type").select_option("ceiling/recessed")
    wait_idle(page, 300)
    # depth field appears only after the type select; fill it now.
    tid(page, "wizard-mounting-depth").fill("50")
    wait_idle(page, 200)
    screenshot(page, "07-wizard-8-mounting")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 9 — Driver. Tick include + fill the three fields.
    # apply_wizard mints ControlGear + Equipment + ControlGearReference
    # on Finish so the downstream Acts can REFINE instead of create.
    tid(page, "wizard-driver-enabled").check()
    wait_idle(page, 300)
    tid(page, "wizard-driver-name").fill("Trahe Driver 24V")
    tid(page, "wizard-driver-dimmable").check()
    tid(page, "wizard-driver-standby").fill("0.3")
    screenshot(page, "08-wizard-9-driver")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 10 — Variant (defaults)
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 11 — Review + Create
    screenshot(page, "09-wizard-11-review")
    tid(page, "wizard-create").click()
    wait_idle(page, 1200)
    expect(tid(page, "nav-overview")).to_be_visible()
    screenshot(page, "10-overview-after-create")

    tid(page, "toolbar-toggle-mode").click()
    wait_idle(page)


def act_2_files(page: Page) -> None:
    """Files page: drop narrow.ldt + wide.ldt + product picture + SPD.

    The wizard already covered the photometric core: middle.ldt arrived
    as `photometryFile_1` (Act 1 Step 2 — normal beam), emergency.ldt
    as `photometryFile_2` (Act 1 Step 4 — dim emergency beam), and
    sensor.ldt landed as `sensorFile_1` with the correct
    `sensor/sensldt` content type AND a minted `<Sensor>` entry (Act 1
    Step 3 — no retype-after-drop dance needed). This Act adds the two
    extra beam shapes the wizard's single-LightSource flow can't
    declare in advance: narrow.ldt → `photometryFile_3` and wide.ldt
    → `photometryFile_4`. Act 8c then renames the auto ids to semantic
    names (`ldt_narrow / ldt_wide / ldt_middle / ldt_emergency`) and
    `sensorFile_1` → `sensor`.

    `emergency.ldt` is the synthetic-but-plausible companion file for
    Variant 3's EmergencyOnly SKU. Most vendor GLDFs ship only the
    normal-mode photometry and rely on an `EmergencyBehaviour` block to
    describe the emergency lumen output — consumers then have to guess
    the emergency beam. This demo declares it as a separate Photometry
    so the LightSource card can render both the normal beam thumbnail
    AND the dimmer emergency one. The wizard's Emergency step is what
    makes that declaration a first-class operation instead of a
    follow-up Files-page detour. See the recipe in the
    `crates/gldf-rs-editor/src/wizard.rs` `WizardDraft.emergency_bytes`
    doc comment for the synthesis ratio (flux 562 lm → 45 lm, watts
    8.2 W → 1.5 W, beam 38° unchanged).

    The .spd auto-registers a `<Spectrum>` entry (per the Files-page
    drop handler — see crates/gldf-rs-editor action AddSpectrumFile-by-
    drop path). Act 3 then binds it to lightsource_2 so the TM-33
    export in Act 23 has both a photometric matrix AND an SPD curve to
    embed in ONE file."""
    print("\n[Act 2] Files — drop 2 extra LDTs + product picture + SPD")
    tid(page, "nav-files").click()
    wait_idle(page)
    extras = [
        str(ASSETS / "narrow.ldt"),
        str(ASSETS / "wide.ldt"),
    ]
    # Prefer the SLV catalogue hero shot (JPG, 260x260) when present —
    # it's the manufacturer marketing image for SLV_1008510, the Tria 2
    # this demo recreates. Falls back to the older placeholder PNG.
    pic_jpg = ASSETS / "product_picture.jpg"
    pic_png = ASSETS / "product_picture.png"
    if pic_jpg.is_file():
        extras.append(str(pic_jpg))
    elif pic_png.is_file():
        extras.append(str(pic_png))
    spd = ASSETS / "spectrum_2700K.spd"
    if spd.is_file():
        extras.append(str(spd))
    tid(page, "files-add-input").set_input_files(extras)
    wait_idle(page, 1500)
    print(f"  · dropped {len(extras)} files")

    # Rename the wizard's auto sensor id to the semantic `sensor`.
    # RenameFile cascades through SensorFileReference so the <Sensor>
    # entry follows automatically.
    import json as _json
    sensor_file_rename = {
        "action": "RenameFile",
        "data": {"old_id": "sensorFile_1", "new_id": "sensor"},
    }
    err = page.evaluate(
        "(json) => window.gldf_apply_action_json(json)",
        _json.dumps(sensor_file_rename),
    )
    if err:
        print(f"  · sensorFile_1 → sensor ERR: {err}")
    else:
        print("  · file sensorFile_1 → sensor")
    wait_idle(page, 400)
    screenshot(page, "11-files")


def act_3_light_sources(page: Page) -> None:
    """Light Sources: tune LS1 (CCT 4000K + name), add LS2 (3000K) + LS3 (5000K).

    Also binds the auto-registered spectrum_1 (created by the Act 2
    .spd drop) to lightsource_2 — the 3000 K variant — so the per-LS
    TM-33 export in Act 23 has both photometry AND an SPD curve.
    """
    print("\n[Act 3] Light Sources — 3 with name + CCT")
    tid(page, "nav-light-sources").click()
    wait_idle(page)

    for i, v in enumerate(VARIANTS):
        if i == 0:
            edit = tid(page, "ls-edit-lightsource_1")
        else:
            tid(page, "ls-add-fixed").click()
            wait_idle(page, 800)
            edit = tid(page, f"ls-edit-lightsource_{i + 1}")
        if edit.count() == 0:
            print(f"  · ls-edit lightsource_{i + 1} missing — skipping")
            continue
        edit.click()
        wait_idle(page, 500)
        tid(page, "ls-edit-name").fill(v["ls_name"])
        tid(page, "ls-edit-cct").fill(str(v["cct"]))
        tid(page, "ls-edit-manufacturer").fill("Trahe ACME LED Components")
        tid(page, "ls-edit-power").fill(str(v["ls_power_w"]))
        wait_idle(page, 200)
        tid(page, "ls-edit-save").click()
        wait_idle(page, 600)
        print(f"  · lightsource_{i + 1} → {v['ls_name']} "
              f"({v['cct']} K, {v['ls_power_w']} W)")

    # Bind the auto-registered spectrum_1 to lightsource_2 via the JS
    # bridge. (The SpectrumPicker lives inside the edit modal — using
    # the bridge keeps Act 3 stable while the modal-race fix beds in.)
    import json as _json
    bind_payload = {
        "action": "SetLightSourceSpectrum",
        "data": {
            "light_source_id": "lightsource_2",
            "spectrum_id": "spectrum_1",
        },
    }
    err = page.evaluate(
        "(json) => window.gldf_apply_action_json(json)",
        _json.dumps(bind_payload),
    )
    if err:
        print(f"  · spectrum bind ERR: {err}")
    else:
        print("  · lightsource_2 → spectrum_1 (2700K)")

    # Stamp a GTIN-13 onto each light source. Trahe-prefixed fake EAN
    # values so the SKU passes ERP / BIM ingestion checks. To set a
    # single field on a FixedLightSource we read the current snapshot
    # via gldf_product_state, mutate the field, then dispatch
    # UpdateFixedLightSource with the whole payload (the action takes
    # the entire struct).
    state_json = page.evaluate(
        "() => (window.gldf_product_state ? window.gldf_product_state() : null)"
    )
    ls_gtins = {
        "lightsource_1": "4260000000048",
        "lightsource_2": "4260000000055",
        "lightsource_3": "4260000000062",
    }
    try:
        snap = _json.loads(state_json) if state_json else None
        fls_list = (
            snap.get("GeneralDefinitions", {})
            .get("LightSources", {})
            .get("FixedLightSource", [])
        ) if snap else []
        for fls in fls_list:
            ls_id = fls.get("@id")
            if ls_id not in ls_gtins:
                continue
            payload_ls = dict(fls)
            payload_ls["GTIN"] = ls_gtins[ls_id]
            update = {
                "action": "UpdateFixedLightSource",
                "data": {"id": ls_id, "payload": payload_ls},
            }
            e2 = page.evaluate(
                "(json) => window.gldf_apply_action_json(json)",
                _json.dumps(update),
            )
            if e2:
                print(f"  · {ls_id} GTIN ERR: {e2}")
            else:
                print(f"  · {ls_id} GTIN → {ls_gtins[ls_id]}")
    except Exception as e:
        print(f"  · ls GTIN dispatch: {type(e).__name__}: {e}")

    # Rename `lightsource_N` → role-first semantic ids while still on
    # the Light Sources page so the viewer sees each card's id chip
    # flip from auto-name to human-readable. One dispatch per LS with a
    # short hold so the UI redraws cleanly between renames.
    print("  · rename LS ids visibly on the Light Sources page")
    for i, v in enumerate(VARIANTS):
        old_id = f"lightsource_{i + 1}"
        new_id = v["ls_id_semantic"]
        rename_payload = {
            "action": "RenameFixedLightSource",
            "data": {"old_id": old_id, "new_id": new_id},
        }
        err = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)",
            _json.dumps(rename_payload),
        )
        if err:
            print(f"  · {old_id} → {new_id} ERR: {err}")
        else:
            print(f"  · {old_id} → {new_id}")
        wait_idle(page, 700)

    # Rename `photometry_N` → semantic ids. The cascade in the lib
    # updates every Emitter.PhotometryReference, so downstream Acts
    # (6 emitters, etc.) stay consistent. We're still on the Light
    # Sources page; the Photometry list itself is on the Photometry
    # tab — to make this visible we navigate there briefly.
    print("  · rename Photometry ids (cascades through emitters)")
    # Live file order at Act 3 time (wizard-first):
    #   photometry_1 = middle.ldt    (wizard Step 2)
    #   photometry_2 = emergency.ldt (wizard Step 4 — NEW)
    #   photometry_3 = narrow.ldt    (Act 2 drop)
    #   photometry_4 = wide.ldt      (Act 2 drop)
    # sensor.ldt landed as sensorFile_1 (no Photometry minted) and was
    # already renamed to `sensor` in Act 2.
    photometry_renames = [
        ("photometry_1", "phot_middle"),
        ("photometry_2", "phot_emergency"),
        ("photometry_3", "phot_narrow"),
        ("photometry_4", "phot_wide"),
    ]
    try:
        tid(page, "nav-photometry").click(timeout=3_000)
        wait_idle(page, 800)
    except Exception:
        pass
    for old, new in photometry_renames:
        rename_payload = {
            "action": "RenamePhotometry",
            "data": {"old_id": old, "new_id": new},
        }
        err = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)",
            _json.dumps(rename_payload),
        )
        if err:
            print(f"  · {old} → {new} ERR: {err}")
        else:
            print(f"  · {old} → {new}")
        wait_idle(page, 500)
    screenshot(page, "08b-photometry-renamed")

    # Back to LS page so the trailing screenshot still captures the
    # final state the chapter expects.
    try:
        tid(page, "nav-light-sources").click(timeout=3_000)
        wait_idle(page, 500)
    except Exception:
        pass
    screenshot(page, "08-light-sources")


def act_4_control_gear(page: Page) -> None:
    """Control Gear: ✎ Edit the wizard-minted controlGear_1 via the UI modal.

    The wizard's Driver step minted controlGear_1 with basic values
    ("Trahe Driver 24V", dimmable, 0.3W standby). Act 4 REFINES it via
    the in-modal form — adding the voltage and a fuller name. This
    still showcases the #136 modal-race fix (stable in-modal typing)
    without duplicating what the wizard already did."""
    print("\n[Act 4] Control Gear — refine controlGear_1 via UI modal")
    tid(page, "nav-control-gear").click()
    wait_idle(page)
    edit_btn = tid(page, "cg-edit-controlGear_1")
    if edit_btn.count() == 0:
        print("  · cg-edit-controlGear_1 missing — wizard Driver step may have skipped")
        screenshot(page, "09-control-gear")
        return
    edit_btn.click()
    wait_idle(page, 600)

    def fill_blur(testid: str, value: str) -> None:
        try:
            el = tid(page, testid)
            el.fill(value)
            el.press("Tab")
        except Exception as e:
            print(f"  · {testid} fill: {type(e).__name__}: {e}")

    fill_blur("cg-edit-name", "Trahe Driver 24V (dimmable, CLO)")
    fill_blur("cg-edit-voltage", "24")
    fill_blur("cg-edit-standby", "0.3")
    # Dimmable checkbox — already ticked by the wizard, leave it on.
    try:
        dim = tid(page, "cg-edit-dimmable")
        if not dim.is_checked():
            dim.check()
    except Exception as e:
        print(f"  · dimmable: {type(e).__name__}: {e}")
    wait_idle(page, 300)
    save = tid(page, "cg-edit-save")
    if save.count() > 0:
        save.click()
        wait_idle(page, 600)
        print("  · controlGear_1 → refined name + 24V (modal save)")
    else:
        print("  · cg-edit-save missing")
    screenshot(page, "09-control-gear")


def act_5_equipment(page: Page) -> None:
    """Equipment: refine the wizard-minted equipment_1 via the JS bridge.

    The wizard's Driver step minted equipment_1 bundling lightsource_1
    + controlGear_1. Act 5 refines its rated_input_power via the bridge
    (the Equipment edit form's Save button still has no testid; ports
    of the in-modal save flow can land in a follow-up cut).

    We DON'T click `+ New equipment` here — the wizard already minted
    equipment_1 in Act 1, so a `+ New` would create a redundant
    equipment_2."""
    print("\n[Act 5] Equipment — refine equipment_1")
    tid(page, "nav-equipment").click()
    wait_idle(page)

    # Refine the wizard-minted Equipment via the bridge.
    import json as _json
    # NOTE: per rc-3 XSD the Equipment.LightSourceReference is restricted
    # to `@changeableLightSourceId`. Our demo creates lightsource_1 as a
    # FixedLightSource (the LED module ships with the luminaire and isn't
    # field-swappable), so we don't bind any LightSource here — the
    # exporter's serialize-time filter would drop the whole `<Equipment>`
    # anyway. The ControlGear binding still makes it through.
    payload = {
        "action": "UpdateEquipment",
        "data": {
            "id": "equipment_1",
            "light_source_id": None,
            "light_source_kind": None,
            "control_gear_id": "controlGear_1",
            "rated_input_power": 24.0,
            "emergency_ballast_lumen_factor": None,
            "emergency_rated_luminous_flux": None,
        },
    }
    err = page.evaluate(
        "(json) => window.gldf_apply_action_json(json)",
        _json.dumps(payload),
    )
    if err:
        print(f"  · UpdateEquipment ERR: {err}")
    else:
        print("  · equipment_1 → lightsource_1 + controlGear_1, 24W (refined)")
    wait_idle(page, 400)
    screenshot(page, "10-equipment")


def act_6_emitters(page: Page) -> None:
    """Emitters: mint one Emitter per variant binding (7 total for the
    4 variants — V0/V1/V2 have left+right LEO bindings, V3 has the
    centre Sensor binding only). Each emitter carries its own
    PhotometryReference + LightSourceReference + flux. EmergencyBehavior
    flags the V3 emitter so gldf-bevy's 0.1× emergency multiplier dims
    the centre LED appropriately.

    All dispatches go through the JS bridge: that's 7 emitters × 3
    actions = 21 calls; using the in-page form would make this Act
    long and slow. The form is still showcased in the demo via the
    wizard's Emitter step in Act 1.

    AddEmitterWithChild auto-generates ids — they land as
    `emitter_1..7` in the order of dispatch. Act 8c renames to
    semantic names afterwards.
    """
    print("\n[Act 6] Emitters — mint 7 across 4 variants (multi-LEO)")
    tid(page, "nav-emitters").click()
    wait_idle(page)
    import json as _json

    for v in VARIANTS:
        for b in v["bindings"]:
            add_payload = {
                "action": "AddEmitterWithChild",
                "data": {
                    "child_name": b["leo_name"],
                    "kind": "Fixed",
                    "photometry_id": b["photometry_auto_id"],
                    "fixed_light_source_id": v["lightsource_id"],
                    "multi_channel_light_source_id": "",
                    "sensor_id": "",
                },
            }
            err = page.evaluate(
                "(json) => window.gldf_apply_action_json(json)",
                _json.dumps(add_payload),
            )
            if err:
                print(
                    f"  · v{v['idx']} {b['leo_name']} AddEmitter ERR: {err}"
                )
            else:
                print(
                    f"  · v{v['idx']} {b['leo_name']} → {b['emitter_auto_id']}"
                    f" ({b['photometry_auto_id']} ↔ {v['lightsource_id']})"
                )
            # Visible footage: 700 ms between dispatches lets the
            # newly-minted card render on screen before the next one is
            # added. Without this, all 7 cards pop in within the same
            # frame and the narrator has nothing to talk over.
            wait_idle(page, 700)

    # Per-emitter flux + ControlGear binding. The flux belongs to the
    # variant, not the binding (both heads of a normal variant share
    # the same LightSource and therefore the same module wattage), so
    # we just stamp each binding's emitter with the variant's flux.
    for v in VARIANTS:
        for b in v["bindings"]:
            flux_payload = {
                "action": "SetFixedLightEmitterFlux",
                "data": {
                    "emitter_id": b["emitter_auto_id"],
                    "index": 0,
                    "rated_luminous_flux": v["flux"],
                },
            }
            err = page.evaluate(
                "(json) => window.gldf_apply_action_json(json)",
                _json.dumps(flux_payload),
            )
            if err:
                print(
                    f"  · {b['emitter_auto_id']} flux set ERR: {err}"
                )

            # EmergencyOnly behaviour for the V3 sensor binding. Bevy
            # reads this off the EmitterConfig and applies a 0.1×
            # flux multiplier so the centre LED renders dim.
            if v["emergency_behavior"]:
                emerg_payload = {
                    "action": "SetFixedLightEmitterEmergencyBehaviour",
                    "data": {
                        "emitter_id": b["emitter_auto_id"],
                        "index": 0,
                        "behaviour": v["emergency_behavior"],
                    },
                }
                err = page.evaluate(
                    "(json) => window.gldf_apply_action_json(json)",
                    _json.dumps(emerg_payload),
                )
                if err:
                    print(
                        f"  · {b['emitter_auto_id']} EmergencyOnly ERR: {err}"
                    )

            cg_payload = {
                "action": "SetEmitterControlGearReference",
                "data": {
                    "emitter_id": b["emitter_auto_id"],
                    "kind": "Fixed",
                    "light_emitter_index": 0,
                    "control_gear_id": "controlGear_1",
                },
            }
            err = page.evaluate(
                "(json) => window.gldf_apply_action_json(json)",
                _json.dumps(cg_payload),
            )
            if err:
                print(
                    f"  · {b['emitter_auto_id']} ControlGear ERR: {err}"
                )
    screenshot(page, "11-emitters")


def act_7_variants_duplicate(page: Page) -> None:
    """Variants page → duplicate variant 0 three times so 4 variants
    exist (3 normal + 1 emergency-only), then assign a GTIN-13 to each.
    GTINs are de-facto required by ERP/BIM consumers — without one most
    catalog systems silently drop the variant. Trahe-prefixed fake EAN
    values."""
    print("\n[Act 7] Variants — duplicate × 3 + GTINs")
    tid(page, "nav-variants").click()
    wait_idle(page)
    for n in (1, 2, 3):
        btn = tid(page, "variant-duplicate-0")
        if btn.count() == 0:
            print(f"  · variant-duplicate-0 missing — stopped at {n - 1}")
            break
        btn.click()
        wait_idle(page, 600)
        print(f"  · duplicated → variant {n}")

    # Stamp GTIN-13 onto each variant via the JS bridge — the variant
    # editor GTIN input lives inside the edit modal and has no
    # data-testid, so the bridge is the robust path.
    import json as _json
    variant_gtins = [
        "4260000000017",  # V0 yellow green
        "4260000000024",  # V1 traffic yellow
        "4260000000031",  # V2 signal white
        "4260000000048",  # V3 jet black emergency-only
    ]
    for idx, gtin in enumerate(variant_gtins):
        payload = {
            "action": "SetVariantGTIN",
            "data": {"index": idx, "gtin": gtin},
        }
        err = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)",
            _json.dumps(payload),
        )
        if err:
            print(f"  · variant[{idx}] GTIN ERR: {err}")
        else:
            print(f"  · variant[{idx}] GTIN → {gtin}")
        # Visible beat: 400 ms between GTIN stamps so the cards visibly
        # update one at a time on the Variants list.
        wait_idle(page, 400)

    # Set a per-variant ProductNumber too. The XSD makes it optional on
    # Variant (minOccurs=0), but some downstream readers (gldf-search.de,
    # certain BIM importers) hard-deref `.ProductNumber.Locale` and
    # crash with "null is not an object" when the element is absent.
    # Emitting an empty-but-present <ProductNumber><Locale/></ProductNumber>
    # silences those readers without breaking the spec. We tag each
    # variant with a Trahe-ACME-style product number derived from RAL.
    product_numbers = [
        "LP-Y-6018",  # V0 yellow green
        "LP-Y-1023",  # V1 traffic yellow
        "LP-W-9003",  # V2 signal white
        "LP-E-9005",  # V3 jet black emergency-only
    ]
    for idx, pn in enumerate(product_numbers):
        payload = {
            "action": "SetVariantProductNumber",
            # SetVariantProductNumber is a tuple-variant
            # `SetVariantProductNumber(usize, LocaleFoo)` so the `data`
            # field is a positional array. LocaleFoo's serde shape is
            # XML-flavored — `Locale` is PascalCase (per the lib's
            # `rename = "Locale"`) and each Locale entry uses `@language`
            # / `$text` as quick-xml's attribute / text markers. See
            # crates/gldf-rs-lib/src/gldf/header.rs:339 for the typed
            # surface and crates/gldf-rs-editor/tests/
            # set_variant_product_number_json.rs for the JSON contract.
            "data": [idx, {"Locale": [{"@language": "en", "$text": pn}]}],
        }
        err = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)",
            _json.dumps(payload),
        )
        if err:
            print(f"  · variant[{idx}] ProductNumber ERR: {err}")
        else:
            print(f"  · variant[{idx}] ProductNumber → {pn}")
    screenshot(page, "12-variants-after-dup")


def act_8_rebind_emitters_per_variant(page: Page) -> None:
    """For each variant, open Edit → flip Model geometry mode → repoint
    binding 0 at variant's own emitter. Works because the geometry-mode
    button re-fires SetVariantGeometryModel which auto-adds binding 0
    pointing at the first emitter; we then change it to the right one.

    Two-pass design so the video shows what's happening:

      Pass A (visible UI tour): for each variant, open the Edit modal,
      click the Model-geometry mode button, snap a screenshot, close
      the modal. This is the footage you see in the chapter mp4 — it
      makes the editor's variant-binding UI clearly visible to the
      voice-over while the AI narrates "we wire each variant to its
      own emitter set."

      Pass B (JS-bridge state set): dispatch the real multi-binding
      SetVariantGeometryModel via window.gldf_apply_action_json. The
      modal UI alone can't set per-binding `external_name` (LEO0 /
      LEO1 / Sensor) — only the action payload can — so the bridge
      pass is what actually wires emitters to LEO partNames.
    """
    print("\n[Act 8] Rebind each variant's geometry → variant's emitter")
    tid(page, "nav-variants").click()
    wait_idle(page, 600)

    # ── Pass A removed ────────────────────────────────────────────────
    # The previous "visible UI tour" opened + Escape-closed the variant
    # Edit modal 4 times in quick succession to capture screenshots of
    # the geometry-mode picker. The VariantModalHost's `is_editing:
    # RwSignal<bool>` race (#136 / #153) reappears under headed slow_mo
    # — the modal disposes faster than the open closure can read the
    # signal, and the browser console drowns in "reactive value
    # disposed" panics, wedging the page for the rest of chapter 1. The
    # panics are recoverable on the lib side (the dispatches in Pass B
    # still land cleanly — the headless dry-run confirms a ✓ XSD clean
    # export) but the headed footage becomes unwatchable. Pass B alone
    # is what actually wires the variant→emitter bindings, so dropping
    # the UI tour costs no correctness. The variant-binding workflow is
    # still visible elsewhere in the demo (Act 7 + the Variants pane
    # walkthrough in Act 8c's UI tour at the end).

    # ── Pass B: JS-bridge state set ───────────────────────────────────
    # Multi-binding payload per variant. Each binding's external_name
    # IS the L3D partName (LEO0 / LEO1 / Sensor), so gldf-bevy can
    # match each emitter config against the real LEO and spawn a
    # tinted light at the right position.
    print("  · Pass B: JS-bridge SetVariantGeometryModel")
    import json as _json
    for v in VARIANTS:
        idx = v["idx"]
        bindings_payload = [
            {
                "emitter_id": b["emitter_auto_id"],
                "external_name": b["leo_name"],
                "target_model_type": "l3d",
            }
            for b in v["bindings"]
        ]
        payload = {
            "action": "SetVariantGeometryModel",
            "data": {
                "variant_idx": idx,
                "geometry_id": "geometry_1",
                "bindings": bindings_payload,
            },
        }
        result = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)",
            _json.dumps(payload),
        )
        if result:
            print(
                f"  · variant {idx} → {len(bindings_payload)} bindings: ERR {result}"
            )
        else:
            slots = ", ".join(f"{b['leo_name']}={b['emitter_auto_id']}" for b in v["bindings"])
            print(f"  · variant {idx} → {slots}")
        wait_idle(page, 200)
    screenshot(page, "13-variants-after-rebind")


def act_8c_rename_ids(page: Page) -> None:
    """Rename auto-generated ids to semantic names. Catalog / BIM
    tools store these strings verbatim — `photometryFile_1` is a
    forensic smell, `ldt_narrow` is a feature.

    The rename actions cascade through every reference site (Emitter →
    LightSource, Variant → Emitter, Spectrum → File, etc.) so the
    rename is safe to dispatch in any order.

    The rename UI exists per-card (inline editable id label) but is
    fiddly to drive at scale — the demo uses the JS bridge for the
    bulk rename, then takes a short visible tour through Files,
    Light Sources, and Emitters pages so the chapter footage shows
    the result (all cards now display human-readable ids).
    """
    print("\n[Act 8c] Rename auto-ids → semantic names")
    import json as _json
    # File renames — semantic by content. Order matters and reflects
    # the new wizard-first layout: the wizard mints middle.ldt as
    # `photometryFile_1` (Act 1 Step 2) and emergency.ldt as
    # `photometryFile_2` (Act 1 Step 4). Act 2 then drops narrow.ldt →
    # `photometryFile_3` and wide.ldt → `photometryFile_4`. The
    # wizard's sensor.ldt landed as `sensorFile_1` and was renamed to
    # `sensor` already in Act 2, so it doesn't show up here.
    file_renames = [
        ("photometryFile_1", "ldt_middle"),
        ("photometryFile_2", "ldt_emergency"),
        ("photometryFile_3", "ldt_narrow"),
        ("photometryFile_4", "ldt_wide"),
        ("file_1", "geom_tria2_l3d"),
        ("file_2", "img_product_main"),
        ("spectrumFile_1", "spd_2700k_warm"),
    ]
    for old, new in file_renames:
        payload = {"action": "RenameFile", "data": {"old_id": old, "new_id": new}}
        err = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)", _json.dumps(payload)
        )
        if err:
            print(f"  · file {old} → {new} ERR: {err}")
        else:
            print(f"  · file {old} → {new}")
    # LightSource semantic ids — role-first so they sort sensibly in
    # a catalog UI ("ls_main_*" cluster together, the emergency module
    # stays distinct). The display name still carries the friendly
    # "LED 4000K module" text; the id is what BIM/ERP keys on.
    #
    # NOTE: LightSource id renames moved to Act 3 so the viewer
    # actually SEES the id chips flip while the Light Sources page
    # is on screen. Nothing to do here for FixedLightSource.
    # Emitter semantic ids — `em_v<variant>_<role>`. Act 6 creates 7
    # emitters in this exact order (one per binding in VARIANTS):
    #   V0 LEO0 (narrow) → emitter_1
    #   V0 LEO1 (wide)   → emitter_2
    #   V1 LEO0 (wide)   → emitter_3
    #   V1 LEO1 (narrow) → emitter_4
    #   V2 LEO0 (middle) → emitter_5
    #   V2 LEO1 (middle) → emitter_6
    #   V3 Sensor (sensor) → emitter_7
    em_renames = [
        ("emitter_1", "em_v0_left_narrow"),
        ("emitter_2", "em_v0_right_wide"),
        ("emitter_3", "em_v1_left_wide"),
        ("emitter_4", "em_v1_right_narrow"),
        ("emitter_5", "em_v2_left_middle"),
        ("emitter_6", "em_v2_right_middle"),
        ("emitter_7", "em_v3_sensor_emergency"),
    ]
    for old, new in em_renames:
        payload = {"action": "RenameEmitter", "data": {"old_id": old, "new_id": new}}
        err = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)", _json.dumps(payload)
        )
        if err:
            print(f"  · Emitter {old} → {new} ERR: {err}")
        else:
            print(f"  · Emitter {old} → {new}")
    # Variant ids — by RAL housing colour. `DuplicateVariant` inserts
    # right after the source, so duplications of variant_0 three times
    # produce array order [variant_1, variant_4, variant_3, variant_2]
    # (each new clone slots into position 1 and pushes prior clones
    # down). The idx→auto-id map below is therefore non-monotonic;
    # see crates/gldf-rs-editor/src/state.rs:362 for the insertion
    # rule that justifies it.
    variant_renames = [
        ("variant_1", "v_ral6018_yellowgreen"),       # idx 0
        ("variant_4", "v_ral1023_trafficyellow"),     # idx 1 (latest dup)
        ("variant_3", "v_ral9003_signalwhite"),       # idx 2
        ("variant_2", "v_ral9005_emergency_only"),    # idx 3 (oldest dup)
    ]
    for old, new in variant_renames:
        payload = {"action": "RenameVariant", "data": {"old_id": old, "new_id": new}}
        err = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)", _json.dumps(payload)
        )
        if err:
            print(f"  · Variant {old} → {new} ERR: {err}")
        else:
            print(f"  · Variant {old} → {new}")

    # Variant DISPLAY names — the human-readable <Variant><Name> that
    # shows up on the variants list, overview chips, the 3D viewer
    # picker, and downstream catalog UIs. Act 7's DuplicateVariant
    # clones the source name plus " (copy)", so without an explicit
    # rename the four variants ship as "Standard / Standard (copy) /
    # Standard (copy) / Standard (copy)" — useless for browsing. Stamp
    # the semantic names from VARIANTS.display_name via the tuple-
    # variant SetVariantName(usize, LocaleFoo).
    print("  · rename Variant display names")
    for v in VARIANTS:
        name_payload = {
            "action": "SetVariantName",
            "data": [v["idx"], {"Locale": [{"@language": "en", "$text": v["display_name"]}]}],
        }
        err = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)", _json.dumps(name_payload)
        )
        if err:
            print(f"  · Variant[{v['idx']}] Name ERR: {err}")
        else:
            print(f"  · Variant[{v['idx']}] Name → {v['display_name']}")
    wait_idle(page, 500)
    screenshot(page, "13b-after-id-rename")

    # Visible tour: navigate to Files → Light Sources → Emitters →
    # Variants so the chapter footage shows the result of the rename
    # (every card now displays a human-readable id like `ldt_narrow`
    # instead of `photometryFile_2`). Hold each page ~2 s so the
    # narrator can actually point at each tab — short holds make the
    # voice-over overrun the video.
    print("  · UI tour of renamed-id cards")
    for nav_testid, screen_name, label in [
        ("nav-files", "13c-tour-files-renamed", "Files"),
        ("nav-light-sources", "13d-tour-light-sources-renamed", "Light Sources"),
        ("nav-emitters", "13e-tour-emitters-renamed", "Emitters"),
        ("nav-variants", "13f-tour-variants-renamed", "Variants"),
    ]:
        try:
            tid(page, nav_testid).click(timeout=4_000)
            wait_idle(page, 1_800)
            screenshot(page, screen_name)
        except Exception as e:
            print(f"  · tour skipped {label}: {type(e).__name__}")


def act_9_header_polish(page: Page) -> None:
    print("\n[Act 9] Header pane")
    tid(page, "nav-header").click()
    wait_idle(page)
    screenshot(page, "14-header")


def act_10_photometry(page: Page) -> None:
    print("\n[Act 10] Photometry pane")
    tid(page, "nav-photometry").click()
    wait_idle(page, 600)
    screenshot(page, "15-photometry")


def act_11_electrical(page: Page) -> None:
    print("\n[Act 11] Electrical pane")
    tid(page, "nav-electrical").click()
    wait_idle(page)
    safe_select(page, by_id(page, "safety-class"), "III")
    safe_select(page, by_id(page, "ip-code"), "IP20")
    pf = by_id(page, "power-factor")
    if pf.count() > 0:
        safe_fill(page, pf, "0.95")
    safe_select(page, by_id(page, "light-distribution"), "Direct")
    wait_idle(page, 400)
    screenshot(page, "16-electrical")


def act_12_mechanical(page: Page) -> None:
    print("\n[Act 12] Mechanical pane")
    tid(page, "nav-mechanical").click()
    wait_idle(page)
    safe_select(page, by_id(page, "ik-rating"), "IK06")
    safe_select(page, by_id(page, "product-form"), "Recessed")
    wait_idle(page, 300)
    safe_fill(page, by_id(page, "size-length"), "170")
    safe_fill(page, by_id(page, "size-width"), "90")
    safe_fill(page, by_id(page, "size-height"), "55")
    safe_fill(page, by_id(page, "weight"), "0.379")
    wait_idle(page, 400)

    # Dispatch Mountings to every variant via the JS bridge. All four
    # variants share the same physical fixture (Ceiling / Recessed /
    # 50mm depth, rectangular cut-out 170x90mm). The Mechanical pane
    # doesn't currently surface a Mountings picker — drive the reducer
    # directly so the export carries the data.
    import json as _json
    for idx in range(len(VARIANTS)):
        payload = {
            "action": "SetVariantMountings",
            "data": {
                "index": idx,
                "payload": {
                    "Ceiling": {
                        "Recessed": {
                            "@recessedDepth": 50,
                            "RectangularCutout": {
                                "Width": 90,
                                "Length": 170,
                                "Depth": 50,
                            },
                        }
                    }
                },
            },
        }
        err = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)",
            _json.dumps(payload),
        )
        if err:
            print(f"  · SetVariantMountings v{idx} ERR: {err}")
        else:
            print(f"  · v{idx}: Ceiling/Recessed 50mm, 170x90mm cutout")

    screenshot(page, "17-mechanical")


def act_13_applications(page: Page) -> None:
    print("\n[Act 13] Applications pane")
    tid(page, "nav-applications").click()
    wait_idle(page)
    for label_text in ("Office", "Industrial", "Retail"):
        try:
            page.get_by_label(label_text, exact=False).first.check(timeout=2_000)
            print(f"  · checked {label_text}")
        except Exception as e:
            print(f"  · {label_text}: {type(e).__name__}")
    wait_idle(page, 300)
    screenshot(page, "18-applications")


def act_14_labels(page: Page) -> None:
    """Approval Marks pane — unified home for the Labels (closed enum)
    and ApprovalMarks (free text) XSD slots. Add CE via the enum
    checkboxes + ENEC as a free-text approval mark."""
    print("\n[Act 14] Approval Marks pane")
    tid(page, "nav-labels").click()
    wait_idle(page)
    # Expand the closed-enum checkbox details + check CE.
    try:
        details = page.locator("details").first
        details.locator("summary").click(timeout=2_000)
        wait_idle(page, 200)
        # Tick the first canonical label (CE is alphabetically first).
        page.locator('input[type="checkbox"]').first.check(timeout=2_000)
        wait_idle(page, 200)
    except Exception as e:
        print(f"  · labels check: {type(e).__name__}")
    # Add an approval mark via the new merged page button.
    btn = tid(page, "approval-marks-add")
    if btn.count() > 0:
        btn.click()
        wait_idle(page, 200)
        try:
            inp = page.locator('input[placeholder="CE"]').last
            inp.fill("ENEC")
            inp.press("Tab")
            print("  · CE (label) + ENEC (approval mark)")
        except Exception as e:
            print(f"  · approval mark fill: {type(e).__name__}")
    screenshot(page, "19-labels")


def act_15_hyperlinks(page: Page) -> None:
    print("\n[Act 15] Hyperlinks pane")
    tid(page, "nav-hyperlinks").click()
    wait_idle(page)
    targets = [
        ("https://trahe.eu", "Manufacturer site"),
        ("https://trahe.eu/lumina-pro.pdf", "Datasheet"),
    ]
    for href, label in targets:
        if not click_button(page, "+ Add hyperlink"):
            break
        wait_idle(page, 400)
        try:
            href_input = page.locator("input.hyperlink-href").last
            label_input = page.locator("input.hyperlink-label").last
            href_input.fill(href)
            href_input.press("Tab")  # blur → on:change → UpdateHyperlink
            label_input.fill(label)
            label_input.press("Tab")
            print(f"  · {label}: {href}")
        except Exception as e:
            print(f"  · hyperlink fill: {type(e).__name__}")
        wait_idle(page, 300)
    screenshot(page, "20-hyperlinks")


def act_16_pictures(page: Page) -> None:
    """Pictures: bind the product picture file we dropped in Act 2.

    Discover the image File id via the JS bridge (instead of relying on
    select.nth() positional guesses that broke when the file order
    shifted), then dispatch AddProductMetaPicture directly. Falls back
    to the UI path if the bridge isn't available."""
    print("\n[Act 16] Pictures pane")
    tid(page, "nav-pictures").click()
    wait_idle(page)
    import json as _json
    # Snapshot the current product to find the first image/png File id.
    # gldf_product_state() returns a JSON string directly, so we just
    # hand it back to Playwright unchanged.
    state_json = page.evaluate(
        "() => (window.gldf_product_state ? window.gldf_product_state() : null)"
    )
    file_id = None
    try:
        snap = _json.loads(state_json) if state_json else None
        if snap and isinstance(snap, dict):
            # serde renames are PascalCase to match the GLDF XML
            # element names. JSON serialisation uses the same renames.
            for f in (
                snap.get("GeneralDefinitions", {})
                .get("Files", {})
                .get("File", [])
            ):
                # @contentType is the XSD attribute name → serde
                # renames the Rust field `content_type` to "@contentType"
                # in serde output. Try both keys for robustness.
                ct = (f.get("@contentType") or f.get("content_type") or "").lower()
                if ct.startswith("image/"):
                    file_id = f.get("@id") or f.get("id")
                    break
            files_dbg = (
                snap.get("GeneralDefinitions", {})
                .get("Files", {})
                .get("File", [])
            )
            summary = [
                (
                    f.get("@id") or f.get("id"),
                    f.get("@contentType") or f.get("content_type"),
                )
                for f in files_dbg
            ]
            print(f"  · probe: files={summary}, image file_id={file_id}")
    except Exception as e:
        print(f"  · product-state probe: {type(e).__name__}: {e}")
    # Plan A: JS bridge dispatch (the modal-race-proof path).
    if file_id:
        payload = {
            "action": "AddProductMetaPicture",
            "data": {
                "scope": "Product",
                "file_id": file_id,
                "image_type": "Product Picture",
            },
        }
        err = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)",
            _json.dumps(payload),
        )
        if err:
            print(f"  · AddProductMetaPicture ERR: {err}")
        else:
            print(f"  · {file_id} → ProductMetaData.Pictures (Product Picture)")
    else:
        # Plan B: original UI fallback.
        try:
            selects = page.locator("select")
            if selects.count() >= 2:
                selects.nth(0).select_option(index=1, timeout=2_500)
                wait_idle(page, 200)
                click_button(page, "+ Add", timeout=3_000)
                wait_idle(page, 500)
                print("  · added picture binding (UI fallback)")
        except Exception as e:
            print(f"  · pictures: {type(e).__name__}")
    wait_idle(page, 500)
    screenshot(page, "21-pictures")


def act_17_emergency(page: Page) -> None:
    print("\n[Act 17] Emergency pane")
    tid(page, "nav-emergency").click()
    wait_idle(page)
    # Real option values come from EMERGENCY_LIGHTING_TYPES — strings
    # like "Exit Light", "Guide Light", etc.
    safe_select(page, by_id(page, "dedicated-emergency-type"), "Guide Light")
    click_button(page, "+ Add row")
    wait_idle(page, 500)
    screenshot(page, "22-emergency")


def act_18_marketing(page: Page) -> None:
    """Marketing: 3 housing colours (RAL per variant), 2 markets, 2 approvals, 1 award."""
    print("\n[Act 18] Marketing — housing colors + markets + approvals + awards")
    tid(page, "nav-marketing").click()
    wait_idle(page)
    screenshot(page, "23-marketing-empty")

    # Housing colors — scope per variant
    for v in VARIANTS:
        try:
            tid(page, "scope-picker-select").select_option(value=str(v["idx"]))
        except Exception:
            print(f"  · scope variant {v['idx']} missing — skipping")
            continue
        wait_idle(page, 400)
        tid(page, "marketing-add-housing-color").click()
        wait_idle(page, 400)
        select = tid(page, "marketing-ral-select-0")
        if select.count() > 0:
            select.select_option(value=str(v["ral"]))
            wait_idle(page, 400)
            print(f"  · variant {v['idx']} → RAL {v['ral']}")
            screenshot(page, f"24-marketing-variant{v['idx']}-ral{v['ral']}")

    # Product-scope: markets, approvals, awards.
    try:
        tid(page, "scope-picker-select").select_option(value="product")
        wait_idle(page)
    except Exception:
        pass

    # Markets / approvals / awards — fill THEN Tab so on:change fires
    # (Leptos inputs use on:change which only triggers on blur, not on
    # every keystroke). Without the Tab the values stayed in the DOM
    # but never landed in state, which is the "long unused pause"
    # users noticed: keystrokes happened, nothing visibly persisted.
    def fill_and_blur(selector: str, value: str) -> None:
        try:
            el = page.locator(selector).last
            el.fill(value)
            el.press("Tab")
        except Exception:
            pass

    # Markets — placeholder="EU" disambiguates from other inputs.
    for region in ("Europe", "DACH"):
        if click_button(page, "+ Add market"):
            wait_idle(page, 200)
            fill_and_blur('input[placeholder="EU"]', region)

    # Approval marks moved to their own sidebar entry (Act 14 fills
    # them on the "Approval Marks" page). Marketing pane no longer
    # carries the duplicate UI — one home only.

    # Design awards — placeholder="Red Dot 2024".
    if click_button(page, "+ Add design award"):
        wait_idle(page, 200)
        fill_and_blur(
            'input[placeholder="Red Dot 2024"]', "Trahe Light Award 2026"
        )

    screenshot(page, "25-marketing-final")


def act_19_custom_properties(page: Page) -> None:
    """Custom Properties is read-only — just navigate + screenshot."""
    print("\n[Act 19] Custom Properties (read-only)")
    tid(page, "nav-custom-properties").click()
    wait_idle(page, 400)
    screenshot(page, "26-custom-properties")


def act_20_bevy(page: Page) -> None:
    """🎬 3D variants: click each chip and HOLD each pose long enough
    that the user can read the housing colour + light CCT change in the
    recorded video.

    Pacing: each chip click is followed by ~4 seconds of hold (split
    over two screenshots so the SRT cue spans the full beat). Total
    runtime ≈ 4 variants × 4s + initial framing ≈ 18-20 s. No
    animation — chapter 3 is the static colour/CCT showcase. The Lisp
    REPL choreography lives in a separate bonus chapter at the end of
    the demo.
    """
    print("\n[Act 20] 3D variants (static)")
    tid(page, "nav-bevy-viewer").click()
    wait_idle(page, 4_000)
    page.wait_for_selector("#bevy-canvas", timeout=15_000)
    wait_idle(page, 2_500)
    screenshot(page, "27-bevy-default")
    for v in VARIANTS:
        chip = tid(page, f"bevy-variant-chip-{v['idx']}")
        if chip.count() == 0:
            continue
        chip.click()
        # Wait for the canvas to repaint with the new variant — the
        # L3D reload + housing material swap takes ~1.5 s. The second
        # wait extends the on-screen hold so the colour reads.
        wait_idle(page, 1_500)
        screenshot(page, f"28-bevy-variant{v['idx']}-ral{v['ral']}")
        wait_idle(page, 2_500)
        screenshot(page, f"28b-bevy-variant{v['idx']}-ral{v['ral']}-hold")
        print(f"  · variant {v['idx']} ({v['ral_name']}) held for ~4s")


def act_20a_variant_mountings_view(page: Page) -> None:
    """Show the Mountings on the Variants page (read view + Edit modal).

    Three frames captured:
      1. Variants page in View mode — the new `Mounting: Ceiling ·
         Recessed 50 mm · cutout 90×170 mm` row visible on every card.
      2. Per-variant ✎ Edit button clicked → modal opens.
      3. Scrolled to the Mountings editor section inside the modal —
         shows Recessed mode + depth 50 prefilled (proving the read
         value matches the editable value).

    Closes the modal at the end so subsequent acts run against the
    expected base state.
    """
    print("\n[Act 20a] Variants page — read Mountings + open Edit modal")
    tid(page, "nav-variants").click()
    wait_idle(page, 1_000)
    # Edit buttons only render in Edit mode. After a re-import or panic
    # the editor may have flipped back to View mode silently.
    ensure_edit_mode(page, label="act 20a")
    screenshot(page, "62a-variants-read-mountings")
    print("  · variants page — Mounting row visible per card")

    # Open the per-card Edit modal on variant 0. The button's testid is
    # `variant-edit-<id>`; the demo's variant ids after rename are
    # v_ral6018_yellowgreen, v_ral1023_trafficyellow, etc.
    edit_btn = page.locator('[data-testid^="variant-edit-"]').first
    if edit_btn.count() == 0:
        print("  · no ✎ Edit button found — skipping modal capture")
        return
    edit_btn.click()
    wait_idle(page, 800)
    screenshot(page, "62b-variant-edit-modal-open")
    print("  · ✎ Edit clicked, modal open")

    # The Mountings section sits below ProductSeries. Two scrolls:
    #
    #   62c — scroll to the Mountings *header* so the layout is obvious
    #   62d — scroll FURTHER so the recessed-depth numeric input + the
    #         value "50" is in frame (the actual confirmation that the
    #         editor reads back what the demo wrote)
    #
    # Both are captured because the recording user wants to see both
    # "yes there's a Mountings section" AND "yes the 50mm value is
    # already in there".
    try:
        page.get_by_text("Mountings", exact=False).first.scroll_into_view_if_needed(timeout=2_000)
    except Exception:
        pass
    wait_idle(page, 600)
    screenshot(page, "62c-variant-edit-mountings-section")
    print("  · scrolled to Mountings header")

    # Try to scroll the depth numeric input itself into view. The editor
    # uses a generic <input type="number"> for recessed_depth; look for
    # the labels "Recessed depth" / "depth" near it.
    try:
        # Easiest reliable handle: the numeric input that holds 50.
        page.locator('input[type="number"]').filter(has_text="50").first.scroll_into_view_if_needed(timeout=2_000)
    except Exception:
        try:
            page.get_by_text("Recessed depth", exact=False).first.scroll_into_view_if_needed(timeout=2_000)
        except Exception:
            # Fall back: scroll the inner modal content panel to the bottom.
            page.evaluate(
                "() => { const m = document.querySelector('.edit-modal-body, .modal-body, [data-testid=\"edit-modal\"] .scroll'); if (m) m.scrollTop = m.scrollHeight; }"
            )
    wait_idle(page, 800)
    screenshot(page, "62d-variant-edit-recessed-depth-50")
    print("  · scrolled to Recessed depth field — should show 50")

    # Close the modal so the page returns to a clean state for Act 20.
    try:
        page.keyboard.press("Escape")
        wait_idle(page, 400)
    except Exception:
        pass


def act_20b_lisp_repl(page: Page) -> None:
    """🎬 λ Lisp REPL: open the floating panel, run an *animated*
    choreography that orbits the camera while sine-tilting LEO0.

    Uses the engine's animation primitives — `ANIMATE-CAMERA` and
    `ANIMATE-TILT` — which on the Bevy side smoothstep-interpolate
    between successive targets. So a 4-call script becomes a 4-leg
    continuous trajectory, not 4 cuts.

    The on-screen sequence the user sees:
      1. λ button clicked → panel slides in with a clear comment-led
         script that reads like product choreography
      2. ▶ Run → camera flies through 3 viewpoints (front → side →
         below) while LEO0's spot sweeps ±30° on a sine
      3. screenshots mid-animation prove the *motion*, not just the
         end pose
      4. close button → panel disappears, leaving the final pose

    The visible payoff for the Lisp engine work: same primitive
    smoke-tested in `crates/gldf-rs-lisp/tests/smoke.rs` drives a
    real product showcase on the Bevy canvas.
    """
    print("\n[Act 20b] λ Lisp REPL — animated camera orbit + LEO tilt sweep")

    # Make sure we're on the Bevy viewer page (Act 20a parked us on
    # Variants).
    tid(page, "nav-bevy-viewer").click()
    wait_idle(page, 3_500)
    # Park on V0 (both LEOs lit) so the gimbal motion is dramatic.
    v0 = tid(page, f"bevy-variant-chip-0")
    if v0.count() > 0:
        v0.click()
        wait_idle(page, 2_500)

    # Capture the recessed-mounting baseline BEFORE the Lisp animation
    # touches anything — the luminaire should sit higher into the
    # ceiling now that variant Mountings (Ceiling/Recessed/50mm) reach
    # the Bevy side.
    screenshot(page, "65a-bevy-recessed-baseline")
    print("  · captured recessed-baseline (Mountings wired through to Bevy)")

    # 1) Open the panel.
    tid(page, "toolbar-lisp").click()
    wait_idle(page, 500)
    screenshot(page, "66-lisp-panel-open")
    print("  · panel open")

    # 2) Type the choreography script — slowly so the viewer can read it.
    #
    #    Total runtime ≈ 2.5 + 2.5 + 2.5 + 2.5 + 2.5 + 1.5 ≈ 14 s of
    #    continuous animation. Each leg is much longer than before
    #    (was ~700ms each) so the LEO tilt is clearly *animated*, not a
    #    snap. The script reads as one connected story; the engine
    #    chains each leg from wherever the previous one finished.
    #
    #    Camera path: a U-shape — front-and-low → side-and-low →
    #    overhead-and-near. Tilt: 0° → 30° outward → -30° opposite →
    #    back to 0° (a sine wobble).
    # Simplified, focused choreography:
    # - Step A: tilt LEO0 to +35° and pause to SEE it. Camera stays put
    #   at the auto-framed default pose, so the luminaire stays in
    #   frame. The tilt of the cone IS the show.
    # - Step B: tilt LEO0 to -35°, slow.
    # - Step C: orbit the camera 90° around the luminaire (one smooth
    #   move so we *also* see it from the side).
    # - Step D: tilt back to 0°, camera back to home.
    #
    # Target = LEO centroid (hand-tuned from `frame_camera_on_l3d`
    # comment: ~ (2.000, 2.493, 2.500)). Camera home is exactly the
    # baked default pose: target + (0.561, -0.400, 2.058) =
    # (2.561, 2.093, 4.558).
    # FULL choreography — the Lisp engine drives EVERYTHING:
    # - camera glides (ANIMATE-CAMERA, smoothstep chained)
    # - LEO inner-ring tilts (ANIMATE-TILT, both LEOs)
    # - variant swaps mid-flight (GLDF-VARIANT-SET → Leptos Scope signal
    #   → BevyViewerPage reload with new housing colour + emitter CCT)
    #
    # The script reads top-to-bottom as ONE continuous story; the engine
    # dispatches each command as the parser hits it, and the Bevy side
    # chains animations from wherever the previous one finished.
    #
    # Target = LEO centroid for Tria 2-style luminaire, ~(2.0, 2.5, 2.5).
    # Home pose comes from frame_camera_on_l3d's baked default:
    # eye = (2.561, 2.093, 4.558), looking at LEO centroid.
    #
    # Sequence (~30s):
    #   1. (0–4s)  glide to a low front view — see the housing from below
    #   2. (4–8s)  swap to variant 1 (traffic yellow) + counter-tilt LEO1
    #   3. (8–14s) orbit to side view while tilting LEO0 +30°
    #   4. (14–20s) swap to variant 2 (signal white) + camera rises overhead
    #   5. (20–26s) swap to variant 3 (jet black emergency) + zoom in
    #   6. (26–30s) return to variant 0, camera glides home, tilts reset
    # ANIMATE-TILT signature: (ANIMATE-TILT joint x-deg y-deg z-deg duration-ms)
    # The three angles are the TARGET Euler rotation, NOT a from/via/to path.
    # So (… 0.0 0.0 0.0 …) animates the joint from its current rotation
    # back to (0,0,0) over the given duration. Order is x, y, z.
    #
    # Chapter 6 bonus showcase — user's hand-tuned demo Lisp:
    #   * snap camera to a hero pose
    #   * tilt both heads outward to +33° on variant 0 (yellow-green)
    #   * tilt them back to -33°
    #   * swap to variant 1 (traffic yellow), tilt out again
    #   * swap to variant 2 (signal white), tilt back
    # The OUTBOX drainer waits for each ANIMATE-TILT's full duration_ms
    # so the script's wall-clock matches the sum of tilt durations.
    # Variant swaps fire mid-tilt and read against the moving gimbal.
    # Hero pose — set once at the top, then re-used as the return
    # target after each "under the lamp" excursion. Keeping the same
    # six numbers everywhere means the camera always comes back to
    # the EXACT same framing the user picked with C.
    hero_eye = "2.619 2.303 4.968"
    hero_tgt = "1.987 1.870 2.657"
    script = (
        "; SLV — Lisp-driven hero shot.\n"
        "; Each variant gets: dip under the lamp for the colour\n"
        "; reveal, fly back to the hero pose, then tilt the heads.\n"
        f"(RESET-TILTS)\n"
        f"(GLDF-VARIANT-SET 0)\n"
        f"(CAMERA-SET {hero_eye}  {hero_tgt})\n"
        # Variant 0 — yellow-green hero
        f"(ANIMATE-TILT \"l3d://LEO0\" 33.0 0.0 0.0  2000.0)\n"
        f"(ANIMATE-TILT \"l3d://LEO1\" 33.0 0.0 0.0  2000.0)\n"
        f"(SLEEP 500)\n"
        f"(ANIMATE-TILT \"l3d://LEO0\" -33.0 0.0 0.0  2000.0)\n"
        f"(ANIMATE-TILT \"l3d://LEO1\" -33.0 0.0 0.0  2000.0)\n"
        # Dip-camera pose used between variants (eye + look-at target,
        # all six numbers captured with the C key over the canvas).
        # Same pose for every dip so the camera is in a known shape
        # when the variant swap fires.
        f"(ANIMATE-CAMERA 2.338 2.303 3.395  2.068 2.640 2.464  1500.0)\n"
        f"(GLDF-VARIANT-SET 1)\n"
        f"(SLEEP 1500)\n"
        f"(ANIMATE-CAMERA {hero_eye}  {hero_tgt}  1500.0)\n"
        # Variant 1 — traffic yellow
        f"(ANIMATE-TILT \"l3d://LEO0\" 33.0 0.0 0.0  2000.0)\n"
        f"(ANIMATE-TILT \"l3d://LEO1\" 33.0 0.0 0.0  2000.0)\n"
        # Dip, swap, back to hero
        f"(ANIMATE-CAMERA 2.338 2.303 3.395  2.068 2.640 2.464  1500.0)\n"
        f"(GLDF-VARIANT-SET 2)\n"
        f"(SLEEP 1500)\n"
        f"(ANIMATE-CAMERA {hero_eye}  {hero_tgt}  1500.0)\n"
        # Variant 2 — signal white
        f"(ANIMATE-TILT \"l3d://LEO0\" -33.0 0.0 0.0  2000.0)\n"
        f"(ANIMATE-TILT \"l3d://LEO1\" -33.0 0.0 0.0  2000.0)\n"
        # Final return to hero + zero tilt — clean re-runnable end state
        f"(ANIMATE-CAMERA {hero_eye}  {hero_tgt}  1500.0)\n"
        f"(ANIMATE-TILT \"l3d://LEO0\" 0.0 0.0 0.0  1500.0)\n"
        f"(ANIMATE-TILT \"l3d://LEO1\" 0.0 0.0 0.0  1500.0)\n"
    )
    tid(page, "lisp-panel-source").fill(script)
    wait_idle(page, 800)
    # SHOW the typed script for ~7s so the viewer can actually read the
    # longer choreography (it's now 30+ lines including comments + 4
    # variant swaps). Three holds with wait_idle keep the panel on
    # screen during recording.
    screenshot(page, "67-lisp-panel-typed")
    wait_idle(page, 2_500)
    screenshot(page, "67b-lisp-panel-typed-hold")
    wait_idle(page, 2_500)
    screenshot(page, "67c-lisp-panel-typed-hold")
    print("  · script visible for ~7s before Run")

    # 3) Run — fire-and-forget; the engine fills its OUTBOX, then the
    # WASM panel's setTimeout drainer walks the commands, waiting for
    # each ANIMATE-TILT's full duration before the next dispatch.
    tid(page, "lisp-panel-run").click()
    print("  · ▶ Run clicked — choreography playing (~28-30s)")

    # 4) Capture frames at the natural beats. Timeline:
    #   0–4s    variant 0 tilt out to +33°
    #   4.5s    SLEEP
    #   4.5–8.5s variant 0 tilt back to -33°
    #   8.5–10s dip under lamp
    #   10–11.5s VARIANT-SET 1 + 1.5s SLEEP for canvas reload
    #   11.5–13s fly back to hero
    #   13–17s  variant 1 tilt to +33° (yellow housing)
    #   17–18.5s dip under
    #   18.5–20s VARIANT-SET 2 + reload
    #   20–21.5s fly back to hero
    #   21.5–25.5s variant 2 tilt to -33° (white housing)
    #   25.5–27s fly back to hero (final)
    #   27–28.5s reset both heads to 0°
    wait_idle(page, 2_000)
    screenshot(page, "68-lisp-anim-variant0-tilt-out")
    wait_idle(page, 2_500)
    screenshot(page, "68a-lisp-anim-variant0-tilt-back")
    wait_idle(page, 4_000)
    screenshot(page, "68b-lisp-anim-dip-under-lamp")
    wait_idle(page, 3_000)
    screenshot(page, "68c-lisp-anim-variant1-traffic-yellow")
    wait_idle(page, 2_000)
    screenshot(page, "68d-lisp-anim-variant1-back-hero")
    wait_idle(page, 3_000)
    screenshot(page, "68e-lisp-anim-variant1-tilting")
    wait_idle(page, 2_500)
    screenshot(page, "68f-lisp-anim-variant2-signal-white")
    wait_idle(page, 3_500)
    screenshot(page, "68g-lisp-anim-variant2-tilting")
    wait_idle(page, 3_000)
    screenshot(page, "68h-lisp-anim-final-hero")
    print("  · played ~28s choreography (3 variants × tilt + dip + return)")

    # 5) Close.
    tid(page, "lisp-panel-close").click()
    wait_idle(page, 400)
    screenshot(page, "70-lisp-panel-closed")
    print("  · panel closed")


def act_21_export(page: Page) -> Path | None:
    """Showcase the new export-options modal: tick DIALux-compat,
    then confirm. Mirrors `bridge_export_gldf` but with on-screen
    screenshots so the recording captures the dialog."""
    print("\n[Act 21] Export GLDF")
    tid(page, "toolbar-export-gldf").click()
    wait_idle(page, 400)
    screenshot(page, "28-export-options-modal")
    tid(page, "export-opt-dialux").check()
    with page.expect_download(timeout=15_000) as dl:
        tid(page, "export-options-confirm").click()
    download = dl.value
    download.save_as(str(EXPORTED_GLDF))
    print(f"  · saved {EXPORTED_GLDF.relative_to(REPO_ROOT)} ({EXPORTED_GLDF.stat().st_size // 1024} KB)")
    screenshot(page, "29-after-export")
    return EXPORTED_GLDF


def act_21b_xsd_validate(gldf_path: Path) -> None:
    """Spawn `gldf-cli validate` against the exported GLDF. Non-blocking:
    the demo prints the verdict but doesn't abort if findings remain —
    several known issues live across the Variant + Hyperlink models and
    are tracked separately. The point of this Act is the *trend line*:
    the export pipeline now runs the official XSD on every shipped GLDF
    and surfaces real bugs early."""
    import subprocess
    print("\n[Act 21b] XSD validation (uppsala 0.4 against rc-3.xsd)")
    proc = subprocess.run(
        ["cargo", "run", "-q", "-p", "gldf-rs-cli", "--", "validate",
         "-i", str(gldf_path)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
    )
    out = (proc.stdout or "") + (proc.stderr or "")
    if proc.returncode == 0:
        print(f"  ✓ XSD clean")
    else:
        # Count findings from the CLI output (each starts with `  [`).
        lines = [ln for ln in out.splitlines() if ln.strip().startswith("[")]
        print(f"  ⚠ {len(lines)} XSD finding(s) (first 5):")
        for ln in lines[:5]:
            print(f"    {ln.strip()}")
        if len(lines) > 5:
            print(f"    ... +{len(lines) - 5} more")


def act_22_reimport_walkthrough(page: Page, gldf_path: Path) -> None:
    """Clear → drop saved GLDF → walk every read-mode pane to prove persistence."""
    print("\n[Act 22] Re-import + walk every pane in view mode")
    page.on("dialog", lambda d: d.accept())
    tid(page, "toolbar-clear").click()
    wait_idle(page, 800)
    expect(tid(page, "welcome-open-wizard")).to_be_visible(timeout=10_000)
    tid(page, "welcome-file-input").set_input_files(str(gldf_path))
    wait_idle(page, 2_500)
    expect(tid(page, "nav-overview")).to_be_visible()
    screenshot(page, "30-reimport-overview")

    walks = [
        ("nav-overview", "31-rv-overview"),
        ("nav-linkage", "32-rv-linkage"),
        ("nav-raw-data", "33-rv-rawdata"),
        ("nav-file-viewer", "34-rv-fileviewer"),
        ("nav-header", "35-rv-header"),
        ("nav-electrical", "36-rv-electrical"),
        ("nav-mechanical", "37-rv-mechanical"),
        ("nav-applications", "38-rv-applications"),
        ("nav-labels", "39-rv-labels"),
        ("nav-hyperlinks", "40-rv-hyperlinks"),
        ("nav-pictures", "41-rv-pictures"),
        ("nav-marketing", "42-rv-marketing"),
        ("nav-emergency", "43-rv-emergency"),
        ("nav-custom-properties", "44-rv-custom-properties"),
        ("nav-photometry", "45-rv-photometry"),
        ("nav-files", "46-rv-files"),
        ("nav-light-sources", "47-rv-light-sources"),
        ("nav-emitters", "48-rv-emitters"),
        ("nav-variants", "49-rv-variants"),
        ("nav-geometries", "50-rv-geometries"),
        ("nav-control-gear", "51-rv-control-gear"),
        ("nav-equipment", "52-rv-equipment"),
        ("nav-bevy-viewer", "53-rv-bevy"),
    ]
    for nav, shot in walks:
        loc = tid(page, nav)
        if loc.count() == 0:
            print(f"  · {nav} missing")
            continue
        loc.click()
        wait_idle(page, 1_200)
        if nav == "nav-bevy-viewer":
            wait_idle(page, 3_000)
        screenshot(page, shot)
        print(f"  · {nav} ✓")

    for v in VARIANTS:
        chip = tid(page, f"bevy-variant-chip-{v['idx']}")
        if chip.count() == 0:
            continue
        chip.click()
        wait_idle(page, 1_500)
        screenshot(page, f"54-rv-bevy-variant{v['idx']}-ral{v['ral']}")
        print(f"  · re-import bevy variant {v['idx']} ✓")


def act_23_export_tm33(page: Page) -> Path | None:
    """Download lightsource_2's TM-33 XML (photometry + SPD in ONE file).

    Re-import in Act 22 doesn't enter edit mode on the Light Sources
    page, so the ✎ Edit / 📥 Download menus aren't visible. Flip the
    toolbar to Editor mode first so the LightSourceExportMenu shows
    up next to each LS card."""
    print("\n[Act 23] Per-LS TM-33 export — lightsource_2 (photometry + SPD)")
    # Editor mode toggles the ✎/📥 cluster on each LS card.
    tid(page, "toolbar-toggle-mode").click()
    wait_idle(page, 600)
    tid(page, "nav-light-sources").click()
    wait_idle(page, 800)

    # After Act 8c the LS got renamed to ls_led_3000k_warm.
    sel = tid(page, "ls-export-ls_led_3000k_warm")
    if sel.count() == 0:
        print("  · ls-export-ls_led_3000k_warm missing — skipped")
        return None
    tm33 = OUT_DIR / "ls_led_3000k_warm.tm33.xml"
    try:
        with page.expect_download(timeout=15_000) as dl:
            sel.select_option(value="tm33")
        download = dl.value
        download.save_as(str(tm33))
    except Exception as e:
        print(f"  · TM-33 download ERR: {e}")
        return None

    size = tm33.stat().st_size
    body = tm33.read_text(errors="replace")
    # TM-33-23 uses <LuminousData>, the older ATLA-S001 schema uses
    # <IntensityDistribution>. Accept either so the assertion isn't
    # version-locked. Same for SPD ("SpectralDistribution") vs the
    # generic "Wavelength" rows.
    has_photo = any(t in body for t in ("LuminousData", "IntensityDistribution", "Candela"))
    has_spd = ("SpectralDistribution" in body) or ("Wavelength" in body)
    print(f"  · saved {tm33.relative_to(REPO_ROOT)} ({size // 1024} KB)")
    print(f"  · photometry block: {'✓' if has_photo else '✗'}  "
          f"SPD block: {'✓' if has_spd else '✗'}")
    if not has_photo or not has_spd:
        print("  ⚠ TM-33 missing one of the halves — check that "
              "lightsource_2 is bound by an emitter AND has a SpectrumReference")
    screenshot(page, "55-tm33-exported")
    return tm33


def act_24_reimport_tm33_to_files(page: Page, tm33_path: Path) -> None:
    """Drop the saved .tm33.xml onto the Files page so it appears next
    to the LDTs and the .spd, proving both photometry+spectrum-in-one
    survived round-trip outside the GLDF container."""
    print("\n[Act 24] Drop TM-33 XML back into Files")
    tid(page, "nav-files").click()
    wait_idle(page, 600)
    inp = tid(page, "files-add-input")
    if inp.count() == 0:
        print("  · files-add-input missing — skipped")
        return
    inp.set_input_files(str(tm33_path))
    wait_idle(page, 1500)
    print(f"  · {tm33_path.name} added to Files")
    screenshot(page, "56-files-with-tm33")


def act_25_tm33_as_starter(page: Page, tm33_path: Path) -> None:
    """Clear the editor, drop the TM-33 XML on the Welcome screen, and
    confirm the editor bootstraps a fresh GLDF from it: one Photometry
    (synthesised LDT), one Spectrum (synthesised .spd), one
    FixedLightSource with SpectrumReference, one Emitter binding both,
    one Variant. Then walk the Files / Light Sources / Spectra panes
    to prove the new starter project is structurally complete."""
    print("\n[Act 25] Drop TM-33 XML as starter — bootstraps a fresh GLDF")
    # Bail-out path: Clear button pops a confirm() dialog.
    page.on("dialog", lambda d: d.accept())
    tid(page, "toolbar-clear").click()
    wait_idle(page, 800)
    expect(tid(page, "welcome-open-wizard")).to_be_visible(timeout=10_000)
    screenshot(page, "57-welcome-before-tm33-drop")
    tid(page, "welcome-file-input").set_input_files(str(tm33_path))
    wait_idle(page, 2_500)
    # tm33_to_gldf success = Overview pane available.
    expect(tid(page, "nav-overview")).to_be_visible(timeout=10_000)
    screenshot(page, "58-after-tm33-bootstrap-overview")
    for nav, shot in [
        ("nav-header", "59-tm33-bootstrap-header"),
        ("nav-files", "60-tm33-bootstrap-files"),
        ("nav-light-sources", "61-tm33-bootstrap-light-sources"),
        ("nav-emitters", "62-tm33-bootstrap-emitters"),
        ("nav-variants", "63-tm33-bootstrap-variants"),
        ("nav-photometry", "64-tm33-bootstrap-photometry"),
    ]:
        loc = tid(page, nav)
        if loc.count() == 0:
            print(f"  · {nav} missing")
            continue
        loc.click()
        wait_idle(page, 1_200)
        screenshot(page, shot)
        print(f"  · {nav} ✓")
    # Spectra tab uses NavItem::Spectra ("nav-spectra"). Take a look so
    # we capture the SPD curve the TM-33 brought along.
    spectra = tid(page, "nav-spectra")
    if spectra.count() > 0:
        spectra.click()
        wait_idle(page, 1_500)
        screenshot(page, "65-tm33-bootstrap-spectra")
        print("  · nav-spectra ✓")


# ──────────────────────────────────────────────────────────────────────
# Video post-process (ffmpeg slo-mo → demo.mp4)


def slow_down_video(raw: Path, dst: Path, factor: float) -> bool:
    if not shutil.which("ffmpeg"):
        print("⚠ ffmpeg not on PATH — skipping demo.mp4")
        return False
    if not raw.exists():
        return False
    cmd = [
        "ffmpeg", "-y", "-i", str(raw),
        "-filter:v", f"setpts={factor}*PTS",
        "-an", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "23",
        str(dst),
    ]
    print(f"\n🎞  ffmpeg slo-mo ({factor}×) → {dst.relative_to(REPO_ROOT)}")
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode != 0:
        print(r.stderr[-800:], file=sys.stderr)
        return False
    return True


# ──────────────────────────────────────────────────────────────────────
# Driver


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--headless", action="store_true")
    parser.add_argument("--slow-mo", type=int, default=300,
                        help="ms between Playwright actions (default 300)")
    parser.add_argument("--slowdown", type=float, default=2.5,
                        help="ffmpeg playback slowdown factor (default 2.5)")
    parser.add_argument("--no-video", action="store_true")
    parser.add_argument("--url", default=URL)
    parser.add_argument(
        "--chapters",
        default=None,
        help="Chapter subset to record, e.g. '1', '1,3', '2-4'. "
             "Default: all 5 chapters.",
    )
    parser.add_argument(
        "--fail-fast",
        action="store_true",
        help="Abort the run on the first chapter that errors. "
             "Default: keep going (each chapter flushes its own video).",
    )
    parser.add_argument(
        "--reset-state",
        action="store_true",
        help="Wipe the saved chapter storage_state snapshot before "
             "starting. Use when re-recording chapter 1 from scratch.",
    )
    args = parser.parse_args()

    import urllib.request
    try:
        with urllib.request.urlopen(args.url, timeout=2) as resp:
            if resp.status != 200:
                print(f"⚠ {args.url} HTTP {resp.status}", file=sys.stderr)
                return 2
    except Exception as e:
        print(f"⚠ {args.url} unreachable: {e}", file=sys.stderr)
        return 2

    for fname in ("middle.ldt", "narrow.ldt", "wide.ldt", "example_007.l3d"):
        if not (ASSETS / fname).is_file():
            print(f"⚠ missing {ASSETS / fname}", file=sys.stderr)
            return 2

    OUT_DIR.mkdir(parents=True, exist_ok=True)

    # Chapter manifest with docstrings pulled from this module so the
    # narration scripts stay in sync with the actual Act docstrings.
    # `tests/e2e/` isn't a package, so use a sys.path insert for the
    # sibling-module import (this script is always invoked via the
    # `run_trahe.sh` wrapper which already activates a venv).
    import os as _os
    import sys as _sys
    _sys.path.insert(0, _os.path.dirname(__file__))
    import chapters as _chmod  # type: ignore  # noqa
    chapters_list = _chmod.build_chapters(_sys.modules[__name__])
    selected = _chmod.parse_chapter_selection(args.chapters, len(chapters_list))

    # --reset-state OR running chapter 1 explicitly: wipe the saved
    # storage_state so the welcome page starts fresh. Otherwise chapter 1
    # would resume from whatever the previous run left in localStorage,
    # which is almost certainly NOT what the user wants when re-recording
    # chapter 1.
    saved_state = _chmod._storage_state_path(OUT_DIR)
    if args.reset_state or (1 in selected and saved_state.exists()):
        try:
            saved_state.unlink()
            print(f"  🧹 wiped {saved_state.relative_to(REPO_ROOT)}")
        except Exception:
            pass

    print(f"\n🎬 Trahe ACME demo — recording chapters {sorted(selected)}\n")

    def _on_console_error(text: str) -> None:
        print(f"  [browser:error] {text}")

    # Shared cross-chapter state. Chapter 4 produces `gldf` (Act 21).
    # Chapter 5 consumes `gldf` indirectly (its acts re-export TM-33 from
    # the current editor state) and produces `tm33` (Act 23) that flows
    # into Acts 24+25 INSIDE the same chapter.
    state: dict = {"gldf": None, "tm33": None}

    overall_rc = 0
    with sync_playwright() as p:
        for ch in chapters_list:
            if ch.number not in selected:
                continue
            try:
                with _chmod.open_chapter(
                    p,
                    ch,
                    OUT_DIR,
                    REPO_ROOT,
                    headless=args.headless,
                    slow_mo=args.slow_mo,
                    slowdown=args.slowdown,
                    no_video=args.no_video,
                    on_console_error=_on_console_error,
                ) as session:
                    global _ACTIVE_SESSION
                    _ACTIVE_SESSION = session
                    try:
                        _run_chapter(ch, session, state)
                        _post_chapter_bridge(ch, session)
                    finally:
                        _ACTIVE_SESSION = None
            except Exception as e:
                print(f"\n❌ chapter {ch.dir_name} failed: {e}", file=sys.stderr)
                overall_rc = 1
                if args.fail_fast:
                    break

    print(f"\n✅ done — chapters in {(OUT_DIR / 'chapters').relative_to(REPO_ROOT)}/")
    return overall_rc


BRIDGE_GLDF = OUT_DIR / "_bridge.gldf"


def _run_chapter(chapter, session, state: dict) -> None:
    """Invoke the Acts for one chapter. Each Act is preceded by a
    `session.cue(...)` so the SRT lines up with the recording.

    Chapters 2–5 begin by importing the previous chapter's exported
    GLDF (`BRIDGE_GLDF`); chapters 1–4 end by re-exporting it so the
    next chapter resumes the same editor state. Without this each
    chapter would start at the welcome page and have nothing to drive."""
    page = session.page
    n = chapter.number

    # Bridge-load at chapter start (skip chapter 1 — it builds from
    # scratch via the wizard).
    if n > 1:
        bridge_import_gldf(page, BRIDGE_GLDF)
        ensure_edit_mode(page, label=f"chapter {n} start")

    if n == 1:
        # 1–8c: build the GLDF (wizard + extras + variants wired).
        session.cue("Act 1 — Wizard")
        act_1_wizard(page)
        session.cue("Act 2 — Files")
        act_2_files(page)
        session.cue("Act 3 — Light Sources")
        act_3_light_sources(page)
        session.cue("Act 4 — Control Gear")
        act_4_control_gear(page)
        session.cue("Act 5 — Equipment")
        act_5_equipment(page)
        session.cue("Act 6 — Emitters")
        act_6_emitters(page)
        session.cue("Act 7 — Variants (duplicate + GTINs)")
        act_7_variants_duplicate(page)
        session.cue("Act 8 — Rebind emitters per variant")
        act_8_rebind_emitters_per_variant(page)
        session.cue("Act 8c — Rename auto-ids → semantic")
        act_8c_rename_ids(page)
        return

    if n == 2:
        # 9–19: fill every ProductMetaData pane.
        session.cue("Act 9 — Header pane")
        act_9_header_polish(page)
        session.cue("Act 10 — Photometry pane")
        act_10_photometry(page)
        session.cue("Act 11 — Electrical pane")
        act_11_electrical(page)
        session.cue("Act 12 — Mechanical pane")
        act_12_mechanical(page)
        session.cue("Act 13 — Applications pane")
        act_13_applications(page)
        session.cue("Act 14 — Approval Marks pane")
        act_14_labels(page)
        session.cue("Act 15 — Hyperlinks pane")
        act_15_hyperlinks(page)
        session.cue("Act 16 — Pictures pane")
        act_16_pictures(page)
        session.cue("Act 17 — Emergency pane")
        act_17_emergency(page)
        session.cue("Act 18 — Marketing pane")
        act_18_marketing(page)
        session.cue("Act 19 — Custom Properties pane")
        act_19_custom_properties(page)
        return

    if n == 3:
        # Edit variants first (open the Edit modal to show what's
        # configurable per variant), then walk through each variant
        # statically in the 3D viewer so the housing colour + CCT
        # change is visible on a held pose. The Lisp REPL act is
        # NOT part of chapter 3 — it lives in a separate bonus
        # chapter at the end.
        session.cue("Act 20a — Variant mountings view (edit)")
        act_20a_variant_mountings_view(page)
        session.cue("Act 20 — Bevy 3D variants (static)")
        act_20_bevy(page)
        return

    if n == 4:
        # 21–22: export, validate, re-import walkthrough.
        session.cue("Act 21 — Export GLDF")
        gldf = act_21_export(page)
        state["gldf"] = gldf
        if gldf is not None:
            session.cue("Act 21b — XSD validate")
            act_21b_xsd_validate(gldf)
            session.cue("Act 22 — Re-import walkthrough")
            act_22_reimport_walkthrough(page, gldf)
        else:
            print("⚠ chapter 4: Act 21 returned no GLDF, skipping validate + reimport")
        return

    if n == 5:
        # 23–25: TM-33 round-trip + bootstrap.
        session.cue("Act 23 — Export TM-33")
        tm33 = act_23_export_tm33(page)
        state["tm33"] = tm33
        if tm33 is not None:
            session.cue("Act 24 — Re-import TM-33 to Files")
            act_24_reimport_tm33_to_files(page, tm33)
            session.cue("Act 25 — TM-33 as starter")
            act_25_tm33_as_starter(page, tm33)
        else:
            print("⚠ chapter 5: Act 23 produced no TM-33, skipping reimport + bootstrap")
        # Chapter 5's final state is the TM-33-bootstrapped GLDF, which
        # has only 1 variant — but the Lisp REPL bonus (chapter 6)
        # wants the original 4-variant Trahe ACME product so the
        # `(GLDF-VARIANT-SET …)` calls work. We skip the bridge-save
        # here so chapter 6 picks up chapter 4's bridge GLDF instead.
        return

    if n == 6:
        # Bonus chapter — Lisp REPL choreography on the 4-variant
        # Trahe Lumina Pro from chapter 4's bridge GLDF. Pure
        # showcase, no workflow steps.
        session.cue("Act 20b — Lisp REPL choreography")
        act_20b_lisp_repl(page)
        return

    raise RuntimeError(f"unknown chapter number {n}")


def _post_chapter_bridge(chapter, session) -> None:
    """Called after `_run_chapter` returns; saves the current editor
    state in two places:

      * ``BRIDGE_GLDF`` — the rolling hand-off used by the next chapter.
        Skipped after chapter 5 (TM-33 bootstrap loses variants we need
        downstream) and chapter 6 (nothing follows).

      * ``OUT_DIR / chapter-NN.gldf`` — a per-chapter snapshot we keep
        around so we can validate / DIALux-import / inspect the
        editor's intermediate state at each phase. Always written, for
        every chapter, so the validate loop downstream has a complete
        set of artefacts to chew on."""
    snapshot = OUT_DIR / f"chapter-{chapter.number:02d}.gldf"
    # Per-chapter snapshots get DIALux-compat so the artifacts on disk
    # can be dropped straight into DIALux without further sanitisation.
    # The rolling bridge save (below) stays canonical — chapter N+1
    # imports it back into the editor and the editor expects original
    # bytes (no comment-stripped SPDs).
    if bridge_export_gldf(session.page, snapshot, dialux_compat=True):
        # Inline validate so the demo log shows the verdict per chapter
        # — quickly tells us whether the always-emit UniqueProductId
        # fix lands and which chapter, if any, regresses XSD-cleanness.
        act_21b_xsd_validate(snapshot)
    if chapter.number >= 5:
        return
    bridge_export_gldf(session.page, BRIDGE_GLDF)


if __name__ == "__main__":
    raise SystemExit(main())
