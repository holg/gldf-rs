//! Wizard-built GLDFs must carry the two XSD-required Header fields
//! `UniqueGldfId` and `GldfCreationTimeCode`. Without this guarantee,
//! every fresh export from gldf.icu would fail validation in tools
//! that check rc-3 (gldf-search.de, gldf-validator).
//!
//! The lib's serializer intentionally doesn't auto-stamp these fields
//! (that would break round-trip stability — see
//! `roundtrip_fixtures.rs`). Instead the wizard's `apply_wizard` calls
//! `Header::ensure_xsd_required` once at creation time. This test
//! pins that contract.

use gldf_rs_editor::{apply_wizard, GldfState, WizardDraft, WizardLightSourceKind};

fn minimal_draft() -> WizardDraft {
    WizardDraft {
        header_author: "tester".into(),
        header_manufacturer: "ACME".into(),
        header_created_with: "test".into(),
        header_default_language: "en".into(),
        photometry_bytes: None,
        photometry_filename: String::new(),
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "LED".into(),
        ls_manufacturer: String::new(),
        ls_rated_power: String::new(),
        emitter_draft: None,
        variant_name: String::new(),
        geometry_bytes: None,
        geometry_filename: String::new(),
        driver_enabled: false,
        driver_name: String::new(),
        driver_dimmable: false,
        driver_standby_power: String::new(),
        mounting_type: String::new(),
        mounting_recessed_depth_mm: String::new(),
        mounting_pendant_length_mm: String::new(),
        mounting_height_mm: String::new(),
        mounting_pole_height_mm: String::new(),
        sensor_bytes: None,
        sensor_filename: String::new(),
        emergency_bytes: None,
        emergency_filename: String::new(),
    }
}

#[test]
fn wizard_stamps_unique_gldf_id_and_creation_time() {
    let mut state = GldfState::default();
    // Before: both empty/None.
    assert!(state.product.header.unique_gldf_id.is_none());
    assert!(state.product.header.creation_time_code.is_empty());

    apply_wizard(&mut state, &minimal_draft());

    let h = &state.product.header;
    let id = h
        .unique_gldf_id
        .as_deref()
        .expect("UniqueGldfId stamped by wizard");
    assert!(
        id.len() == 36 && id.chars().filter(|c| *c == '-').count() == 4,
        "UniqueGldfId should be UUIDv4-shaped (8-4-4-4-12), got {id:?}"
    );
    let ts = &h.creation_time_code;
    assert!(
        ts.starts_with("20") && ts.contains('T'),
        "GldfCreationTimeCode should be an ISO 8601 timestamp, got {ts:?}"
    );
}

#[test]
fn wizard_does_not_stamp_optional_mountings() {
    // The rc-3 XSD declares `Variant > Mountings` as `minOccurs="0"`.
    // We must NOT stamp a default Mountings on wizard-created variants
    // — the user picks it through the per-variant Edit modal when their
    // physical fixture actually has one. Anything else makes the
    // wizard opinionated about luminaire type, which it must not be.
    let mut state = GldfState::default();
    apply_wizard(&mut state, &minimal_draft());

    let variants = state
        .product
        .product_definitions
        .variants
        .as_ref()
        .expect("wizard creates a Variants block");
    assert_eq!(variants.variant.len(), 1);
    assert!(
        variants.variant[0].mountings.is_none(),
        "wizard must leave Mountings empty — XSD says minOccurs=0, user owns it"
    );
}

#[test]
fn wizard_preserves_existing_unique_gldf_id() {
    // If the caller already populated the field, the wizard must NOT
    // overwrite it. This protects callers that bootstrap state from
    // a known id (e.g. a server-side template).
    let mut state = GldfState::default();
    state.product.header.unique_gldf_id = Some("preset-uuid-xxxx".to_string());

    apply_wizard(&mut state, &minimal_draft());

    assert_eq!(
        state.product.header.unique_gldf_id.as_deref(),
        Some("preset-uuid-xxxx"),
        "wizard should not overwrite a preset UniqueGldfId"
    );
}
