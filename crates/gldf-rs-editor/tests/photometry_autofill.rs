//! Auto-fill of `DescriptivePhotometry` from a dropped LDT/IES file.
//! The wizard's photometry step + the Files page drop handler dispatch
//! `AddPhotometryFile { bytes, filename }`. After the file embed, the
//! reducer parses the bytes via eulumdat / IesParser and populates the
//! six derivable summary fields so the user doesn't have to re-enter
//! them. Existing user overrides are preserved (the helper only
//! writes `None` fields).

use gldf_rs_editor::{GldfAction, GldfState};

#[test]
fn drop_ldt_populates_descriptive_photometry() {
    let ldt_bytes = include_bytes!("../../../tests/test-create/road.ldt").to_vec();
    let mut state = GldfState::default();
    state.apply(GldfAction::AddPhotometryFile {
        bytes: ldt_bytes,
        filename: "road.ldt".to_string(),
    });
    assert!(state.last_error.is_none(), "{:?}", state.last_error);

    let photometries = state
        .product
        .general_definitions
        .photometries
        .as_ref()
        .expect("photometries created");
    assert_eq!(photometries.photometry.len(), 1);
    let dp = photometries.photometry[0]
        .descriptive_photometry
        .as_ref()
        .expect("DescriptivePhotometry block auto-filled");

    // The four ratios should all be in [0, 100] or [0, 1] depending on
    // eulumdat's convention. Just assert they're set and finite.
    assert!(dp.light_output_ratio.is_some(), "LOR populated");
    assert!(dp.downward_light_output_ratio.is_some(), "DLOR populated");
    assert!(dp.upward_light_output_ratio.is_some(), "ULOR populated");
    assert!(dp.cie_flux_code.is_some(), "CIE flux code populated");
    // road.ldt has known luminous efficacy data; should be > 0.
    assert!(
        dp.luminous_efficacy.unwrap_or(0.0) > 0.0,
        "luminous_efficacy > 0"
    );
}

#[test]
fn drop_garbage_keeps_file_silently() {
    // The bytes don't parse as LDT or IES — the file should still be
    // embedded (the user dropped what they dropped) and the action is
    // reported as successful so apply_script doesn't abort. The auto-
    // fill failure is logged via `log::warn!` but does NOT set
    // `last_error` (that's reserved for hard failures that would
    // require user intervention).
    let mut state = GldfState::default();
    state.apply(GldfAction::AddPhotometryFile {
        bytes: b"this is definitely not a photometric file\n".to_vec(),
        filename: "garbage.ldt".to_string(),
    });
    // The File entry must still exist; we don't roll back the embed.
    let files = &state.product.general_definitions.files.file;
    assert_eq!(files.len(), 1, "file embedded despite parse failure");
    // DescriptivePhotometry should stay None / absent.
    let dp = state
        .product
        .general_definitions
        .photometries
        .as_ref()
        .and_then(|p| p.photometry.first())
        .and_then(|p| p.descriptive_photometry.as_ref());
    assert!(dp.is_none(), "no descriptive photometry on parse failure");
    // No last_error — the action is considered successful.
    assert!(
        state.last_error.is_none(),
        "no error surfaced: {:?}",
        state.last_error
    );
}

#[test]
fn auto_fill_does_not_overwrite_user_values() {
    // First drop fills the summary; subsequent manual edits stay even
    // if the same file is re-uploaded. (This test covers a single
    // drop; the no-overwrite guarantee is exercised at the lib helper
    // level via `dp.field.is_none()` checks.)
    let ldt_bytes = include_bytes!("../../../tests/test-create/road.ldt").to_vec();
    let mut state = GldfState::default();
    state.apply(GldfAction::AddPhotometryFile {
        bytes: ldt_bytes,
        filename: "road.ldt".to_string(),
    });

    // Manual override.
    state.apply(GldfAction::SetPhotometryLuminousEfficacy {
        index: 0,
        value: Some(123.456),
    });

    let dp = state
        .product
        .general_definitions
        .photometries
        .as_ref()
        .unwrap()
        .photometry[0]
        .descriptive_photometry
        .as_ref()
        .unwrap();
    assert_eq!(
        dp.luminous_efficacy,
        Some(123.456),
        "manual override survives"
    );
}
