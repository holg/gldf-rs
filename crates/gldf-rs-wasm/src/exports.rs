//! Toolbar export helpers.
//!
//! Each function reads the current editor state, builds the export payload
//! (XML / JSON / GLDF / IFC), and triggers a browser download via the
//! Blob + `<a download>` click trick. Same as the Yew side's `Msg::Export*`
//! handlers, only the dispatch is direct (no message enum needed) because
//! every helper is called from the same Leptos signal context.

use crate::state::GldfContext;
use anyhow::{anyhow, Result};
use gldf_rs::GldfProduct;
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Export the current product as `product.xml`.
pub fn export_xml(state: GldfContext) -> Result<()> {
    let xml = state.with(|s| s.product.to_xml())?;
    download_text(&xml, "product.xml", "application/xml");
    Ok(())
}

/// Export the current product as `product.json` (pretty).
pub fn export_json(state: GldfContext) -> Result<()> {
    let json = state.with(|s| s.product.to_pretty_json())?;
    download_text(&json, "product.json", "application/json");
    Ok(())
}

/// Export the current product as a `.gldf` ZIP, via `EditableGldf::save_to_buf`.
///
/// This is the path the Builder uses too — `save_to_buf` writes `product.xml`
/// with the canonical online XSD and zips every embedded file under its
/// content-type-derived folder. Round-trip-safe.
pub fn export_gldf(state: GldfContext) -> Result<()> {
    let bytes = state.with(|s| s.save_to_buf())?;
    download_bytes(&bytes, "exported.gldf", "application/zip");
    Ok(())
}

/// Same as [`export_gldf`] but routes through
/// `save_to_buf_with_options`. Wired to the export-options dialog so
/// the user can opt into DIALux-compat / inline-spectrum / TM-33 merge
/// transforms. Filename is unchanged — the consumer is identical, only
/// the bytes shift.
pub fn export_gldf_with_options(
    state: GldfContext,
    opts: gldf_rs::editable::ExportOptions,
) -> Result<()> {
    let bytes = state.with(|s| s.save_to_buf_with_options(&opts))?;
    download_bytes(&bytes, "exported.gldf", "application/zip");
    Ok(())
}

/// Export an IFC representation of the current product.
///
/// Pulls the L3D mesh from `embedded_files` if present; otherwise emits IFC
/// with the symbolic luminaire only. Errors propagate so the caller can
/// surface them (slice 1 stubs to a log message).
pub fn export_ifc(state: GldfContext) -> Result<()> {
    use gldf_rs::ifc::GldfToIfc;
    let ifc = state
        .with(|s| {
            // GldfToIfc::export_with_mesh_and_files takes a &[BufFile]; we don't
            // have those on the Leptos side since the state model dropped the
            // legacy FileBufGldf in favor of (product, embedded_files). For slice
            // 1 we hand it the product only — geometry export falls back to the
            // symbolic luminaire, which still validates as IFC.
            GldfToIfc::export_with_mesh_and_files(&s.product, None, &[])
        })
        .map_err(|e| anyhow!("ifc export: {}", e))?;
    download_text(&ifc, "exported.ifc", "application/x-step");
    Ok(())
}

/// Download arbitrary bytes for a single embedded file id. Used by Files /
/// Variants pages when the user clicks a per-file download button.
pub fn download_embedded_file(state: GldfContext, file_id: &str) -> Result<()> {
    let (bytes, name) = state
        .with(|s| {
            let bytes = s.embedded_files.get(file_id).cloned()?;
            let name = s
                .product
                .general_definitions
                .files
                .file
                .iter()
                .find(|f| f.id == file_id)
                .map(|f| f.file_name.clone())
                .unwrap_or_else(|| file_id.to_string());
            Some((bytes, name))
        })
        .ok_or_else(|| anyhow!("file id '{}' not found in embedded_files", file_id))?;
    download_bytes(&bytes, &name, mime_for(&name));
    Ok(())
}

/// Download the raw photometry file behind a `Photometry` entry — no variant
/// override applied. Used when the caller has no variant context.
pub fn export_variant_photometry(state: GldfContext, photometry_id: &str) -> Result<()> {
    download_raw_photometry(state, photometry_id)
}

/// Download the photometry file PATCHED with this variant's flux / power
/// overrides, emitting in the chosen [`gldf_rs::PhotometryExportFormat`]. The
/// filename is rewritten so its extension matches the format — important
/// when the source photometry is TM-33 XML but the user picked LDT (or
/// vice versa); the old "always LDT, original name" path produced
/// e.g. `wide.xml` containing EULUMDAT bytes.
///
/// When neither override is set, the lib's patcher returns `None` and
/// we fall back to converting the raw bytes via `export_photometry` so
/// the format choice still works.
pub fn export_patched_variant_photometry(
    state: GldfContext,
    photometry_id: &str,
    lumens: Option<i32>,
    watts: Option<f64>,
    format: gldf_rs::PhotometryExportFormat,
) -> Result<()> {
    let (file_name, raw_bytes) = lookup_photometry_bytes(state, photometry_id)?;
    // Prefer the patched bytes when an override exists; fall back to a
    // pure conversion of the raw bytes when there's nothing to patch.
    // Both paths route through eulumdat-rs so format choice is honoured.
    let bytes = if let Some(patched) =
        gldf_rs::patch_photometry_for_variant(&raw_bytes, lumens, watts, format)
    {
        patched
    } else {
        // Need to parse the raw bytes to convert: easiest is to call the
        // lib's patcher with a no-op override that still routes through
        // export_photometry. We pass `Some(_)` for one field with the
        // original value so the patcher's "no-op" guard doesn't bail
        // — using a `0` placeholder isn't safe (would zero out flux),
        // so parse manually here.
        gldf_rs::photometry::convert_raw_photometry(&raw_bytes, format)
            .ok_or_else(|| anyhow!("can't convert photometry to {:?}", format))?
    };
    let stem = file_name
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(&file_name);
    let new_name = format!("{}.{}", stem, format.extension());
    download_bytes(&bytes, &new_name, format.mime());
    Ok(())
}

