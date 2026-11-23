//! rc.4 schema tests — validate the four PR #30 features against the
//! bundled rc.4 XSD, through the SHIPPING selectable-schema validator
//! (`xsd_validate_product_xml_with_schema(.., Rc4)`), not a bespoke
//! uppsala call. This proves the public API respects rc-4 today.
//!
//! rc.4 is public on the (still-open) upstream PR branch
//! (globallightingdata/gldf#30, holg/gldf @ typo-fixes) at the flat
//! `xsd/gldf.xsd` path — NOT yet at the versioned
//! `gldf.io/xsd/gldf/1.0.0-rc.4/gldf.xsd` URL — so it's bundled at
//! `tests/data/gldf-1.0.0-rc-4.xsd` and embedded by the validator.
//!
//! Run: cargo test -p gldf-rs --features xsd --test xsd_rc4

#![cfg(feature = "xsd")]

use std::path::PathBuf;

use gldf_rs::validation::{
    xsd_validate_product_xml, xsd_validate_product_xml_with_schema, GldfSchemaVersion,
    DEFAULT_VALIDATION_SCHEMA,
};

const RC4_XSD: &str = include_str!("../../../tests/data/gldf-1.0.0-rc-4.xsd");

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../tests/data");
    p.push(name);
    p
}

/// The `product.xml` out of the canonical SLV fixture — a real, complete,
/// rc-3-clean document we patch feature values into.
fn slv_product_xml() -> String {
    use std::io::Read;
    let bytes = std::fs::read(fixture("SLV - Tria 2.gldf")).expect("read fixture");
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("open zip");
    let mut entry = zip.by_name("product.xml").expect("product.xml");
    let mut xml = String::new();
    entry.read_to_string(&mut xml).expect("read product.xml");
    xml
}

#[test]
fn default_validation_schema_is_latest_published_rc() {
    // We must NOT silently make an unpublished rc the default: producers
    // who validate without opting in get the latest PUBLISHED schema.
    assert_eq!(DEFAULT_VALIDATION_SCHEMA, GldfSchemaVersion::Rc3);
}

#[test]
fn slv_fixture_validates_clean_under_both_schemas() {
    // rc.4 is backward compatible: an rc-3-clean file stays clean under
    // rc-4 too — and the version-less default (rc-3) still passes.
    let xml = slv_product_xml();
    let default_errs = xsd_validate_product_xml(&xml).expect("default validate");
    assert!(
        default_errs.is_empty(),
        "SLV rc-3 default: {default_errs:?}"
    );
    for v in [GldfSchemaVersion::Rc3, GldfSchemaVersion::Rc4] {
        let errs = xsd_validate_product_xml_with_schema(&xml, v).expect("validate");
        assert!(errs.is_empty(), "SLV should be {v:?}-clean; got: {errs:?}");
    }
}

#[test]
fn rc4_accepts_decimal_sdcm_but_rc3_rejects_it() {
    // Feature 4: the unified ColorTolerance pattern accepts decimals under
    // rc-4; the same value is still rejected under rc-3 — proving the
    // selectable schema is a real behavioural difference, not a no-op.
    let base = slv_product_xml();
    assert!(
        base.contains("<InitialColorTolerance>3 SDCM</InitialColorTolerance>"),
        "fixture precondition: SLV carries an integer SDCM value"
    );
    for decimal in ["1.5 SDCM", "1.47 SDCM", "6.5 SDCM"] {
        let patched = base.replace(
            "<InitialColorTolerance>3 SDCM</InitialColorTolerance>",
            &format!("<InitialColorTolerance>{decimal}</InitialColorTolerance>"),
        );
        let rc4 = xsd_validate_product_xml_with_schema(&patched, GldfSchemaVersion::Rc4)
            .expect("rc4 validate");
        assert!(
            rc4.is_empty(),
            "rc.4 should accept {decimal:?}; got: {rc4:?}"
        );

        let rc3 = xsd_validate_product_xml_with_schema(&patched, GldfSchemaVersion::Rc3)
            .expect("rc3 validate");
        assert!(
            !rc3.is_empty(),
            "rc-3 must reject decimal SDCM {decimal:?} (integer enum only)"
        );
    }
}

#[test]
fn rc4_xsd_declares_new_control_protocols() {
    // Feature 2: the six new Interface protocols are in the rc-4 schema.
    // (An instance-level positive would need a ControlGear with Interfaces,
    // which the SLV fixture lacks; the enum text is the schema-level proof.)
    for proto in [
        "Zigbee",
        "Matter",
        "Thread",
        "Bluetooth Mesh",
        "DALI-2",
        "DALI D4i",
    ] {
        assert!(
            RC4_XSD.contains(&format!("<xs:enumeration value=\"{proto}\"")),
            "rc.4 XSD should enumerate the {proto:?} control protocol"
        );
    }
}

#[test]
fn rc4_xsd_declares_decimal_colortolerance_pattern() {
    // Feature 4, schema-level: the eight inline SDCM enums collapse into
    // one pattern-restricted `ColorTolerance` type.
    assert!(
        RC4_XSD.contains("<xs:simpleType name=\"ColorTolerance\">"),
        "rc.4 should define a unified ColorTolerance type"
    );
    assert!(
        RC4_XSD.contains(r#"<xs:pattern value="[0-9](\.[0-9]{1,2})? SDCM"/>"#),
        "rc.4 ColorTolerance should use the decimal-SDCM pattern"
    );
}

#[test]
fn rc4_xsd_has_corrected_spellings_and_per_channel_power() {
    // Features 1 + 3, schema-level.
    assert!(
        RC4_XSD.contains("RatedChromaticityCoordinateValues"),
        "rc.4 should carry the corrected chromaticity spelling"
    );
    assert!(
        RC4_XSD.contains("Fluorescent Triphosphor"),
        "rc.4 should carry the corrected Fluorescent enum spelling"
    );
    assert!(
        RC4_XSD.contains("RatedInputPower"),
        "rc.4 should add per-channel RatedInputPower"
    );
}
