//! End-to-end export smoke tests: load a real demo GLDF from
//! `tests/data/`, run [`Exporter::export`], and assert the bundle layout
//! is what we expect.
//!
//! These tests are deliberately tolerant about *content* (the exact OBJ
//! payload, exact XML attribute order) — they assert the *shape* of the
//! output (files exist at the right paths, contains the expected element
//! names). Schema-level correctness against UE 5.4 is tracked by the
//! manual import checklist in `crates/gldf-unreal/README.md`.

use std::path::{Path, PathBuf};

use gldf_unreal::{ExportOptions, Exporter, UnitSystem, VariantSelector};
use tempfile::tempdir;

/// Recursively check whether any file under `dir` has the given extension.
fn walk_for_extension(dir: &Path, ext: &str) -> bool {
    let Ok(read) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in read.flatten() {
        let p = entry.path();
        if p.is_dir() {
            if walk_for_extension(&p, ext) {
                return true;
            }
        } else if p
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case(ext))
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

/// Path to the workspace-level fixture directory.
fn fixtures_dir() -> PathBuf {
    // `CARGO_MANIFEST_DIR` is set to crates/gldf-unreal/. Walk up two
    // levels to reach the workspace root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/data")
        .canonicalize()
        .expect("tests/data should exist at workspace root")
}

#[test]
fn alurays_3000mm_first_variant_emits_bundle() {
    let gldf = fixtures_dir().join("alurays-3000mm.gldf");
    if !gldf.exists() {
        eprintln!("skipping: {gldf:?} not present");
        return;
    }
    let out = tempdir().unwrap();

    let exporter = Exporter::from_path(&gldf).expect("load GLDF");
    let opts = ExportOptions {
        out_dir: out.path().to_path_buf(),
        bundle_name: "alurays".into(),
        units: UnitSystem::Cm,
        variants: VariantSelector::First,
        embed_textures: false,
        apply_mounting: false,
        overwrite: true,
    };

    let report = exporter.export(&opts).expect("export should succeed");
    assert_eq!(
        report.bundles.len(),
        1,
        "VariantSelector::First should emit exactly one bundle"
    );

    let b = &report.bundles[0];
    assert!(b.udatasmith_path.is_file(), "udatasmith file written");
    assert!(
        b.udatasmith_path
            .to_string_lossy()
            .ends_with(".udatasmith"),
        "extension is .udatasmith"
    );
    // An .fbx asset was written under Assets/Geometry/. UE 5.7 Datasmith
    // accepts external FBX via its first-party FBX translator; the OBJ
    // path hung the importer in an infinite read loop, so we switched
    // to FBX (produced by l3d_rs's fbx-export feature).
    let geom_dir = b.udatasmith_path.parent().unwrap().join("Assets/Geometry");
    assert!(geom_dir.is_dir());
    let any_fbx = walk_for_extension(&geom_dir, "fbx");
    assert!(any_fbx, "an .fbx asset was written under Assets/Geometry/");

    // The .udatasmith mentions a StaticMesh referencing the OBJ and at
    // least one ActorLight with an IES ref (Phase 3).
    let xml = std::fs::read_to_string(&b.udatasmith_path).unwrap();
    assert!(xml.contains("<DatasmithUnrealScene>"));
    assert!(xml.contains("<StaticMesh "));
    assert!(xml.contains("Assets/Geometry/"));
    assert!(
        xml.contains("<ActorLight ") && xml.contains("type=\"Spot\""),
        "expected at least one spot ActorLight"
    );
    assert!(xml.contains("<Ies file=\"Assets/Ies/"), "expected IES ref");

    // The Assets/Ies/ directory holds at least one .ies file.
    let ies_dir = b.udatasmith_path.parent().unwrap().join("Assets/Ies");
    assert!(ies_dir.is_dir());
    let any_ies = walk_for_extension(&ies_dir, "ies");
    assert!(any_ies, "an .ies asset was written under Assets/Ies/");
}

#[test]
fn variant_selector_only_unknown_id_errors() {
    let gldf = fixtures_dir().join("alurays-3000mm.gldf");
    if !gldf.exists() {
        eprintln!("skipping: {gldf:?} not present");
        return;
    }
    let out = tempdir().unwrap();
    let exporter = Exporter::from_path(&gldf).unwrap();
    let opts = ExportOptions {
        out_dir: out.path().to_path_buf(),
        bundle_name: "alurays".into(),
        variants: VariantSelector::Only(vec!["definitely-not-a-real-variant".into()]),
        overwrite: true,
        ..Default::default()
    };
    let err = exporter.export(&opts).unwrap_err();
    assert_eq!(err.code(), 3, "VariantNotFound error code is 3");
}
