//! Post-load XSD-enum validation pass.
//!
//! The library accepts any `String` for closed-enum fields so it can parse
//! malformed legacy files without losing data. The editor side (this crate)
//! is the gate that catches schema violations: this module walks a
//! [`GldfProduct`] after loading and produces a `Vec<Violation>` listing
//! every closed-enum field whose value isn't in the XSD's
//! `<xs:enumeration>` set.
//!
//! Coverage matches `super::xsd_enums::canonical_set_for(...)` —
//! every closed enum in `tests/data/gldf-1.0.0-rc-3.xsd` is checked
//! except the polymorphic `type` attribute (which needs context to
//! disambiguate between File / FluxFactor / MelanopicFactor variants).
//!
//! Output is consumed by the toolbar banner so the user sees "this file
//! has 3 schema violations: ..." with click-through to the offending
//! sites.
//!
//! Violations are reported, never auto-fixed. Auto-correcting "ldc/dialux"
//! → "ldc/eulumdat" would silently change semantics; the user decides.

use super::xsd_enums::{canonical_set_for, is_canonical};
use crate::gldf::GldfProduct;

/// One XSD enum violation discovered while walking the product tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    /// Path-like locator pointing at the offending field, in roughly the
    /// XSD/XPath shape readers expect (`Product/Variant[id=…]/…`). Not a
    /// strict XPath — we don't escape — but readable.
    pub path: String,
    /// XSD element / attribute name (the key into
    /// [`canonical_set_for`]). e.g. `"IngressProtectionIPCode"`.
    pub field: &'static str,
    /// The offending string actually in the file.
    pub value: String,
    /// Human-readable hint about what *would* be valid. Empty when the
    /// canonical set is too long to inline (e.g. 47 IP codes).
    pub hint: String,
}

/// Walk a parsed [`GldfProduct`] and report every closed-enum violation.
/// Empty result = file is XSD-enum-clean.
pub fn validate(product: &GldfProduct) -> Vec<Violation> {
    let mut out = Vec::new();
    walk_general_definitions(product, &mut out);
    walk_product_meta_data(product, &mut out);
    walk_variants(product, &mut out);
    out
}

/// Helper — push a violation if value is non-empty AND not in the
/// canonical set for the named XSD enum. Silently passes when the field
/// is empty or the name has no canonical set registered.
fn check_field(out: &mut Vec<Violation>, path: &str, field_name: &'static str, value: &str) {
    if value.is_empty() {
        return;
    }
    if let Some(set) = canonical_set_for(field_name) {
        if !is_canonical(set, value) {
            // Build a short hint: first 3 canonical values + ellipsis when
            // the set is bigger than that. Long sets (IP codes, application
            // strings) get just a count.
            let hint = if set.len() <= 3 {
                format!("expected one of: {}", set.join(", "))
            } else if set.len() <= 8 {
                format!("expected one of: {}", set.join(", "))
            } else {
                format!(
                    "{} canonical values defined; sample: {} …",
                    set.len(),
                    set.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                )
            };
            out.push(Violation {
                path: path.to_string(),
                field: field_name,
                value: value.to_string(),
                hint,
            });
        }
    }
}

fn check_field_opt(
    out: &mut Vec<Violation>,
    path: &str,
    field_name: &'static str,
    value: &Option<String>,
) {
    if let Some(v) = value {
        check_field(out, path, field_name, v);
    }
}

// ── GeneralDefinitions walkers ────────────────────────────────────────

