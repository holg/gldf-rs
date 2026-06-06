//! Bevy 3D viewer page.
//!
//! Mounts the lazy-loaded `gldf-bevy` WASM bundle into a `<canvas>`
//! and feeds it the current document's L3D bytes + LDT bytes +
//! mounting config + **per-variant housing-color override**. The
//! Bevy crate reads everything from `localStorage` (the existing
//! contract) — this page is the source-of-truth Rust pusher.
//!
//! # Per-variant flow
//!
//! Each [`Variant`] can carry its own [`HousingColor`] under
//! `DescriptiveAttributes.Marketing.HousingColors`. A dropdown at
//! the top of the page lists every variant in the document; picking
//! one (a) sets the active variant signal, (b) pushes that variant's
//! HousingColor sRGB into `gldf_housing_color_rgb` in localStorage,
//! and (c) bumps `gldf_timestamp` so the running Bevy app's
//! `poll_gldf_changes` system picks up the change on its next frame
//! and respawns the luminaire with the new material.
//!
//! Variants with no HousingColor or an unknown RAL number push an
//! empty string (cleared key) — Bevy falls back to its default
//! light-grey luminaire material.
//!
//! # Why localStorage and not Rust-side state
//!
//! The Bevy bundle is a **separate WASM module** loaded on demand
//! via `bevy-loader.js`. It runs in the same browser context but
//! has its own memory space. localStorage is the only inter-module
//! bus we have. Same pattern the eulumdat viewer + IFC viewer use.

use crate::state::{use_active_scope, use_gldf, Scope};
use gldf_rs::gldf::product_definitions::Variant;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// Lazy-loads the Bevy WASM bundle and attaches it to
    /// `#bevy-canvas`. Idempotent — subsequent calls are no-ops.
    #[wasm_bindgen(js_name = loadBevyViewer, catch)]
    async fn js_load_bevy_viewer() -> Result<JsValue, JsValue>;

    /// Pushes data to localStorage and bumps the timestamp so the
    /// running Bevy app picks it up on the next poll. All
    /// non-required params accept `null`.
    #[wasm_bindgen(js_name = saveL3dForBevy)]
    fn js_save_l3d_for_bevy(
        l3d_data: &[u8],
        ldt_data: Option<String>,
        emitter_config: Option<String>,
        mounting_config: Option<String>,
        housing_color_rgb: Option<String>,
    );

    /// Clears every key in the Bevy localStorage namespace. Called
    /// when the editor loads a different GLDF so stale variant data
    /// from the previous document doesn't bleed through.
    #[wasm_bindgen(js_name = clearL3dData)]
    fn js_clear_l3d_data();
}

