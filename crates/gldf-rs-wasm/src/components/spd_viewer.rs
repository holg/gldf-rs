//! Spectral / TM-30 visualization for embedded `<Spectrum>` entries.
//!
//! Renders, for every spectrum in `GeneralDefinitions/Spectrums`:
//!
//! - **SPD curve** — `SpectralDiagram::from_spectral(&spd).to_svg(...)`
//! - **TM-30 CVG** — `Tm30Result::to_svg(...)` (color vector graphic, 16 hue bins)
//! - **TM-30 Rf-hue** — `Tm30Result::rf_hue_svg(...)` (per-hue fidelity bars)
//! - **Colorimetry box** — CCT, Duv, x/y, peak λ, FWHM (from `colorimetry::analyze`)
//!
//! All four are inline SVG strings + a small key/value list, so the bundle
//! impact is zero new deps (eulumdat is already in the workspace).
//!
//! Source order of preference:
//! 1. `Spectrum/SpectrumFileReference` → bytes from `state.embedded_files`,
//!    parsed with `eulumdat::atla::spd_loader::parse`.
//! 2. Inline `<Intensity>` entries (the SOLART pattern) → build
//!    `SpectralDistribution` directly from the wavelength/value pairs.

use crate::state::use_gldf;
use eulumdat::atla::{
    colorimetry,
    greenhouse::{GreenhouseDiagram, GreenhouseTheme},
    spd_loader,
    spectral::{SpectralDiagram, SpectralTheme},
    tm30::{calculate_tm30, Tm30Theme},
    types::{Emitter, LuminaireOpticalData, SpectralDistribution, SpectralUnits},
};
use gldf_rs::gldf::general_definitions::photometries::Spectrum;
use gldf_rs::gldf::GldfProduct;
use web_sys::{Element, HtmlInputElement};
use yew::prelude::*;

/// Mount an SVG string into the DOM by calling `Element::set_inner_html` on a
/// real `<div>` after render. We can't use `Html::from_html_unchecked` because
/// Yew parses the string in a context that drops the SVG geometry primitives
/// (`<path>`, `<rect>`, `<line>`, `<circle>`) and only keeps text — the user
/// then sees the labels but no chart. Going through `Element::set_inner_html`
/// preserves the SVG namespace and the figures render.
#[derive(Properties, PartialEq, Clone)]
struct SvgEmbedProps {
    svg: String,
    #[prop_or_default]
    style: String,
}

#[function_component(SvgEmbed)]
fn svg_embed(props: &SvgEmbedProps) -> Html {
    let node_ref = use_node_ref();

    {
        let node_ref = node_ref.clone();
        let svg = props.svg.clone();
        use_effect_with(svg.clone(), move |svg| {
            if let Some(el) = node_ref.cast::<Element>() {
                el.set_inner_html(svg);
            }
            || ()
        });
    }

    html! { <div ref={node_ref} style={props.style.clone()}></div> }
}

/// Strip `<text>...</text>` elements whose body starts with any of the given
/// prefixes (e.g. "Duv =", "Rf =", "CCT =", "Rg =").
///
/// eulumdat-rs bakes in-SVG header labels for these metrics, but two of them
/// are broken at the source: the TM-30 module's `self.duv` is pre-scaled by
/// 1000× (so the CVG header reads e.g. `Duv = -33.9128` instead of
/// `Duv = +0.0033`), and the Rf-hue chart's `Rf = 60` label sometimes renders
/// with a stray glyph in the browser. We drop those header labels and render
/// our own caption underneath using the correct `colorimetry::analyze` and
/// `tm30::calculate_tm30` numbers. The plot bodies (axes, curve, bars, color
/// vectors) are untouched.
fn strip_text_elements_with_prefix(svg: &str, prefixes: &[&str]) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(start) = rest.find("<text") {
        // Copy everything before this <text ...>.
        out.push_str(&rest[..start]);
        // Find the end of the element.
        let after_start = &rest[start..];
        let end_rel = match after_start.find("</text>") {
            Some(idx) => idx + "</text>".len(),
            None => {
                // Malformed — bail out: append the rest and stop.
                out.push_str(after_start);
                return out;
            }
        };
        let element = &after_start[..end_rel];
        // Locate the inner text (between the first '>' and the trailing "</text>").
        let inner = element
            .find('>')
            .and_then(|gt| element.get(gt + 1..element.len() - "</text>".len()))
            .unwrap_or("");
        let trimmed = inner.trim_start();
        if prefixes.iter().any(|p| trimmed.starts_with(p)) {
            // Drop the element entirely.
        } else {
            out.push_str(element);
        }
        rest = &after_start[end_rel..];
    }
    out.push_str(rest);
    out
}

