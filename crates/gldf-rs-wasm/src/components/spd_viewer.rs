//! Spectral / TM-30 visualization for embedded `<Spectrum>` entries.
//!
//! Renders, for every spectrum in `GeneralDefinitions/Spectrums`:
//!
//! - **SPD curve** — `SpectralDiagram::from_spectral(&spd).to_svg(...)`
//! - **TM-30 CVG** — `Tm30Result::to_svg(...)` (color vector graphic, 16 hue bins)
//! - **TM-30 Rf-hue** — `Tm30Result::rf_hue_svg(...)` (per-hue fidelity bars)
//! - **Colorimetry box** — CCT, Duv, x/y, peak λ, FWHM (from `colorimetry::analyze`)
//! - **Greenhouse PPFD** — `GreenhouseDiagram::to_svg(...)` with a height slider
//!
//! All five are inline SVG strings + a small key/value list, so the bundle
//! impact is zero new deps (eulumdat is already in the workspace).
//!
//! Source order of preference:
//! 1. `Spectrum/SpectrumFileReference` → bytes from `state.embedded_files`,
//!    parsed with `eulumdat::atla::spd_loader::parse`.
//! 2. Inline `<Intensity>` entries (the SOLART pattern) → build
//!    `SpectralDistribution` directly from the wavelength/value pairs.
//!
//! Leptos-specific bits: `SvgEmbed` uses a `NodeRef` + `Effect` to mount the
//! SVG string via `Element::set_inner_html`. Setting `inner_html` on a div
//! through the DOM API preserves the SVG namespace; piping the same string
//! through Leptos's `inner_html` attribute would strip the geometry the same
//! way Yew's `Html::from_html_unchecked` did.

use crate::components::pages::link_hint::LinkHint;
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
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Mount an SVG string in the DOM via `Element::set_inner_html`.
///
/// Why a custom component: the browser's HTML parser, given an SVG string
/// through Leptos's `inner_html` attribute, treats the content as HTML — it
/// keeps `<svg>` and `<text>` (recognized HTML5 elements) but drops the SVG
/// geometry primitives (`<path>`, `<rect>`, `<line>`, `<circle>`). The fix is
/// to hand the string to the live DOM `innerHTML` setter on a real element,
/// which constructs the proper SVG namespace subtree.
///
/// The `svg` prop is a `Signal<String>`, so callers can pass a `RwSignal`,
/// `Memo`, or a derived signal and we re-mount whenever it changes (the
/// greenhouse height slider exercises this path).
#[component]
pub fn SvgEmbed(
    #[prop(into)] svg: Signal<String>,
    #[prop(optional, into)] style: String,
) -> impl IntoView {
    let node_ref = NodeRef::<leptos::html::Div>::new();

    Effect::new(move |_| {
        // Track the signal so any update re-runs this effect.
        let s = svg.get();
        if let Some(div) = node_ref.get() {
            div.set_inner_html(&s);
        }
    });

    view! { <div node_ref=node_ref style=style></div> }
}

/// Top-level Spectra page. Reads the editor state and renders one
/// `<SpectrumPanel>` per `<Spectrum>` entry. Empty-state copy if the loaded
/// GLDF doesn't declare any spectra.
#[component]
pub fn SpdViewer() -> impl IntoView {
    let state = use_gldf();

    view! {
        {move || state.with(|s| {
            let spectrums = s.product.general_definitions.spectrums.as_ref();
            let count = spectrums.map(|sp| sp.spectrum.len()).unwrap_or(0);
            let hint = view! {
                <LinkHint
                    title="Where do I link this spectrum to a light source?".to_string()
                    body="A <Spectrum> only declares the SPD curve (or points at an embedded .spd file). The GLDF schema does not allow a direct light-source link here — the SpectrumReference lives on FixedLightSource / ChangeableLightSource. Attach a spectrum to a lamp on the Light Sources page; the Emitters page then shows the resolved Photometry · LightSource · Spectrum chain.".to_string()
                />
            };
            if count == 0 {
                view! {
                    <div style="padding: 16px; color: #666;">
                        { hint }
                        <h3 style="margin-top: 0;">{ "No Spectra" }</h3>
                        <p>
                            { "This GLDF has no <Spectrum> entries. Add a .spd file in the Builder (once ported) to see SPD + TM-30 + Greenhouse plots here." }
                        </p>
                    </div>
                }.into_any()
            } else {
                let spectra: Vec<Spectrum> = spectrums
                    .map(|sp| sp.spectrum.clone())
                    .unwrap_or_default();
                let panels: Vec<_> = spectra
                    .into_iter()
                    .map(|spectrum| view! { <SpectrumPanel spectrum=spectrum /> })
                    .collect();
                let label = format!(
                    "{} spectrum{} — SPD + Metrics + TM-30 CVG / Rf-hue + Greenhouse PPFD, via eulumdat-rs.",
                    count,
                    if count == 1 { "" } else { "s" },
                );
                view! {
                    <div style="padding: 8px;">
                        { hint }
                        <p style="color: #666; margin-top: 0;">{ label }</p>
                        { panels }
                    </div>
                }.into_any()
            }
        })}
    }
}

