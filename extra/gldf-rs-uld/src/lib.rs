//! DIALux `.uld` input for the gldf-rs batch pipeline (local builds only,
//! see Cargo.toml). Call [`register`] once at start; from then on
//! `gldf_rs_editor::process_one`, `collect_inputs` and `run_batch` accept
//! `.uld` containers.

use gldf_rs_editor::ConvertedContainer;

/// Register the ULD converter with gldf-rs-editor.
pub fn register() {
    gldf_rs_editor::register_container_converter("uld", convert);
}

/// A `.uld` container (by content or by name) converted to GLDF.
fn convert(bytes: &[u8], filename: &str) -> Option<anyhow::Result<ConvertedContainer>> {
    if !(light_convert::is_uld(bytes) || filename.to_ascii_lowercase().ends_with(".uld")) {
        return None;
    }
    Some(
        light_convert::convert_to_gldf_full(bytes, Some(filename), None)
            .map(|conv| ConvertedContainer {
                gldf_bytes: conv.gldf_bytes,
                product_name: conv.product_name,
                manufacturer: conv.manufacturer,
                article_number: conv.article_number,
            })
            .map_err(|e| anyhow::anyhow!("{e}")),
    )
}
