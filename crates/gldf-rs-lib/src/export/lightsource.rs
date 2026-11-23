//! Per-LightSource exports — LDT / IES / TM-33 XML / IFC.
//!
//! Each function takes `(product, embedded_files, ls_id)` and resolves
//! the chain: `LightSource → FixedLightEmitter → PhotometryReference →
//! File`, fetches the LDT bytes from `embedded_files`, parses them, then
//! re-emits in the requested format. LightSource metadata
//! (manufacturer, CCT, rated input power, spectrum) is overlaid where
//! the target format has a slot for it.
//!
//! **TM-33 is the headline format**: it combines photometry AND the SPD
//! spectrum into one XML file (via `LuminaireOpticalData.emitters[0]
//! .spectral_distribution`). When the GLDF's `<Spectrum>` block has
//! inline `<Intensity wavelength="…">…</Intensity>` pairs, they land
//! directly in the TM-33 SpectralDistribution — no separate `.spd` file
//! shipped alongside.
//!
//! Requires the `eulumdat` feature.

use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use eulumdat::atla::{
    self,
    types::{Emitter as AtlaEmitter, Header as AtlaHeader, LuminaireOpticalData, SchemaVersion},
};
use eulumdat::Eulumdat;

use crate::gldf::general_definitions::lightsources::{
    ChangeableLightSource, FixedLightSource, MultiChannelLightSource,
};
use crate::gldf::general_definitions::photometries::Spectrum;
use crate::gldf::general_definitions::{File, GeneralDefinitions};
use crate::gldf::GldfProduct;

/// Resolution shape used by every per-LS export function.
///
/// Walks the GLDF reference chain once and hands the callers the LDT
/// content + the overlay metadata they need to amend it. Returned by
/// [`resolve_lightsource_export_input`].
struct LightSourceExportInput<'a> {
    /// The LightSource id being exported (back-reference for naming).
    ls_id: String,
    /// Manufacturer string for the header / TM-33 Header.
    manufacturer: Option<String>,
    /// Optional CCT (Kelvin) — overlay onto `Eulumdat.lamp_sets[0]
    /// .color_appearance` and `AtlaEmitter.cct`.
    cct: Option<i32>,
    /// Optional CRI — TM-33 `ColorRendering.ra`.
    cri: Option<i32>,
    /// Optional rated input power (W) — overlay onto
    /// `Eulumdat.lamp_sets[0].wattage_with_ballast` and TM-33
    /// `Emitter.input_watts`.
    rated_input_power: Option<f64>,
    /// Bytes of the LDT/IES file referenced by the bound Photometry.
    ldt_bytes: Vec<u8>,
    /// Whether `ldt_bytes` is IES LM-63 (true) or EULUMDAT (false).
    /// Decided from the GLDF `<File>` content_type (`ldc/ies` vs
    /// `ldc/eulumdat`) with a filename-extension fallback for files
    /// where the content_type was sniffed wrong on import.
    is_ies: bool,
    /// Optional inline GLDF Spectrum — when present its
    /// `<Intensity wavelength="…">…</Intensity>` pairs land in TM-33's
    /// `SpectralDistribution.{wavelengths,values}`.
    spectrum: Option<&'a Spectrum>,
    /// Bytes of the `.spd` file referenced by `spectrum` (if the
    /// spectrum uses a `SpectrumFileReference` rather than inline
    /// `<Intensity>` pairs). Parsed by `eulumdat::atla::spd_loader` in
    /// `overlay_emitter` so file-referenced SPDs still land in TM-33
    /// — the dropped-`.spd` editor flow registers spectra this way,
    /// not inline.
    spectrum_bytes: Option<Vec<u8>>,
}

/// Public surface — accepts the same input shape as the other `export_*`
/// functions and returns the merged [`Eulumdat`] for callers that want
/// to inspect / further mutate before serializing.
///
/// Same chain walk as [`export_ldt`] / [`export_ies`] but stops one step
/// short so the caller can patch fields the per-format wrappers don't
/// touch (e.g. tilt angle, custom luminaire dimensions).
pub fn build_eulumdat_for_lightsource(
    product: &GldfProduct,
    embedded_files: &HashMap<String, Vec<u8>>,
    ls_id: &str,
) -> Result<Eulumdat> {
    let input = resolve_lightsource_export_input(product, embedded_files, ls_id)?;
    parse_and_overlay_eulumdat(&input)
}

/// Serialize the per-LS [`Eulumdat`] as an EULUMDAT (`.ldt`) string.
///
/// Industry-standard European photometric format. Lossy compared with
/// TM-33 — drops the SPD spectrum and most TM-33 metadata.
pub fn export_ldt(
    product: &GldfProduct,
    embedded_files: &HashMap<String, Vec<u8>>,
    ls_id: &str,
) -> Result<String> {
    let ldt = build_eulumdat_for_lightsource(product, embedded_files, ls_id)?;
    Ok(ldt.to_ldt())
}

/// Serialize the per-LS [`Eulumdat`] as an IES LM-63 (`.ies`) string.
///
/// Industry-standard North-American photometric format. Same shape as
/// LDT — angular intensity matrix + minimal header — without spectrum.
pub fn export_ies(
    product: &GldfProduct,
    embedded_files: &HashMap<String, Vec<u8>>,
    ls_id: &str,
) -> Result<String> {
    let ldt = build_eulumdat_for_lightsource(product, embedded_files, ls_id)?;
    Ok(eulumdat::IesExporter::export(&ldt))
}

