//! Contract test for the graphical LEO→photometry assignment editor.
//!
//! The `LeoPlanPicker` component (in gldf-rs-wasm) turns a click on a
//! LEO footprint into a merged `SetVariantGeometryModel` whose
//! `ModelEmitterBinding.external_name` is the LEO's part name. This test
//! drives that exact action sequence at the reducer level (no Leptos)
//! and proves:
//!   1. the resulting `EmitterReference` carries the real LEO name
//!      (`LEO0`/`LEO1`) — NOT the old hard-coded `"Primary"`;
//!   2. binding a second LEO does not clobber the first (merge);
//!   3. the product round-trips rc-3 XSD-valid.

use gldf_rs::gldf::general_definitions::lightsources::FixedLightSource;
use gldf_rs_editor::{GldfAction, GldfState, ModelEmitterBinding};

/// Seed a minimal XSD-valid product (header id + creation time +
/// ProductNumber, a light source with RatedInputPower, an emitter, a
/// variant) and import `example_007.l3d` so a ModelGeometry exists.
/// Returns (state, geometry_id, emitter_id).
fn seed_with_l3d() -> (GldfState, String, String) {
    let mut s = GldfState::default();
    s.product.header.ensure_xsd_required();
    {
        use gldf_rs::gldf::product_definitions::ProductMetaData;
        s.product
            .product_definitions
            .product_meta_data
            .get_or_insert_with(ProductMetaData::default)
            .ensure_xsd_required();
    }
    s.apply(GldfAction::SetAuthor("Tester".into()));

    // A light source with the XSD-required RatedInputPower.
    let mut ls = FixedLightSource::default();
    ls.rated_input_power = Some(30.0);
    s.apply(GldfAction::AddFixedLightSource {
        payload: Box::new(ls),
    });
    let ls_id = s
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .unwrap()
        .fixed_light_source[0]
        .id
        .clone();

    // A photometry (the emitter must reference one for an XSD-valid
    // FixedLightEmitter).
    let ldt_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/test-create/middle.ldt"
    );
    let ldt = std::fs::read(ldt_path).expect("read middle.ldt");
    s.apply(GldfAction::AddPhotometryFile {
        bytes: ldt,
        filename: "middle.ldt".into(),
    });
    let photometry_id = s
        .product
        .general_definitions
        .photometries
        .as_ref()
        .unwrap()
        .photometry[0]
        .id
        .clone();

    // An emitter bound to that light source + photometry, with a flux
    // (XSD requires RatedLuminousFlux on FixedLightEmitter).
    s.apply(GldfAction::AddFixedEmitter {
        emitter_id: String::new(),
        child_name: Some("LEO0".into()),
        photometry_id,
        light_source_id: ls_id,
        rated_luminous_flux: 1600,
    });
    let emitter_id = s
        .product
        .general_definitions
        .emitters
        .as_ref()
        .unwrap()
        .emitter[0]
        .id
        .clone();

    s.apply(GldfAction::AddVariantSimple {
        name: "Standard".into(),
    });

    // Import the demo L3D → registers a File + ModelGeometry.
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/test-create/example_007.l3d"
    );
    let bytes = std::fs::read(path).expect("read example_007.l3d");
    s.apply(GldfAction::ImportL3dFile {
        bytes,
        filename: "example_007.l3d".into(),
    });
    let geometry_id = s
        .product
        .general_definitions
        .geometries
        .as_ref()
        .unwrap()
        .model_geometry[0]
        .id
        .clone();

    (s, geometry_id, emitter_id)
}

#[test]
fn leo_binding_uses_real_leo_name_and_is_xsd_valid() {
    let (mut s, geometry_id, emitter_id) = seed_with_l3d();

    // Simulate two LEO clicks: bind LEO0 and LEO1 to the same emitter.
    // The picker always sends the FULL merged binding list.
    s.apply(GldfAction::SetVariantGeometryModel {
        variant_idx: 0,
        geometry_id: geometry_id.clone(),
        bindings: vec![ModelEmitterBinding {
            emitter_id: emitter_id.clone(),
            external_name: "LEO0".into(),
            target_model_type: Some("l3d".into()),
        }],
    });
    // Second click merges LEO1 without dropping LEO0.
    s.apply(GldfAction::SetVariantGeometryModel {
        variant_idx: 0,
        geometry_id: geometry_id.clone(),
        bindings: vec![
            ModelEmitterBinding {
                emitter_id: emitter_id.clone(),
                external_name: "LEO0".into(),
                target_model_type: Some("l3d".into()),
            },
            ModelEmitterBinding {
                emitter_id: emitter_id.clone(),
                external_name: "LEO1".into(),
                target_model_type: Some("l3d".into()),
            },
        ],
    });
    assert!(s.last_error.is_none(), "{:?}", s.last_error);

    // The EmitterReferences carry the real LEO names — gap closed.
    let v = &s.product.product_definitions.variants.as_ref().unwrap().variant[0];
    let refs = &v
        .geometry
        .as_ref()
        .unwrap()
        .model_geometry_reference
        .as_ref()
        .unwrap()
        .emitter_reference;
    let names: Vec<&str> = refs
        .iter()
        .map(|r| r.emitter_object_external_name.as_str())
        .collect();
    assert!(names.contains(&"LEO0"), "LEO0 bound, got {names:?}");
    assert!(names.contains(&"LEO1"), "LEO1 bound, got {names:?}");
    assert!(
        !names.contains(&"Primary"),
        "no hard-coded 'Primary' — the gap is closed"
    );

    // rc-3 XSD-valid (editor dev-dep enables the `xsd` feature).
    let xml = s.product.to_xml().expect("serialize");
    let findings =
        gldf_rs::validation::xsd_validate_product_xml(&xml).expect("validator runs");
    let errors: Vec<String> = findings.iter().map(|f| format!("{f:?}")).collect();
    assert!(
        errors.is_empty(),
        "LEO-bound product must be XSD-valid, got {} finding(s):\n{}",
        errors.len(),
        errors.join("\n")
    );
}