#[component]
pub fn BevyViewerPage() -> impl IntoView {
    let state = use_gldf();
    // The shared, shell-level scope signal. When the user picked
    // Variant 1 on Marketing and lands here, we mirror that selection
    // — and vice versa, picking a chip here writes back to the same
    // signal so when they go back to Marketing they're still on
    // Variant 1.
    let scope = use_active_scope();

    // On first entry to this page: if the user hasn't picked a
    // variant yet (scope is Product) AND the document has variants,
    // default to Variant(0) so something useful renders. We only do
    // this when there's actually >=1 variant — otherwise we'd snap
    // a variant-less document into a non-existent slot.
    Effect::new(move |_| {
        if matches!(scope.get(), Scope::Product) {
            let has_variants = state.with(|s| {
                s.product
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|vs| !vs.variant.is_empty())
                    .unwrap_or(false)
            });
            if has_variants {
                scope.set(Scope::Variant(0));
            }
        }
    });

    // Derived: the variant index. `Product` becomes index 0 (the
    // viewer always renders *some* variant; the only meaningful
    // distinction here is which one).
    let active_idx = Signal::derive(move || match scope.get() {
        Scope::Variant(i) => i,
        Scope::Product => 0,
    });

    // Push the *current* variant's data into localStorage and trigger
    // the loader. Re-runs whenever `active_idx` or the underlying
    // document changes — covers both "user picked another variant"
    // and "edit a HousingColor and the live update should flow into
    // the 3D viewer".
    Effect::new(move |_prev: Option<()>| {
        let idx = active_idx.get();
        let payload = state.with(|s| build_payload(s, idx));
        if let Some(p) = payload {
            js_save_l3d_for_bevy(
                &p.l3d_bytes,
                p.ldt_text,
                p.emitter_config_json,
                p.mounting_config_json,
                p.housing_color_json,
            );
            // First call boots the bundle; subsequent calls are
            // idempotent and the bundle picks up the new data via its
            // own poll loop.
            spawn_local(async move {
                if let Err(e) = js_load_bevy_viewer().await {
                    log::error!("loadBevyViewer failed: {:?}", e);
                }
            });
        } else {
            // No L3D → no viewer. Don't leave stale data in the cache.
            js_clear_l3d_data();
        }
    });

    // The `<canvas id="bevy-canvas">` itself lives at the app shell
    // (see `App` in `main.rs`). It's always in the DOM so the Bevy
    // WebGL context survives navigation away from this page; we just
    // hide it via CSS when the user is on a different page. This
    // page only renders the variant picker + chrome on top.
    //
    // The shell wraps the canvas in a fixed positioned container
    // sized to the main content area, and toggles `display: block`
    // ↔ `display: none` based on `nav == BevyViewer`. From the
    // page's perspective the canvas is "just there"; we leave a
    // placeholder div with the same height so the layout doesn't
    // collapse.
    view! {
        <div class="editor-section bevy-viewer-page" style="display: flex; flex-direction: column; gap: 12px; height: calc(100vh - 120px);">
            <div>
                <h2>{ "🎬 3D variant viewer" }</h2>
                <p class="section-description">
                    { "Pick a variant to preview its housing colour live in the Bevy 3D viewer. The colour comes from each variant's Marketing → HousingColor RAL value; variants without one render in the default light grey." }
                </p>
            </div>
            <VariantPicker scope=scope />
            // Placeholder that reserves the layout slot. The actual
            // canvas is rendered by the app shell and overlaid on
            // this region via fixed positioning (see `BevyCanvasShell`).
            <div style="flex: 1; min-height: 400px;"></div>
        </div>
    }
}

