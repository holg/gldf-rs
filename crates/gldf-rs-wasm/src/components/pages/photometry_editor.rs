//! Photometry editor — DescriptivePhotometry per-photometry fields with
//! a GLDF-declared vs LDT/IES-calculated side-by-side comparison.
//!
//! GLDF model: DescriptivePhotometry lives on `<Photometry>` under
//! GeneralDefinitions/Photometries — there's no per-variant override. The
//! values in the block are what the **file vendor declared** in the GLDF.
//! That declaration is what the file says, period — we display it as
//! authored.
//!
//! For comparison we also compute the same fields ourselves from the
//! embedded LDT/IES via `eulumdat::GldfPhotometricData`. Differences
//! between the declared and the calculated values are useful for catching
//! vendor mistakes (or for understanding rounding choices), but the
//! calculator never overrides what the file says — the GLDF column stays
//! authoritative.
//!
//! In edit mode the GLDF column is editable. The "Use calculated values"
//! button fills **empty** GLDF fields from the calculator; it never
//! overwrites a vendor-declared value.
//!
//! Unit conversion: `GldfPhotometricData` returns percentages (LOR, DLOR,
//! ULOR, DFF) but GLDF stores those as fractions in [0, 1]. We divide by
//! 100 when copying calc → GLDF (matching the Yew side's behaviour).

use crate::components::edit_util::{event_f64, event_i32, event_value, optional_string};
use crate::components::pages::link_hint::LinkHint;
use crate::components::{IdRenameChip, LdtPanel};
use crate::i18n::ui::t as ui_t;
use crate::state::{use_gldf, GldfAction};
use eulumdat::{Eulumdat, GldfPhotometricData};
use gldf_rs::gldf::general_definitions::photometries::Photometry;
use leptos::prelude::*;
use std::sync::Arc;

/// Read view — declared GLDF values + calculated comparison, both display-only.
#[component]
pub fn PhotometryReadPage() -> impl IntoView {
    photometry_page(false)
}

/// Edit view — declared GLDF values as editable inputs + calculated comparison + Use-calc button.
#[component]
pub fn PhotometryEditorPage() -> impl IntoView {
    photometry_page(true)
}