/// Serialize the per-LS data as TM-33-23 XML — **photometry + SPD
/// spectrum in one file**.
///
/// This is the differentiator of the per-LS export menu. The XML
/// document carries the LDT-derived angular intensity matrix AND the
/// GLDF Spectrum's `<Intensity wavelength>` pairs in
/// `Emitter.spectral_distribution`. Downstream consumers (DIALux,
/// AGi32, Radiance forks) that read TM-33 get both halves at once.
pub fn export_tm33_xml(
    product: &GldfProduct,
    embedded_files: &HashMap<String, Vec<u8>>,
    ls_id: &str,
) -> Result<String> {
    let input = resolve_lightsource_export_input(product, embedded_files, ls_id)?;
    let ldt = parse_and_overlay_eulumdat(&input)?;
    let mut doc: LuminaireOpticalData = (&ldt).into();

    // Force TM-33-23 schema — the GLDF→TM-33 conversion defaults to
    // AtlaS001 (its base), but TM-33-23 is the spec that combines
    // photometry + SPD natively, and that's the headline of this menu.
    doc.schema_version = SchemaVersion::Tm3323;
    doc.version = SchemaVersion::Tm3323.version_string().to_string();

    overlay_header(&mut doc.header, &input);
    overlay_emitter(&mut doc.emitters, &input);

    atla::xml::write(&doc).context("serialize TM-33 XML")
}

/// Export the LightSource as a minimal IFC fragment.
///
/// Builds a stub `GldfProduct` containing just this LS and its bound
/// Photometry + Spectrum + one synthetic Variant pointing at a single
/// Emitter, then delegates to the existing product-level
/// [`crate::ifc::GldfToIfc::export`]. The result is one
/// `IFCLIGHTFIXTURETYPE` carrying the LS's photometry — suitable for
/// dropping into a BIM scene as a stand-alone luminaire definition.
pub fn export_ifc_fragment(
    product: &GldfProduct,
    embedded_files: &HashMap<String, Vec<u8>>,
    ls_id: &str,
) -> Result<String> {
    let _ = embedded_files;
    let stub = build_ifc_stub_product(product, ls_id)?;
    crate::ifc::GldfToIfc::export(&stub).context("export per-LS IFC fragment")
}

// ============================================================================
// Internal helpers
// ============================================================================

fn resolve_lightsource_export_input<'a>(
    product: &'a GldfProduct,
    embedded_files: &HashMap<String, Vec<u8>>,
    ls_id: &str,
) -> Result<LightSourceExportInput<'a>> {
    let general = &product.general_definitions;

    // 1. Locate the LightSource (Fixed / Changeable / MultiChannel) and
    //    pull the metadata we want to overlay on the photometry export.
    let (manufacturer, cct, cri, rated_input_power, spectrum_id) =
        resolve_lightsource_metadata(general, ls_id)
            .ok_or_else(|| anyhow!("LightSource not found: {ls_id}"))?;

    // 2. Walk Emitters to find the FixedLightEmitter whose
    //    light_source_reference points at this LS. Use the first
    //    matching binding — most LSs are bound by one emitter; for
    //    multi-emitter shared-LS cases this picks the first arbitrarily.
    let photometry_id = find_photometry_for_lightsource(general, ls_id)
        .ok_or_else(|| anyhow!("No emitter binds LightSource '{ls_id}' to a Photometry"))?;

    // 3. Look up the Photometry entry and its referenced File.
    let photometry = general
        .photometries
        .as_ref()
        .and_then(|ps| ps.photometry.iter().find(|p| p.id == photometry_id))
        .ok_or_else(|| anyhow!("Photometry '{photometry_id}' not declared"))?;

    let file_id = photometry
        .photometry_file_reference
        .as_ref()
        .map(|r| r.file_id.clone())
        .ok_or_else(|| anyhow!("Photometry '{photometry_id}' has no PhotometryFileReference"))?;

    // 4. Fetch the LDT/IES bytes. We accept either keying convention —
    //    EditableGldf uses file_id, the raw-zip path is the secondary
    //    fallback (matches how load_gldf_from_buf seeds the map).
    let ldt_bytes = fetch_file_bytes(embedded_files, general, &file_id)
        .ok_or_else(|| anyhow!("File '{file_id}' not present in embedded_files"))?;

    // Decide LDT vs IES from the GLDF File entry. content_type is the
    // authoritative signal (`ldc/ies` vs `ldc/eulumdat`); filename
    // extension is the fallback for files where the content_type was
    // sniffed wrong on import. The header sniff (content starts with
    // `IESNA`) is the third-tier fallback for genuinely untagged bytes.
    let is_ies = general
        .files
        .file
        .iter()
        .find(|f| f.id == file_id)
        .map(|f| {
            f.content_type.eq_ignore_ascii_case("ldc/ies")
                || f.file_name.to_lowercase().ends_with(".ies")
        })
        .unwrap_or(false)
        || std::str::from_utf8(&ldt_bytes)
            .map(|s| s.trim_start().starts_with("IESNA"))
            .unwrap_or(false);

    let spectrum = spectrum_id.and_then(|sid| {
        general
            .spectrums
            .as_ref()
            .and_then(|ss| ss.spectrum.iter().find(|s| s.id == sid))
    });
    // When the spectrum carries a SpectrumFileReference rather than
    // inline <Intensity> pairs (the dropped-`.spd` editor flow), pull
    // the file bytes so overlay_emitter can parse them via
    // `eulumdat::atla::spd_loader::parse`. Without this the TM-33
    // output skips the SPD block whenever the SPD lives as a separate
    // file — which is the common case for editor-built fixtures.
    let spectrum_bytes = spectrum
        .and_then(|s| s.spectrum_file_reference.as_ref())
        .and_then(|r| fetch_file_bytes(embedded_files, general, &r.file_id));

    Ok(LightSourceExportInput {
        ls_id: ls_id.to_string(),
        manufacturer,
        cct,
        cri,
        rated_input_power,
        ldt_bytes,
        is_ies,
        spectrum,
        spectrum_bytes,
    })
}

