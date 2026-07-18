//! rc.4 schema tests — validate the four PR #30 features against the
//! actual rc.4 XSD, fetched from the (still-open) upstream PR branch and
//! bundled at `tests/data/gldf-1.0.0-rc-4.xsd`.
//!
//! We compile the rc.4 XSD directly with uppsala (the same engine the
//! shipping rc-3 validator uses) so this works BEFORE gldf.io publishes
//! the versioned URL — the schema is public on the PR branch, just at the
//! flat `xsd/gldf.xsd` path rather than `/xsd/gldf/1.0.0-rc.4/gldf.xsd`.
//!
//! PR: globallightingdata/gldf#30 (holg/gldf @ typo-fixes). Features:
//!   1. spelling cleanup (RatedChromaticityCoordinateValues, Fluorescent…)
//!   2. six control protocols (Zigbee, Matter, Thread, Bluetooth Mesh,
//!      DALI-2, DALI D4i) + unbounded Interface maxOccurs
//!   3. per-channel RatedInputPower on MultiChannelLightSource
//!   4. decimal ColorTolerance pattern ("1.5 SDCM", "1.47 SDCM")
//!
//! Run: cargo test -p gldf-rs --features xsd --test xsd_rc4

#![cfg(feature = "xsd")]

use std::path::PathBuf;

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

fn rc4_validator() -> uppsala::XsdValidator {
    let doc = uppsala::parse(RC4_XSD).expect("parse rc.4 xsd");
    uppsala::XsdValidator::from_schema(&doc).expect("compile rc.4 xsd")
}

/// Validate `xml` against the rc.4 schema; returns the violation messages.
fn rc4_errors(v: &uppsala::XsdValidator, xml: &str) -> Vec<String> {
    let doc = uppsala::parse(xml).expect("parse product xml");
    v.validate(&doc).into_iter().map(|e| e.message).collect()
}

#[test]
fn rc4_xsd_compiles() {
    let _ = rc4_validator();
}

#[test]
fn slv_fixture_validates_clean_against_rc4() {
    // rc.4 is backward compatible: an rc-3-clean file stays clean.
    let v = rc4_validator();
    let xml = slv_product_xml();
    let errs = rc4_errors(&v, &xml);
    assert!(errs.is_empty(), "SLV should be rc.4-clean; got: {errs:?}");
}

#[test]
fn rc4_accepts_decimal_sdcm() {
    // Feature 4: the unified ColorTolerance pattern accepts decimals.
    let v = rc4_validator();
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
        let errs = rc4_errors(&v, &patched);
        assert!(
            errs.is_empty(),
            "rc.4 should accept decimal SDCM {decimal:?}; got: {errs:?}"
        );
    }
}

#[test]
fn rc3_rejects_decimal_sdcm_negative_control() {
    // Sanity: the SAME decimal value is NOT valid against the shipping
    // rc-3 validator (its ColorTolerance was a discrete integer enum), so
    // the rc.4 acceptance above is a real schema change, not a no-op.
    let base = slv_product_xml();
    let patched = base.replace(
        "<InitialColorTolerance>3 SDCM</InitialColorTolerance>",
        "<InitialColorTolerance>1.5 SDCM</InitialColorTolerance>",
    );
    let errs = gldf_rs::validation::xsd_validate_product_xml(&patched)
        .expect("rc-3 validate");
    assert!(
        !errs.is_empty(),
        "rc-3 must reject the decimal SDCM '1.5 SDCM' (enum was integers only)"
    );
}

#[test]
fn rc4_xsd_declares_new_control_protocols() {
    // Feature 2: the six new Interface protocols are in the schema's
    // enumeration. (An instance-level positive test would need a Control
    // Gear with Interfaces, which the SLV fixture lacks; asserting the
    // enum text is the schema-level proof.)
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
