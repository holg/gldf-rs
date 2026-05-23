//! Photometric file export and variant-aware patching.
//!
//! Pure-eulumdat helpers extracted from the WASM viewer so any consumer
//! (CLI, FFI, Python, server) can produce per-variant LDT/IES/ATLA
//! downloads with the variant's resolved lumens/watts patched onto the
//! source photometry.
//!
//! Requires the `eulumdat` feature.

use eulumdat::Eulumdat;

use crate::gldf::general_definitions::GeneralDefinitions;
use crate::gldf::product_definitions::{ProductDefinitions, Variant};

/// Export format for per-variant photometry downloads.
///
/// LDT/IES are the industry-standard photometric formats; ATLA JSON/XML
/// carry richer metadata (lossless-ish round-trip via the eulumdat-rs
/// `atla` module). LDT↔IES round-trips lose some details, so prefer the
/// source format when the downstream tool accepts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhotometryExportFormat {
    /// EULUMDAT (`.ldt`) — European industry standard.
    Ldt,
    /// IES LM-63 (`.ies`) — North American industry standard.
    Ies,
    /// ATLA JSON — richer-metadata photometric exchange format.
    AtlaJson,
    /// ATLA XML — richer-metadata photometric exchange format.
    AtlaXml,
}

impl PhotometryExportFormat {
    /// Recommended filename extension (without leading dot).
    pub fn extension(self) -> &'static str {
        match self {
            Self::Ldt => "ldt",
            Self::Ies => "ies",
            Self::AtlaJson => "json",
            Self::AtlaXml => "xml",
        }
    }

    /// MIME type for HTTP responses / `Blob` construction.
    pub fn mime(self) -> &'static str {
        match self {
            Self::Ldt | Self::Ies => "text/plain",
            Self::AtlaJson => "application/json",
            Self::AtlaXml => "application/xml",
        }
    }

    /// Human-readable label suitable for dropdown menus.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ldt => "LDT (EULUMDAT)",
            Self::Ies => "IES",
            Self::AtlaJson => "ATLA JSON",
            Self::AtlaXml => "ATLA XML",
        }
    }

    /// Parse a stable identifier (`"ldt"`, `"ies"`, `"atla_json"`,
    /// `"atla_xml"`). Returns `None` for unrecognized values.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ldt" => Some(Self::Ldt),
            "ies" => Some(Self::Ies),
            "atla_json" => Some(Self::AtlaJson),
            "atla_xml" => Some(Self::AtlaXml),
            _ => None,
        }
    }

    /// Stable identifier (inverse of [`Self::from_str`]). Suitable for
    /// `<select>` option values, query strings, etc.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ldt => "ldt",
            Self::Ies => "ies",
            Self::AtlaJson => "atla_json",
            Self::AtlaXml => "atla_xml",
        }
    }

    /// Smart default based on the source filename's extension. `.ies`
    /// input → [`Self::Ies`]; everything else → [`Self::Ldt`]. JSON/XML
    /// never auto-default — they're explicit user choices.
    pub fn default_for_source(filename: Option<&str>) -> Self {
        let lower = filename.map(|n| n.to_lowercase()).unwrap_or_default();
        if lower.ends_with(".ies") {
            Self::Ies
        } else {
            Self::Ldt
        }
    }
}

/// Render the chosen format from a parsed [`Eulumdat`]. Returns the
/// serialized bytes, or `None` if ATLA serialization fails.
///
/// All four formats round-trip the *patched* `Eulumdat` object, so a 50 W
/// variant downloaded as IES carries the 50 W wattage even when the
/// source LDT had 0 W. ATLA JSON/XML preserve fields LDT/IES drop, so
/// prefer them when the downstream tool can consume them.
pub fn export_photometry(ldt: &Eulumdat, format: PhotometryExportFormat) -> Option<Vec<u8>> {
    match format {
        PhotometryExportFormat::Ldt => Some(ldt.to_ldt().into_bytes()),
        PhotometryExportFormat::Ies => Some(eulumdat::IesExporter::export(ldt).into_bytes()),
        PhotometryExportFormat::AtlaJson => {
            let doc = eulumdat::atla::LuminaireOpticalData::from(ldt);
            eulumdat::atla::json::write(&doc)
                .ok()
                .map(String::into_bytes)
        }
        PhotometryExportFormat::AtlaXml => {
            let doc = eulumdat::atla::LuminaireOpticalData::from(ldt);
            eulumdat::atla::xml::write(&doc)
                .ok()
                .map(String::into_bytes)
        }
    }
}

