//! `<LightSourceExportMenu>` — four-format per-LightSource download
//! dropdown. Surfaces LDT / IES / TM-33 XML / IFC fragment writers from
//! [`gldf_rs::export::lightsource`].
//!
//! TM-33 is the headline option — it combines the angular photometry
//! AND the SPD spectrum into one XML file, so users with a populated
//! `<Spectrum>` block get a single self-contained download instead of
//! one `.ldt` + one `.spd` they have to ship side-by-side.

use anyhow::Result;
use gldf_rs::export::lightsource as ls_export;
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlSelectElement;

use crate::state::{use_gldf, GldfContext};

/// One row of buttons (or a single `<select>`) that triggers a download
/// for the chosen format. Renders next to the `✎ Edit` / `Remove`
/// actions on a LightSource card.
#[component]
pub fn LightSourceExportMenu(
    /// LightSource id to export. Resolves via
    /// `LS → FixedLightEmitter → Photometry → File` to find the source
    /// LDT, then re-emits in the chosen format.
    light_source_id: String,
) -> impl IntoView {
    let state = use_gldf();
    let ls_id = light_source_id.clone();

    let on_pick = move |e: crate::components::edit_util::EditEvent| {
        let Some(format) = crate::components::edit_util::event_value(&e) else {
            return;
        };
        // Reset to placeholder so the same format can be picked twice
        // in a row without the change event being suppressed.
        #[cfg(target_arch = "wasm32")]
        if let Some(target) = e.target() {
            if let Ok(select) = target.dyn_into::<HtmlSelectElement>() {
                select.set_value("");
            }
        }
        if format.is_empty() {
            return;
        }
        if let Err(e) = trigger_download(state, &ls_id, &format) {
            log::error!("LS export ({format}) failed: {e}");
            #[cfg(target_arch = "wasm32")]
            if let Some(win) = web_sys::window() {
                let _ = win.alert_with_message(&format!(
                    "Export failed: {e}\n\nThis LightSource needs to be bound by an Emitter that references a Photometry with embedded LDT/IES bytes.",
                ));
            }
        }
    };

    let ls_id_for_testid = light_source_id.clone();
    view! {
        <select
            class="ls-export-menu"
            on:change=on_pick
            data-testid=format!("ls-export-{}", ls_id_for_testid)
            title="Download this LightSource as a stand-alone photometric / BIM file"
            style="padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
        >
            <option value="">{ "📥 Download…" }</option>
            <option value="ldt">{ "EULUMDAT (.ldt)" }</option>
            <option value="ies">{ "IES LM-63 (.ies)" }</option>
            <option value="tm33">{ "TM-33 XML (photometry + spectrum)" }</option>
            <option value="ifc">{ "IFC fragment (.ifc)" }</option>
        </select>
    }
}

fn trigger_download(state: GldfContext, ls_id: &str, format: &str) -> Result<()> {
    let (filename, mime, payload) = state.with(|s| -> Result<(String, &str, Vec<u8>)> {
        match format {
            "ldt" => {
                let text = ls_export::export_ldt(&s.product, &s.embedded_files, ls_id)?;
                Ok((format!("{ls_id}.ldt"), "text/plain", text.into_bytes()))
            }
            "ies" => {
                let text = ls_export::export_ies(&s.product, &s.embedded_files, ls_id)?;
                Ok((format!("{ls_id}.ies"), "text/plain", text.into_bytes()))
            }
            "tm33" => {
                let text = ls_export::export_tm33_xml(&s.product, &s.embedded_files, ls_id)?;
                Ok((
                    format!("{ls_id}.tm33.xml"),
                    "application/xml",
                    text.into_bytes(),
                ))
            }
            "ifc" => {
                let text = ls_export::export_ifc_fragment(&s.product, &s.embedded_files, ls_id)?;
                Ok((
                    format!("{ls_id}.ifc"),
                    "application/x-step",
                    text.into_bytes(),
                ))
            }
            other => Err(anyhow::anyhow!("Unknown export format: {other}")),
        }
    })?;
    download_bytes(&payload, &filename, mime);
    Ok(())
}

/// Native: write into the same gldf-exports temp dir as
/// `crate::exports`' native download path.
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

/// Same Blob / `<a download>` plumbing as [`crate::exports::download_bytes`],
/// copied here to keep this component standalone. Single browser API call
/// chain — split would only obscure intent.
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