#[component]
fn VariantPicker(scope: RwSignal<Scope>) -> impl IntoView {
    let state = use_gldf();
    let ui_language = state.with(|s| s.ui_language.clone());

    view! {
        {move || state.with(|s| {
            let variants = s
                .product
                .product_definitions
                .variants
                .as_ref()
                .map(|vs| vs.variant.clone())
                .unwrap_or_default();
            if variants.is_empty() {
                return view! {
                    <p class="empty-message">
                        { "This document has no variants. Add one on the Variants page to see it here." }
                    </p>
                }.into_any();
            }
            let lang = ui_language.clone();
            view! {
                <div style="display: flex; flex-wrap: wrap; gap: 8px; align-items: center;">
                    <strong style="font-size: 12px; color: var(--text-secondary);">{ "Variant:" }</strong>
                    {variants.iter().enumerate().map(|(idx, v)| {
                        let active_idx = match scope.get() {
                            Scope::Variant(i) => Some(i),
                            Scope::Product => None,
                        };
                        let is_active = active_idx == Some(idx);
                        let label = variant_label(v);
                        let chip_style = housing_swatch_style(v);
                        let bg = if is_active { "var(--accent-blue)" } else { "var(--bg-secondary)" };
                        let fg = if is_active { "white" } else { "var(--text-primary)" };
                        view! {
                            <button
                                type="button"
                                on:click=move |_| scope.set(Scope::Variant(idx))
                                data-testid=format!("bevy-variant-chip-{}", idx)
                                style=format!(
                                    "display: flex; align-items: center; gap: 6px; padding: 6px 10px; border: 1px solid var(--border-color); border-radius: 16px; background: {}; color: {}; cursor: pointer; font-size: 12px;",
                                    bg, fg
                                )
                            >
                                <span style=chip_style></span>
                                <span>{ label }</span>
                                {housing_label(v, &lang).map(|name| view! {
                                    <span style="font-size: 11px; opacity: 0.85;">{ format!("· {}", name) }</span>
                                })}
                            </button>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            }.into_any()
        })}
    }
}

// ---- helpers ----

/// Bundle of everything we need to push to localStorage for a given
/// variant. All fields are precomputed in Rust so the localStorage
/// write is a single batched call.
struct BevyPayload {
    l3d_bytes: Vec<u8>,
    ldt_text: Option<String>,
    emitter_config_json: Option<String>,
    mounting_config_json: Option<String>,
    housing_color_json: Option<String>,
}

/// Walk the document for variant `variant_idx` and assemble the
/// payload. Returns `None` when there's no L3D to show — the page
/// then clears localStorage so the viewer doesn't render stale data.
fn build_payload(state: &crate::state::GldfState, variant_idx: usize) -> Option<BevyPayload> {
    let variants = state
        .product
        .product_definitions
        .variants
        .as_ref()?;
    let variant = variants.variant.get(variant_idx)?;

    // 1. L3D bytes via the variant's ModelGeometryReference chain.
    let l3d_bytes = resolve_variant_l3d_bytes(&state.product, &state.embedded_files, variant)?;

    // 2. Housing-color override (sRGB triple JSON-encoded) from the
    //    variant's first HousingColor. Unknown RAL → None.
    let housing_color_json = variant
        .descriptive_attributes
        .as_ref()
        .and_then(|a| a.marketing.as_ref())
        .and_then(|m| m.housing_colors.as_ref())
        .and_then(|hc| hc.housing_color.first())
        .and_then(|c| c.ral)
        .and_then(gldf_rs::ral::by_number)
        .map(|e| {
            let rgb = e.rgb_f32();
            format!("[{:.6},{:.6},{:.6}]", rgb[0], rgb[1], rgb[2])
        });

    // 3. LDT bytes for the variant's first emitter binding. The
    //    Bevy app feeds this to eulumdat-bevy's photometric system
    //    so the rendered photometric solid + light colour reflect
    //    the variant's actual photometry. Without it, the viewer
    //    falls back to eulumdat-bevy's default ~4000K white light
    //    and an empty photometric solid.
    let ldt_text = resolve_variant_ldt_text(&state.product, &state.embedded_files, variant);

    // 4. Emitter config — per-LEO CCT/flux/emergency. The Bevy app
    //    keys on `leo_name` (an OBJ object name inside the L3D's
    //    structure.xml) and uses `color_temperature` to tint the
    //    spawned light + LEO emissive surface. Without this every
    //    variant renders at eulumdat-bevy's default 4000K.
    //
    //    `get_variant_emitter_data` walks Variant → Geometry →
    //    Emitter → LightSource and collects exactly the fields we
    //    need. Variants with bare-EmitterRef geometry (no LEO name
    //    mapping) come back with empty leo_names — Bevy then falls
    //    through to its default CCT for those slots, which is fine.
    let variant_id = variant.id.clone();
    let emitter_data = gldf_rs::mapping::get_variant_emitter_data(&state.product, &variant_id);
    let configs: Vec<serde_json::Value> = emitter_data
        .emitters
        .iter()
        .filter(|e| !e.leo_name.is_empty())
        .map(|e| {
            // `leo_name` comes from the binding's `emitter_object_external_name`,
            // which can now point at a SensorObject partName (e.g. "Sensor")
            // as well as an actual LEO partName. We push it under BOTH keys
            // so Bevy can match either way:
            //   - `leo_name`: backwards-compat with older Bevy bundles
            //   - `external_name`: explicit signal that this targets ANY
            //     partName (LEO or SensorObject); preferred going forward
            // Same string under both keys = no contradiction; cheap; safe.
            serde_json::json!({
                "leo_name": e.leo_name,
                "external_name": e.leo_name,
                "luminous_flux": e.luminous_flux,
                "color_temperature": e.color_temperature,
                "emergency_behavior": e.emergency_behavior,
            })
        })
        .collect();
    let emitter_config_json = if configs.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&configs).unwrap_or_default())
    };

    // 5. Mounting — defer.
    let mounting_config_json = None;

    Some(BevyPayload {
        l3d_bytes,
        ldt_text,
        emitter_config_json,
        mounting_config_json,
        housing_color_json,
    })
}

/// Variant → ModelGeometryReference → ModelGeometry → File → embedded
/// bytes. Mirrors `gldf_rs::l3d_mtl_sync::sync_l3d_mtl_for_housing_color`'s
/// resolver; lighter because we only need the bytes, not the parsed
/// product structures.
/// Variant → emitter → first FixedLightEmitter/ChangeableLightEmitter
/// → PhotometryReference → Photometry → PhotometryFileReference →
/// File → embedded bytes → UTF-8 string. The Bevy bundle expects an
/// LDT string (not bytes) because eulumdat parses from `&str`.
///
/// Returns `None` when:
/// - the variant has no emitter binding, or
/// - the bound emitter has no PhotometryReference, or
/// - the file id doesn't resolve, or
/// - the bytes aren't valid UTF-8 (rare for LDT/IES files but possible
///   on Latin-1 IES — those won't drive Bevy's CCT for now)
fn resolve_variant_ldt_text(
    product: &gldf_rs::GldfProduct,
    embedded: &std::collections::HashMap<String, Vec<u8>>,
    variant: &Variant,
) -> Option<String> {
    use gldf_rs::mapping;
    // The Yew flow used `get_l3d_ldt_mappings` for this; same helper
    // resolves variant_id → ldt_file_name. We then look up bytes by
    // either file_id or file_name (the embedded_files map keys vary
    // depending on whether the GLDF was freshly loaded or had imports).
    let mappings = mapping::get_l3d_ldt_mappings(product);
    let mapping = mappings.iter().find(|m| m.variant_id == variant.id)?;
    let ldt_file_id = mapping.ldt_file_id.as_ref()?;
    let bytes = embedded.get(ldt_file_id).or_else(|| {
        // Fallback: some load paths key on file_name instead of id.
        mapping.ldt_file_name.as_ref().and_then(|n| embedded.get(n))
    })?;
    // Latin-1 fallback handled inside the LDT parser; for the Bevy
    // hand-off we send the raw UTF-8 (or lossy) string and let
    // eulumdat::Eulumdat::parse do its own decoding.
    Some(String::from_utf8_lossy(bytes).into_owned())
}

fn resolve_variant_l3d_bytes(
    product: &gldf_rs::GldfProduct,
    embedded: &std::collections::HashMap<String, Vec<u8>>,
    variant: &Variant,
) -> Option<Vec<u8>> {
    let geometry_id = variant
        .geometry
        .as_ref()?
        .model_geometry_reference
        .as_ref()?
        .geometry_id
        .clone();
    let model_geom = product
        .general_definitions
        .geometries
        .as_ref()?
        .model_geometry
        .iter()
        .find(|g| g.id == geometry_id)?;
    let file_id = &model_geom.geometry_file_reference.first()?.file_id;
    embedded.get(file_id).cloned()
}

fn variant_label(v: &Variant) -> String {
    v.name
        .as_ref()
        .and_then(|n| n.locale.first())
        .map(|l| l.value.clone())
        .unwrap_or_else(|| v.id.clone())
}

fn housing_label(v: &Variant, ui_language: &str) -> Option<String> {
    let c = v
        .descriptive_attributes
        .as_ref()
        .and_then(|a| a.marketing.as_ref())
        .and_then(|m| m.housing_colors.as_ref())
        .and_then(|hc| hc.housing_color.first())?;
    if let Some(ral) = c.ral.and_then(gldf_rs::ral::by_number) {
        Some(ral.fallback_name(ui_language).to_string())
    } else if !c.locale.value.is_empty() {
        Some(c.locale.value.clone())
    } else {
        None
    }
}

fn housing_swatch_style(v: &Variant) -> String {
    let entry = v
        .descriptive_attributes
        .as_ref()
        .and_then(|a| a.marketing.as_ref())
        .and_then(|m| m.housing_colors.as_ref())
        .and_then(|hc| hc.housing_color.first())
        .and_then(|c| c.ral)
        .and_then(gldf_rs::ral::by_number);
    let common = "width: 14px; height: 14px; border-radius: 50%; border: 1px solid rgba(0,0,0,0.15); flex-shrink: 0;";
    match entry {
        Some(e) => format!("{} background: {};", common, e.hex()),
        None => format!("{} background: transparent; border-style: dashed;", common),
    }
}