/// Per-emitter photometry resolution for a single variant.
///
/// One variant can have multiple emitters (multi-emitter floodlights,
/// mixed direct/indirect, etc.), so [`resolve_variant_photometries`]
/// returns a `Vec` of these. Each entry binds together:
///
/// * which photometry (LDT shape) the emitter uses,
/// * the variant-resolved lumens / watts (walking
///   `FixedLightEmitter → LightSourceReference → FixedLightSource`), and
/// * driver / emergency metadata needed by viewer chrome.
///
/// `lumens` / `watts` may be `None` when the file doesn't declare them at
/// the resolved level — the LDT-native value is the fallback in that case.
#[derive(Debug, Clone)]
pub struct VariantPhotometryResolution {
    /// Photometry id this emitter resolves to (the LDT shape).
    pub photometry_id: String,
    /// Variant-level rated luminous flux, if `FixedLightEmitter.RatedLuminousFlux`
    /// or `ChangeableLightEmitter`'s nominal flux is set.
    ///
    /// For `EmergencyOnly` emitters the XSD's `RatedLuminousFlux` is the
    /// *emergency-mode* flux (those emitters have no normal mode), so we
    /// don't surface it as a normal-operation lumens override — `lumens`
    /// stays `None` for that case to avoid claiming `100 lm / 81 W = 1.2
    /// lm/W` as the variant's efficacy.
    pub lumens: Option<i32>,
    /// Variant-level rated input power, walked through the
    /// `LightSourceReference` to `FixedLightSource.RatedInputPower`.
    /// Suppressed for `EmergencyOnly` emitters (same reason as `lumens`).
    pub watts: Option<f64>,
    /// Light source id that supplied the wattage (for display / tooltip).
    pub light_source_id: Option<String>,
    /// `FixedLightEmitter.@emergencyBehaviour` — `None` (no attribute),
    /// `"None"` (explicit normal), `"Combined"` (normal + emergency), or
    /// `"EmergencyOnly"`. Drives viewer chrome and suppresses the
    /// calc/patch for `EmergencyOnly`.
    pub emergency_behaviour: Option<String>,
    /// `FixedLightEmitter.ControlGearReference.@controlGearId` — points at
    /// the driver in `GeneralDefinitions/ControlGears`. Used by the
    /// Electrical viewer to render driver details per variant.
    pub control_gear_id: Option<String>,
}

impl VariantPhotometryResolution {
    /// `true` when this emitter only operates during emergency conditions.
    /// Callers typically skip the calc column and the patch for these —
    /// emergency flux against full-load wattage is a meaningless ratio.
    pub fn is_emergency_only(&self) -> bool {
        matches!(self.emergency_behaviour.as_deref(), Some("EmergencyOnly"))
    }
}

