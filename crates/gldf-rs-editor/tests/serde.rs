//! Serde round-trip tests for the `GldfAction` JSON surface.
//!
//! These tests are the contract between Rust and the external world
//! (Python, CLI, future FFI consumers). If a variant fails to
//! serialize or deserialize, the JSON action surface is broken for
//! that variant — catch it here before consumers do.
//!
//! `GldfAction` is not `PartialEq` (some payloads contain non-Eq
//! types), so each test asserts on the parsed variant's fields
//! individually after the round-trip.

use gldf_rs_editor::{GldfAction, LightEmitterKind, Scope};

fn roundtrip(action: &GldfAction) -> GldfAction {
    let json = serde_json::to_string(action).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
}

#[test]
fn set_author_roundtrips() {
    let action = GldfAction::SetAuthor("Acme Engineering".to_string());
    let parsed = roundtrip(&action);
    match parsed {
        GldfAction::SetAuthor(s) => assert_eq!(s, "Acme Engineering"),
        other => panic!("variant mismatch: {:?}", other),
    }
}

#[test]
fn set_manufacturer_roundtrips() {
    let action = GldfAction::SetManufacturer("Acme".to_string());
    match roundtrip(&action) {
        GldfAction::SetManufacturer(s) => assert_eq!(s, "Acme"),
        other => panic!("variant mismatch: {:?}", other),
    }
}

#[test]
fn add_photometry_file_roundtrips_bytes() {
    let action = GldfAction::AddPhotometryFile {
        bytes: b"IESNA:LM-63-2002\nfake\n".to_vec(),
        filename: "fake.ies".to_string(),
    };
    match roundtrip(&action) {
        GldfAction::AddPhotometryFile { bytes, filename } => {
            assert_eq!(filename, "fake.ies");
            assert_eq!(&bytes[..], b"IESNA:LM-63-2002\nfake\n");
        }
        other => panic!("variant mismatch: {:?}", other),
    }
}

#[test]
fn add_fixed_light_source_roundtrips_boxed_payload() {
    use gldf_rs::gldf::general_definitions::lightsources::FixedLightSource;
    let action = GldfAction::AddFixedLightSource {
        payload: Box::new(FixedLightSource::default()),
    };
    match roundtrip(&action) {
        GldfAction::AddFixedLightSource { payload: _ } => { /* ok */ }
        other => panic!("variant mismatch: {:?}", other),
    }
}

#[test]
fn remove_light_emitter_roundtrips_kind_enum() {
    let action = GldfAction::RemoveLightEmitter {
        emitter_id: "e1".to_string(),
        kind: LightEmitterKind::MultiChannel,
        index: 2,
    };
    match roundtrip(&action) {
        GldfAction::RemoveLightEmitter {
            emitter_id,
            kind,
            index,
        } => {
            assert_eq!(emitter_id, "e1");
            assert!(matches!(kind, LightEmitterKind::MultiChannel));
            assert_eq!(index, 2);
        }
        other => panic!("variant mismatch: {:?}", other),
    }
}

#[test]
fn rename_fixed_light_source_roundtrips() {
    let action = GldfAction::RenameFixedLightSource {
        old_id: "ls_1".to_string(),
        new_id: "ls_main".to_string(),
    };
    match roundtrip(&action) {
        GldfAction::RenameFixedLightSource { old_id, new_id } => {
            assert_eq!(old_id, "ls_1");
            assert_eq!(new_id, "ls_main");
        }
        other => panic!("variant mismatch: {:?}", other),
    }
}

#[test]
fn add_variant_simple_roundtrips() {
    let action = GldfAction::AddVariantSimple {
        name: "Variant A".to_string(),
    };
    match roundtrip(&action) {
        GldfAction::AddVariantSimple { name } => assert_eq!(name, "Variant A"),
        other => panic!("variant mismatch: {:?}", other),
    }
}

#[test]
fn scope_enum_roundtrips_both_variants() {
    let p = serde_json::to_string(&Scope::Product).expect("serialize Product");
    let v = serde_json::to_string(&Scope::Variant(3)).expect("serialize Variant");
    let p_back: Scope = serde_json::from_str(&p).expect("parse Product");
    let v_back: Scope = serde_json::from_str(&v).expect("parse Variant");
    assert!(matches!(p_back, Scope::Product));
    assert!(matches!(v_back, Scope::Variant(3)));
}

#[test]
fn light_emitter_kind_roundtrips() {
    for kind in [
        LightEmitterKind::Fixed,
        LightEmitterKind::Changeable,
        LightEmitterKind::MultiChannel,
        LightEmitterKind::Sensor,
    ] {
        let json = serde_json::to_string(&kind).expect("serialize");
        let back: LightEmitterKind = serde_json::from_str(&json).expect("deserialize");
        // Use Debug equality since the enum derives Debug.
        assert_eq!(format!("{:?}", kind), format!("{:?}", back));
    }
}

#[test]
fn tagged_envelope_is_stable_for_consumers() {
    // The JSON envelope shape that Python / CLI / JS consumers depend
    // on: `{"action": "<variant>", "data": ...}`. If this assertion
    // breaks, downstream consumers must update — coordinate the
    // change.
    let action = GldfAction::SetAuthor("x".to_string());
    let json = serde_json::to_string(&action).expect("serialize");
    assert!(
        json.contains("\"action\":\"SetAuthor\""),
        "missing tag: {}",
        json
    );
    assert!(json.contains("\"data\":\"x\""), "missing data: {}", json);
}