/// Resolve a spectrum to a `SpectralDistribution`. Returns `None` if the
/// spectrum references a missing file or has neither a file ref nor inline
/// intensities.
fn resolve_spd(
    spectrum: &Spectrum,
    embedded_files: &std::collections::HashMap<String, Vec<u8>>,
) -> Option<(SpectralDistribution, String)> {
    // (a) File reference — load bytes, parse via eulumdat's auto-format loader.
    if let Some(file_ref) = spectrum.spectrum_file_reference.as_ref() {
        if let Some(bytes) = embedded_files.get(&file_ref.file_id) {
            let content = std::str::from_utf8(bytes).ok()?;
            match spd_loader::parse(content) {
                Ok(loaded) => return Some((loaded.spd, format!("file {}", file_ref.file_id))),
                Err(e) => return Some((SpectralDistribution::default(), format!("parse error: {}", e))),
            }
        }
        return Some((
            SpectralDistribution::default(),
            format!("file id {} not in embedded_files", file_ref.file_id),
        ));
    }

    // (b) Inline <Intensity wavelength="..."> entries. Skip rows missing either
    //     side; sort by wavelength because XML order isn't guaranteed.
    if !spectrum.intensity.is_empty() {
        let mut pairs: Vec<(f64, f64)> = spectrum
            .intensity
            .iter()
            .filter_map(|i| Some((i.wavelength? as f64, i.value?)))
            .collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        if pairs.is_empty() {
            return None;
        }
        let wavelengths: Vec<f64> = pairs.iter().map(|p| p.0).collect();
        let values: Vec<f64> = pairs.iter().map(|p| p.1).collect();
        let start = wavelengths.first().copied();
        // Constant-step detection: if every gap matches the first, mark it.
        let interval = if wavelengths.len() >= 2 {
            let step = wavelengths[1] - wavelengths[0];
            if wavelengths
                .windows(2)
                .all(|w| (w[1] - w[0] - step).abs() < 1e-6)
            {
                Some(step)
            } else {
                None
            }
        } else {
            None
        };
        return Some((
            SpectralDistribution {
                wavelengths,
                values,
                units: SpectralUnits::Relative,
                start_wavelength: start,
                wavelength_interval: interval,
            },
            "inline <Intensity>".to_string(),
        ));
    }

    None
}