/// (manufacturer, CCT, CRI, rated input power, spectrum id) of a light source.
type LightSourceMetadata = (
    Option<String>,
    Option<i32>,
    Option<i32>,
    Option<f64>,
    Option<String>,
);

fn resolve_lightsource_metadata(
    general: &GeneralDefinitions,
    ls_id: &str,
) -> Option<LightSourceMetadata> {
    let lss = general.light_sources.as_ref()?;
    if let Some(ls) = lss.fixed_light_source.iter().find(|s| s.id == ls_id) {
        return Some(metadata_from_fixed(ls));
    }
    if let Some(ls) = lss.changeable_light_source.iter().find(|s| s.id == ls_id) {
        return Some(metadata_from_changeable(ls));
    }
    if let Some(ls) = lss
        .multi_channel_light_source
        .iter()
        .find(|s| s.id == ls_id)
    {
        return Some(metadata_from_multi_channel(ls));
    }
    None
}

fn metadata_from_fixed(ls: &FixedLightSource) -> LightSourceMetadata {
    let (cct, cri) = ls
        .color_information
        .as_ref()
        .map(|ci| (ci.correlated_color_temperature, ci.color_rendering_index))
        .unwrap_or((None, None));
    (
        ls.manufacturer.clone(),
        cct,
        cri,
        ls.rated_input_power,
        ls.spectrum_reference
            .as_ref()
            .map(|r| r.spectrum_id.clone()),
    )
}

fn metadata_from_changeable(ls: &ChangeableLightSource) -> LightSourceMetadata {
    let (cct, cri) = ls
        .color_information
        .as_ref()
        .map(|ci| (ci.correlated_color_temperature, ci.color_rendering_index))
        .unwrap_or((None, None));
    (
        ls.manufacturer.clone(),
        cct,
        cri,
        ls.rated_input_power,
        ls.spectrum_reference
            .as_ref()
            .map(|r| r.spectrum_id.clone()),
    )
}

fn metadata_from_multi_channel(ls: &MultiChannelLightSource) -> LightSourceMetadata {
    // MultiChannel has per-channel spectra; surfacing channel 0's
    // spectrum is the pragmatic default. Per-channel exports are a
    // future extension (see the export plan).
    let spectrum_id = ls
        .channels
        .channel
        .first()
        .map(|c| c.spectrum_reference.spectrum_id.clone());
    (
        ls.manufacturer.clone(),
        None,
        None,
        Some(ls.rated_input_power),
        spectrum_id,
    )
}

fn find_photometry_for_lightsource(general: &GeneralDefinitions, ls_id: &str) -> Option<String> {
    // FixedLightSource: the binding lives on FixedLightEmitter — walk
    // Emitters and match the light_source_reference. This is the most
    // direct case.
    if let Some(emitters) = general.emitters.as_ref() {
        for em in &emitters.emitter {
            for fle in &em.fixed_light_emitter {
                if fle.light_source_reference.fixed_light_source_id.as_deref() == Some(ls_id) {
                    return Some(fle.photometry_reference.photometry_id.clone());
                }
                if fle
                    .light_source_reference
                    .changeable_light_source_id
                    .as_deref()
                    == Some(ls_id)
                {
                    return Some(fle.photometry_reference.photometry_id.clone());
                }
            }
        }
    }

    // ChangeableLightSource: the XSD attaches `<PhotometryReference>`
    // directly to the LightSource (no emitter binding required — a
    // replaceable bulb ships its own photometry, the luminaire's
    // ChangeableLightEmitter just picks a slot for it). Use that
    // direct reference when present.
    if let Some(lss) = general.light_sources.as_ref() {
        if let Some(cls) = lss.changeable_light_source.iter().find(|s| s.id == ls_id) {
            if let Some(p) = cls.photometry_reference.as_ref() {
                return Some(p.photometry_id.clone());
            }
        }
    }
    None
}

fn fetch_file_bytes(
    embedded_files: &HashMap<String, Vec<u8>>,
    general: &GeneralDefinitions,
    file_id: &str,
) -> Option<Vec<u8>> {
    if let Some(bytes) = embedded_files.get(file_id) {
        return Some(bytes.clone());
    }
    // Secondary lookup: load_gldf_from_buf_all keys files by zip path
    // (e.g. "ldc/luminaire.ldt"). Map file_id → File → zip path.
    let f: &File = general.files.file.iter().find(|f| f.id == file_id)?;
    let zip_path =
        crate::editable::EditableGldf::get_zip_path_for_file(&f.content_type, &f.file_name);
    embedded_files.get(&zip_path).cloned()
}

