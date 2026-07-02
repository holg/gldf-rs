//! Build the same artifact the slv_tria_2_end_to_end test produces,
//! and write it to tmp/SLV-Tria-2_RAL3020.gldf for manual inspection
//! in the WASM viewer.

use gldf_rs::GldfProduct;
use gldf_rs_editor::{GldfAction, GldfState, Scope};
use std::collections::HashMap;

fn main() {
    let gldf_bytes = std::fs::read("tests/data/SLV - Tria 2.gldf")
        .expect("SLV Tria 2 fixture missing");

    let bundle = GldfProduct::load_gldf_from_buf_all(gldf_bytes).expect("load");
    let mut embedded_files: HashMap<String, Vec<u8>> = HashMap::new();
    for f in bundle.files {
        if let Some(content) = f.content {
            let key = f
                .file_id
                .or_else(|| f.path)
                .or_else(|| f.name)
                .expect("file has key");
            embedded_files.insert(key, content);
        }
    }

    let mut state = GldfState::default();
    state.apply(GldfAction::LoadWithFiles {
        product: bundle.gldf,
        embedded_files,
    });

    state.apply(GldfAction::AddHousingColor(Scope::Variant(0)));
    state.apply(GldfAction::UpdateHousingColor {
        scope: Scope::Variant(0),
        index: 0,
        ral: Some(3020),
        language: "en".to_string(),
        value: "Traffic red".to_string(),
    });
    assert!(state.last_error.is_none(), "{:?}", state.last_error);

    let saved = state.save_to_buf().expect("save");
    std::fs::write("tmp/SLV-Tria-2_RAL3020.gldf", &saved).expect("write");
    println!("Wrote tmp/SLV-Tria-2_RAL3020.gldf ({} bytes)", saved.len());
}
