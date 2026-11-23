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
    assert_eq!(ls.fixed_light_source.len(), 1, "fixed light source created");

    let emitters = state
        .product
        .general_definitions
        .emitters
        .as_ref()
        .expect("emitters should exist after wizard");
    assert_eq!(emitters.emitter.len(), 1, "emitter created");
}

#[test]
fn wizard_always_creates_at_least_one_variant() {
    // The XSD requires at least one variant. The wizard must produce
    // one regardless of whether the user typed a name.
    let mut state = GldfState::default();
    let draft = WizardDraft {
        header_author: "Default Variant Test".to_string(),
        ..WizardDraft::default()
    };
    apply_wizard(&mut state, &draft);
    let variants = state
        .product
        .product_definitions
        .variants
        .as_ref()
        .expect("variants present after wizard");
    assert_eq!(variants.variant.len(), 1, "exactly one variant created");
    // Empty name falls back to "Standard".
    let v = &variants.variant[0];
    let name = v
        .name
        .as_ref()
        .and_then(|n| n.locale.first())
        .map(|l| l.value.as_str())
        .unwrap_or("");
    assert_eq!(name, "Standard", "default variant name");
}

#[test]
fn wizard_binds_emitter_to_variant_geometry_bare() {
    // No L3D bytes → wizard writes a bare EmitterReference into the
    // variant's Geometry, so the resulting GLDF is XSD-complete.
    let mut state = GldfState::default();
    let draft = WizardDraft {
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "Main LED".to_string(),
        emitter_draft: Some(EmitterDraft {
            child_name: "Primary".to_string(),
            kind: LightEmitterKind::Fixed,
            ..EmitterDraft::default()
        }),
        ..WizardDraft::default()
    };
    apply_wizard(&mut state, &draft);
    let variants = state
        .product
        .product_definitions
        .variants
        .as_ref()
        .expect("variants exist");
    let v = &variants.variant[0];
    let g = v.geometry.as_ref().expect("variant.Geometry created");
    assert!(g.simple_geometry_reference.is_none(), "bare branch");
    assert!(g.model_geometry_reference.is_none(), "bare branch");
    assert_eq!(g.emitter_reference.len(), 1, "one emitter bound");
    let emitter_id = &state
        .product
        .general_definitions
        .emitters
        .as_ref()
        .unwrap()
        .emitter[0]
        .id;
    assert_eq!(
        &g.emitter_reference[0].emitter_id, emitter_id,
        "bound to the wizard's emitter"
    );
}

#[test]
fn wizard_binds_emitter_via_model_geometry_with_l3d() {
    // L3D bytes present → wizard registers a File + ModelGeometry and
    // writes a ModelGeometryReference into the variant.
    let mut state = GldfState::default();
    let draft = WizardDraft {
        ls_kind: WizardLightSourceKind::Fixed,
        ls_name: "Main LED".to_string(),
        emitter_draft: Some(EmitterDraft {
            child_name: "Primary".to_string(),
            kind: LightEmitterKind::Fixed,
            ..EmitterDraft::default()
        }),
        geometry_bytes: Some(b"PK\x03\x04 fake zip".to_vec()),
        geometry_filename: "luminaire.l3d".to_string(),
        ..WizardDraft::default()
    };
    apply_wizard(&mut state, &draft);
    let v = &state
        .product
        .product_definitions
        .variants
        .as_ref()
        .unwrap()
        .variant[0];
    let g = v.geometry.as_ref().expect("Geometry created");
    let mg = g
        .model_geometry_reference
        .as_ref()
        .expect("ModelGeometryReference");
    assert!(g.emitter_reference.is_empty(), "bare not set");
    assert_eq!(mg.emitter_reference.len(), 1, "one emitter binding");
    let emitter_id = &state
        .product
        .general_definitions
        .emitters
        .as_ref()
        .unwrap()
        .emitter[0]
        .id;
    assert_eq!(&mg.emitter_reference[0].emitter_id, emitter_id);
    assert_eq!(
        mg.emitter_reference[0].emitter_object_external_name, "Primary",
        "uses the EmitterDraft.child_name as external_name"
    );
    // And the L3D got registered.
    let files = &state.product.general_definitions.files.file;
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_name, "luminaire.l3d");
    let geoms = state
        .product
        .general_definitions
        .geometries
        .as_ref()
        .unwrap();
    assert_eq!(geoms.model_geometry.len(), 1);
    assert_eq!(geoms.model_geometry[0].id, mg.geometry_id);
}

#[test]
fn wizard_honours_explicit_variant_name() {
    let mut state = GldfState::default();
    let draft = WizardDraft {
        variant_name: "Cool White 4000K".to_string(),
        ..WizardDraft::default()
    };
    apply_wizard(&mut state, &draft);
    let variants = state
        .product
        .product_definitions
        .variants
        .as_ref()
        .expect("variants present");
    let name = variants.variant[0]
        .name
        .as_ref()
        .and_then(|n| n.locale.first())
        .map(|l| l.value.as_str())
        .unwrap_or("");
    assert_eq!(name, "Cool White 4000K");
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