/// Resolve every emitter chain on a single variant to its
/// `(photometry_id, lumens, watts)` tuple plus driver / emergency
/// metadata.
///
/// Walks all three emitter-reference branches the XSD allows:
/// `Variant > Geometry > EmitterReference` (bare),
/// `... > SimpleGeometryReference` (with `@emitterId`),
/// `... > ModelGeometryReference > EmitterReference` (nested form).
///
/// `product` is currently reserved for future use (Equipment lookups for
/// `ChangeableLightEmitter` driver wiring). `general` carries the
/// `<Emitters>` and `<LightSources>` pools that the resolver dereferences.
pub fn resolve_variant_photometries(
    product: &ProductDefinitions,
    variant: &Variant,
    general: &GeneralDefinitions,
) -> Vec<VariantPhotometryResolution> {
    let _ = product; // Reserved for future use (Equipment lookups, etc.)

    let mut emitter_ids: Vec<String> = Vec::new();
    if let Some(ref geom) = variant.geometry {
        for er in &geom.emitter_reference {
            emitter_ids.push(er.emitter_id.clone());
        }
        if let Some(ref sgr) = geom.simple_geometry_reference {
            emitter_ids.push(sgr.emitter_id.clone());
        }
        if let Some(ref mgr) = geom.model_geometry_reference {
            for er in &mgr.emitter_reference {
                emitter_ids.push(er.emitter_id.clone());
            }
        }
    }

    let emitters = match general.emitters.as_ref() {
        Some(e) => &e.emitter,
        None => return Vec::new(),
    };
    let light_sources = general.light_sources.as_ref();

    let mut out = Vec::new();
    for eid in &emitter_ids {
        let Some(emitter) = emitters.iter().find(|e| &e.id == eid) else {
            continue;
        };
        // FixedLightEmitter: variant-level lumens + light_source_reference for watts.
        for fle in &emitter.fixed_light_emitter {
            let photometry_id = fle.photometry_reference.photometry_id.clone();
            let is_emergency_only =
                matches!(fle.emergency_behaviour.as_deref(), Some("EmergencyOnly"));
            // For EmergencyOnly emitters, RatedLuminousFlux is the emergency
            // flux and there is no normal-mode wattage on the
            // light_source_reference (the LightSource it points at carries
            // the *full-load* W, not the emergency W). Pairing those two
            // gives a meaningless ratio (100 lm / 81 W ≈ 1.2 lm/W on the
            // SLV Tria 2). Skip the override entirely; the LDT-native calc
            // is similarly off but viewers suppress Calc for these.
            let lumens = if is_emergency_only {
                None
            } else {
                fle.rated_luminous_flux
            };
            let (light_source_id, watts) = if is_emergency_only {
                (
                    fle.light_source_reference.fixed_light_source_id.clone(),
                    None,
                )
            } else {
                match (
                    fle.light_source_reference.fixed_light_source_id.as_ref(),
                    light_sources,
                ) {
                    (Some(ls_id), Some(ls)) => {
                        let watts = ls
                            .fixed_light_source
                            .iter()
                            .find(|s| &s.id == ls_id)
                            .and_then(|s| s.rated_input_power);
                        (Some(ls_id.clone()), watts)
                    }
                    _ => (None, None),
                }
            };
            out.push(VariantPhotometryResolution {
                photometry_id,
                lumens,
                watts,
                light_source_id,
                emergency_behaviour: fle.emergency_behaviour.clone(),
                control_gear_id: fle
                    .control_gear_reference
                    .as_ref()
                    .map(|cgr| cgr.control_gear_id.clone()),
            });
        }
        // ChangeableLightEmitter has no light_source_reference at the emitter
        // level (the XSD pairs Changeable bulbs with control gear via
        // Equipment, not at the emitter). Phase 1: surface them with
        // photometry_id only — wattage shows as None and the calc falls
        // back to the LDT value, which is correct for replaceable bulbs.
        for cle in &emitter.changeable_light_emitter {
            out.push(VariantPhotometryResolution {
                photometry_id: cle.photometry_reference.photometry_id.clone(),
                lumens: None,
                watts: None,
                light_source_id: None,
                emergency_behaviour: cle.emergency_behaviour.clone(),
                // ChangeableLightEmitter has no ControlGearReference at
                // the emitter level (XSD wires gear via Equipment for
                // replaceable bulbs). Phase 1 leaves this None.
                control_gear_id: None,
            });
        }
    }
    out
}

/// Re-emit an LDT with the variant-resolved lumens / watts patched onto
/// `lamp_sets[0]`. This is what makes downstream eulumdat consumers
/// report the correct lm/W instead of the LDT-native 0.0 lm/W when the
/// GLDF variant overrides them.
///
/// Why `[0]` only: most LDT fixtures carry one lamp_set; multi-set LDTs
/// (stacked driver options) are rare and the GLDF variant model already
/// disambiguates them via per-variant references, so each variant
/// effectively pins one lamp_set anyway.
///
/// Returns `None` when the bytes don't parse as LDT, or when neither
/// override is supplied (no work to do — caller should keep the original).
pub fn patch_ldt_for_variant(
    raw_bytes: &[u8],
    lumens: Option<i32>,
    watts: Option<f64>,
) -> Option<Vec<u8>> {
    if lumens.is_none() && watts.is_none() {
        return None;
    }
    let s = std::str::from_utf8(raw_bytes).ok()?;
    let mut ldt = Eulumdat::parse(s).ok()?;
    if let Some(set) = ldt.lamp_sets.first_mut() {
        if let Some(w) = watts {
            set.wattage_with_ballast = w;
        }
        if let Some(lm) = lumens {
            set.total_luminous_flux = lm as f64;
        }
    }
    Some(ldt.to_ldt().into_bytes())
}