fn photometry_page(editable: bool) -> impl IntoView {
    let state = use_gldf();

    view! {
        <div class="editor-section photometry-editor">
            <h2>{ "Photometry" }</h2>
            <p class="section-description">
                { "DescriptivePhotometry values per photometry entry. The GLDF column shows what the file declares; the Calc column shows the value we compute from the embedded LDT/IES for comparison." }
            </p>
            <LinkHint
                title="Where do I link this photometry to a light source or spectrum?".to_string()
                body="A <Photometry> only declares the beam shape and a pointer to its LDT/IES file — the GLDF schema does not allow a direct spectrum or light-source link here. The link happens one level up: an Emitter binds a PhotometryReference (this photometry) to a LightSourceReference (the lamp), and the light source carries the SpectrumReference. Edit those connections on the Light Sources and Emitters pages.".to_string()
            />

            {move || state.with(|s| {
                let entries: Vec<Photometry> = s
                    .product
                    .general_definitions
                    .photometries
                    .as_ref()
                    .map(|p| p.photometry.clone())
                    .unwrap_or_default();
                if entries.is_empty() {
                    return view! {
                        <p class="empty-message">{ "No photometry entries in this file." }</p>
                    }.into_any();
                }

                // Per-entry probe: what file id does the Photometry point to,
                // is it in embedded_files, did parsing succeed, what does the
                // <File> definition say? Surfaced via a `<details>` block on
                // each row so we can diagnose missing-Calc cases without
                // round-tripping through a rebuild.
                let with_calc: Vec<(Photometry, Option<CalculatedFields>, PhotometryProbe, Option<Vec<u8>>)> = entries
                    .into_iter()
                    .map(|p| {
                        use crate::components::photometry_decode::{
                            decode_photometry_bytes, parse_photometry_with_fallback,
                        };
                        let mut probe = PhotometryProbe::default();
                        probe.file_id = p.photometry_file_reference.as_ref().map(|r| r.file_id.clone());
                        let raw_bytes: Option<Vec<u8>> = probe
                            .file_id
                            .as_ref()
                            .and_then(|fid| s.embedded_files.get(fid).cloned());
                        if let Some(ref fid) = probe.file_id {
                            probe.in_embedded = s.embedded_files.contains_key(fid);
                            if let Some(file_def) = s.product.general_definitions.files.file.iter().find(|f| &f.id == fid) {
                                probe.file_def_content_type = Some(file_def.content_type.clone());
                                probe.file_def_file_name = Some(file_def.file_name.clone());
                            }
                            if let Some(bytes) = s.embedded_files.get(fid) {
                                probe.bytes_len = bytes.len();
                                let (s_text, decode_kind) = decode_photometry_bytes(bytes);
                                probe.utf8_ok = decode_kind
                                    == crate::components::photometry_decode::DecodeKind::Utf8;
                                probe.decode_kind = Some(decode_kind.label().to_string());
                                let trimmed = s_text.trim_start();
                                probe.starts_with_iesna = trimmed.starts_with("IESNA")
                                    || trimmed.starts_with("IES:");
                                probe.first_chars = Some(
                                    s_text.chars().take(80).collect::<String>().replace('\n', "⏎"),
                                );
                            }
                        }
                        let calc = probe
                            .file_id
                            .as_ref()
                            .and_then(|fid| s.embedded_files.get(fid))
                            .and_then(|bytes| {
                                let (content, _) = decode_photometry_bytes(bytes);
                                let parsed = parse_photometry_with_fallback(&content);
                                probe.parse_ok = parsed.is_some();
                                probe.parse_kind = parsed.as_ref().map(|(_, k)| k.label().to_string());
                                parsed.map(|(ldt, _)| ldt)
                            })
                            .map(|ldt| CalculatedFields::from_ldt(&ldt));
                        (p, calc, probe, raw_bytes)
                    })
                    .collect();

                // Orphan photometry files — <File> entries with an ldc/*
                // content_type that NO <Photometry> references. Legal GLDF
                // (a File is just a content blob) but invisible elsewhere
                // in the editor outside FileViewer. Surface them here so
                // the user can see why the FileViewer count exceeds the
                // Photometry entry count, and so they can decide whether
                // to wire them into a Photometry block.
                let referenced_ids: std::collections::HashSet<String> = with_calc
                    .iter()
                    .filter_map(|(p, _, _, _)| {
                        p.photometry_file_reference.as_ref().map(|r| r.file_id.clone())
                    })
                    .collect();
                let orphan_files: Vec<(String, String, String)> = s
                    .product
                    .general_definitions
                    .files
                    .file
                    .iter()
                    .filter(|f| f.content_type.starts_with("ldc/") && !referenced_ids.contains(&f.id))
                    .map(|f| (f.id.clone(), f.content_type.clone(), f.file_name.clone()))
                    .collect();

                view! {
                    <div class="photometry-entries" style="display: flex; flex-direction: column; gap: 16px;">
                        {with_calc.into_iter().enumerate().map(|(idx, (photometry, calc, probe, raw_bytes))| view! {
                            <PhotometryRow idx=idx photometry=photometry calculated=calc probe=probe raw_bytes=raw_bytes editable=editable />
                        }).collect::<Vec<_>>()}

                        {if orphan_files.is_empty() {
                            ().into_any()
                        } else {
                            view! {
                                <OrphanPhotometryFiles files=orphan_files />
                            }.into_any()
                        }}
                    </div>
                }.into_any()
            })}
        </div>
    }
}