fn walk_general_definitions(product: &GldfProduct, out: &mut Vec<Violation>) {
    // Files: contentType + type
    for f in &product.general_definitions.files.file {
        let path = format!("GeneralDefinitions/Files/File[@id={}]", f.id);
        check_field(
            out,
            &format!("{}/@contentType", path),
            "@contentType",
            &f.content_type,
        );
        // File@type uses the canonical set FILE_TYPES; canonical_set_for
        // doesn't expose this polymorphic attribute, so do the explicit
        // check inline.
        if !f.type_attr.is_empty() && !is_canonical(super::xsd_enums::FILE_TYPES, &f.type_attr) {
            out.push(Violation {
                path: format!("{}/@type", path),
                field: "@type (File)",
                value: f.type_attr.clone(),
                hint: "expected one of: localFileName, url".to_string(),
            });
        }
    }

    // LightSources — Cie97LampType (under LightSourceMaintenance), plus
    // InitialColorTolerance / MaintainedColorTolerance under
    // ColorInformation. Both Fixed- and ChangeableLightSource expose the
    // maintenance block; only FixedLightSource exposes ColorInformation.
    if let Some(ls) = product.general_definitions.light_sources.as_ref() {
        for fls in &ls.fixed_light_source {
            let path = format!(
                "GeneralDefinitions/LightSources/FixedLightSource[@id={}]",
                fls.id
            );
            if let Some(maint) = fls.light_source_maintenance.as_ref() {
                check_field_opt(
                    out,
                    &format!("{}/LightSourceMaintenance/Cie97LampType", path),
                    "Cie97LampType",
                    &maint.cie97_lamp_type,
                );
            }
            if let Some(ci) = fls.color_information.as_ref() {
                check_field_opt(
                    out,
                    &format!("{}/ColorInformation/InitialColorTolerance", path),
                    "InitialColorTolerance",
                    &ci.initial_color_tolerance,
                );
                check_field_opt(
                    out,
                    &format!("{}/ColorInformation/MaintainedColorTolerance", path),
                    "MaintainedColorTolerance",
                    &ci.maintained_color_tolerance,
                );
            }
        }
        for cls in &ls.changeable_light_source {
            let path = format!(
                "GeneralDefinitions/LightSources/ChangeableLightSource[@id={}]",
                cls.id
            );
            if let Some(maint) = cls.light_source_maintenance.as_ref() {
                check_field_opt(
                    out,
                    &format!("{}/LightSourceMaintenance/Cie97LampType", path),
                    "Cie97LampType",
                    &maint.cie97_lamp_type,
                );
            }
        }
    }

    // Emitters — emergencyBehaviour on FixedLightEmitter / ChangeableLightEmitter
    if let Some(emitters) = product.general_definitions.emitters.as_ref() {
        for em in &emitters.emitter {
            let em_path = format!("GeneralDefinitions/Emitters/Emitter[@id={}]", em.id);
            for (i, fle) in em.fixed_light_emitter.iter().enumerate() {
                let path = format!("{}/FixedLightEmitter[{}]", em_path, i);
                check_field_opt(
                    out,
                    &format!("{}/@emergencyBehaviour", path),
                    "@emergencyBehaviour",
                    &fle.emergency_behaviour,
                );
            }
            for (i, cle) in em.changeable_light_emitter.iter().enumerate() {
                let path = format!("{}/ChangeableLightEmitter[{}]", em_path, i);
                check_field_opt(
                    out,
                    &format!("{}/@emergencyBehaviour", path),
                    "@emergencyBehaviour",
                    &cle.emergency_behaviour,
                );
            }
        }
    }

    // ControlGears — Interface entries
    if let Some(cgs) = product.general_definitions.control_gears.as_ref() {
        for cg in &cgs.control_gear {
            let path = format!("GeneralDefinitions/ControlGears/ControlGear[@id={}]", cg.id);
            if let Some(ifs) = cg.interfaces.as_ref() {
                for (i, val) in ifs.interface.iter().enumerate() {
                    check_field(
                        out,
                        &format!("{}/Interfaces/Interface[{}]", path, i),
                        "Interface",
                        val,
                    );
                }
            }
        }
    }

    // Photometries — InitialColorTolerance / MaintainedColorTolerance
    // are on DescriptivePhotometry.
    if let Some(ps) = product.general_definitions.photometries.as_ref() {
        for ph in &ps.photometry {
            if let Some(_dp) = ph.descriptive_photometry.as_ref() {
                let _path = format!("GeneralDefinitions/Photometries/Photometry[@id={}]", ph.id);
                // The Rust binding for DescriptivePhotometry doesn't
                // currently expose initial_color_tolerance /
                // maintained_color_tolerance as fields — they're inside
                // the schema but the struct skipped them. When added,
                // wire here:
                //   check_field_opt(out, &path, "InitialColorTolerance",
                //       &dp.initial_color_tolerance);
            }
        }
    }

    // Geometries — @levelOfDetail lives on GeometryFileReference, not on
    // ModelGeometry directly (the XSD nests it as an attribute on the
    // file reference). One ModelGeometry can ship multiple
    // GeometryFileReferences (one per LoD); each gets its own check.
    if let Some(gs) = product.general_definitions.geometries.as_ref() {
        for mg in &gs.model_geometry {
            let path = format!("GeneralDefinitions/Geometries/ModelGeometry[@id={}]", mg.id);
            for (i, gfr) in mg.geometry_file_reference.iter().enumerate() {
                check_field_opt(
                    out,
                    &format!("{}/GeometryFileReference[{}]/@levelOfDetail", path, i),
                    "@levelOfDetail",
                    &gfr.level_of_detail,
                );
            }
        }
    }

    // Pictures (ProductMetaData/Pictures/Image) — handled inside
    // walk_product_meta_data alongside the other ProductMetaData fields.
}