fn parse_and_overlay_eulumdat(input: &LightSourceExportInput<'_>) -> Result<Eulumdat> {
    // The LDT/IES specs mandate ASCII / Latin-1; we accept UTF-8 too
    // because the WASM editor sometimes round-trips through String.
    // Lossy decoding is safe — only header strings ever carry non-ASCII.
    let content = match std::str::from_utf8(&input.ldt_bytes) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(&input.ldt_bytes).into_owned(),
    };
    // Pick the parser by format:
    //   * TM-33 XML — the bytes start with `<` (after BOM/whitespace
    //     strip); parse via atla::xml then convert to Eulumdat. This
    //     covers the "GLDF bootstrapped from TM-33" case where the
    //     ldc/iesxml File entry holds TM-33 content.
    //   * IES → IesParser::parse.
    //   * Default → Eulumdat::parse (LDT).
    let trimmed = content.trim_start_matches('\u{feff}').trim_start();
    let mut ldt = if trimmed.starts_with('<') {
        let doc = eulumdat::atla::xml::parse(&content)
            .map_err(|e| anyhow!("parse TM-33 XML for LightSource '{}': {e}", input.ls_id))?;
        (&doc).into()
    } else if input.is_ies {
        eulumdat::IesParser::parse(&content)
            .map_err(|e| anyhow!("parse IES for LightSource '{}': {e}", input.ls_id))?
    } else {
        Eulumdat::parse(&content)
            .map_err(|e| anyhow!("parse LDT for LightSource '{}': {e}", input.ls_id))?
    };

    // Overlay LS metadata onto the LDT header / first lamp set.
    if let Some(m) = &input.manufacturer {
        // EULUMDAT line 1 is "company identification"; many writers
        // stuff manufacturer + product into it. Only overwrite if the
        // source LDT left it blank — otherwise we'd erase richer data.
        if ldt.identification.trim().is_empty() {
            ldt.identification = m.clone();
        }
    }
    if let Some(set) = ldt.lamp_sets.first_mut() {
        if let Some(w) = input.rated_input_power {
            set.wattage_with_ballast = w;
        }
        if let Some(cct) = input.cct {
            // LDT's color_appearance is a free-text string; "4000 K" is
            // the conventional form so downstream parsers can pick it
            // up. Only overwrite when the LS supplies a value.
            set.color_appearance = format!("{cct} K");
        }
    }

    Ok(ldt)
}

fn overlay_header(header: &mut AtlaHeader, input: &LightSourceExportInput<'_>) {
    if header.manufacturer.is_none() {
        header.manufacturer = input.manufacturer.clone();
    }
    if header.description.is_none() {
        header.description = Some(format!("Per-LightSource export: {}", input.ls_id));
    }
}

fn overlay_emitter(emitters: &mut Vec<AtlaEmitter>, input: &LightSourceExportInput<'_>) {
    // From<&Eulumdat> for LuminaireOpticalData always produces at
    // least one emitter; defensively push one if it didn't.
    if emitters.is_empty() {
        emitters.push(AtlaEmitter::default());
    }
    let em = &mut emitters[0];
    em.id.get_or_insert_with(|| input.ls_id.clone());
    em.description
        .get_or_insert_with(|| format!("LightSource {}", input.ls_id));
    if let Some(w) = input.rated_input_power {
        em.input_watts = Some(w);
    }
    if let Some(cct) = input.cct {
        em.cct = Some(cct as f64);
    }
    if let Some(cri) = input.cri {
        let mut cr = em.color_rendering.clone().unwrap_or_default();
        cr.ra = Some(cri as f64);
        em.color_rendering = Some(cr);
    }
    // Spectrum overlay — the headline of TM-33: photometry + SPD in
    // ONE file. Two source paths:
    //
    //  (a) Inline GLDF <Spectrum> with <Intensity wavelength="…">
    //      pairs — map straight to SpectralDistribution.
    //  (b) <SpectrumFileReference> pointing at an embedded `.spd` —
    //      parse the file bytes via `eulumdat::atla::spd_loader::parse`
    //      and take its `LoadedSpd.spd` (already a SpectralDistribution).
    //
    // The editor's drag-and-drop flow takes path (b) almost exclusively
    // (ImportSpectrumFile registers a SpectrumFileReference, not inline
    // intensities), so without the file fallback TM-33 exports from
    // editor-built GLDFs would silently ship without the SPD half.
    if let Some(sp) = input.spectrum {
        let from_inline: Option<(Vec<f64>, Vec<f64>)> = {
            let (w, v): (Vec<f64>, Vec<f64>) = sp
                .intensity
                .iter()
                .filter_map(|i| Some((i.wavelength? as f64, i.value?)))
                .unzip();
            if w.is_empty() {
                None
            } else {
                Some((w, v))
            }
        };
        if let Some((wavelengths, values)) = from_inline {
            em.spectral_distribution = Some(eulumdat::atla::types::SpectralDistribution {
                wavelengths,
                values,
                units: Default::default(),
                start_wavelength: None,
                wavelength_interval: None,
            });
        } else if let Some(bytes) = input.spectrum_bytes.as_ref() {
            // Lossy UTF-8: headers are ASCII; this only matters for
            // rare comment lines with non-ASCII characters.
            let text = String::from_utf8_lossy(bytes);
            let trimmed = text.trim_start_matches('\u{feff}').trim_start();
            // Two file shapes can land here:
            //   * `.spd` (Luxeon / two-column / Signify CSV) — feed
            //     to spd_loader::parse, take loaded.spd.
            //   * TM-33 XML — the bytes start with `<`. Parse via
            //     atla::xml and pull spectral_distribution directly.
            //     This is the round-trip path for a GLDF that was
            //     bootstrapped from TM-33: Photometry and Spectrum
            //     both reference the same ldc/iesxml file, so the
            //     SPD lives inside the SAME bytes the photometry
            //     parser just consumed.
            if trimmed.starts_with('<') {
                if let Ok(doc) = eulumdat::atla::xml::parse(&text) {
                    if let Some(spd) = doc
                        .emitters
                        .first()
                        .and_then(|e| e.spectral_distribution.clone())
                    {
                        if !spd.wavelengths.is_empty() {
                            em.spectral_distribution = Some(spd);
                        }
                    }
                }
            } else if let Ok(loaded) = eulumdat::atla::spd_loader::parse(&text) {
                if !loaded.spd.wavelengths.is_empty() {
                    em.spectral_distribution = Some(loaded.spd);
                }
            }
        }
    }
}

