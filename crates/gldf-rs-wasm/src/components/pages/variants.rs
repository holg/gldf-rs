//! Variants page (read view).
//!
//! Slice 1: variant cards with id, name, description, product number,
//! tender text, per-emitter references (flux + CCT + emergency badge),
//! per-variant photometry download buttons, AND a per-emitter LDT preview
//! rendered from the **variant-resolved** photometry (the raw .ies/.ldt
//! patched with this variant's RatedLuminousFlux / RatedInputPower via
//! `gldf_rs::patch_ldt_for_variant`). The patched LDT is what the variant
//! actually emits — showing the raw file would misrepresent variants that
//! override flux or power.
//!
//! The Bevy 3D viewer button lands in slice 3 alongside the BevySceneViewer
//! port; the mounting editor/render and per-variant DescriptiveAttributes
//! badges land in slice 2 with the editor pages.

use crate::components::pages::variant_editor::VariantModalHost;
use crate::components::LocaleFooView;
use crate::exports::export_patched_variant_photometry;
use crate::state::use_gldf;
use gldf_rs::{
    get_variant_emitter_data, patch_ldt_for_variant, resolve_variant_photometries,
    VariantPhotometryResolution,
};
use leptos::prelude::*;
use std::sync::Arc;

/// One resolved photometry slot, ready to render: the patched (or raw) LDT
/// bytes plus the resolution metadata (lumens/watts/emitter id). Built once
/// per variant card on render and handed to the photometry sub-grid.
#[derive(Clone)]
struct VariantPhotometrySlot {
    resolution: VariantPhotometryResolution,
    /// Bytes to feed to `<LdtPanel>` — patched when the resolution carried
    /// a flux or power override, raw otherwise. `None` means the embedded
    /// file was missing (no preview possible).
    ldt_bytes: Option<Vec<u8>>,
}

/// Why a variant card did or didn't render an LDT panel. Same idea as the
/// PhotometryProbe block on the Photometry page — surfaces the resolution
/// chain so missing-LDT cases are easy to diagnose. Removed once Variants
/// is reliably green.
#[derive(Clone, Default, Debug)]
struct VariantProbe {
    has_geometry: bool,
    emitter_ref_count: usize,
    resolution_count: usize,
    slots_with_bytes: usize,
    photometry_ids: Vec<String>,
    missing_bytes_for: Vec<String>,
}