/// Optional<value> form of `GldfPhotometricData` with the
/// LOR/DLOR/ULOR/DFF percent → fraction conversion applied.
#[derive(Clone, Default)]
struct CalculatedFields {
    cie_flux_code: Option<String>,
    light_output_ratio: Option<f64>,
    luminous_efficacy: Option<f64>,
    downward_flux_fraction: Option<f64>,
    downward_light_output_ratio: Option<f64>,
    upward_light_output_ratio: Option<f64>,
    cut_off_angle: Option<f64>,
    luminaire_luminance: Option<i32>,
    ugr_x: Option<f64>,
    ugr_y: Option<f64>,
    photometric_code: Option<String>,
    bug_rating: Option<String>,
}

impl CalculatedFields {
    fn from_ldt(ldt: &Eulumdat) -> Self {
        let d = GldfPhotometricData::from_eulumdat(ldt);
        Self {
            cie_flux_code: Some(d.cie_flux_code),
            light_output_ratio: Some(d.light_output_ratio / 100.0),
            luminous_efficacy: Some(d.luminous_efficacy),
            downward_flux_fraction: Some(d.downward_flux_fraction / 100.0),
            downward_light_output_ratio: Some(d.downward_light_output_ratio / 100.0),
            upward_light_output_ratio: Some(d.upward_light_output_ratio / 100.0),
            cut_off_angle: Some(d.cut_off_angle),
            luminaire_luminance: Some(d.luminaire_luminance as i32),
            ugr_x: d.ugr_4h_8h_705020.as_ref().map(|u| u.crosswise),
            ugr_y: d.ugr_4h_8h_705020.as_ref().map(|u| u.endwise),
            photometric_code: Some(d.photometric_code),
            bug_rating: Some(d.light_distribution_bug_rating),
        }
    }
}

/// Diagnostic snapshot of why the Calc column did or didn't render. Shown
/// in a `<details>` block under each entry while we debug IES support.
#[derive(Default, Clone, Debug)]
struct PhotometryProbe {
    file_id: Option<String>,
    in_embedded: bool,
    file_def_content_type: Option<String>,
    file_def_file_name: Option<String>,
    bytes_len: usize,
    utf8_ok: bool,
    /// Which decode path the helper took ("UTF-8" or "Latin-1 (fallback)").
    decode_kind: Option<String>,
    starts_with_iesna: bool,
    parse_ok: bool,
    /// Which parser variant accepted the content ("IES" or "EULUMDAT").
    /// `None` when the parse failed.
    parse_kind: Option<String>,
    first_chars: Option<String>,
}

// Photometric parsing is now centralised in
// `components::photometry_decode` — UTF-8 then Latin-1 fallback, IES then
// EULUMDAT trial parse. Callers use those helpers directly.