/// One panel per `<Spectrum>`. Owns the greenhouse height-slider signal so
/// each spectrum has an independent slider. The panel reads the editor state
/// on demand so a Builder edit that mutates the product or embedded_files
/// re-renders this panel automatically.
#[component]
fn SpectrumPanel(spectrum: Spectrum) -> impl IntoView {
    let state = use_gldf();

    // Height slider — default 2.0 m matches eulumdat's `from_atla(&doc)`
    // (which delegates to `from_atla_with_height(_, 2.0)`).
    let height = RwSignal::new(2.0_f64);

    // Capture the spectrum id once so the closures don't have to clone.
    let spectrum_id = spectrum.id.clone();
    let spectrum_for_resolve = spectrum.clone();

    // Resolve the SPD inline each render (SpectralDistribution doesn't
    // implement PartialEq so we can't memoise it; the cost is small —
    // resolution re-runs only when the state signal changes).
    view! {
        <div style="border: 1px solid #ccc; padding: 12px; margin: 8px 0;">
            <h3 style="margin-top: 0;">{ format!("Spectrum {}", spectrum_id) }</h3>
            {move || {
                let resolved = state.with(|s| resolve_spd(&spectrum_for_resolve, &s.embedded_files));
                match resolved {
                    None => view! {
                        <em style="color: #c00;">
                            { "No SPD source — neither a SpectrumFileReference nor inline <Intensity>." }
                        </em>
                    }.into_any(),
                    Some((spd, source)) => {
                        render_panel_body(spd, source, height).into_any()
                    }
                }
            }}
        </div>
    }
}

/// Render a standalone SPD panel from raw `.spd` bytes. Used by the
/// FileViewer when the user clicks an embedded spectrum file. Parses via
/// `eulumdat::atla::spd_loader::parse` and feeds the result into the same
/// renderer the Spectra page uses, so SPD curve + colorimetry + TM-30 CVG +
/// Rf-hue + Greenhouse PPFD all show up identically.
#[component]
pub fn SpdPanel(bytes: Vec<u8>) -> impl IntoView {
    let height = RwSignal::new(2.0_f64);
    let parsed: Result<(SpectralDistribution, String), String> = {
        let content = match std::str::from_utf8(&bytes) {
            Ok(s) => s,
            Err(e) => return view! {
                <em style="color: #c00;">{ format!("Not UTF-8 .spd content: {}", e) }</em>
            }.into_any(),
        };
        // TM-33 fallback: the GLDF Spectrum may point at a TM-33 XML
        // file (shared with Photometry via the same @fileId — see
        // tm33_to_gldf). Detect by `<` opener + atla::xml::parse and
        // pull the SpectralDistribution out directly.
        let trimmed = content.trim_start_matches('\u{feff}').trim_start();
        if trimmed.starts_with('<') {
            if let Ok(doc) = eulumdat::atla::xml::parse(content) {
                if let Some(spd) = doc
                    .emitters
                    .first()
                    .and_then(|e| e.spectral_distribution.clone())
                {
                    if !spd.wavelengths.is_empty()
                        && spd.wavelengths.len() == spd.values.len()
                    {
                        Ok((spd, "embedded TM-33 XML".to_string()))
                    } else {
                        Err("TM-33 carries no SpectralDistribution".to_string())
                    }
                } else {
                    Err("TM-33 has no <Emitter><SpectralDistribution>".to_string())
                }
            } else {
                Err("Looks like XML but not TM-33".to_string())
            }
        } else {
            match spd_loader::parse(content) {
                Ok(loaded) => Ok((loaded.spd, "embedded .spd".to_string())),
                Err(e) => Err(format!("Parse error: {}", e)),
            }
        }
    };
    match parsed {
        Ok((spd, source)) => render_panel_body(spd, source, height).into_any(),
        Err(e) => view! { <em style="color: #c00;">{ e }</em> }.into_any(),
    }
}

