//! Variant Geometry binding tests — covers the five new actions added
//! in the Phase A variant-emitter binding cut. Each test seeds a state,
//! dispatches one or two actions, and asserts on the resulting
//! Variant.Geometry shape + xs:choice exclusivity.

use gldf_rs::gldf::general_definitions::lightsources::FixedLightSource;
use gldf_rs_editor::{GldfAction, GldfState, LightEmitterKind, ModelEmitterBinding};

fn seed_state_with_variant_and_emitter() -> GldfState {
    let mut s = GldfState::default();
    // One LightSource so AddEmitterWithChild has something to bind.
    s.apply(GldfAction::AddFixedLightSource {
        payload: Box::new(FixedLightSource::default()),
    });
    s.apply(GldfAction::AddEmitterWithChild {
        child_name: Some("Primary".into()),
        kind: LightEmitterKind::Fixed,
        photometry_id: String::new(),
        fixed_light_source_id: String::new(),
        multi_channel_light_source_id: String::new(),
        sensor_id: String::new(),
    });
    s.apply(GldfAction::AddVariantSimple {
        name: "Standard".into(),
    });
    s
}

fn first_emitter_id(s: &GldfState) -> String {
    s.product
        .general_definitions
        .emitters
        .as_ref()
        .unwrap()
        .emitter[0]
        .id
        .clone()
}

#[test]
fn set_emitter_refs_creates_bare_geometry() {
    let mut s = seed_state_with_variant_and_emitter();
    let eid = first_emitter_id(&s);
    s.apply(GldfAction::SetVariantGeometryEmitterRefs {
        variant_idx: 0,
        emitter_ids: vec![eid.clone()],
    });
    assert!(s.last_error.is_none(), "{:?}", s.last_error);
    let v = &s.product.product_definitions.variants.as_ref().unwrap().variant[0];
    let g = v.geometry.as_ref().expect("geometry created");
    assert_eq!(g.emitter_reference.len(), 1);
    assert_eq!(g.emitter_reference[0].emitter_id, eid);
    assert!(g.simple_geometry_reference.is_none(), "xs:choice — simple cleared");
    assert!(g.model_geometry_reference.is_none(), "xs:choice — model cleared");
}

#[test]
fn set_geometry_simple_overrides_emitter_refs() {
    let mut s = seed_state_with_variant_and_emitter();
    let eid = first_emitter_id(&s);
    // First set bare emitter refs, then switch to simple — xs:choice
    // means bare must be cleared.
    s.apply(GldfAction::SetVariantGeometryEmitterRefs {
        variant_idx: 0,
        emitter_ids: vec![eid.clone()],
    });
    s.apply(GldfAction::SetVariantGeometrySimple {
        variant_idx: 0,
        geometry_id: "simple_1".into(),
        emitter_id: eid.clone(),
    });
    let v = &s.product.product_definitions.variants.as_ref().unwrap().variant[0];
    let g = v.geometry.as_ref().unwrap();
    assert!(g.emitter_reference.is_empty(), "bare emitters cleared");
    let sg = g.simple_geometry_reference.as_ref().unwrap();
    assert_eq!(sg.geometry_id, "simple_1");
    assert_eq!(sg.emitter_id, eid);
    assert!(g.model_geometry_reference.is_none());
}

#[test]
fn set_geometry_model_holds_per_object_bindings() {
    let mut s = seed_state_with_variant_and_emitter();
    let eid = first_emitter_id(&s);
    s.apply(GldfAction::SetVariantGeometryModel {
        variant_idx: 0,
        geometry_id: "model_1".into(),
        bindings: vec![ModelEmitterBinding {
            emitter_id: eid.clone(),
            external_name: "LedChip01".into(),
            target_model_type: Some("l3d".into()),
        }],
    });
    let v = &s.product.product_definitions.variants.as_ref().unwrap().variant[0];
    let g = v.geometry.as_ref().unwrap();
    let mg = g.model_geometry_reference.as_ref().unwrap();
    assert_eq!(mg.geometry_id, "model_1");
    assert_eq!(mg.emitter_reference.len(), 1);
    let b = &mg.emitter_reference[0];
    assert_eq!(b.emitter_id, eid);
    assert_eq!(b.emitter_object_external_name, "LedChip01");
    assert_eq!(b.target_model_type.as_deref(), Some("l3d"));
}

#[test]
fn empty_emitter_refs_clears_geometry() {
    let mut s = seed_state_with_variant_and_emitter();
    let eid = first_emitter_id(&s);
    s.apply(GldfAction::SetVariantGeometryEmitterRefs {
        variant_idx: 0,
        emitter_ids: vec![eid],
    });
    s.apply(GldfAction::SetVariantGeometryEmitterRefs {
        variant_idx: 0,
        emitter_ids: vec![],
    });
    let v = &s.product.product_definitions.variants.as_ref().unwrap().variant[0];
    assert!(v.geometry.is_none(), "empty list clears Geometry entirely");
}

#[test]
fn clear_variant_geometry_drops_it() {
    let mut s = seed_state_with_variant_and_emitter();
    let eid = first_emitter_id(&s);
    s.apply(GldfAction::SetVariantGeometrySimple {
        variant_idx: 0,
        geometry_id: "g1".into(),
        emitter_id: eid,
    });
    s.apply(GldfAction::ClearVariantGeometry { variant_idx: 0 });
    let v = &s.product.product_definitions.variants.as_ref().unwrap().variant[0];
    assert!(v.geometry.is_none());
}

#[test]
fn import_l3d_file_registers_file_and_model_geometry() {
    let mut s = GldfState::default();
    let fake_l3d_bytes = b"PK\x03\x04 fake zip header".to_vec();
    s.apply(GldfAction::ImportL3dFile {
        bytes: fake_l3d_bytes.clone(),
        filename: "luminaire.l3d".into(),
    });
    assert!(s.last_error.is_none(), "{:?}", s.last_error);

    // The File entry must exist and reference an embedded payload.
    let files = &s.product.general_definitions.files.file;
    assert_eq!(files.len(), 1);
    let f = &files[0];
    assert_eq!(f.file_name, "luminaire.l3d");
    assert_eq!(f.content_type, "geo/l3d");
    assert!(
        s.embedded_files.contains_key(&f.id),
        "bytes embedded under file id"
    );
    assert_eq!(s.embedded_files.get(&f.id), Some(&fake_l3d_bytes));

    // And a matching ModelGeometry should reference the file id.
    let geoms = s.product.general_definitions.geometries.as_ref().unwrap();
    assert_eq!(geoms.model_geometry.len(), 1);
    let g = &geoms.model_geometry[0];
    assert_eq!(g.geometry_file_reference.len(), 1);
    assert_eq!(g.geometry_file_reference[0].file_id, f.id);
}

#[test]
fn variant_geometry_actions_out_of_range_set_last_error() {
    let mut s = GldfState::default();
    s.apply(GldfAction::SetVariantGeometryEmitterRefs {
        variant_idx: 99,
        emitter_ids: vec!["e1".into()],
    });
    assert!(s.last_error.is_some());
    assert!(!s.is_modified, "out-of-range should not flip is_modified");
}