#[component]
fn PhotometryRow(
    idx: usize,
    photometry: Photometry,
    calculated: Option<CalculatedFields>,
    probe: PhotometryProbe,
    raw_bytes: Option<Vec<u8>>,
    editable: bool,
) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    let id = photometry.id.clone();
    let id_for_rename = id.clone();
    let id_for_remove = id.clone();
    let on_rename_id: Arc<dyn Fn(String) + Send + Sync> = {
        let old_id = id_for_rename.clone();
        Arc::new(move |new_id| {
            state.update(|s| s.apply(GldfAction::RenamePhotometry {
                old_id: old_id.clone(),
                new_id,
            }));
        })
    };
    let on_remove = move |_| {
        let template = ui_t(
            "gldf.photometry.remove_confirm",
            &state.with(|s| s.ui_language.clone()),
        );
        let msg = template.replace("{id}", &id_for_remove);
        if crate::components::edit_util::confirm(&msg)
        {
            let id_now = id_for_remove.clone();
            state.update(|s| s.apply(GldfAction::RemovePhotometry { id: id_now }));
        }
    };
    let file_ref = photometry.photometry_file_reference.as_ref().map(|r| r.file_id.clone());
    let desc = photometry.descriptive_photometry.clone();
    let ugr = desc.as_ref().and_then(|d| d.ugr4_h8_h705020_lq.clone());

    // GLDF-declared values as Option<String>. Empty → "-" placeholder.
    let gldf_cie = desc.as_ref().and_then(|d| d.cie_flux_code.clone());
    let gldf_lor = desc.as_ref().and_then(|d| d.light_output_ratio).map(|v| v.to_string());
    let gldf_eff = desc.as_ref().and_then(|d| d.luminous_efficacy).map(|v| v.to_string());
    let gldf_dff = desc.as_ref().and_then(|d| d.downward_flux_fraction).map(|v| v.to_string());
    let gldf_dlor = desc.as_ref().and_then(|d| d.downward_light_output_ratio).map(|v| v.to_string());
    let gldf_ulor = desc.as_ref().and_then(|d| d.upward_light_output_ratio).map(|v| v.to_string());
    let gldf_cutoff = desc.as_ref().and_then(|d| d.cut_off_angle).map(|v| v.to_string());
    let gldf_lum = desc.as_ref().and_then(|d| d.luminaire_luminance).map(|v| v.to_string());
    let gldf_ux = ugr.as_ref().and_then(|u| u.x).map(|v| v.to_string());
    let gldf_uy = ugr.as_ref().and_then(|u| u.y).map(|v| v.to_string());
    let gldf_code = desc.as_ref().and_then(|d| d.photometric_code.clone());
    let gldf_bug = desc.as_ref().and_then(|d| d.light_distribution_bug_rating.clone());

    // Calculated values as display strings.
    let calc_cie = calculated.as_ref().and_then(|c| c.cie_flux_code.clone());
    let calc_lor = calculated.as_ref().and_then(|c| c.light_output_ratio).map(|v| format!("{:.3}", v));
    let calc_eff = calculated.as_ref().and_then(|c| c.luminous_efficacy).map(|v| format!("{:.1}", v));
    let calc_dff = calculated.as_ref().and_then(|c| c.downward_flux_fraction).map(|v| format!("{:.3}", v));
    let calc_dlor = calculated.as_ref().and_then(|c| c.downward_light_output_ratio).map(|v| format!("{:.3}", v));
    let calc_ulor = calculated.as_ref().and_then(|c| c.upward_light_output_ratio).map(|v| format!("{:.3}", v));
    let calc_cutoff = calculated.as_ref().and_then(|c| c.cut_off_angle).map(|v| format!("{:.1}", v));
    let calc_lum = calculated.as_ref().and_then(|c| c.luminaire_luminance).map(|v| v.to_string());
    let calc_ux = calculated.as_ref().and_then(|c| c.ugr_x).map(|v| format!("{:.1}", v));
    let calc_uy = calculated.as_ref().and_then(|c| c.ugr_y).map(|v| format!("{:.1}", v));
    let calc_code = calculated.as_ref().and_then(|c| c.photometric_code.clone());
    let calc_bug = calculated.as_ref().and_then(|c| c.bug_rating.clone());

    let has_calculated = calculated.is_some();
    let calc_for_button = calculated.clone();

    // "Use calculated values" — fill EMPTY GLDF fields from the calculator.
    // Never overwrites an existing declared value: the GLDF file's own
    // values are authoritative as authored.
    let on_use_calc = move |_| {
        let Some(c) = calc_for_button.clone() else { return };
        state.update(|s| {
            let descriptive = s
                .product
                .general_definitions
                .photometries
                .as_ref()
                .and_then(|p| p.photometry.get(idx))
                .and_then(|p| p.descriptive_photometry.clone());
            macro_rules! fill_if_empty_str {
                ($accessor:ident, $action:ident, $calc:expr) => {
                    if descriptive.as_ref().and_then(|d| d.$accessor.clone()).is_none() {
                        if let Some(v) = $calc.clone() {
                            s.apply(GldfAction::$action { index: idx, value: Some(v) });
                        }
                    }
                };
            }
            macro_rules! fill_if_empty {
                ($accessor:ident, $action:ident, $calc:expr) => {
                    if descriptive.as_ref().and_then(|d| d.$accessor).is_none() {
                        if let Some(v) = $calc {
                            s.apply(GldfAction::$action { index: idx, value: Some(v) });
                        }
                    }
                };
            }
            fill_if_empty_str!(cie_flux_code, SetPhotometryCieFluxCode, c.cie_flux_code);
            fill_if_empty!(light_output_ratio, SetPhotometryLightOutputRatio, c.light_output_ratio);
            fill_if_empty!(luminous_efficacy, SetPhotometryLuminousEfficacy, c.luminous_efficacy);
            fill_if_empty!(downward_flux_fraction, SetPhotometryDownwardFluxFraction, c.downward_flux_fraction);
            fill_if_empty!(downward_light_output_ratio, SetPhotometryDownwardLOR, c.downward_light_output_ratio);
            fill_if_empty!(upward_light_output_ratio, SetPhotometryUpwardLOR, c.upward_light_output_ratio);
            fill_if_empty!(cut_off_angle, SetPhotometryCutOffAngle, c.cut_off_angle);
            fill_if_empty!(luminaire_luminance, SetPhotometryLuminaireLuminance, c.luminaire_luminance);
            // UGR x/y nest under ugr4_h8_h705020_lq; if the whole struct is
            // missing we treat both as empty.
            let ugr = descriptive.as_ref().and_then(|d| d.ugr4_h8_h705020_lq.as_ref());
            if ugr.and_then(|u| u.x).is_none() {
                if let Some(v) = c.ugr_x { s.apply(GldfAction::SetPhotometryUgrX { index: idx, value: Some(v) }); }
            }
            if ugr.and_then(|u| u.y).is_none() {
                if let Some(v) = c.ugr_y { s.apply(GldfAction::SetPhotometryUgrY { index: idx, value: Some(v) }); }
            }
            fill_if_empty_str!(photometric_code, SetPhotometryPhotometricCode, c.photometric_code);
            fill_if_empty_str!(light_distribution_bug_rating, SetPhotometryBugRating, c.bug_rating);
        });
    };

    // String-on-change → action dispatch. One per field.
    let on_cie = move |e: crate::components::edit_util::EditEvent| {
        let v = event_value(&e).unwrap_or_default();
        state.update(|s| s.apply(GldfAction::SetPhotometryCieFluxCode { index: idx, value: optional_string(v) }));
    };
    let on_lor = move |e: crate::components::edit_util::EditEvent| {
        state.update(|s| s.apply(GldfAction::SetPhotometryLightOutputRatio { index: idx, value: event_f64(&e) }));
    };
    let on_eff = move |e: crate::components::edit_util::EditEvent| {
        state.update(|s| s.apply(GldfAction::SetPhotometryLuminousEfficacy { index: idx, value: event_f64(&e) }));
    };
    let on_dff = move |e: crate::components::edit_util::EditEvent| {
        state.update(|s| s.apply(GldfAction::SetPhotometryDownwardFluxFraction { index: idx, value: event_f64(&e) }));
    };
    let on_dlor = move |e: crate::components::edit_util::EditEvent| {
        state.update(|s| s.apply(GldfAction::SetPhotometryDownwardLOR { index: idx, value: event_f64(&e) }));
    };
    let on_ulor = move |e: crate::components::edit_util::EditEvent| {
        state.update(|s| s.apply(GldfAction::SetPhotometryUpwardLOR { index: idx, value: event_f64(&e) }));
    };
    let on_cutoff = move |e: crate::components::edit_util::EditEvent| {
        state.update(|s| s.apply(GldfAction::SetPhotometryCutOffAngle { index: idx, value: event_f64(&e) }));
    };
    let on_luminance = move |e: crate::components::edit_util::EditEvent| {
        state.update(|s| s.apply(GldfAction::SetPhotometryLuminaireLuminance { index: idx, value: event_i32(&e) }));
    };
    let on_ugr_x = move |e: crate::components::edit_util::EditEvent| {
        state.update(|s| s.apply(GldfAction::SetPhotometryUgrX { index: idx, value: event_f64(&e) }));
    };
    let on_ugr_y = move |e: crate::components::edit_util::EditEvent| {
        state.update(|s| s.apply(GldfAction::SetPhotometryUgrY { index: idx, value: event_f64(&e) }));
    };
    let on_code = move |e: crate::components::edit_util::EditEvent| {
        let v = event_value(&e).unwrap_or_default();
        state.update(|s| s.apply(GldfAction::SetPhotometryPhotometricCode { index: idx, value: optional_string(v) }));
    };
    let on_bug = move |e: crate::components::edit_util::EditEvent| {
        let v = event_value(&e).unwrap_or_default();
        state.update(|s| s.apply(GldfAction::SetPhotometryBugRating { index: idx, value: optional_string(v) }));
    };

    view! {
        <div class="photometry-entry" style="border: 1px solid var(--border-color); border-radius: 6px; padding: 12px;">
            <div class="entry-header" style="display: flex; gap: 8px; align-items: center; margin-bottom: 8px; flex-wrap: wrap;">
                <IdRenameChip current_id=id editable=editable on_rename=on_rename_id />
                {file_ref.map(|f| view! {
                    <span style="color: var(--text-tertiary); font-size: 11px;">{ format!("file: {}", f) }</span>
                })}
                {if editable && has_calculated {
                    view! {
                        <button
                            type="button"
                            on:click=on_use_calc
                            style="margin-left: auto; padding: 4px 10px; background: var(--accent-orange); color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 12px;"
                            title=tr("gldf.photometry.fill_calc_title")
                        >
                            { tr("gldf.photometry.fill_calc") }
                        </button>
                    }.into_any()
                } else { ().into_any() }}
                {if editable {
                    let ml = if has_calculated { "" } else { "margin-left: auto; " };
                    let style = format!("{}padding: 2px 10px; background: none; border: 1px solid var(--accent-orange); color: var(--accent-orange); border-radius: 4px; cursor: pointer; font-size: 12px;", ml);
                    view! {
                        <button type="button" on:click=on_remove style=style title=tr("gldf.photometry.remove_title")>{ tr("gldf.photometry.remove") }</button>
                    }.into_any()
                } else { ().into_any() }}
            </div>

            <CalcLegend has_calculated=has_calculated />

            <ProbeBlock probe=probe.clone() />

            // Small angular-distribution thumbnail — this is the file
            // as authored. The "real" rendering (per-emitter, with
            // rotations and the right LightSource binding) lives on
            // the LightSource card; this is just an at-a-glance shape
            // check while editing DescriptivePhotometry values.
            {raw_bytes.map(|bytes| view! {
                <div style="margin-bottom: 12px;">
                    <LdtPanel bytes=bytes width=320.0 height=240.0 />
                </div>
            })}

            <div class="form-grid" style="display: flex; flex-direction: column; gap: 8px;">
                <DualField label="CIE Flux Code" gldf=gldf_cie calc=calc_cie input_type="text" step="" editable=editable on_change=Box::new(on_cie) />
                <DualField label="Light Output Ratio (LOR)" gldf=gldf_lor calc=calc_lor input_type="number" step="0.001" editable=editable on_change=Box::new(on_lor) />
                <DualField label="Luminous Efficacy (lm/W)" gldf=gldf_eff calc=calc_eff input_type="number" step="0.1" editable=editable on_change=Box::new(on_eff) />
                <DualField label="Downward Flux Fraction" gldf=gldf_dff calc=calc_dff input_type="number" step="0.001" editable=editable on_change=Box::new(on_dff) />
                <DualField label="Downward LOR" gldf=gldf_dlor calc=calc_dlor input_type="number" step="0.001" editable=editable on_change=Box::new(on_dlor) />
                <DualField label="Upward LOR" gldf=gldf_ulor calc=calc_ulor input_type="number" step="0.001" editable=editable on_change=Box::new(on_ulor) />
                <DualField label="Cut-off Angle (°)" gldf=gldf_cutoff calc=calc_cutoff input_type="number" step="0.1" editable=editable on_change=Box::new(on_cutoff) />
                <DualField label="Luminaire Luminance (cd/m²)" gldf=gldf_lum calc=calc_lum input_type="number" step="1" editable=editable on_change=Box::new(on_luminance) />
                <DualField label="UGR x" gldf=gldf_ux calc=calc_ux input_type="number" step="0.1" editable=editable on_change=Box::new(on_ugr_x) />
                <DualField label="UGR y" gldf=gldf_uy calc=calc_uy input_type="number" step="0.1" editable=editable on_change=Box::new(on_ugr_y) />
                <DualField label="Photometric Code" gldf=gldf_code calc=calc_code input_type="text" step="" editable=editable on_change=Box::new(on_code) />
                <DualField label="BUG Rating" gldf=gldf_bug calc=calc_bug input_type="text" step="" editable=editable on_change=Box::new(on_bug) />
            </div>
        </div>
    }
}

