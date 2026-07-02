//! Test T4: XSD validation surface.
//!
//! Canonical fixtures (`SLV - Tria 2.gldf`, `spctral_gldf-example.gldf`)
//! must validate clean against the rc-3 XSD right out of the archive.
//! The wizard-bootstrapped-Header test lives in the editor crate (see
//! `gldf-rs-editor/tests/wizard_xsd_clean.rs`) because that's where the
//! wizard apply path is.
//!
//! Run:
//!   cargo test -p gldf-rs --features xsd --test xsd_validate

#![cfg(feature = "xsd")]

use std::path::PathBuf;

use gldf_rs::validation::xsd_validate_gldf_archive;

fn fixture(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../tests/data");
    p.push(name);
    p
}

#[test]
fn canonical_fixtures_validate_clean() {
    for name in ["SLV - Tria 2.gldf", "spctral_gldf-example.gldf"] {
        let bytes = std::fs::read(fixture(name)).expect("read fixture");
        let errs = xsd_validate_gldf_archive(&bytes).expect("validate");
        if !errs.is_empty() {
            for e in &errs {
                eprintln!("  {}: {}", name, e);
            }
        }
        assert!(
            errs.is_empty(),
            "{name} produced {} XSD violation(s) — see stderr",
            errs.len()
        );
    }
}
