//! Slice 3 end-to-end: drive `apply_wizard` from a `WizardDraft`, save
//! to GLDF bytes, reload via `EditableGldf::from_buf`, and verify every
//! field the wizard set survives the round-trip. This is the test that
//! gates the New GLDF wizard slice — the same draft → bytes → reload
//! contract the CLI / Python / TUI / WASM frontends rely on.

use gldf_rs::EditableGldf;
use gldf_rs_editor::{
    apply_wizard, EmitterDraft, GldfAction, GldfState, LightEmitterKind, WizardDraft,
    WizardLightSourceKind,
};

#[test]
fn full_wizard_run_roundtrips_through_gldf_bytes() {
    let draft = WizardDraft {
        header_author: "Slice 3 Test Engineer".to_string(),
        header_manufacturer: "Acme Lighting".to_string(),
        header_created_with: "wizard-roundtrip test".to_string(),
        header_default_language: "en".to_string(),
        photometry_bytes: None,
        photometry_filename: String::new(),
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "Main LED".to_string(),
        ls_manufacturer: "Acme Optics".to_string(),
        ls_rated_power: "42.5".to_string(),
        emitter_draft: Some(EmitterDraft {
            child_name: "Primary emitter".to_string(),
            kind: LightEmitterKind::Fixed,
            ..EmitterDraft::default()
        }),
    };

    // 1. Run the wizard against a fresh state — same code the CLI /
    //    Python / TUI / WASM frontends drive.
    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    assert!(state.last_error.is_none(), "{:?}", state.last_error);

    // 2. Serialize to GLDF bytes (the same call every Save button hits).
    let bytes = state.save_to_buf().expect("wizard output saves");
    assert!(!bytes.is_empty(), "non-empty GLDF bytes");

    // 3. Reload via `EditableGldf::from_buf` — the entry point every
    //    consumer uses to import a GLDF.
    let editable = EditableGldf::from_buf(bytes).expect("reload from bytes");

    // 4. Pump it back through the reducer the same way `gldf-cli dump`
    //    and the WASM file-open path do, to prove the loaded shape
    //    survives the action layer too.
    let mut reloaded = GldfState::default();
    reloaded.apply(GldfAction::LoadWithFiles {
        product: editable.product,
        embedded_files: editable.embedded_files,
    });

    // 5. Every field the wizard set is present after round-trip.
    let p = &reloaded.product;
    assert_eq!(p.header.author, "Slice 3 Test Engineer");
    assert_eq!(p.header.manufacturer, "Acme Lighting");
    assert_eq!(p.header.created_with_application, "wizard-roundtrip test");
    assert_eq!(p.header.default_language.as_deref(), Some("en"));

    let ls = p
        .general_definitions
        .light_sources
        .as_ref()
        .expect("light_sources present after reload");
    assert_eq!(ls.fixed_light_source.len(), 1);
    let fls = &ls.fixed_light_source[0];
    assert!(fls.id.starts_with("lightsource_"));
    assert_eq!(fls.manufacturer.as_deref(), Some("Acme Optics"));
    assert_eq!(fls.rated_input_power, Some(42.5));

    let emitters = p
        .general_definitions
        .emitters
        .as_ref()
        .expect("emitters present after reload");
    assert_eq!(emitters.emitter.len(), 1);
    assert_eq!(emitters.emitter[0].fixed_light_emitter.len(), 1);
}