#[component]
fn CalcLegend(has_calculated: bool) -> impl IntoView {
    let badge_style = "display: inline-block; padding: 1px 6px; border-radius: 3px; font-size: 10px; color: white; margin-right: 4px;";
    let caption = if has_calculated {
        "Calculated from the embedded LDT/IES file."
    } else {
        "No LDT/IES file embedded — Calc column shows '-'."
    };
    view! {
        <div style="display: flex; gap: 8px; align-items: center; font-size: 11px; color: var(--text-tertiary); margin-bottom: 8px; flex-wrap: wrap;">
            <span style=format!("{} background: var(--accent-blue);", badge_style)>{ "GLDF" }</span>
            <span>{ "= declared in the GLDF file (authoritative)." }</span>
            <span style=format!("{} background: var(--accent-orange); margin-left: 12px;", badge_style)>{ "Calc" }</span>
            <span>{ caption }</span>
        </div>
    }
}

/// Diagnostic block — what we know about the photometry's embedded file
/// and why the calculator did or didn't run. Collapsed by default; expand
/// to see the chain. Remove once IES rendering is reliably green.
#[component]
fn ProbeBlock(probe: PhotometryProbe) -> impl IntoView {
    let summary = match (probe.file_id.is_some(), probe.in_embedded, probe.parse_ok) {
        (false, _, _) => "No file_id on photometry".to_string(),
        (true, false, _) => format!("file_id `{}` NOT in embedded_files", probe.file_id.clone().unwrap_or_default()),
        (true, true, false) => "Bytes present but eulumdat parse failed".to_string(),
        (true, true, true) => "Bytes present, parse ok".to_string(),
    };
    view! {
        <details style="margin-bottom: 8px; font-size: 11px; color: var(--text-tertiary);">
            <summary style="cursor: pointer;">{ format!("debug: {}", summary) }</summary>
            <pre style="background: var(--bg-secondary); padding: 6px; border-radius: 4px; overflow: auto; white-space: pre-wrap; font-size: 11px;">
                { format!(
                    "file_id          : {:?}\nin_embedded      : {}\nfile_def.ct      : {:?}\nfile_def.name    : {:?}\nbytes_len        : {}\nutf8_ok          : {}\ndecode_kind      : {:?}\nstarts_with_iesna: {}\nparse_ok         : {}\nparse_kind       : {:?}\nfirst_chars      : {:?}",
                    probe.file_id,
                    probe.in_embedded,
                    probe.file_def_content_type,
                    probe.file_def_file_name,
                    probe.bytes_len,
                    probe.utf8_ok,
                    probe.decode_kind,
                    probe.starts_with_iesna,
                    probe.parse_ok,
                    probe.parse_kind,
                    probe.first_chars,
                ) }
            </pre>
        </details>
    }
}