// ── ProductMetaData walker ────────────────────────────────────────────

fn walk_product_meta_data(product: &GldfProduct, out: &mut Vec<Violation>) {
    let Some(meta) = product.product_definitions.product_meta_data.as_ref() else {
        return;
    };
    let path = "ProductDefinitions/ProductMetaData".to_string();

    // Pictures/Image@imageType
    if let Some(pictures) = meta.pictures.as_ref() {
        for (i, img) in pictures.image.iter().enumerate() {
            check_field(
                out,
                &format!("{}/Pictures/Image[{}]/@imageType", path, i),
                "@imageType",
                &img.image_type,
            );
        }
    }

    let Some(attrs) = meta.descriptive_attributes.as_ref() else {
        return;
    };
    walk_descriptive_attributes(attrs, &path, out);
}

// ── Variants walker ───────────────────────────────────────────────────

fn walk_variants(product: &GldfProduct, out: &mut Vec<Violation>) {
    let Some(variants) = product.product_definitions.variants.as_ref() else {
        return;
    };
    for v in &variants.variant {
        let path = format!("ProductDefinitions/Variants/Variant[@id={}]", v.id);

        // Pictures
        if let Some(pictures) = v.pictures.as_ref() {
            for (i, img) in pictures.image.iter().enumerate() {
                check_field(
                    out,
                    &format!("{}/Pictures/Image[{}]/@imageType", path, i),
                    "@imageType",
                    &img.image_type,
                );
            }
        }

        // Geometry — @targetModelType on nested EmitterReference (model branch).
        if let Some(geom) = v.geometry.as_ref() {
            if let Some(mgr) = geom.model_geometry_reference.as_ref() {
                for (i, er) in mgr.emitter_reference.iter().enumerate() {
                    check_field_opt(
                        out,
                        &format!(
                            "{}/Geometry/ModelGeometryReference/EmitterReference[{}]/@targetModelType",
                            path, i
                        ),
                        "@targetModelType",
                        &er.target_model_type,
                    );
                }
            }
        }

        // DescriptiveAttributes — same checks as ProductMetaData.
        if let Some(attrs) = v.descriptive_attributes.as_ref() {
            walk_descriptive_attributes(attrs, &path, out);
        }
    }
}

// ── DescriptiveAttributes walker (shared by Product- and Variant-level) ─

