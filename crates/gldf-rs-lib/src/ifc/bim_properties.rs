//! buildingSMART BIM-property mapping for the GLDF→IFC exporter.
//!
//! Source of truth: `docs/bim_properties_to_gldf.tsv` (extracted from
//! `docs/BIM_Properties_to_GLDF_RH240312.xlsx`, authored by RH
//! 2024-03-12). Each row maps a GLDF XPath to a buildingSMART
//! property name + stable bSDD GlobalId.
//!
//! Why GlobalIds matter: BIM tools (Revit, Solibri, …) recognise a
//! property as a *standard* one by its bSDD GlobalId, not its name.
//! Emitting `IngressProtectionIPCode` without its id makes it a
//! vendor-specific string; emitting it WITH `BfV1...` lets a BIM
//! consumer match it against the dictionary and roll it into IFC's
//! native Pset_LightFixtureTypeCommon-style slots.
//!
//! Scope: this module covers the **product-level DescriptiveAttributes**
//! (Electrical / Mechanical / Marketing / OperationsAndMaintenance /
//! Emergency) — the properties most BIM consumers actually read. The
//! full 480-row table also maps light-source, control-gear,
//! photometry and per-variant paths; those land in later slices.
//!
//! The properties produced here are domain data; the actual STEP
//! emission goes through `StepWriter::add_property_set_with_ids`,
//! which mirrors `bimifc-writer`'s `write_property_set` so a future
//! refactor can swap implementations.

use crate::GldfProduct;

/// A resolved BIM property ready to emit: name, bSDD GlobalId, value,
/// and which Pset bucket it belongs in.
#[derive(Debug, Clone, PartialEq)]
pub struct BimProperty {
    /// IFC property name (the buildingSMART "Name EN", PascalCase-ish).
    pub name: String,
    /// bSDD GlobalId from column A of the mapping table. Empty if the
    /// property has no dictionary id (shouldn't happen for the ones
    /// we emit, but kept non-fatal).
    pub bsdd_id: String,
    /// Stringified value. The IFC layer decides the IFC value type
    /// (label / measure) from [`Self::kind`].
    pub value: String,
    /// Hint for the IFC value type.
    pub kind: BimValueKind,
    /// Which Pset this property groups under (per-domain bucketing
    /// matching the existing exporter's `Pset_GLDF_*` convention).
    pub pset: BimPset,
}

/// IFC value-type hint. Keeps the domain layer decoupled from the
/// STEP-writer's `IfcValue` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BimValueKind {
    Label,
    Boolean,
    PowerMeasure,
    MassMeasure,
    /// Real with no unit semantics (power factor, etc.).
    Real,
    /// Length in millimetres (GLDF native) — emitted as
    /// IFCPOSITIVELENGTHMEASURE after mm→m conversion at the IFC layer.
    LengthMm,
}

/// Per-domain Pset buckets. Names follow the existing exporter's
/// `Pset_GLDF_*` style so a BIM tool sees consistent grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BimPset {
    Electrical,
    Mechanical,
    OperationsAndMaintenance,
    Emergency,
    Marketing,
}

impl BimPset {
    pub fn pset_name(self) -> &'static str {
        match self {
            Self::Electrical => "Pset_GLDF_Electrical",
            Self::Mechanical => "Pset_GLDF_Mechanical",
            Self::OperationsAndMaintenance => "Pset_GLDF_OperationsAndMaintenance",
            Self::Emergency => "Pset_GLDF_Emergency",
            Self::Marketing => "Pset_GLDF_Marketing",
        }
    }
}

/// bSDD GlobalIds for the product-level DescriptiveAttributes
/// properties we emit. Keyed by IFC property name. Pulled verbatim
/// from column A of `docs/bim_properties_to_gldf.tsv`.
///
/// Keeping this as a small explicit table (rather than parsing the
/// TSV at runtime) avoids a build-time codegen step and keeps the
/// WASM build dependency-free. When we expand to the full 480 rows
/// it'll be worth a build.rs that reads the TSV.
// IDs verified against docs/bim_properties_to_gldf.tsv column A
// (cross-referenced by the GLDF XPath in column D so they match the
// product-level DescriptiveAttributes paths we read). The `Weight`
// id is the table's "mass" row (column D ...Mechanical/Weight).
const BSDD_IDS: &[(&str, &str)] = &[
    // Electrical
    ("ElectricalSafetyClass", "0Ias3th8D2cg4EvP$KqQcv"),
    ("IngressProtectionIPCode", "1K_1K1q6n7KgOFJw5Cuc_i"),
    ("PowerFactor", "3LBrQkLDDFheBOxrGk8Zr_"),
    ("ConstantLightOutput", "3kbFNwkkb7gPUsnc_COBg4"),
    // Mechanical
    ("Weight", "1RPyGAgMf4hRTZ0DogLFnU"),
    ("IKRating", "16XdmXw8T3HOIi0TOcc8LH"),
    ("ProductForm", "1zs4Cj96j3d8TWAeeixJga"),
    ("OverallWidth", "1dPvrZN3vEIB0n0vwYvDcX"),
    ("OverallHeight", "19Z9XKYDT4p8HR0ZbD$wO_"),
    ("OverallLength", "1uJglYpRnFpQ4tStHaR2Pf"),
    // OperationsAndMaintenance
    ("MedianUsefulLife", "0NMpOQzNH679doR_IuOYUl"),
    // Emergency
    ("DedicatedEmergencyLightingType", "3Q6UsxAKD0Q8mO0x0ZsmhD"),
];