/// Build a minimal ATLA `LuminaireOpticalData` from a `GldfProduct` and a
/// parsed SPD, suitable for handing to `GreenhouseDiagram::from_atla_with_height`.
///
/// We only fill the fields greenhouse actually reads:
/// - `input_watts` from the first FixedLightSource's `rated_input_power`
/// - `measured_lumens` from the first emitter's `rated_luminous_flux`
/// - `spectral_distribution` from the SPD we just rendered
///
/// Beam angle is left unset so greenhouse falls back to its 120° default. The
/// PPF→lumens conversion factor (~1.0 µmol/lm for full-spectrum white) is
/// derived inside eulumdat from the SPD curve, so the result reacts to the
/// actual spectrum rather than a constant.
fn build_atla_for_greenhouse(product: &GldfProduct, spd: &SpectralDistribution) -> LuminaireOpticalData {
    let watts = product
        .general_definitions
        .light_sources
        .as_ref()
        .and_then(|ls| ls.fixed_light_source.first().and_then(|s| s.rated_input_power));
    let lumens = product
        .general_definitions
        .emitters
        .as_ref()
        .and_then(|emitters| {
            emitters
                .emitter
                .iter()
                .flat_map(|e| e.fixed_light_emitter.iter())
                .find_map(|fle| fle.rated_luminous_flux.map(|v| v as f64))
        });

    let mut doc = LuminaireOpticalData::new();
    doc.emitters.push(Emitter {
        input_watts: watts,
        measured_lumens: lumens,
        spectral_distribution: Some(spd.clone()),
        quantity: 1,
        ..Default::default()
    });
    doc
}

#[function_component(SpdViewer)]
pub fn spd_viewer() -> Html {
    let gldf = use_gldf();

    let spectrums = gldf.product.general_definitions.spectrums.as_ref();
    let spectrum_count = spectrums.map(|s| s.spectrum.len()).unwrap_or(0);

    if spectrum_count == 0 {
        return html! {
            <div class="empty-state">
                <div class="icon">{ "🌈" }</div>
                <h3>{ "No Spectra" }</h3>
                <p>{ "This GLDF has no <Spectrum> entries. Add a .spd file in the Builder to see SPD + TM-30 plots here." }</p>
            </div>
        };
    }

    let panels: Vec<Html> = spectrums
        .unwrap()
        .spectrum
        .iter()
        .map(|spectrum| {
            html! {
                <SpectrumPanel spectrum={spectrum.clone()} />
            }
        })
        .collect();

    html! {
        <div class="spd-viewer" style="padding: 8px;">
            <p style="color: #666; margin-top: 0;">
                { format!("{} spectrum{} — SPD curve + Metrics + TM-30 CVG / Rf-hue + Greenhouse PPFD, computed via eulumdat-rs.", spectrum_count, if spectrum_count == 1 {""} else {"s"}) }
            </p>
            { for panels }
        </div>
    }
}

#[derive(Properties, PartialEq, Clone)]
struct SpectrumPanelProps {
    spectrum: Spectrum,
}

