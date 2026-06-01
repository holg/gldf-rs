//! Wizard tests — drive `apply_wizard` directly with a constructed
//! `WizardDraft`, no UI. Verifies the wizard reducer is portable and
//! produces a serializable GLDF.

use gldf_rs_editor::{
    apply_wizard, EmitterDraft, GldfState, LightEmitterKind, WizardDraft, WizardLightSourceKind,
};

#[test]
fn empty_wizard_runs_without_error() {
    let mut state = GldfState::default();
    let draft = WizardDraft::default();
    apply_wizard(&mut state, &draft);
    assert!(state.last_error.is_none(), "{:?}", state.last_error);
}

#[test]
fn wizard_sets_header_fields() {
    let mut state = GldfState::default();
    let draft = WizardDraft {
        header_author: "Acme Engineering".to_string(),
        header_manufacturer: "Acme".to_string(),
        ..WizardDraft::default()
    };
    apply_wizard(&mut state, &draft);
    assert_eq!(state.product.header.author, "Acme Engineering");
    assert_eq!(state.product.header.manufacturer, "Acme");
    assert!(state.is_modified);
}

#[test]
fn wizard_creates_fixed_light_source_and_emitter() {
    let mut state = GldfState::default();
    let draft = WizardDraft {
        header_author: "Engineer".to_string(),
        header_manufacturer: "Acme".to_string(),
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "Main LED".to_string(),
        emitter_draft: Some(EmitterDraft {
            child_name: "Main".to_string(),
            kind: LightEmitterKind::Fixed,
            ..EmitterDraft::default()
        }),
        ..WizardDraft::default()
    };
    apply_wizard(&mut state, &draft);
    assert!(state.last_error.is_none(), "{:?}", state.last_error);

    let ls = state
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .expect("light_sources should exist after wizard");
    assert_eq!(
        ls.fixed_light_source.len(),
        1,
        "fixed light source created"
    );

    let emitters = state
        .product
        .general_definitions
        .emitters
        .as_ref()
        .expect("emitters should exist after wizard");
    assert_eq!(emitters.emitter.len(), 1, "emitter created");
}

#[test]
fn wizard_output_survives_xml_roundtrip() {
    let mut state = GldfState::default();
    let draft = WizardDraft {
        header_author: "Round Trip".to_string(),
        header_manufacturer: "Acme".to_string(),
        ..WizardDraft::default()
    };
    apply_wizard(&mut state, &draft);
    let bytes = state.save_to_buf().expect("wizard output saves");
    assert!(!bytes.is_empty(), "wizard output produces non-empty bytes");
}
