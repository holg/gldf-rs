//! End-to-end test using the bundled SLV Tria 2 fixture — the file
//! the user reported the housing-color L3D sync failed on.
//!
//! Reproduces exactly what the WASM viewer does:
//! 1. Read the .gldf bytes from disk
//! 2. Load via `GldfProduct::load_gldf_from_buf_all` and the same
//!    file_id resolution the viewer does in `try_load`
//! 3. Dispatch `LoadWithFiles` to a fresh `GldfState`
//! 4. Dispatch `UpdateHousingColor` at `Scope::Variant(0)` with RAL 3020
//! 5. Save the state back via `save_to_buf`
//! 6. Re-open the bytes and inspect:
//!    - the L3D is still present in the zip
//!    - the L3D now contains an MTL (synthesized — SLV Tria 2 ships
//!      with 3 OBJs and 0 MTLs per `unzip -l`)
//!    - the MTL's `Kd` matches RAL 3020 sRGB
//!
//! If this passes, the bug is in the WASM UI (scope picker default,
//! L3D viewer not reacting to bytes change, …) and NOT in the lib.

use gldf_rs::GldfProduct;
use gldf_rs_editor::{GldfAction, GldfState, Scope};
use l3d_rs::l3d::{BufFile, L3dFile};
use std::collections::HashMap;
use std::io::Read;

const SLV_TRIA_2_PATH: &str = "../../tests/data/SLV - Tria 2.gldf";

fn parse_l3d_from_bytes(bytes: &[u8]) -> L3dFile {
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor).unwrap();
    let mut file = L3dFile::default();
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

#[test]
fn slv_tria_2_housing_color_update_survives_round_trip() {
    let gldf_bytes = std::fs::read(SLV_TRIA_2_PATH).expect("SLV Tria 2 fixture missing");

    // --- Step 1+2: load like the WASM viewer does ---
    let bundle = GldfProduct::load_gldf_from_buf_all(gldf_bytes.clone())
        .expect("load_gldf_from_buf_all");
    let mut embedded_files: HashMap<String, Vec<u8>> = HashMap::new();
    for f in bundle.files {
        if let Some(content) = f.content {
            let key = f
                .file_id
                .clone()
                .or_else(|| f.path.clone())
                .or_else(|| f.name.clone())
                .expect("file has at least one key");
            embedded_files.insert(key, content);
        }
    }

    assert!(
        embedded_files.contains_key("geometryFile"),
        "embedded_files should index the L3D under its file_id 'geometryFile'; got keys: {:?}",
        embedded_files.keys().collect::<Vec<_>>()
    );
    let original_l3d_bytes = embedded_files.get("geometryFile").unwrap().clone();

    // Sanity: the original L3D has 3 OBJs and 0 MTLs.
    let original_l3d = parse_l3d_from_bytes(&original_l3d_bytes);
    let obj_count = original_l3d
        .assets
        .iter()
        .filter(|a| a.name.to_ascii_lowercase().ends_with(".obj"))
        .count();
    let mtl_count = original_l3d.mtl_paths().len();
    assert_eq!(obj_count, 3, "expected 3 OBJs in SLV Tria 2 baseline");
    assert_eq!(mtl_count, 0, "expected 0 MTLs in SLV Tria 2 baseline");

    // --- Step 3: dispatch LoadWithFiles ---
    let mut state = GldfState::default();
    state.apply(GldfAction::LoadWithFiles {
        product: bundle.gldf,
        embedded_files,
    });
    assert!(state.last_error.is_none(), "{:?}", state.last_error);
    assert!(
        state.embedded_files.contains_key("geometryFile"),
        "L3D still in state after LoadWithFiles"
    );

    // --- Step 4: user sets RAL 3020 (Traffic red) on Variant(0) ---
    // Variant 0 needs a HousingColor entry to update; SLV Tria 2 ships
    // without one, so we add it first the same way the UI does.
    state.apply(GldfAction::AddHousingColor(Scope::Variant(0)));
    state.apply(GldfAction::UpdateHousingColor {
        scope: Scope::Variant(0),
        index: 0,
        ral: Some(3020),
        language: "en".to_string(),
        value: "Traffic red".to_string(),
    });
    assert!(
        state.last_error.is_none(),
        "no error expected after UpdateHousingColor, got {:?}",
        state.last_error
    );

    // --- Step 5: re-pack the GLDF ---
    let saved_bytes = state.save_to_buf().expect("save_to_buf");

    // --- Step 6a: the L3D is still present ---
    let saved_bundle =
        GldfProduct::load_gldf_from_buf_all(saved_bytes).expect("re-load saved bytes");
    let saved_files: HashMap<String, Vec<u8>> = saved_bundle
        .files
        .into_iter()
        .filter_map(|f| {
            let content = f.content?;
            let key = f.file_id.or(f.path).or(f.name)?;
            Some((key, content))
        })
        .collect();
    assert!(
        saved_files.contains_key("geometryFile"),
        "L3D missing from saved GLDF — keys: {:?}",
        saved_files.keys().collect::<Vec<_>>()
    );

    // --- Step 6b: the L3D now contains an MTL ---
    let saved_l3d = parse_l3d_from_bytes(saved_files.get("geometryFile").unwrap());
    let saved_mtl_paths = saved_l3d.mtl_paths();
    assert!(
        !saved_mtl_paths.is_empty(),
        "expected at least one MTL in the saved L3D; got {} assets",
        saved_l3d.assets.len()
    );

    // --- Step 6c: an MTL exists with Kd matching RAL 3020 ---
    let expected_rgb = gldf_rs::ral::by_number(3020).unwrap().rgb_f32();
    let mut matched = false;
    for path in &saved_mtl_paths {
        let mtl = saved_l3d.mtl(path).unwrap().unwrap();
        for m in &mtl.materials {
            if let Some(kd) = m.kd {
                if (0..3).all(|i| (kd[i] - expected_rgb[i]).abs() < 1e-3) {
                    matched = true;
                    break;
                }
            }
        }
        if matched {
            break;
        }
    }
    assert!(
        matched,
        "no material in any saved MTL matches RAL 3020; expected Kd ≈ {:?}",
        expected_rgb
    );
}