fn bsdd_id(name: &str) -> String {
    BSDD_IDS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, id)| id.to_string())
        .unwrap_or_default()
}

/// Reverse lookup: a bSDD GlobalId → the IFC property name we mapped
/// it to. Used on import to recognise a property by its stable
/// dictionary id regardless of which Pset it landed in or how the
/// name was localised.
pub fn property_name_for_bsdd(id: &str) -> Option<&'static str> {
    BSDD_IDS
        .iter()
        .find(|(_, bid)| *bid == id)
        .map(|(name, _)| *name)
}

/// A BIM property recognised on IFC import, identified by the spec's
/// canonical property name. The caller maps `name` → the matching
/// GLDF DescriptiveAttributes field. `value` is the raw STEP value
/// text (already unwrapped from `IFCLABEL('…')` etc. by the caller).
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedBimProperty {
    /// Canonical IFC property name (matches the export table).
    pub name: &'static str,
    /// Raw value string.
    pub value: String,
}

/// Resolve a scanned IFC property to its canonical spec property.
///
/// Tries the bSDD GlobalId first (the robust path — survives name
/// localisation / Pset renaming), then falls back to matching the
/// property name directly. Returns `None` if neither the id nor the
/// name is in our mapping table.
///
/// `bsdd_id_from_desc` is whatever was in the property's Description
/// after stripping the `bSDD:` prefix (empty if absent).
pub fn resolve_imported_property(
    property_name: &str,
    bsdd_id_from_desc: &str,
    value: String,
) -> Option<ImportedBimProperty> {
    if !bsdd_id_from_desc.is_empty() {
        if let Some(name) = property_name_for_bsdd(bsdd_id_from_desc) {
            return Some(ImportedBimProperty { name, value });
        }
    }
    // Fall back to name match against the table.
    BSDD_IDS
        .iter()
        .find(|(n, _)| *n == property_name)
        .map(|(n, _)| ImportedBimProperty { name: n, value })
}

/// Extract every product-level DescriptiveAttributes BIM property
/// present in `gldf`. Variant-level overrides are out of scope here
/// (the caller handles per-variant emission); this is the shared
/// product metadata.
pub fn extract_product_bim_properties(gldf: &GldfProduct) -> Vec<BimProperty> {
    let mut out = Vec::new();

    let Some(meta) = &gldf.product_definitions.product_meta_data else {
        return out;
    };
    let Some(attrs) = &meta.descriptive_attributes else {
        return out;
    };

    // ── Electrical ──────────────────────────────────────────────
    if let Some(e) = &attrs.electrical {
        if let Some(v) = &e.electrical_safety_class {
            out.push(prop(
                "ElectricalSafetyClass",
                v.clone(),
                BimValueKind::Label,
                BimPset::Electrical,
            ));
        }
        if let Some(v) = &e.ingress_protection_ip_code {
            out.push(prop(
                "IngressProtectionIPCode",
                v.clone(),
                BimValueKind::Label,
                BimPset::Electrical,
            ));
        }
        if let Some(v) = e.power_factor {
            out.push(prop(
                "PowerFactor",
                fmt_f64(v),
                BimValueKind::Real,
                BimPset::Electrical,
            ));
        }
        if let Some(v) = e.constant_light_output {
            out.push(prop(
                "ConstantLightOutput",
                v.to_string(),
                BimValueKind::Boolean,
                BimPset::Electrical,
            ));
        }
    }

    // ── Mechanical ──────────────────────────────────────────────
    if let Some(m) = &attrs.mechanical {
        if let Some(v) = m.weight {
            out.push(prop(
                "Weight",
                fmt_f64(v),
                BimValueKind::MassMeasure,
                BimPset::Mechanical,
            ));
        }
        if let Some(v) = &m.ik_rating {
            out.push(prop(
                "IKRating",
                v.clone(),
                BimValueKind::Label,
                BimPset::Mechanical,
            ));
        }
        if let Some(v) = &m.product_form {
            out.push(prop(
                "ProductForm",
                v.clone(),
                BimValueKind::Label,
                BimPset::Mechanical,
            ));
        }
        if let Some(size) = &m.product_size {
            // ProductSize children are mm in GLDF.
            if size.width != 0 {
                out.push(prop(
                    "OverallWidth",
                    size.width.to_string(),
                    BimValueKind::LengthMm,
                    BimPset::Mechanical,
                ));
            }
            if size.height != 0 {
                out.push(prop(
                    "OverallHeight",
                    size.height.to_string(),
                    BimValueKind::LengthMm,
                    BimPset::Mechanical,
                ));
            }
            if size.length != 0 {
                out.push(prop(
                    "OverallLength",
                    size.length.to_string(),
                    BimValueKind::LengthMm,
                    BimPset::Mechanical,
                ));
            }
        }
    }

    // ── Operations & Maintenance ────────────────────────────────
    if let Some(ops) = &attrs.operations_and_maintenance {
        if let Some(lifetimes) = &ops.median_useful_life_times {
            if let Some(first) = lifetimes.median_useful_life.first() {
                if !first.is_empty() {
                    out.push(prop(
                        "MedianUsefulLife",
                        first.clone(),
                        BimValueKind::Label,
                        BimPset::OperationsAndMaintenance,
                    ));
                }
            }
        }
    }

    // ── Emergency ───────────────────────────────────────────────
    if let Some(em) = &attrs.emergency {
        if let Some(v) = &em.dedicated_emergency_lighting_type {
            out.push(prop(
                "DedicatedEmergencyLightingType",
                v.clone(),
                BimValueKind::Label,
                BimPset::Emergency,
            ));
        }
    }

    out
}

