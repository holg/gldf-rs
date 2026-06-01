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

use crate::components::{LdtPanel, LocaleFooView};
use crate::exports::export_patched_variant_photometry;
use crate::state::use_gldf;
use gldf_rs::{
    get_variant_emitter_data, patch_ldt_for_variant, resolve_variant_photometries,
    VariantPhotometryResolution,
};
use leptos::prelude::*;

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
                        {variants.into_iter().map(|v| {
                            let id = v.id.clone();
                            let emitter_data = get_variant_emitter_data(&product, &id);
                            // Probe: capture chain state for the diagnostic
                            // block on the card. has_geometry + emitter_ref
                            // count locate failures before resolution; the
                            // resolution_count vs slots_with_bytes split
                            // shows whether photometry IDs resolve but the
                            // bytes lookup misses (e.g. file id mismatch).
                            let mut probe = VariantProbe::default();
                            probe.has_geometry = v.geometry.is_some();
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
    }
}

/// Walk Photometry → File → embedded_files to pull the raw .ies/.ldt bytes
/// for `photometry_id`. Returns `None` if the photometry has no file ref or
/// the referenced file isn't embedded.
fn resolve_raw_photometry_bytes(
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

    let id_for_anchor = format!("variant-{}", id);
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
                                let on_click = move |_| {
                                    if let Err(err) = export_patched_variant_photometry(
                                        state,
                                        &pid_for_dl,
                                        lumens,
                                        watts,
                                    ) {
                                        log::error!("variant {} photometry {}: {}", id_dl, pid_for_dl, err);
                                    }
                                };
                                let ldt_panel = slot.ldt_bytes.map(|bytes| view! {
                                    <div style="margin: 6px 0;">
                                        <LdtPanel bytes=bytes width=420.0 height=320.0 />
                                    </div>
                                });
                                view! {
                                    <div style="margin-bottom: 12px; padding: 8px; background: var(--bg-secondary); border-radius: 6px;">
                                        <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-bottom: 6px;">
                                            <span style="font-family: var(--font-mono); font-size: 11px;">{ pid.clone() }</span>
                                            <span style="color: var(--text-tertiary); font-size: 10px;">{ emitter_label }</span>
                                            <span style="color: var(--text-secondary); font-size: 10px;">{ override_summary }</span>
                                            <button
                                                class="btn-download-file"
                                                on:click=on_click
                                                style="margin-left: auto; background: none; border: 1px solid var(--border-color); border-radius: 4px; padding: 4px 8px; cursor: pointer; color: var(--accent-blue); font-size: 11px;"
                                            >
                                                { "⬇ Download (patched)" }
                                            </button>
                                        </div>
                                        { ldt_panel }
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