/// One panel for one `<Spectrum>`. Owns the greenhouse height-slider state so
/// each spectrum has its own independent slider. Reading from `use_gldf()`
/// (not a prop) keeps the panel reactive: a Builder edit that mutates the
/// product or embedded_files re-renders the panel automatically.
#[function_component(SpectrumPanel)]
fn spectrum_panel(props: &SpectrumPanelProps) -> Html {
    let gldf = use_gldf();
    let spectrum = &props.spectrum;
    let embedded_files = &gldf.embedded_files;

    // Greenhouse height slider state. Default 2.0 m matches eulumdat's
    // `from_atla(&doc)` (which delegates to `from_atla_with_height(_, 2.0)`).
    let greenhouse_height = use_state(|| 2.0_f64);

    let (spd, source) = match resolve_spd(spectrum, embedded_files) {
        Some(pair) => pair,
        None => {
            return html! {
                <div style="border: 1px solid #ccc; padding: 12px; margin: 8px 0;">
                    <h3>{ format!("Spectrum {}", spectrum.id) }</h3>
                    <em style="color: #c00;">{ "No SPD source — neither a SpectrumFileReference nor inline <Intensity>." }</em>
                </div>
            };
        }
    };

    let usable = spd.wavelengths.len() >= 2 && spd.wavelengths.len() == spd.values.len();

    let spd_svg = if usable {
        SpectralDiagram::from_spectral(&spd).to_svg(720.0, 320.0, &SpectralTheme::light())
    } else {
        String::new()
    };

    let tm30 = if usable { calculate_tm30(&spd) } else { None };
    let colorim = if usable { Some(colorimetry::analyze(&spd)) } else { None };

    let theme = Tm30Theme::light();
    // Strip eulumdat-rs's in-SVG header labels (Rf / Rg / CCT / Duv on the CVG,
    // and the trailing "Rf =" label on the Rf-hue chart). eulumdat's Duv field
    // is pre-scaled by 1000× so its CVG header shows a wrong number, and the
    // Rf-hue "Rf =" label renders with a stray glyph. We use the correct
    // numbers from `colorimetry::analyze` + the raw `Tm30Result` and surface
    // them in a caption underneath.
    let label_prefixes = ["Rf =", "Rg =", "CCT =", "Duv ="];
    let cvg_svg = tm30
        .as_ref()
        .map(|r| strip_text_elements_with_prefix(&r.to_svg(420.0, 420.0, &theme), &label_prefixes))
        .unwrap_or_default();
    let rf_hue_svg = tm30
        .as_ref()
        .map(|r| strip_text_elements_with_prefix(&r.rf_hue_svg(540.0, 280.0, &theme), &label_prefixes))
        .unwrap_or_default();

    // Caption with the CORRECT values for the CVG / Rf-hue plots. eulumdat's
    // in-SVG `Duv = ...` header is mis-scaled by 1000× and its `Rf = …` Rf-hue
    // label can render with a stray glyph — those are stripped from the SVG
    // above. We use `colorimetry::analyze` (Duv, CCT) + the raw `Tm30Result`
    // (Rf, Rg) values, which are correct.
    let tm30_caption = match (colorim.as_ref(), tm30.as_ref()) {
        (Some(c), Some(r)) => format!(
            "Rf {:.0} · Rg {:.0} · CCT {:.0} K · Duv {:+.4}",
            r.rf, r.rg, c.cct_k, c.duv
        ),
        _ => String::new(),
    };

    let metric_rows = colorim.as_ref().map(|c| {
        let tm30_rows = tm30
            .as_ref()
            .map(|r| {
                html! {
                    <>
                        <tr><th>{ "TM-30 Rf" }</th><td>{ format!("{:.1}", r.rf) }</td></tr>
                        <tr><th>{ "TM-30 Rg" }</th><td>{ format!("{:.1}", r.rg) }</td></tr>
                    </>
                }
            })
            .unwrap_or(html! { <></> });
        html! {
            <table style="font-size: 0.92em; border-collapse: collapse;">
                <tbody>
                    <tr><th style="text-align: left; padding-right: 14px;">{ "CCT" }</th><td>{ format!("{:.0} K", c.cct_k) }</td></tr>
                    <tr><th style="text-align: left; padding-right: 14px;">{ "Duv" }</th><td>{ format!("{:+.4}", c.duv) }</td></tr>
                    <tr><th style="text-align: left;">{ "x, y (CIE 1931)" }</th><td>{ format!("{:.4}, {:.4}", c.x_1931, c.y_1931) }</td></tr>
                    <tr><th style="text-align: left;">{ "Peak λ" }</th><td>{ format!("{:.1} nm", c.peak_wavelength_nm) }</td></tr>
                    <tr><th style="text-align: left;">{ "FWHM" }</th><td>{ format!("{:.1} nm", c.half_peak_width_nm) }</td></tr>
                    { tm30_rows }
                </tbody>
            </table>
        }
    });

    // Greenhouse (PPFD vs. mounting distance) — adapts a minimal ATLA doc
    // built from the GLDF's first light source / emitter + this SPD. eulumdat
    // computes PPF from the SPD curve, so the result reacts to actual spectrum
    // content. Beam angle falls back to 120° (typical grow-light) because
    // GLDF photometry isn't mapped into ATLA's IntensityDistribution yet.
    let max_height = (*greenhouse_height).max(0.2);
    let greenhouse_svg = if usable {
        let doc = build_atla_for_greenhouse(&gldf.product, &spd);
        let diagram = GreenhouseDiagram::from_atla_with_height(&doc, max_height);
        diagram.to_svg(720.0, 360.0, &GreenhouseTheme::light())
    } else {
        String::new()
    };
    let (greenhouse_ppf, greenhouse_eff, greenhouse_watts) = if usable {
        let doc = build_atla_for_greenhouse(&gldf.product, &spd);
        let d = GreenhouseDiagram::from_atla_with_height(&doc, max_height);
        (d.ppf, d.efficacy, d.watts)
    } else {
        (0.0, 0.0, 0.0)
    };

    let on_height_input = {
        let greenhouse_height = greenhouse_height.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().parse::<f64>() {
                greenhouse_height.set(v);
            }
        })
    };

    html! {
        <div style="border: 1px solid #ccc; padding: 12px; margin: 8px 0;">
            <h3 style="margin-top: 0;">{ format!("Spectrum {}", spectrum.id) }</h3>
            <div style="color: #888; font-size: 0.85em; margin-bottom: 8px;">
                { format!("source: {} · {} samples", source, spd.wavelengths.len()) }
            </div>

            if !usable {
                <em style="color: #c00;">{ "Could not build a usable SPD (need ≥2 wavelength/value pairs)." }</em>
            } else {
                <div style="display: flex; flex-wrap: wrap; gap: 16px; align-items: flex-start;">
                    <div style="min-width: 480px;">
                        <SvgEmbed svg={spd_svg.clone()} />
                    </div>
                    if let Some(rows) = metric_rows {
                        <div style="min-width: 220px;">{ rows }</div>
                    }
                </div>

                if !cvg_svg.is_empty() && !rf_hue_svg.is_empty() {
                    <div style="display: flex; flex-wrap: wrap; gap: 16px; margin-top: 16px;">
                        <div style="min-width: 420px;">
                            <h4 style="margin: 0 0 4px 0;">{ "TM-30 CVG" }</h4>
                            <SvgEmbed svg={cvg_svg.clone()} />
                            <div style="font-size: 0.9em; color: #444; margin-top: 4px;">
                                { tm30_caption.clone() }
                            </div>
                        </div>
                        <div style="min-width: 540px;">
                            <h4 style="margin: 0 0 4px 0;">{ "TM-30 Rf per hue" }</h4>
                            <SvgEmbed svg={rf_hue_svg.clone()} />
                            <div style="font-size: 0.9em; color: #444; margin-top: 4px;">
                                { tm30_caption }
                            </div>
                        </div>
                    </div>
                } else {
                    <div style="color: #888; font-style: italic; margin-top: 8px;">
                        { "TM-30 unavailable (SPD outside the supported wavelength range or too sparse)." }
                    </div>
                }

                // -------- Greenhouse (PPFD heatmap, height-adjustable) --------
                <div style="margin-top: 16px;">
                    <h4 style="margin: 0 0 4px 0;">{ "Greenhouse PPFD" }</h4>
                    <div style="margin: 4px 0 8px 0; font-size: 0.9em;">
                        <label>{ format!("Mounting height: {:.2} m  ", max_height) }</label>
                        <input
                            type="range"
                            min="0.3"
                            max="6.0"
                            step="0.1"
                            value={format!("{:.2}", max_height)}
                            oninput={on_height_input}
                            style="vertical-align: middle; width: 320px;"
                        />
                    </div>
                    if !greenhouse_svg.is_empty() {
                        <SvgEmbed svg={greenhouse_svg.clone()} />
                        <div style="font-size: 0.9em; color: #444; margin-top: 4px;">
                            { format!("PPF {:.0} µmol/s · {:.0} W · {:.2} µmol/J", greenhouse_ppf, greenhouse_watts, greenhouse_eff) }
                        </div>
                    } else {
                        <em style="color: #999;">{ "Greenhouse plot unavailable (SPD not usable)." }</em>
                    }
                </div>
            }
        </div>
    }
}