/// One field row: GLDF box on the left (editable when `editable=true`),
/// Calc box on the right (always display-only).
#[component]
fn DualField(
    label: &'static str,
    gldf: Option<String>,
    calc: Option<String>,
    input_type: &'static str,
    step: &'static str,
    editable: bool,
    on_change: Box<dyn Fn(crate::components::edit_util::EditEvent) + Send + Sync>,
) -> impl IntoView {
    let has_gldf = gldf.is_some();
    let has_calc = calc.is_some();
    let gldf_value = gldf.clone().unwrap_or_default();
    let calc_display = calc.clone().unwrap_or_else(|| "-".to_string());
    let placeholder = calc.clone().unwrap_or_else(|| "-".to_string());

    let gldf_box_style = if has_gldf {
        "padding: 4px 6px; border: 1px solid var(--accent-blue); border-radius: 4px; background: var(--bg-secondary);"
    } else {
        "padding: 4px 6px; border: 1px dashed var(--border-color); border-radius: 4px; background: var(--bg-secondary);"
    };
    let calc_box_style = if has_calc {
        "padding: 4px 6px; border: 1px solid var(--accent-orange); border-radius: 4px; background: var(--bg-secondary);"
    } else {
        "padding: 4px 6px; border: 1px dashed var(--border-color); border-radius: 4px; background: var(--bg-secondary);"
    };

    let gldf_display = if has_gldf { gldf_value.clone() } else { "-".to_string() };

    view! {
        <div class="photometry-field-dual" style="display: grid; grid-template-columns: 200px 1fr 1fr; gap: 8px; align-items: center;">
            <label style="font-size: 12px; color: var(--text-secondary);">{ label }</label>
            <div class="gldf-box" style=gldf_box_style>
                {if editable {
                    view! {
                        <input
                            type=input_type
                            step=step
                            prop:value=gldf_value
                            on:change=move |e| on_change(e)
                            placeholder=placeholder
                            style="width: 100%; padding: 2px 4px; font-size: 12px; background: transparent; border: none; color: var(--text-primary);"
                        />
                    }.into_any()
                } else {
                    view! {
                        <span style="font-size: 12px; color: var(--text-primary);">{ gldf_display }</span>
                    }.into_any()
                }}
            </div>
            <div class="calc-box" style=calc_box_style>
                <span style="font-size: 12px; color: var(--text-primary);">{ calc_display }</span>
            </div>
        </div>
    }
}