fn prop(name: &str, value: String, kind: BimValueKind, pset: BimPset) -> BimProperty {
    BimProperty {
        name: name.to_string(),
        bsdd_id: bsdd_id(name),
        value,
        kind,
        pset,
    }
}

fn fmt_f64(x: f64) -> String {
    // Keep it tidy: drop trailing zeros, but always a number.
    let s = format!("{x}");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gldf::product_definitions::{
        DescriptiveAttributes, Electrical, Mechanical, ProductDefinitions, ProductMetaData,
    };

    fn product_with_attrs(attrs: DescriptiveAttributes) -> GldfProduct {
        GldfProduct {
            product_definitions: ProductDefinitions {
                product_meta_data: Some(ProductMetaData {
                    descriptive_attributes: Some(attrs),
                    ..ProductMetaData::default()
                }),
                ..ProductDefinitions::default()
            },
            ..GldfProduct::default()
        }
    }

    #[test]
    fn empty_product_yields_no_properties() {
        let p = GldfProduct::default();
        assert!(extract_product_bim_properties(&p).is_empty());
    }

    #[test]
    fn electrical_properties_carry_bsdd_ids() {
        let attrs = DescriptiveAttributes {
            electrical: Some(Electrical {
                electrical_safety_class: Some("II".into()),
                ingress_protection_ip_code: Some("IP20".into()),
                power_factor: Some(0.95),
                ..Electrical::default()
            }),
            ..DescriptiveAttributes::default()
        };
        let p = product_with_attrs(attrs);
        let props = extract_product_bim_properties(&p);

        let sc = props
            .iter()
            .find(|p| p.name == "ElectricalSafetyClass")
            .unwrap();
        assert_eq!(sc.value, "II");
        assert!(
            !sc.bsdd_id.is_empty(),
            "safety class should carry a bSDD id"
        );
        assert_eq!(sc.pset, BimPset::Electrical);

        let ip = props
            .iter()
            .find(|p| p.name == "IngressProtectionIPCode")
            .unwrap();
        assert_eq!(ip.value, "IP20");

        let pf = props.iter().find(|p| p.name == "PowerFactor").unwrap();
        assert_eq!(pf.value, "0.95");
        assert_eq!(pf.kind, BimValueKind::Real);
    }

    #[test]
    fn mechanical_weight_is_mass_measure() {
        let attrs = DescriptiveAttributes {
            mechanical: Some(Mechanical {
                weight: Some(0.379),
                ik_rating: Some("IK08".into()),
                ..Mechanical::default()
            }),
            ..DescriptiveAttributes::default()
        };
        let p = product_with_attrs(attrs);
        let props = extract_product_bim_properties(&p);
        let w = props.iter().find(|p| p.name == "Weight").unwrap();
        assert_eq!(w.value, "0.379");
        assert_eq!(w.kind, BimValueKind::MassMeasure);
        assert_eq!(w.pset, BimPset::Mechanical);
    }

    #[test]
    fn median_useful_life_id_matches_table() {
        // From docs/bim_properties_to_gldf.tsv, row "median useful life"
        // → ...OperationsAndMaintenance/MedianUsefulLifeTimes.
        assert_eq!(bsdd_id("MedianUsefulLife"), "0NMpOQzNH679doR_IuOYUl");
    }
}