#[component]
pub fn VariantsPage() -> impl IntoView {
    let state = use_gldf();
    // Shared signal for the per-card Edit affordance: `Some(idx)` while
    // the modal for that variant is open. The modal host below is the
    // same `<VariantModalHost>` the editor page uses — clicking Edit
    // on a read-mode card opens the *full* editor (Mountings, GTIN,
    // ProductSeries, Geometry, etc.) without forcing a global mode flip.
    let editing = RwSignal::new(None::<usize>);
    let open = Memo::new(move |_| editing.get().is_some());
    let on_close: Arc<dyn Fn() + Send + Sync> = Arc::new(move || editing.set(None));

    view! {
        {move || state.with(|s| {
            let variants = s
                .product
                .product_definitions
                .variants
                .as_ref()
                .map(|v| v.variant.clone())
                .unwrap_or_default();
            if variants.is_empty() {
                return view! {
                    <div class="empty-state">
                        <div class="icon">{ "📦" }</div>
                        <h3>{ "No Variants" }</h3>
                        <p>{ "This GLDF has no variant definitions" }</p>
                    </div>
                }.into_any();
            }
            let product = s.product.clone();
            view! {
                <div class="variants-container">
                    <div class="variant-cards">
                        {variants.into_iter().enumerate().map(|(idx, v)| {
                            let id = v.id.clone();
                            let emitter_data = get_variant_emitter_data(&product, &id);
                            // Probe: capture chain state for the diagnostic
                            // block on the card. has_geometry + emitter_ref
                            // count locate failures before resolution; the
                            // resolution_count vs slots_with_bytes split
                            // shows whether photometry IDs resolve but the
                            // bytes lookup misses (e.g. file id mismatch).
                            let mut probe = VariantProbe {
                                has_geometry: v.geometry.is_some(),
                                ..Default::default()
                            };
                            if let Some(ref geom) = v.geometry {
                                probe.emitter_ref_count += geom.emitter_reference.len();
                                if geom.simple_geometry_reference.is_some() {
                                    probe.emitter_ref_count += 1;
                                }
                                if let Some(ref mgr) = geom.model_geometry_reference {
                                    probe.emitter_ref_count += mgr.emitter_reference.len();
                                }
                            }
                            let resolutions = resolve_variant_photometries(
                                &product.product_definitions,
                                &v,
                                &product.general_definitions,
                            );
                            probe.resolution_count = resolutions.len();
                            // Build per-photometry slots: resolve raw bytes
                            // via the (photometry -> file -> embedded_files)
                            // chain, then ask the lib to patch the lamp_set
                            // header for this variant's flux/power. If both
                            // overrides are None patch_ldt_for_variant
                            // returns None — we fall back to the raw bytes
                            // unchanged.
                            let slots: Vec<VariantPhotometrySlot> = resolutions
                                .into_iter()
                                .map(|res| {
                                    probe.photometry_ids.push(res.photometry_id.clone());
                                    let raw = resolve_raw_photometry_bytes(
                                        &product,
                                        &s.embedded_files,
                                        &res.photometry_id,
                                    );
                                    if raw.is_some() {
                                        probe.slots_with_bytes += 1;
                                    } else {
                                        probe.missing_bytes_for.push(res.photometry_id.clone());
                                    }
                                    let ldt_bytes = raw.map(|bytes| {
                                        patch_ldt_for_variant(&bytes, res.lumens, res.watts)
                                            .unwrap_or(bytes)
                                    });
                                    VariantPhotometrySlot {
                                        resolution: res,
                                        ldt_bytes,
                                    }
                                })
                                .collect();
                            view! {
                                <VariantCard
                                    idx=idx
                                    editing=editing
                                    variant=v
                                    emitters=emitter_data.emitters
                                    photometry_slots=slots
                                    probe=probe
                                />
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            }.into_any()
        })}
        // Shared modal — opens whichever variant the user clicked Edit on.
        // The host is rendered unconditionally; `open` derives from
        // `editing.is_some()`, so it stays invisible until the user
        // clicks an Edit button.
        <VariantModalHost editing=editing open=open on_close=on_close />
    }
}

/// Walk Photometry → File → embedded_files to pull the raw .ies/.ldt bytes
/// for `photometry_id`. Returns `None` if the photometry has no file ref or
/// the referenced file isn't embedded.
pub(crate) fn resolve_raw_photometry_bytes(
    product: &gldf_rs::GldfProduct,
    embedded_files: &std::collections::HashMap<String, Vec<u8>>,
    photometry_id: &str,
) -> Option<Vec<u8>> {
    let photometries = product.general_definitions.photometries.as_ref()?;
    let photometry = photometries
        .photometry
        .iter()
        .find(|p| p.id == photometry_id)?;
    let file_id = &photometry.photometry_file_reference.as_ref()?.file_id;
    embedded_files.get(file_id).cloned()
}

#[component]
fn VariantCard(
    /// Position of this variant in `product_definitions.variants.variant`.
    /// The Edit button writes this index into `editing` to open the
    /// shared modal at the right slot.
    idx: usize,
    /// Shared signal driven by the page's `VariantModalHost`. Setting
    /// it to `Some(idx)` opens the per-variant editor.
    editing: RwSignal<Option<usize>>,
    variant: gldf_rs::gldf::product_definitions::Variant,
    emitters: Vec<gldf_rs::EmitterRenderData>,
    photometry_slots: Vec<VariantPhotometrySlot>,
    probe: VariantProbe,
) -> impl IntoView {
    let state = use_gldf();
    let id = variant.id.clone();
    let gtin = variant.gtin.clone();
    let sort_order = variant.sort_order.filter(|&o| o > 0);
    let name = variant.name.clone();
    let description = variant.description.clone();
    let product_number = variant.product_number.clone();
    let tender_text = variant.tender_text.clone();
    let id_for_dl = variant.id.clone();
    // Human-readable summary of `<Mountings>` for the read view. The
    // XSD allows any combination of the four top-level branches
    // (Ceiling / Wall / Ground / WorkingPlane); we join whichever
    // branches are set with " · " separators. Returns `None` when the
    // variant has no Mountings at all so the read view stays quiet.
    let mountings_summary: Option<String> = variant.mountings.as_ref().and_then(|m| {
        let mut parts: Vec<String> = Vec::new();
        if let Some(c) = m.ceiling.as_ref() {
            if let Some(r) = c.recessed.as_ref() {
                let mut s = format!("Ceiling · Recessed {} mm", r.recessed_depth);
                if let Some(rc) = r.rectangular_cutout.as_ref() {
                    s.push_str(&format!(" · cutout {}×{} mm", rc.width, rc.length));
                } else if let Some(cc) = r.circular_cutout.as_ref() {
                    s.push_str(&format!(" · ⌀ {} mm cutout", cc.diameter));
                }
                parts.push(s);
            } else if c.surface_mounted.is_some() {
                parts.push("Ceiling · Surface mounted".to_string());
            } else if let Some(p) = c.pendant.as_ref() {
                parts.push(format!("Ceiling · Pendant {:.0} mm", p.pendant_length));
            } else {
                parts.push("Ceiling".to_string());
            }
        }
        if let Some(w) = m.wall.as_ref() {
            let mut s = format!("Wall · h {} mm", w.mounting_height);
            if let Some(r) = w.recessed.as_ref() {
                s.push_str(&format!(" · Recessed {} mm", r.recessed_depth));
            } else if w.surface_mounted.is_some() {
                s.push_str(" · Surface mounted");
            }
            parts.push(s);
        }
        if let Some(g) = m.ground.as_ref() {
            if let Some(pt) = g.pole_top.as_ref() {
                let h = pt.get_pole_height().unwrap_or(0);
                parts.push(format!("Ground · Pole-top {} mm", h));
            } else if let Some(pi) = g.pole_integrated.as_ref() {
                let h = pi.get_pole_height().unwrap_or(0);
                parts.push(format!("Ground · Pole-integrated {} mm", h));
            } else if g.free_standing.is_some() {
                parts.push("Ground · Free-standing".to_string());
            } else if g.surface_mounted.is_some() {
                parts.push("Ground · Surface mounted".to_string());
            } else {
                parts.push("Ground".to_string());
            }
        }
        if m.working_plane.is_some() {
            parts.push("Working plane".to_string());
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" · "))
        }
    });

    let id_for_anchor = format!("variant-{}", id);
    // `data-testid="variant-edit-<id>"` so the e2e demo can target the
    // button per variant. The `move ||` closure captures `editing` and
    // `idx` — clicking flips the same signal `VariantModalHost` is
    // listening on, so the modal opens for THIS variant.
    let id_for_testid = id.clone();
    let on_edit = move |_| editing.set(Some(idx));
    view! {
        <div id=id_for_anchor class="variant-card variant-card-expanded">
            <div class="card-header-row">
                <span class="card-id">{ id }</span>
                {sort_order.map(|o| view! {
                    <span style="font-size: 11px; color: var(--text-tertiary);">{ format!("#{}", o) }</span>
                })}
                {gtin.map(|g| view! {
                    <span class="card-gtin" style="font-family: var(--font-mono); font-size: 11px; color: var(--text-secondary); margin-left: auto;">
                        { format!("GTIN {}", g) }
                    </span>
                })}
                <button
                    class="btn btn-secondary"
                    data-testid=format!("variant-edit-{}", id_for_testid)
                    on:click=on_edit
                    style="padding: 4px 12px; font-size: 12px; margin-left: 8px;"
                    title="Edit this variant — Mountings, ProductNumber, Name, GTIN, ProductSeries, Geometry bindings"
                >{ "✎ Edit" }</button>
            </div>
            <div class="card-content">
                {name.map(|n| view! {
                    <div style="margin-bottom: 4px;">
                        <span style="font-size: 11px; color: var(--text-secondary);">{ "Name: " }</span>
                        <LocaleFooView value=n compact=true />
                    </div>
                })}
                {description.map(|d| view! {
                    <div style="margin-bottom: 4px;">
                        <span style="font-size: 11px; color: var(--text-secondary);">{ "Description: " }</span>
                        <LocaleFooView value=d compact=true />
                    </div>
                })}
                {product_number.map(|pn| view! {
                    <div style="margin-bottom: 4px;">
                        <span style="font-size: 11px; color: var(--text-secondary);">{ "Product Number: " }</span>
                        <LocaleFooView value=pn compact=true />
                    </div>
                })}
                {tender_text.map(|tt| view! {
                    <div style="margin-bottom: 4px;">
                        <span style="font-size: 11px; color: var(--text-secondary);">{ "Tender: " }</span>
                        <LocaleFooView value=tt compact=true />
                    </div>
                })}
                // Mountings — read-only summary. Same data the Edit modal
                // shows; rendered here so users can confirm depth +
                // cutout without opening the editor. Skipped when the
                // variant has no `<Mountings>` (XSD minOccurs=0).
                {mountings_summary.map(|s| view! {
                    <div
                        data-testid=format!("variant-mountings-{}", id_for_testid.clone())
                        style="margin-bottom: 4px;"
                    >
                        <span style="font-size: 11px; color: var(--text-secondary);">{ "Mounting: " }</span>
                        <span style="font-size: 12px;">{ s }</span>
                    </div>
                })}

                {if !emitters.is_empty() {
                    let total_emitters = emitters.len();
                    view! {
                        <div class="emitter-references" style="margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--border-color);">
                            <div style="font-size: 12px; font-weight: 500; color: var(--text-secondary); margin-bottom: 8px;">
                                { format!("Light Emitters ({})", total_emitters) }
                            </div>
                            <div style="display: flex; flex-direction: column; gap: 8px;">
                                {emitters.into_iter().map(|em| {
                                    let leo_name = em.leo_name.clone();
                                    let emitter_id = em.emitter_id.clone();
                                    let flux = em.luminous_flux.map(|f| format!("{} lm", f)).unwrap_or_else(|| "—".to_string());
                                    let cct = em.color_temperature.map(|t| format!("{} K", t)).unwrap_or_else(|| "—".to_string());
                                    let emergency = em.emergency_behavior.clone();
                                    view! {
                                        <div style="background: var(--bg-secondary); padding: 8px 10px; border-radius: 6px;">
                                            <div style="display: flex; align-items: center; gap: 8px; font-size: 11px; margin-bottom: 4px;">
                                                <span style="font-family: var(--font-mono); font-weight: 500;">{ leo_name }</span>
                                                <span style="color: var(--text-tertiary);">{ "→" }</span>
                                                <span style="color: var(--accent-blue);">{ emitter_id }</span>
                                                {emergency.map(|eb| {
                                                    let color = if eb == "EmergencyOnly" { "var(--accent-orange)" } else { "var(--accent-green)" };
                                                    view! {
                                                        <span style=format!("background: {}; color: white; padding: 1px 4px; border-radius: 3px; font-size: 9px; margin-left: 4px;", color)>{ eb }</span>
                                                    }
                                                })}
                                            </div>
                                            <div style="display: flex; gap: 16px; font-size: 10px; color: var(--text-secondary);">
                                                <span style="display: flex; align-items: center; gap: 4px;">
                                                    <span style="color: var(--accent-yellow);">{ "☀" }</span>{ flux }
                                                </span>
                                                <span style="display: flex; align-items: center; gap: 4px;">
                                                    <span style="color: var(--accent-orange);">{ "🌡" }</span>{ cct }
                                                </span>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }}

                <ProbeBlock probe=probe.clone() />

                {if !photometry_slots.is_empty() {
                    view! {
                        <div style="margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--border-color);">
                            <div style="font-size: 12px; font-weight: 500; color: var(--text-secondary); margin-bottom: 6px;">
                                { "Per-variant photometry" }
                            </div>
                            {photometry_slots.into_iter().map(|slot| {
                                let pid = slot.resolution.photometry_id.clone();
                                let pid_for_dl = pid.clone();
                                let id_dl = id_for_dl.clone();
                                let lumens = slot.resolution.lumens;
                                let watts = slot.resolution.watts;
                                let emitter_label = if slot.resolution.emitter_external_name.is_empty() {
                                    String::new()
                                } else {
                                    format!(" · {}", slot.resolution.emitter_external_name)
                                };
                                let override_summary = match (lumens, watts) {
                                    (Some(lm), Some(w)) => format!("override: {lm} lm · {w:.1} W"),
                                    (Some(lm), None) => format!("override: {lm} lm"),
                                    (None, Some(w)) => format!("override: {w:.1} W"),
                                    (None, None) => "raw photometry (no variant override)".to_string(),
                                };
                                // Per-row format chooser. Default = LDT,
                                // the historical behaviour. The select
                                // sits inline next to the Download button
                                // so the user picks the wire format AT
                                // download time, without a modal. Lib does
                                // the conversion (LDT / IES / TM-33 XML);
                                // the WASM stays a UI shell.
                                let format_sig = RwSignal::new(
                                    gldf_rs::PhotometryExportFormat::Ldt,
                                );
                                let pid_for_change = pid_for_dl.clone();
                                let on_format = move |e: crate::components::edit_util::EditEvent| {
                                    let _ = &pid_for_change;
                                    if let Some(v) = crate::components::edit_util::event_value(&e) {
                                        if let Some(fmt) =
                                            gldf_rs::PhotometryExportFormat::from_str(&v)
                                        {
                                            format_sig.set(fmt);
                                        }
                                    }
                                };
                                let on_click = move |_| {
                                    if let Err(err) = export_patched_variant_photometry(
                                        state,
                                        &pid_for_dl,
                                        lumens,
                                        watts,
                                        format_sig.get(),
                                    ) {
                                        log::error!("variant {} photometry {}: {}", id_dl, pid_for_dl, err);
                                    }
                                };
                                // Inline LdtPanel render lived here, but
                                // moved to the LightSource card: that's
                                // the only place that uniquely owns the
                                // chain (one Emitter can carry many
                                // FixedLightEmitter children with their
                                // own rotations, so Emitter rendering is
                                // ambiguous; a Variant just composes
                                // existing chains). The patched-download
                                // button stays here because variant flux
                                // overrides are what produce the patched
                                // bytes.
                                let _ = slot.ldt_bytes;
                                view! {
                                    <div style="margin-bottom: 12px; padding: 8px; background: var(--bg-secondary); border-radius: 6px;">
                                        <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap;">
                                            <span style="font-family: var(--font-mono); font-size: 11px;">{ pid.clone() }</span>
                                            <span style="color: var(--text-tertiary); font-size: 10px;">{ emitter_label }</span>
                                            <span style="color: var(--text-secondary); font-size: 10px;">{ override_summary }</span>
                                            <select
                                                on:change=on_format
                                                style="margin-left: auto; background: var(--bg-input); color: var(--text-primary); border: 1px solid var(--border-color); border-radius: 4px; padding: 4px 6px; font-size: 11px;"
                                                title="Output format"
                                            >
                                                <option value="ldt" selected>{ "LDT (EULUMDAT)" }</option>
                                                <option value="ies">{ "IES LM-63" }</option>
                                                <option value="atla_xml">{ "TM-33 XML" }</option>
                                                <option value="atla_json">{ "ATLA JSON" }</option>
                                            </select>
                                            <button
                                                class="btn-download-file"
                                                on:click=on_click
                                                style="background: none; border: 1px solid var(--border-color); border-radius: 4px; padding: 4px 8px; cursor: pointer; color: var(--accent-blue); font-size: 11px;"
                                            >
                                                { "⬇ Download (patched)" }
                                            </button>
                                        </div>
                                        <div style="margin-top: 4px; font-size: 10px; color: var(--text-tertiary); font-style: italic;">
                                            { "Diagram on the Light Sources page." }
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }}
            </div>
        </div>
    }
}

/// Variant chain diagnostic — shown above the per-variant photometry section
/// so missing-LDT cases are easy to triage. Mirrors the Photometry page's
/// `ProbeBlock` pattern.
#[component]
fn ProbeBlock(probe: VariantProbe) -> impl IntoView {
    let summary = match (probe.has_geometry, probe.emitter_ref_count, probe.resolution_count, probe.slots_with_bytes) {
        (false, _, _, _) => "no <Geometry> on variant".to_string(),
        (true, 0, _, _) => "<Geometry> present but zero emitter references".to_string(),
        (true, n, 0, _) => format!("{} emitter ref(s) but resolved to 0 photometries — emitter id may not match <Emitters>", n),
        (true, _, r, 0) if r > 0 => "resolved photometries but no bytes in embedded_files".to_string(),
        (true, _, r, b) if b == r => format!("ok — {} slot(s) with bytes", b),
        (true, _, r, b) => format!("partial — {}/{} slots with bytes", b, r),
    };
    view! {
        <details style="margin-top: 8px; font-size: 11px; color: var(--text-tertiary);">
            <summary style="cursor: pointer;">{ format!("debug: {}", summary) }</summary>
            <pre style="background: var(--bg-secondary); padding: 6px; border-radius: 4px; overflow: auto; white-space: pre-wrap; font-size: 11px;">
                { format!(
                    "has_geometry      : {}\nemitter_ref_count : {}\nresolution_count  : {}\nslots_with_bytes  : {}\nphotometry_ids    : {:?}\nmissing_bytes_for : {:?}",
                    probe.has_geometry,
                    probe.emitter_ref_count,
                    probe.resolution_count,
                    probe.slots_with_bytes,
                    probe.photometry_ids,
                    probe.missing_bytes_for,
                ) }
            </pre>
        </details>
    }
}
