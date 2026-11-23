//! Integration test: dispatching `UpdateHousingColor` on a variant
//! that owns a `ModelGeometryReference` rewrites the L3D's MTL with
//! the matching RAL Classic sRGB. The MTL change is visible via the
//! editor's `save_to_buf` → re-load round-trip and inspectable
//! through `l3d_rs`.
//!
//! This test exists to prove the user-visible promise: drag the
//! Marketing → HousingColor RAL field, and the next time the L3D
//! viewer re-reads the bytes (which happens on every export and
//! every state-driven refresh), the housing is the new colour.

use gldf_rs::gldf::general_definitions::files::File as GldfFile;
use gldf_rs::gldf::general_definitions::geometries::{GeometryFileReference, ModelGeometry};
use gldf_rs::gldf::general_definitions::{Files, Geometries};
use gldf_rs::gldf::product_definitions::{
    DescriptiveAttributes, Geometry, HousingColor, HousingColors, Marketing,
    ModelGeometryReference, Variant, Variants,
};
use gldf_rs::gldf::GldfProduct;
use gldf_rs::gldf::LocaleFoo;
use gldf_rs_editor::{GldfAction, GldfState, Scope};
use l3d_rs::l3d::{BufFile, L3dFile};
use std::collections::HashMap;

fn make_l3d_with_mtl() -> Vec<u8> {
    let file = L3dFile {
        structure: "<Structure/>".to_string(),
        assets: vec![
            BufFile {
                name: "geom/luminaire.obj".to_string(),
                content: b"mtllib luminaire.mtl\nusemtl housing\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n".to_vec(),
                size: 0,
            },
            BufFile {
                name: "geom/luminaire.mtl".to_string(),
                content: b"newmtl housing\nKa 0.1 0.1 0.1\nKd 0.5 0.5 0.5\nKs 0.2 0.2 0.2\nNs 100\nd 1\nillum 2\n".to_vec(),
                size: 0,
            },
        ],
    };
    file.to_buffer().unwrap()
}

fn make_product_with_l3d_variant() -> (GldfProduct, HashMap<String, Vec<u8>>) {
    let mut p = GldfProduct::default();
    p.general_definitions.files = Files {
        file: vec![GldfFile {
            id: "l3d-1".to_string(),
            content_type: "geo/l3d".to_string(),
            type_attr: "localFileName".to_string(),
            language: String::new(),
            file_name: "luminaire.l3d".to_string(),
        }],
    };
    p.general_definitions.geometries = Some(Geometries {
        simple_geometry: vec![],
        model_geometry: vec![ModelGeometry {
            id: "geom-1".to_string(),
            geometry_file_reference: vec![GeometryFileReference {
                file_id: "l3d-1".to_string(),
                level_of_detail: None,
            }],
        }],
    });
    p.product_definitions.variants = Some(Variants {
        variant: vec![Variant {
            id: "v1".to_string(),
            geometry: Some(Geometry {
                emitter_reference: vec![],
                simple_geometry_reference: None,
                model_geometry_reference: Some(ModelGeometryReference {
                    geometry_id: "geom-1".to_string(),
                    ..Default::default()
                }),
            }),
            descriptive_attributes: Some(DescriptiveAttributes {
                marketing: Some(Marketing {
                    housing_colors: Some(HousingColors {
                        housing_color: vec![HousingColor {
                            ral: None,
                            locale: LocaleFoo::default(),
                        }],
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }],
    });

    let mut embedded = HashMap::new();
    embedded.insert("l3d-1".to_string(), make_l3d_with_mtl());
    (p, embedded)
}

#[test]
fn variant_scope_housing_color_update_rewrites_l3d_mtl() {
    let (product, embedded) = make_product_with_l3d_variant();
    let mut state = GldfState::default();
    state.apply(GldfAction::LoadWithFiles {
        product,
        embedded_files: embedded,
    });

    // Dispatch the action a user would dispatch by typing "3020"
    // (RAL Traffic red) into the variant's HousingColor field.
    state.apply(GldfAction::UpdateHousingColor {
        scope: Scope::Variant(0),
        index: 0,
        ral: Some(3020),
        language: "en".to_string(),
        value: "Traffic red".to_string(),
    });

    assert!(
        state.last_error.is_none(),
        "no error expected, got {:?}",
        state.last_error
    );

    // Now read the L3D bytes back and verify Kd matches RAL 3020.
    let new_bytes = state
        .embedded_files
        .get("l3d-1")
        .expect("L3D still embedded");
    let l3d = parse_l3d_for_test(new_bytes);
    let mtl = l3d.mtl("geom/luminaire.mtl").unwrap().unwrap();
    let housing = mtl.find("housing").expect("housing material survived");
    let kd = housing.kd.expect("Kd written");
    let expected = gldf_rs::ral::by_number(3020).unwrap().rgb_f32();
    for i in 0..3 {
        assert!(
            (kd[i] - expected[i]).abs() < 1e-3,
            "channel {}: got {}, expected {}",
            i,
            kd[i],
            expected[i]
        );
    }
}

#[test]
fn product_scope_housing_color_does_not_touch_l3d() {
    let (product, embedded) = make_product_with_l3d_variant();
    let original_bytes = embedded.get("l3d-1").unwrap().clone();
    let mut state = GldfState::default();
    state.apply(GldfAction::LoadWithFiles {
        product,
        embedded_files: embedded,
    });

    // Product-scope HousingColor is a catalog default; we deliberately
    // skip the L3D sync because the L3D belongs to a specific variant.
    state.apply(GldfAction::AddHousingColor(Scope::Product));
    state.apply(GldfAction::UpdateHousingColor {
        scope: Scope::Product,
        index: 0,
        ral: Some(3020),
        language: "en".to_string(),
        value: "Traffic red".to_string(),
    });

    assert!(state.last_error.is_none());
    // Embedded L3D unchanged.
    assert_eq!(
        state.embedded_files.get("l3d-1").unwrap(),
        &original_bytes,
        "product-scope sync should NOT touch the L3D"
    );
}

/// Re-parse a `.l3d` blob into an `L3dFile` without going through
/// `l3d_rs::from_buffer` (which doesn't expose IO errors). Mirrors the
/// private helper inside the lib.
fn parse_l3d_for_test(bytes: &[u8]) -> L3dFile {
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor).unwrap();
    let mut file = L3dFile::default();
    use std::io::Read;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).unwrap();
        if !entry.is_file() {
            continue;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).unwrap();
        let name = entry.name().to_string();
        if name == "structure.xml" {
            file.structure = String::from_utf8_lossy(&buf).into_owned();
        } else {
            let size = buf.len() as u64;
            file.assets.push(BufFile {
                name,
                content: buf,
                size,
            });
        }
    }
    file
}