/// Raw download — for callers without a variant context.
fn download_raw_photometry(state: GldfContext, photometry_id: &str) -> Result<()> {
    let photometry_file_id = state
        .with(|s| {
            s.product
                .general_definitions
                .photometries
                .as_ref()
                .and_then(|p| {
                    p.photometry
                        .iter()
                        .find(|x| x.id == photometry_id)
                        .and_then(|x| {
                            x.photometry_file_reference
                                .as_ref()
                                .map(|r| r.file_id.clone())
                        })
                })
        })
        .ok_or_else(|| anyhow!("photometry '{}' has no file ref", photometry_id))?;
    download_embedded_file(state, &photometry_file_id)
}

/// Shared lookup: photometry id → (filename, raw bytes).
fn lookup_photometry_bytes(state: GldfContext, photometry_id: &str) -> Result<(String, Vec<u8>)> {
    state
        .with(|s| {
            let photometry = s
                .product
                .general_definitions
                .photometries
                .as_ref()?
                .photometry
                .iter()
                .find(|p| p.id == photometry_id)?;
            let file_id = &photometry.photometry_file_reference.as_ref()?.file_id;
            let bytes = s.embedded_files.get(file_id)?.clone();
            let file_name = s
                .product
                .general_definitions
                .files
                .file
                .iter()
                .find(|f| &f.id == file_id)
                .map(|f| f.file_name.clone())
                .unwrap_or_else(|| file_id.to_string());
            Some((file_name, bytes))
        })
        .ok_or_else(|| anyhow!("photometry '{}' file not in embedded_files", photometry_id))
}

// ---- Native plumbing: "download" = write to a gldf-exports dir under
// the system temp dir and log the path. The native shell's Save button
// offers a proper file dialog; these helpers exist so the shared editor
// pages' export buttons still do something observable.

#[cfg(not(target_arch = "wasm32"))]
fn download_bytes(bytes: &[u8], filename: &str, _mime: &str) {
    let dir = std::env::temp_dir().join("gldf-exports");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::error!("export: create {}: {}", dir.display(), e);
        return;
    }
    let path = dir.join(filename);
    match std::fs::write(&path, bytes) {
        Ok(()) => log::info!("exported {} ({} bytes)", path.display(), bytes.len()),
        Err(e) => log::error!("export {}: {}", path.display(), e),
    }
}

// ---- Browser plumbing ----

fn download_text(text: &str, filename: &str, mime: &str) {
    download_bytes(text.as_bytes(), filename, mime);
}

#[cfg(target_arch = "wasm32")]
fn download_bytes(bytes: &[u8], filename: &str, mime: &str) {
    let uint8arr = js_sys::Uint8Array::new(&unsafe { js_sys::Uint8Array::view(bytes) }.into());
    let array = js_sys::Array::new();
    array.push(&uint8arr.buffer());
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&array, &opts) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Ok(a) = document.create_element("a") {
                let _ = a.set_attribute("href", &url);
                let _ = a.set_attribute("download", filename);
                let _ = a.set_attribute("style", "display: none");
                if let Some(body) = document.body() {
                    let _ = body.append_child(&a);
                    if let Some(html_a) = a.dyn_ref::<web_sys::HtmlElement>() {
                        html_a.click();
                    }
                    let _ = body.remove_child(&a);
                }
                let _ = web_sys::Url::revoke_object_url(&url);
            }
        }
    }
}

fn mime_for(name: &str) -> &'static str {
    let n = name.to_lowercase();
    if n.ends_with(".ies") || n.ends_with(".ldt") || n.ends_with(".txt") {
        "text/plain"
    } else if n.ends_with(".jpg") || n.ends_with(".jpeg") {
        "image/jpeg"
    } else if n.ends_with(".png") {
        "image/png"
    } else if n.ends_with(".l3d") {
        "model/l3d"
    } else if n.ends_with(".pdf") {
        "application/pdf"
    } else if n.ends_with(".xml") || n.ends_with(".iesxml") {
        "application/xml"
    } else {
        "application/octet-stream"
    }
}

// Extension trait for save_to_buf on GldfProduct — slice 1 borrows from
// EditableGldf via the state's save_to_buf method, so no impl needed here.
// Kept as a place to add re-exports / shared helpers later.
#[allow(dead_code)]
trait _Placeholder {}
impl _Placeholder for GldfProduct {}