/// Sub-section listing `<File>` entries with an `ldc/*` content_type that
/// no `<Photometry>` block references. Legal GLDF — a `<File>` is just an
/// embedded payload — but those files don't appear anywhere else in the
/// editor outside FileViewer. Surface them here so the user can see why
/// the FileViewer photometry count can exceed the Photometry entry count,
/// and so they can decide whether to wire each into a `<Photometry>`.
///
/// `files`: list of `(id, content_type, file_name)` triples.
#[component]
fn OrphanPhotometryFiles(files: Vec<(String, String, String)>) -> impl IntoView {
    view! {
        <div class="orphan-photometry-files" style="margin-top: 16px; padding: 12px; border: 1px dashed var(--border-color); border-radius: 6px; background: var(--bg-secondary);">
            <h3 style="margin: 0 0 4px 0; font-size: 14px;">
                { "Photometry-typed files not declared as " } <code>{ "<Photometry>" }</code>
            </h3>
            <p style="margin: 0 0 8px 0; font-size: 12px; color: var(--text-secondary);">
                { "These embedded files use an " }<code>{ "ldc/*" }</code>{ " content type but no " }<code>{ "<Photometry>" }</code>
                { " block references them. The GLDF schema allows this — files can ship without a Photometry wrapper — but they won't appear in this page's table or in viewers that key off Photometry blocks. Add a Photometry entry referencing the file id to surface it." }
            </p>
            <ul style="margin: 0; padding-left: 18px; font-size: 12px;">
                {files.into_iter().map(|(id, content_type, file_name)| view! {
                    <li style="margin-bottom: 4px;">
                        <code>{ id }</code>
                        { " — " }
                        <span style="color: var(--text-secondary);">{ content_type }</span>
                        { " — " }
                        <span>{ file_name }</span>
                    </li>
                }).collect::<Vec<_>>()}
            </ul>
        </div>
    }
}
