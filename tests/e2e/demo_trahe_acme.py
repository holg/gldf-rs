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

# Variant fixtures. Each one gets its own (LDT, LightSource CCT, RAL paint,
# named Emitter). Order matters: the script duplicates variant 0 twice in
# Act 2 so that variants 1+2 exist BEFORE Marketing tries to scope-switch
# to them.
VARIANTS = [
    {
        "idx": 0,
        "ral": 6018,
        "ral_name": "yellow green",
        "ldt": "middle.ldt",
        "cct": 4000,
        "flux": 1600,
        "weight_kg": 0.379,
        "ls_name": "LED 4000K module",
        # The wizard's emitter draft is skipped in Act 1 (we click Next
        # through the Emitter step without filling fields), so no
        # emitter is auto-created. Act 6 adds 3 fresh ones via the
        # form → emitter_1, emitter_2, emitter_3, one per variant.
        "emitter_id": "emitter_1",
        "photometry_id": "photometry_1",
        "lightsource_id": "lightsource_1",
    },
    {
        "idx": 1,
        "ral": 1023,
        "ral_name": "traffic yellow",
        "ldt": "narrow.ldt",
        "cct": 3000,
        "flux": 1800,
        "weight_kg": 0.479,
        "ls_name": "LED 3000K module",
        "emitter_id": "emitter_2",
        "photometry_id": "photometry_2",
        "lightsource_id": "lightsource_2",
    },
    {
        "idx": 2,
        "ral": 9003,
        "ral_name": "signal white",
        "ldt": "wide.ldt",
        "cct": 5000,
        "flux": 2200,
        "weight_kg": 0.479,
        "ls_name": "LED 5000K module",
        "emitter_id": "emitter_3",
        "photometry_id": "photometry_3",
        "lightsource_id": "lightsource_3",
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


def screenshot(page: Page, name: str) -> None:
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
    """Welcome → 8-step wizard. Trahe ACME / Holger Trahe / Lumina Pro /
    middle.ldt + L3D + Driver (Trahe 24V dimmable).

    The wizard now has 8 steps (Driver landed between Geometry and
    Variant — task #102). Act 1 USES it: ticks `Include a driver`, fills
    name + dimmable + standby. On Finish the lib mints a baseline
    ControlGear + Equipment + ControlGearReference on the wizard's
    emitter. Acts 4 and 5 below then REFINE those entries instead of
    creating new ones."""
    print("\n[Act 1] Wizard — Trahe ACME seed (with Driver step)")
    page.goto(URL)
    expect(tid(page, "welcome-open-wizard")).to_be_visible(timeout=15_000)
    screenshot(page, "01-welcome")
    tid(page, "welcome-open-wizard").click()
    wait_idle(page)

    # Step 1 — Header
    tid(page, "wizard-header-author").fill("Holger Trahe")
    tid(page, "wizard-header-manufacturer").fill("Trahe ACME")
    screenshot(page, "02-wizard-header")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 2 — Photometry
    tid(page, "wizard-photometry-input").set_input_files(str(ASSETS / "middle.ldt"))
    wait_idle(page, 700)
    screenshot(page, "03-wizard-photometry")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 3 — LightSource (defaults)
    tid(page, "wizard-next").click()
    wait_idle(page)
    # Step 4 — Emitter (defaults)
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 5 — Geometry (drop the L3D)
    tid(page, "wizard-geometry-input").set_input_files(str(ASSETS / "example_007.l3d"))
    wait_idle(page, 700)
    screenshot(page, "04-wizard-geometry")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 6 — Driver (NEW). Tick include + fill the three fields.
    # apply_wizard mints ControlGear + Equipment + ControlGearReference
    # on Finish so the downstream Acts can REFINE instead of create.
    tid(page, "wizard-driver-enabled").check()
    wait_idle(page, 300)
    tid(page, "wizard-driver-name").fill("Trahe Driver 24V")
    tid(page, "wizard-driver-dimmable").check()
    tid(page, "wizard-driver-standby").fill("0.3")
    screenshot(page, "04b-wizard-driver")
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 7 — Variant (defaults)
    tid(page, "wizard-next").click()
    wait_idle(page)

    # Step 8 — Review + Create
    screenshot(page, "05-wizard-review")
    tid(page, "wizard-create").click()
    wait_idle(page, 1200)
    expect(tid(page, "nav-overview")).to_be_visible()
    screenshot(page, "06-overview-after-create")

    tid(page, "toolbar-toggle-mode").click()
    wait_idle(page)


def act_2_files(page: Page) -> None:
    """Files page: drop narrow.ldt + wide.ldt + product picture + 2700K SPD.

    The .spd auto-registers a `<Spectrum>` entry (per the Files-page
    drop handler — see crates/gldf-rs-editor action AddSpectrumFile-by-
    drop path). Act 3 then binds it to lightsource_2 so the TM-33
    export in Act 23 has both a photometric matrix AND an SPD curve to
    embed in ONE file."""
    print("\n[Act 2] Files — drop 2 LDTs + product picture + SPD")
    tid(page, "nav-files").click()
    wait_idle(page)
    extras = [
        str(ASSETS / "narrow.ldt"),
        str(ASSETS / "wide.ldt"),
    ]
    pic = ASSETS / "product_picture.png"
    if pic.is_file():
        extras.append(str(pic))
    spd = ASSETS / "spectrum_2700K.spd"
    if spd.is_file():
        extras.append(str(spd))
    tid(page, "files-add-input").set_input_files(extras)
    wait_idle(page, 1500)
    print(f"  · dropped {len(extras)} files")
    screenshot(page, "07-files")


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
        tid(page, "ls-edit-power").fill("18.5")
        wait_idle(page, 200)
        tid(page, "ls-edit-save").click()
        wait_idle(page, 600)
        print(f"  · lightsource_{i + 1} → {v['ls_name']} ({v['cct']} K)")

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
    payload = {
        "action": "UpdateEquipment",
        "data": {
            "id": "equipment_1",
            "light_source_id": "lightsource_1",
            "light_source_kind": "Fixed",
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
    """Emitters: add 3 fresh emitters (the wizard skipped its emitter
    step), then set each emitter's RatedLuminousFlux via the JS bridge.

    The form's AddFixedLightEmitter reducer hard-codes RatedLuminousFlux=0
    (the wizard doesn't expose a flux field); that makes the Bevy 3D
    viewer render the scene dark because spot-light intensity scales by
    flux. Patch each emitter post-hoc with SetFixedLightEmitterFlux."""
    print("\n[Act 6] Emitters — add 3, set flux")
    tid(page, "nav-emitters").click()
    wait_idle(page)
    import json as _json

    for v in VARIANTS:
        tid(page, "emitter-add").click()
        wait_idle(page, 400)
        safe_select(page, tid(page, "emitter-form-photometry"), v["photometry_id"])
        safe_select(page, tid(page, "emitter-form-lightsource"), v["lightsource_id"])
        wait_idle(page, 200)
        tid(page, "emitter-form-submit").click()
        wait_idle(page, 600)
        print(
            f"  · {v['emitter_id']} = {v['photometry_id']} ↔ {v['lightsource_id']}"
        )

    # Set RatedLuminousFlux per emitter via the JS bridge.
    for v in VARIANTS:
        flux_payload = {
            "action": "SetFixedLightEmitterFlux",
            "data": {
                "emitter_id": v["emitter_id"],
                "index": 0,
                "rated_luminous_flux": v["flux"],
            },
        }
        err = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)",
            _json.dumps(flux_payload),
        )
        if err:
            print(f"  · {v['emitter_id']} flux set ERR: {err}")
        else:
            print(f"  · {v['emitter_id']} flux → {v['flux']} lm")

    # Bind each emitter to controlGear_1 so the per-variant driver
    # wiring lands in product.xml. The Equipment row binds the LS
    # globally; per-emitter binding is what variant-aware viewers
    # actually read.
    for v in VARIANTS:
        cg_payload = {
            "action": "SetEmitterControlGearReference",
            "data": {
                "emitter_id": v["emitter_id"],
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
            print(f"  · {v['emitter_id']} ControlGear bind ERR: {err}")
        else:
            print(f"  · {v['emitter_id']} → controlGear_1")
    screenshot(page, "11-emitters")


def act_7_variants_duplicate(page: Page) -> None:
    """Variants page → duplicate variant 0 twice so 3 variants exist,
    then assign a GTIN-13 to each. GTINs are de-facto required by ERP/
    BIM consumers — without one most catalog systems silently drop
    the variant. Trahe-prefixed fake EAN values."""
    print("\n[Act 7] Variants — duplicate × 2 + GTINs")
    tid(page, "nav-variants").click()
    wait_idle(page)
    for n in (1, 2):
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
    variant_gtins = ["4260000000017", "4260000000024", "4260000000031"]
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
    screenshot(page, "12-variants-after-dup")


def act_8_rebind_emitters_per_variant(page: Page) -> None:
    """For each variant, open Edit → flip Model geometry mode → repoint
    binding 0 at variant's own emitter. Works because the geometry-mode
    button re-fires SetVariantGeometryModel which auto-adds binding 0
    pointing at the first emitter; we then change it to the right one."""
    print("\n[Act 8] Rebind each variant's geometry → variant's emitter")
    tid(page, "nav-variants").click()
    wait_idle(page)

    # Bypass the variant-edit modal entirely: dispatch
    # SetVariantGeometryModel via the JS bridge. The in-modal binding
    # picker has a Leptos signal race (#136) that closes the modal
    # before state.apply settles; running the action through
    # window.gldf_apply_action_json avoids that path. SLV TRIA 2's
    # per-variant CCT colours in the 3D viewer rely on each variant
    # pointing at its own emitter, so we MUST land this rebind.
    import json as _json
    for v in VARIANTS:
        idx = v["idx"]
        # GldfAction is `#[serde(tag = "action", content = "data")]` —
        # see crates/gldf-rs-editor/src/action.rs:60.
        payload = {
            "action": "SetVariantGeometryModel",
            "data": {
                "variant_idx": idx,
                "geometry_id": "geometry_1",
                "bindings": [{
                    "emitter_id": v["emitter_id"],
                    "external_name": "Primary",
                    "target_model_type": "l3d",
                }],
            },
        }
        result = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)",
            _json.dumps(payload),
        )
        if result:
            print(f"  · variant {idx} → {v['emitter_id']}: ERR {result}")
        else:
            print(f"  · variant {idx} → {v['emitter_id']}")
        wait_idle(page, 200)
    screenshot(page, "13-variants-after-rebind")


def act_8c_rename_ids(page: Page) -> None:
    """Rename auto-generated ids to semantic names. Catalog / BIM
    tools store these strings verbatim — `photometryFile_1` is a
    forensic smell, `ldt_narrow` is a feature.

    The rename actions cascade through every reference site (Emitter →
    LightSource, Variant → Emitter, Spectrum → File, etc.) so the
    rename is safe to dispatch in any order.
    """
    print("\n[Act 8c] Rename auto-ids → semantic names")
    import json as _json
    # File renames — semantic by content (narrow/middle/wide LDT + l3d + image + spd).
    file_renames = [
        ("photometryFile_1", "ldt_narrow"),
        ("photometryFile_2", "ldt_middle"),
        ("photometryFile_3", "ldt_wide"),
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
    # LightSource semantic ids — by CCT.
    ls_renames = [
        ("lightsource_1", "ls_led_4000k"),
        ("lightsource_2", "ls_led_3000k_warm"),
        ("lightsource_3", "ls_led_5000k_cool"),
    ]
    for old, new in ls_renames:
        payload = {
            "action": "RenameFixedLightSource",
            "data": {"old_id": old, "new_id": new},
        }
        err = page.evaluate(
            "(json) => window.gldf_apply_action_json(json)", _json.dumps(payload)
        )
        if err:
            print(f"  · LS {old} → {new} ERR: {err}")
        else:
            print(f"  · LS {old} → {new}")
    # Emitter semantic ids — by beam.
    em_renames = [
        ("emitter_1", "em_narrow_4000k"),
        ("emitter_2", "em_middle_3000k"),
        ("emitter_3", "em_wide_5000k"),
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
    # Variant ids — by RAL housing colour.
    variant_renames = [
        ("variant_1", "v_ral6018_yellowgreen"),
        ("variant_3", "v_ral1023_trafficyellow"),
        ("variant_2", "v_ral9003_signalwhite"),
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
    wait_idle(page, 500)
    screenshot(page, "13b-after-id-rename")


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
    """🎬 3D variants: click each chip; camera auto-frames once, persists across switches."""
    print("\n[Act 20] 3D variants")
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
        wait_idle(page, 1_500)
        screenshot(page, f"28-bevy-variant{v['idx']}-ral{v['ral']}")
        print(f"  · variant {v['idx']} ({v['ral_name']}) rendered")


def act_21_export(page: Page) -> Path | None:
    print("\n[Act 21] Export GLDF")
    with page.expect_download(timeout=15_000) as dl:
        tid(page, "toolbar-export-gldf").click()
    download = dl.value
    download.save_as(str(EXPORTED_GLDF))
    print(f"  · saved {EXPORTED_GLDF.relative_to(REPO_ROOT)} ({EXPORTED_GLDF.stat().st_size // 1024} KB)")
    screenshot(page, "29-after-export")
    return EXPORTED_GLDF


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
    for old in OUT_DIR.glob("*.png"):
        old.unlink()
    for old in list(OUT_DIR.glob("*.webm")) + list(OUT_DIR.glob("*.mp4")):
        old.unlink()

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=args.headless, slow_mo=args.slow_mo)
        ctx_args = {"viewport": {"width": 1440, "height": 900}}
        if not args.no_video:
            ctx_args["record_video_dir"] = str(OUT_DIR)
            ctx_args["record_video_size"] = {"width": 1440, "height": 900}
        context = browser.new_context(**ctx_args)
        page = context.new_page()
        page.on("console", lambda m: (
            print(f"  [browser:{m.type}] {m.text}")
            if m.type in ("error",) else None
        ))

        rc = 0
        try:
            # 1–8: build the GLDF (wizard + extras + variants wired)
            act_1_wizard(page)
            act_2_files(page)
            act_3_light_sources(page)
            act_4_control_gear(page)
            act_5_equipment(page)
            act_6_emitters(page)
            act_7_variants_duplicate(page)
            act_8_rebind_emitters_per_variant(page)
            act_8c_rename_ids(page)

            # 9–19: fill every ProductMetaData pane.
            # Marketing (18) runs AFTER variants exist so per-variant
            # scopes are populated.
            act_9_header_polish(page)
            act_10_photometry(page)
            act_11_electrical(page)
            act_12_mechanical(page)
            act_13_applications(page)
            act_14_labels(page)
            act_15_hyperlinks(page)
            act_16_pictures(page)
            act_17_emergency(page)
            act_18_marketing(page)
            act_19_custom_properties(page)

            # 20–22: 3D, export, re-import walk
            act_20_bevy(page)
            gldf = act_21_export(page)
            if gldf is not None:
                act_22_reimport_walkthrough(page, gldf)
                # 23–25: per-LS TM-33 export + reimport into Files +
                # TM-33-as-starter (clear editor, drop TM-33, verify
                # the editor bootstraps a fresh GLDF).
                tm33 = act_23_export_tm33(page)
                if tm33 is not None:
                    act_24_reimport_tm33_to_files(page, tm33)
                    act_25_tm33_as_starter(page, tm33)
            print(f"\n✅ done — screenshots in {OUT_DIR.relative_to(REPO_ROOT)}/")
        except Exception as e:
            print(f"\n❌ failed: {e}", file=sys.stderr)
            try:
                screenshot(page, "99-error")
            except Exception:
                pass
            rc = 1
        finally:
            video_obj = page.video if not args.no_video else None
            context.close()
            browser.close()

            if video_obj is not None:
                try:
                    raw = Path(video_obj.path())
                    final = OUT_DIR / "run.webm"
                    if raw.exists() and raw != final:
                        shutil.move(str(raw), str(final))
                    if final.exists():
                        print(f"  🎬 raw: {final.relative_to(REPO_ROOT)} "
                              f"({final.stat().st_size // 1024} KB)")
                        if args.slowdown > 1.0:
                            demo = OUT_DIR / "demo.mp4"
                            if slow_down_video(final, demo, args.slowdown):
                                print(f"  🎞  demo: {demo.relative_to(REPO_ROOT)} "
                                      f"({demo.stat().st_size // 1024} KB)")
                except Exception as e:
                    print(f"⚠ video post-process: {e}", file=sys.stderr)

        return rc


if __name__ == "__main__":
    raise SystemExit(main())