fn walk_descriptive_attributes(
    attrs: &crate::gldf::product_definitions::DescriptiveAttributes,
    base_path: &str,
    out: &mut Vec<Violation>,
) {
    let path = format!("{}/DescriptiveAttributes", base_path);

    if let Some(mech) = attrs.mechanical.as_ref() {
        let mp = format!("{}/Mechanical", path);
        check_field_opt(
            out,
            &format!("{}/IKRating", mp),
            "IKRating",
            &mech.ik_rating,
        );
        check_field_opt(
            out,
            &format!("{}/ProductForm", mp),
            "ProductForm",
            &mech.product_form,
        );
        if let Some(adj) = mech.adjustabilities.as_ref() {
            for (i, val) in adj.adjustability.iter().enumerate() {
                check_field(
                    out,
                    &format!("{}/Adjustabilities/Adjustability[{}]", mp, i),
                    "Adjustability",
                    val,
                );
            }
        }
        if let Some(pa) = mech.protective_areas.as_ref() {
            for (i, val) in pa.area.iter().enumerate() {
                check_field(
                    out,
                    &format!("{}/ProtectiveAreas/Area[{}]", mp, i),
                    "Area",
                    val,
                );
            }
        }
    }

    if let Some(elec) = attrs.electrical.as_ref() {
        let ep = format!("{}/Electrical", path);
        check_field_opt(
            out,
            &format!("{}/ElectricalSafetyClass", ep),
            "ElectricalSafetyClass",
            &elec.electrical_safety_class,
        );
        check_field_opt(
            out,
            &format!("{}/IngressProtectionIPCode", ep),
            "IngressProtectionIPCode",
            &elec.ingress_protection_ip_code,
        );
        check_field_opt(
            out,
            &format!("{}/LightDistribution", ep),
            "LightDistribution",
            &elec.light_distribution,
        );
    }

    if let Some(emer) = attrs.emergency.as_ref() {
        let ep = format!("{}/Emergency", path);
        check_field_opt(
            out,
            &format!("{}/DedicatedEmergencyLightingType", ep),
            "DedicatedEmergencyLightingType",
            &emer.dedicated_emergency_lighting_type,
        );
    }

    if let Some(mkt) = attrs.marketing.as_ref() {
        let mp = format!("{}/Marketing", path);
        if let Some(apps) = mkt.applications.as_ref() {
            for (i, val) in apps.application.iter().enumerate() {
                check_field(
                    out,
                    &format!("{}/Applications/Application[{}]", mp, i),
                    "Application",
                    val,
                );
            }
        }
        if let Some(labels) = mkt.labels.as_ref() {
            for (i, val) in labels.label.iter().enumerate() {
                check_field(out, &format!("{}/Labels/Label[{}]", mp, i), "Label", val);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gldf::product_definitions::{
        Applications, DescriptiveAttributes, Electrical, Marketing,
    };

    /// A clean product passes validation with zero violations.
    #[test]
    fn clean_product_has_no_violations() {
        let product = GldfProduct::default();
        assert!(validate(&product).is_empty());
    }

    /// A product with a non-canonical IP code surfaces exactly one
    /// violation, with the right field name and value.
    #[test]
    fn invalid_ip_code_is_flagged() {
        let mut product = GldfProduct::default();
        let mut meta = crate::gldf::product_definitions::ProductMetaData::default();
        let mut attrs = DescriptiveAttributes::default();
        attrs.electrical = Some(Electrical {
            ingress_protection_ip_code: Some("IP99".to_string()),
            ..Default::default()
        });
        meta.descriptive_attributes = Some(attrs);
        product.product_definitions.product_meta_data = Some(meta);

        let v = validate(&product);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].field, "IngressProtectionIPCode");
        assert_eq!(v[0].value, "IP99");
    }

    /// Multiple violations are accumulated, not coalesced.
    #[test]
    fn multiple_violations_accumulate() {
        let mut product = GldfProduct::default();
        let mut meta = crate::gldf::product_definitions::ProductMetaData::default();
        let mut attrs = DescriptiveAttributes::default();
        attrs.marketing = Some(Marketing {
            applications: Some(Applications {
                application: vec![
                    "Bogus".to_string(),            // non-canonical
                    "Interior: Office".to_string(), // canonical
                    "Another fake".to_string(),     // non-canonical
                ],
            }),
            ..Default::default()
        });
        meta.descriptive_attributes = Some(attrs);
        product.product_definitions.product_meta_data = Some(meta);

        let v = validate(&product);
        assert_eq!(v.len(), 2);
        assert!(v.iter().any(|x| x.value == "Bogus"));
        assert!(v.iter().any(|x| x.value == "Another fake"));
    }
}