fn build_ifc_stub_product(product: &GldfProduct, ls_id: &str) -> Result<GldfProduct> {
    // Walk the chain once to pick out the photometry/spectrum/emitter
    // wired to this LS. Then clone the source product and trim
    // everything that doesn't belong.
    let general = &product.general_definitions;
    let photometry_id = find_photometry_for_lightsource(general, ls_id)
        .ok_or_else(|| anyhow!("No emitter binds LightSource '{ls_id}'"))?;
    let emitter_id = general
        .emitters
        .as_ref()
        .and_then(|es| {
            es.emitter
                .iter()
                .find(|e| {
                    e.fixed_light_emitter.iter().any(|fle| {
                        fle.light_source_reference.fixed_light_source_id.as_deref() == Some(ls_id)
                            || fle
                                .light_source_reference
                                .changeable_light_source_id
                                .as_deref()
                                == Some(ls_id)
                    })
                })
                .map(|e| e.id.clone())
        })
        .ok_or_else(|| anyhow!("Emitter binding for LightSource '{ls_id}' not found"))?;

    let mut stub = product.clone();
    let g = &mut stub.general_definitions;

    // Trim LightSources to just this one (keep the *_light_source
    // collection that contains it).
    if let Some(lss) = g.light_sources.as_mut() {
        lss.fixed_light_source.retain(|s| s.id == ls_id);
        lss.changeable_light_source.retain(|s| s.id == ls_id);
        lss.multi_channel_light_source.retain(|s| s.id == ls_id);
    }

    // Trim Photometries / Spectrums / Emitters / Files to the chain.
    if let Some(ps) = g.photometries.as_mut() {
        ps.photometry.retain(|p| p.id == photometry_id);
    }
    let keep_spectrum: Option<String> = g.light_sources.as_ref().and_then(|lss| {
        lss.fixed_light_source
            .iter()
            .find_map(|s| s.spectrum_reference.as_ref().map(|r| r.spectrum_id.clone()))
            .or_else(|| {
                lss.changeable_light_source
                    .iter()
                    .find_map(|s| s.spectrum_reference.as_ref().map(|r| r.spectrum_id.clone()))
            })
    });
    if let Some(ss) = g.spectrums.as_mut() {
        if let Some(sid) = keep_spectrum.as_ref() {
            ss.spectrum.retain(|s| &s.id == sid);
        } else {
            ss.spectrum.clear();
        }
    }
    if let Some(es) = g.emitters.as_mut() {
        es.emitter.retain(|e| e.id == emitter_id);
    }
    // Keep only the LDT file referenced by the kept Photometry (plus
    // any product images — those are harmless and the IFC exporter
    // doesn't require them).
    let keep_file_id: Option<String> = g
        .photometries
        .as_ref()
        .and_then(|ps| ps.photometry.first())
        .and_then(|p| {
            p.photometry_file_reference
                .as_ref()
                .map(|r| r.file_id.clone())
        });
    if let Some(fid) = keep_file_id {
        g.files
            .file
            .retain(|f| f.id == fid || f.content_type.starts_with("image"));
    }

    // Replace Variants with one synthetic variant pointing at the
    // kept Emitter via a bare EmitterReference. The IFC exporter
    // emits one IFCLIGHTFIXTURETYPE per variant — exactly the shape
    // we want for a per-LS fragment.
    use crate::gldf::product_definitions::{
        EmitterReferenceBare, Geometry as ProductGeometry, Variant, Variants,
    };
    let synthetic = Variant {
        id: format!("variant_{ls_id}"),
        sort_order: Some(1),
        geometry: Some(ProductGeometry {
            emitter_reference: vec![EmitterReferenceBare {
                emitter_id: emitter_id.clone(),
            }],
            simple_geometry_reference: None,
            model_geometry_reference: None,
        }),
        ..Default::default()
    };
    stub.product_definitions.variants = Some(Variants {
        variant: vec![synthetic],
    });

    Ok(stub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GldfProduct;

    fn load_fixture(path: &str) -> (GldfProduct, HashMap<String, Vec<u8>>) {
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        let bundle = GldfProduct::load_gldf_from_buf_all(bytes).expect("load_gldf_from_buf_all");
        // Mirror the WASM EditableGldf path: index embedded files by
        // file id (the export module's primary lookup key) plus fall
        // back to zip path.
        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        for bf in &bundle.files {
            if let (Some(id), Some(content)) = (bf.file_id.as_ref(), bf.content.as_ref()) {
                files.insert(id.clone(), content.clone());
            }
            if let (Some(path), Some(content)) = (bf.path.as_ref(), bf.content.as_ref()) {
                files.insert(path.clone(), content.clone());
            }
        }
        (bundle.gldf, files)
    }

    fn first_fixed_ls_id(p: &GldfProduct) -> String {
        p.general_definitions
            .light_sources
            .as_ref()
            .and_then(|lss| lss.fixed_light_source.first())
            .map(|s| s.id.clone())
            .expect("fixture has at least one FixedLightSource")
    }

    /// Sanity: LDT export round-trips through `Eulumdat::parse`. Proves
    /// the writer emits a valid LDT and that LS metadata overlay didn't
    /// corrupt the grid.
    #[test]
    fn slv_tria_2_ldt_round_trips() {
        let path = "../../tests/data/SLV - Tria 2.gldf";
        if std::fs::metadata(path).is_err() {
            eprintln!("skipping: fixture {path} missing");
            return;
        }
        let (product, files) = load_fixture(path);
        let ls_id = first_fixed_ls_id(&product);

        let ldt = export_ldt(&product, &files, &ls_id).expect("export_ldt");
        assert!(!ldt.is_empty(), "LDT empty");
        let parsed = eulumdat::Eulumdat::parse(&ldt).expect("parse exported LDT");
        assert!(!parsed.intensities.is_empty(), "no intensity matrix");
    }

    /// TM-33 sanity: the XML must mention both the photometric matrix
    /// AND (for a fixture with a `<Spectrum>`) wavelength data.
    /// SLV Tria 2 has no spectrum, so only the photometric half is
    /// asserted here — the spectral fixture test below covers SPD.
    #[test]
    fn slv_tria_2_tm33_has_photometry() {
        let path = "../../tests/data/SLV - Tria 2.gldf";
        if std::fs::metadata(path).is_err() {
            eprintln!("skipping: fixture {path} missing");
            return;
        }
        let (product, files) = load_fixture(path);
        let ls_id = first_fixed_ls_id(&product);

        let xml = export_tm33_xml(&product, &files, &ls_id).expect("export_tm33_xml");
        assert!(xml.contains("IESTM33-22"), "missing TM-33 root: {xml}");
        // TM-33-23 wraps the photometric matrix in <LuminousData>; the
        // older ATLA-S001 schema uses <IntensityDistribution>. Accept
        // either so this isn't pinned to a single schema version.
        assert!(
            xml.contains("LuminousData")
                || xml.contains("IntensityDistribution")
                || xml.contains("Candela"),
            "no photometric block in TM-33 output"
        );
    }

    /// The headline feature: when the source GLDF carries an inline
    /// `<Spectrum>` block, the TM-33 export embeds the SPD as
    /// `SpectralDistribution` so consumers get photometry + spectrum
    /// in ONE file.
    ///
    /// The on-disk spectral fixture binds LSs via the
    /// `ChangeableLightEmitter > EquipmentReference > Equipment >
    /// LightSourceReference` chain, which the export resolver doesn't
    /// walk yet (the `ChangeableLightEmitter` struct doesn't model
    /// `EquipmentReference`). To still exercise the SPD-embedding
    /// codepath, we synthesize a minimal in-memory product wiring a
    /// FixedLightSource + FixedLightEmitter + Photometry + Spectrum,
    /// and assert the SPD lands in the TM-33 output.
    #[test]
    fn synthetic_fixed_ls_tm33_includes_spd() {
        use crate::gldf::general_definitions::lightsources::{
            Emitter, Emitters, FixedLightEmitter, FixedLightSource, LightSourceReference,
            LightSources, PhotometryReference, SpectrumReference,
        };
        use crate::gldf::general_definitions::photometries::{
            Intensity, Photometries, Photometry, PhotometryFileReference, Spectrum, Spectrums,
        };
        use crate::gldf::general_definitions::{File, Files};

        // Borrow a real LDT from the SLV fixture so Eulumdat::parse
        // accepts it. We don't ship a standalone LDT for tests.
        let ldt = match std::fs::read("../../tests/data/SLV - Tria 2.gldf") {
            Ok(bytes) => match GldfProduct::load_gldf_from_buf_all(bytes) {
                Ok(b) => b
                    .files
                    .into_iter()
                    .find(|bf| {
                        bf.path
                            .as_deref()
                            .map(|p| p.ends_with(".ldt"))
                            .unwrap_or(false)
                    })
                    .and_then(|bf| bf.content),
                Err(_) => None,
            },
            Err(_) => None,
        };
        let Some(ldt_bytes) = ldt else {
            eprintln!("skipping: no SLV LDT available to seed synthetic fixture");
            return;
        };

        let mut product = GldfProduct::default();
        product.general_definitions.files = Files {
            file: vec![File {
                id: "ldt1".to_string(),
                content_type: "ldc/eulumdat".to_string(),
                type_attr: "localFileName".to_string(),
                language: String::new(),
                file_name: "test.ldt".to_string(),
            }],
        };
        product.general_definitions.photometries = Some(Photometries {
            photometry: vec![Photometry {
                id: "p1".to_string(),
                photometry_file_reference: Some(PhotometryFileReference {
                    file_id: "ldt1".to_string(),
                }),
                descriptive_photometry: None,
            }],
        });
        product.general_definitions.spectrums = Some(Spectrums {
            spectrum: vec![Spectrum {
                id: "spec1".to_string(),
                spectrum_file_reference: None,
                intensity: vec![
                    Intensity {
                        wavelength: Some(380),
                        value: Some(0.01),
                    },
                    Intensity {
                        wavelength: Some(550),
                        value: Some(1.0),
                    },
                    Intensity {
                        wavelength: Some(780),
                        value: Some(0.02),
                    },
                ],
            }],
        });
        product.general_definitions.light_sources = Some(LightSources {
            fixed_light_source: vec![FixedLightSource {
                id: "ls1".to_string(),
                manufacturer: Some("Synthetic".to_string()),
                rated_input_power: Some(30.0),
                spectrum_reference: Some(SpectrumReference {
                    spectrum_id: "spec1".to_string(),
                }),
                ..Default::default()
            }],
            changeable_light_source: vec![],
            multi_channel_light_source: vec![],
        });
        product.general_definitions.emitters = Some(Emitters {
            emitter: vec![Emitter {
                id: "em1".to_string(),
                fixed_light_emitter: vec![FixedLightEmitter {
                    photometry_reference: PhotometryReference {
                        photometry_id: "p1".to_string(),
                    },
                    light_source_reference: LightSourceReference {
                        fixed_light_source_id: Some("ls1".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                }],
                ..Default::default()
            }],
        });

        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert("ldt1".to_string(), ldt_bytes);

        let xml = export_tm33_xml(&product, &files, "ls1").expect("export_tm33_xml");
        // SPD presence — atla::xml emits Wavelength / SpectralDistribution.
        assert!(
            xml.contains("Wavelength") || xml.contains("SpectralDistribution"),
            "TM-33 output lacks SPD block: {xml}"
        );
    }

    /// Per-LS export must accept IES photometry, not just LDT. The
    /// Photometry's `<File>` is tagged `ldc/ies` on import, so the
    /// resolver flips `is_ies` and `parse_and_overlay_eulumdat` calls
    /// `IesParser::parse` instead of `Eulumdat::parse`. Without this
    /// branch, GLDFs whose photometry ships as `.ies` (the dominant
    /// North-American convention) failed every per-LS export format.
    #[test]
    fn ies_photometry_per_ls_export_works() {
        use crate::gldf::general_definitions::lightsources::{
            Emitter, Emitters, FixedLightEmitter, FixedLightSource, LightSourceReference,
            LightSources, PhotometryReference,
        };
        use crate::gldf::general_definitions::photometries::{
            Photometries, Photometry, PhotometryFileReference,
        };
        use crate::gldf::general_definitions::{File, Files};

        let ies_path = "../../tests/data/lampset_lumload.ies";
        if std::fs::metadata(ies_path).is_err() {
            eprintln!("skipping: IES fixture {ies_path} missing");
            return;
        }
        let ies_bytes = std::fs::read(ies_path).expect("read IES fixture");

        let mut product = GldfProduct::default();
        product.general_definitions.files = Files {
            file: vec![File {
                id: "ies1".to_string(),
                content_type: "ldc/ies".to_string(),
                type_attr: "localFileName".to_string(),
                language: String::new(),
                file_name: "lampset_lumload.ies".to_string(),
            }],
        };
        product.general_definitions.photometries = Some(Photometries {
            photometry: vec![Photometry {
                id: "p1".to_string(),
                photometry_file_reference: Some(PhotometryFileReference {
                    file_id: "ies1".to_string(),
                }),
                descriptive_photometry: None,
            }],
        });
        product.general_definitions.light_sources = Some(LightSources {
            fixed_light_source: vec![FixedLightSource {
                id: "ls1".to_string(),
                rated_input_power: Some(40.0),
                ..Default::default()
            }],
            changeable_light_source: vec![],
            multi_channel_light_source: vec![],
        });
        product.general_definitions.emitters = Some(Emitters {
            emitter: vec![Emitter {
                id: "em1".to_string(),
                fixed_light_emitter: vec![FixedLightEmitter {
                    photometry_reference: PhotometryReference {
                        photometry_id: "p1".to_string(),
                    },
                    light_source_reference: LightSourceReference {
                        fixed_light_source_id: Some("ls1".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                }],
                ..Default::default()
            }],
        });

        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert("ies1".to_string(), ies_bytes);

        // All three text formats should round-trip without erroring.
        let ldt = export_ldt(&product, &files, "ls1").expect("export_ldt from IES");
        assert!(!ldt.is_empty(), "LDT export empty");
        let ies = export_ies(&product, &files, "ls1").expect("export_ies from IES");
        // IES header is one of IESNA:LM-63-1995, IESNA:LM-63-2002,
        // IES:LM-63-2019, or "IESNA91" (LM-63-1991). Any LM-63 marker
        // proves we emitted a real IES file rather than an empty stub.
        assert!(
            ies.contains("LM-63") || ies.contains("IESNA"),
            "IES export missing LM-63 header: {}",
            &ies[..ies.len().min(80)]
        );
        let xml = export_tm33_xml(&product, &files, "ls1").expect("export_tm33_xml from IES");
        assert!(
            xml.contains("IESTM33-22"),
            "TM-33 root tag missing on IES-backed export"
        );
    }

    /// SPD-file fallback: when the GLDF Spectrum holds only a
    /// `SpectrumFileReference` (no inline `<Intensity>` pairs — the
    /// editor's drop-an-.spd path), overlay_emitter must parse the
    /// referenced `.spd` bytes via `eulumdat::atla::spd_loader::parse`
    /// and embed the SPD in TM-33. Without this fallback the editor-
    /// built GLDFs that ship spectra as separate files lose their SPD
    /// half on TM-33 export.
    #[test]
    fn file_referenced_spd_lands_in_tm33() {
        use crate::gldf::general_definitions::lightsources::{
            Emitter, Emitters, FixedLightEmitter, FixedLightSource, LightSourceReference,
            LightSources, PhotometryReference, SpectrumReference,
        };
        use crate::gldf::general_definitions::photometries::{
            Photometries, Photometry, PhotometryFileReference, Spectrum, SpectrumFileReference,
            Spectrums,
        };
        use crate::gldf::general_definitions::{File, Files};

        let spd_path = "../../tests/test-create/spectrum_2700K.spd";
        if std::fs::metadata(spd_path).is_err() {
            eprintln!("skipping: SPD fixture {spd_path} missing");
            return;
        }
        let spd_bytes = std::fs::read(spd_path).expect("read SPD");

        // Same scaffold as the inline-SPD test, but the Spectrum
        // entry uses SpectrumFileReference instead of inline Intensity
        // pairs.
        let ldt = match std::fs::read("../../tests/data/SLV - Tria 2.gldf") {
            Ok(bytes) => match GldfProduct::load_gldf_from_buf_all(bytes) {
                Ok(b) => b
                    .files
                    .into_iter()
                    .find(|bf| {
                        bf.path
                            .as_deref()
                            .map(|p| p.ends_with(".ldt"))
                            .unwrap_or(false)
                    })
                    .and_then(|bf| bf.content),
                Err(_) => None,
            },
            Err(_) => None,
        };
        let Some(ldt_bytes) = ldt else {
            eprintln!("skipping: no SLV LDT available to seed synthetic fixture");
            return;
        };

        let mut product = GldfProduct::default();
        product.general_definitions.files = Files {
            file: vec![
                File {
                    id: "ldt1".to_string(),
                    content_type: "ldc/eulumdat".to_string(),
                    type_attr: "localFileName".to_string(),
                    language: String::new(),
                    file_name: "test.ldt".to_string(),
                },
                File {
                    id: "spd1".to_string(),
                    content_type: "spectrum/text".to_string(),
                    type_attr: "localFileName".to_string(),
                    language: String::new(),
                    file_name: "spectrum.spd".to_string(),
                },
            ],
        };
        product.general_definitions.photometries = Some(Photometries {
            photometry: vec![Photometry {
                id: "p1".to_string(),
                photometry_file_reference: Some(PhotometryFileReference {
                    file_id: "ldt1".to_string(),
                }),
                descriptive_photometry: None,
            }],
        });
        product.general_definitions.spectrums = Some(Spectrums {
            spectrum: vec![Spectrum {
                id: "spec1".to_string(),
                spectrum_file_reference: Some(SpectrumFileReference {
                    file_id: "spd1".to_string(),
                }),
                intensity: vec![],
            }],
        });
        product.general_definitions.light_sources = Some(LightSources {
            fixed_light_source: vec![FixedLightSource {
                id: "ls1".to_string(),
                rated_input_power: Some(30.0),
                spectrum_reference: Some(SpectrumReference {
                    spectrum_id: "spec1".to_string(),
                }),
                ..Default::default()
            }],
            changeable_light_source: vec![],
            multi_channel_light_source: vec![],
        });
        product.general_definitions.emitters = Some(Emitters {
            emitter: vec![Emitter {
                id: "em1".to_string(),
                fixed_light_emitter: vec![FixedLightEmitter {
                    photometry_reference: PhotometryReference {
                        photometry_id: "p1".to_string(),
                    },
                    light_source_reference: LightSourceReference {
                        fixed_light_source_id: Some("ls1".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                }],
                ..Default::default()
            }],
        });

        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert("ldt1".to_string(), ldt_bytes);
        files.insert("spd1".to_string(), spd_bytes);

        let xml = export_tm33_xml(&product, &files, "ls1").expect("export_tm33_xml");
        assert!(
            xml.contains("Wavelength") || xml.contains("SpectralDistribution"),
            "TM-33 output lacks SPD block (file-ref path): {xml}"
        );
    }
}