/// Body of one spectrum panel, given a resolved SPD. Builds all four SVGs +
/// captions + the greenhouse slider. Split out so the outer panel can keep
/// the `match` over `Option<(SPD, source)>` simple.
fn render_panel_body(
    spd: SpectralDistribution,
    source: String,
    height: RwSignal<f64>,
) -> impl IntoView {
    let usable = spd.wavelengths.len() >= 2 && spd.wavelengths.len() == spd.values.len();

    let sample_count = spd.wavelengths.len();
    let source_line = format!("source: {} · {} samples", source, sample_count);

    // ---- Static pre-computations (don't depend on the slider) ----

    let spd_svg = if usable {
        SpectralDiagram::from_spectral(&spd).to_svg(720.0, 320.0, &SpectralTheme::light())
    } else {
        String::new()
    };

    let tm30 = if usable { calculate_tm30(&spd) } else { None };
    let colorim = if usable { Some(colorimetry::analyze(&spd)) } else { None };

    let theme = Tm30Theme::light();
    // Strip eulumdat's mis-labeled in-SVG headers; we render correct values
    // in the caption underneath using colorimetry::analyze + Tm30Result.
    let label_prefixes = ["Rf =", "Rg =", "CCT =", "Duv ="];
    let cvg_svg = tm30
        .as_ref()
        .map(|r| strip_text_elements_with_prefix(&r.to_svg(420.0, 420.0, &theme), &label_prefixes))
        .unwrap_or_default();
    let rf_hue_svg = tm30
        .as_ref()
        .map(|r| strip_text_elements_with_prefix(&r.rf_hue_svg(540.0, 280.0, &theme), &label_prefixes))
        .unwrap_or_default();

    let tm30_caption = match (colorim.as_ref(), tm30.as_ref()) {
        (Some(c), Some(r)) => format!(
            "Rf {:.0} · Rg {:.0} · CCT {:.0} K · Duv {:+.4}",
            r.rf, r.rg, c.cct_k, c.duv
        ),
        _ => String::new(),
    };
    let tm30_caption_clone = tm30_caption.clone();

    let metric_rows = colorim.as_ref().map(|c| {
        let tm30_rows = tm30.as_ref().map(|r| view! {
            <tr><th>{ "TM-30 Rf" }</th><td>{ format!("{:.1}", r.rf) }</td></tr>
            <tr><th>{ "TM-30 Rg" }</th><td>{ format!("{:.1}", r.rg) }</td></tr>
        });
        view! {
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

    // ---- Reactive greenhouse (height slider drives re-render) ----

    // Build the greenhouse SVG + numbers each time the slider moves. Cloning
    // the product + SPD into the closure is cheap (Arc-internal for SPD, and
    // GldfProduct is small for the greenhouse path).
    let state = use_gldf();
    let spd_for_greenhouse = spd.clone();
    let greenhouse_inputs = Memo::new(move |_| {
        let max_height = height.get().max(0.2);
        if !usable {
            return (max_height, String::new(), 0.0, 0.0, 0.0);
        }
        let doc = state.with(|s| build_atla_for_greenhouse(&s.product, &spd_for_greenhouse));
        let diagram = GreenhouseDiagram::from_atla_with_height(&doc, max_height);
        let svg = diagram.to_svg(720.0, 360.0, &GreenhouseTheme::light());
        (max_height, svg, diagram.ppf, diagram.efficacy, diagram.watts)
    });

    // Slider input — parses the value and writes it into the signal.
    let on_height_input = move |e: web_sys::Event| {
        let Some(target) = e.target() else { return };
        let input: web_sys::HtmlInputElement = match target.dyn_into() {
            Ok(i) => i,
            Err(_) => return,
        };
        if let Ok(v) = input.value().parse::<f64>() {
            height.set(v);
        }
    };

    let cvg_svg_clone = cvg_svg.clone();
    let rf_hue_svg_clone = rf_hue_svg.clone();
    let spd_svg_clone = spd_svg.clone();
    let has_tm30 = !cvg_svg.is_empty() && !rf_hue_svg.is_empty();

    view! {
        <div style="color: #888; font-size: 0.85em; margin-bottom: 8px;">{ source_line }</div>

        {move || if !usable {
            view! {
                <em style="color: #c00;">{ "Could not build a usable SPD (need ≥2 wavelength/value pairs)." }</em>
            }.into_any()
        } else {
            let cvg_svg = cvg_svg_clone.clone();
            let rf_hue_svg = rf_hue_svg_clone.clone();
            let spd_svg = spd_svg_clone.clone();
            let metric_rows = metric_rows.clone();
            let tm30_caption_a = tm30_caption.clone();
            let tm30_caption_b = tm30_caption_clone.clone();
            view! {
                <div style="display: flex; flex-wrap: wrap; gap: 16px; align-items: flex-start;">
                    <div style="min-width: 480px;">
                        <SvgEmbed svg=spd_svg />
                    </div>
                    {metric_rows.map(|rows| view! {
                        <div style="min-width: 220px;">{ rows }</div>
                    })}
                </div>

                {if has_tm30 {
                    view! {
                        <div style="display: flex; flex-wrap: wrap; gap: 16px; margin-top: 16px;">
                            <div style="min-width: 420px;">
                                <h4 style="margin: 0 0 4px 0;">{ "TM-30 CVG" }</h4>
                                <SvgEmbed svg=cvg_svg />
                                <div style="font-size: 0.9em; color: #444; margin-top: 4px;">
                                    { tm30_caption_a }
                                </div>
                            </div>
                            <div style="min-width: 540px;">
                                <h4 style="margin: 0 0 4px 0;">{ "TM-30 Rf per hue" }</h4>
                                <SvgEmbed svg=rf_hue_svg />
                                <div style="font-size: 0.9em; color: #444; margin-top: 4px;">
                                    { tm30_caption_b }
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div style="color: #888; font-style: italic; margin-top: 8px;">
                            { "TM-30 unavailable (SPD outside the supported wavelength range or too sparse)." }
                        </div>
                    }.into_any()
                }}

                // -------- Greenhouse (PPFD heatmap, height-adjustable) --------
                <div style="margin-top: 16px;">
                    <h4 style="margin: 0 0 4px 0;">{ "Greenhouse PPFD" }</h4>
                    <div style="margin: 4px 0 8px 0; font-size: 0.9em;">
                        <label>{move || format!("Mounting height: {:.2} m  ", greenhouse_inputs.get().0)}</label>
                        <input
                            type="range"
                            min="0.3"
                            max="6.0"
                            step="0.1"
                            prop:value=move || format!("{:.2}", greenhouse_inputs.get().0)
                            on:input=on_height_input
                            style="vertical-align: middle; width: 320px;"
                        />
                    </div>
                    {move || {
                        let (_, svg, ppf, eff, watts) = greenhouse_inputs.get();
                        if svg.is_empty() {
                            view! {
                                <em style="color: #999;">{ "Greenhouse plot unavailable (SPD not usable)." }</em>
                            }.into_any()
                        } else {
                            let svg_signal = Signal::derive(move || greenhouse_inputs.get().1);
                            view! {
                                <SvgEmbed svg=svg_signal />
                                <div style="font-size: 0.9em; color: #444; margin-top: 4px;">
                                    { format!("PPF {:.0} µmol/s · {:.0} W · {:.2} µmol/J", ppf, watts, eff) }
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            }.into_any()
        }}
    }
}

// ---- Framework-agnostic helpers (copied verbatim from the Yew commit) ----

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
        out.push_str(&rest[..start]);
        let after_start = &rest[start..];
        let end_rel = match after_start.find("</text>") {
            Some(idx) => idx + "</text>".len(),
            None => {
                out.push_str(after_start);
                return out;
            }
        };
        let element = &after_start[..end_rel];
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
    // (a) File reference — load bytes. Three parsers in order:
    //   (a1) TM-33 XML: file may carry TM-33 (content_type ldc/iesxml)
    //        where the SpectralDistribution lives under the Emitter.
    //        Shared with Photometry via the same @fileId — see
    //        gldf_rs::convert::tm33_to_gldf.
    //   (a2) Generic SPD via spd_loader::parse (Luxeon `.spd`,
    //        Signify CSV, two-column whitespace, etc.).
    if let Some(file_ref) = spectrum.spectrum_file_reference.as_ref() {
        if let Some(bytes) = embedded_files.get(&file_ref.file_id) {
            let content = std::str::from_utf8(bytes).ok()?;
            let trimmed = content.trim_start_matches('\u{feff}').trim_start();
            if trimmed.starts_with('<') {
                if let Ok(doc) = eulumdat::atla::xml::parse(content) {
                    if let Some(spd) = doc
                        .emitters
                        .first()
                        .and_then(|e| e.spectral_distribution.clone())
                    {
                        if !spd.wavelengths.is_empty()
                            && spd.wavelengths.len() == spd.values.len()
                        {
                            return Some((
                                spd,
                                format!("TM-33 XML in file {}", file_ref.file_id),
                            ));
                        }
                    }
                    return Some((
                        SpectralDistribution::default(),
                        format!(
                            "file {} parses as TM-33 but carries no <SpectralDistribution>",
                            file_ref.file_id
                        ),
                    ));
                }
                // XML but not TM-33 — fall through to spd_loader.
            }
            match spd_loader::parse(content) {
                Ok(loaded) => return Some((loaded.spd, format!("file {}", file_ref.file_id))),
                Err(e) => {
                    return Some((SpectralDistribution::default(), format!("parse error: {}", e)));
                }
            }
        }
        return Some((
            SpectralDistribution::default(),
            format!("file id {} not in embedded_files", file_ref.file_id),
        ));
    }

    // (b) Inline <Intensity wavelength="..."> entries.
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
fn build_atla_for_greenhouse(
    product: &GldfProduct,
    spd: &SpectralDistribution,
) -> LuminaireOpticalData {
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
