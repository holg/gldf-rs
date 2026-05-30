//! GLDF WASM Editor — Leptos scaffold (slice 1.2).
//!
//! The editor state is held in a `RwSignal<GldfState>` (see
//! `crates/gldf-rs-wasm/src/state/gldf_state.rs`), installed at the App root
//! via `provide_gldf_state()`. Every page component below the root can read /
//! dispatch via the `use_gldf()` helper.
//!
//! Slice 1.2 wires:
//! - Drop / file-picker → parse GLDF → dispatch `GldfAction::LoadWithFiles`.
//! - One-page summary showing `state.product.header.manufacturer`, photometry
//!   / spectrum / embedded-file counts.
//!
//! Subsequent slices add the SPD viewer, the editor tabs, and the New GLDF
//! wizard. See `docs/leptos_migration.md` (untracked) for the full plan.

use gldf_rs::GldfProduct;
use gldf_rs_wasm::components::SpdViewer;
use gldf_rs_wasm::state::{provide_gldf_state, use_gldf, GldfAction, GldfContext};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use wasm_bindgen::JsCast;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    // Install the editor-state signal as Leptos context. Any descendant
    // component can pull it back out with `use_gldf()`.
    let state = provide_gldf_state();

    view! {
        <div
            style="min-height: 100vh; padding: 24px; font-family: system-ui, sans-serif;"
            on:dragover=move |e: web_sys::DragEvent| e.prevent_default()
            on:drop=move |e: web_sys::DragEvent| handle_drop(e, state)
        >
            <header style="border-bottom: 1px solid #ddd; padding-bottom: 12px; margin-bottom: 16px;">
                <h1 style="margin: 0;">{ "GLDF" }</h1>
                <p style="color: #888; margin: 4px 0 0 0; font-size: 0.9em;">
                    { "Leptos port — slice 1.2 scaffold. Drop a .gldf / .ies / .ldt to load." }
                </p>
            </header>

            <input
                type="file"
                accept=".gldf,.ies,.ldt"
                on:change=move |e: web_sys::Event| handle_file_input(e, state)
            />

            <ContentArea />
        </div>
    }
}

/// Body component. Reads the state via `use_gldf()` — same path every later
/// editor page will use, so the wiring is exercised end-to-end already.
#[component]
fn ContentArea() -> impl IntoView {
    let state = use_gldf();

    view! {
        {move || {
            let is_loaded = state.with(|s| !s.product.header.manufacturer.is_empty()
                || s.product.product_definitions.product_meta_data.is_some()
                || !s.embedded_files.is_empty());
            if is_loaded {
                view! { <LoadedSummary /> }.into_any()
            } else {
                view! {
                    <div style="margin-top: 24px; color: #666;">
                        { "No file loaded. Drop one anywhere on the page." }
                    </div>
                }.into_any()
            }
        }}
    }
}

/// Summary card — slice-1 acceptance criterion. Reads header + counts.
#[component]
fn LoadedSummary() -> impl IntoView {
    let state = use_gldf();

    view! {
        <div style="margin-top: 24px; padding: 16px; border: 1px solid #ddd;">
            {move || state.with(|s| {
                let header = &s.product.header;
                let name = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.name.as_ref())
                    .and_then(|lf| lf.locale.first())
                    .map(|l| l.value.clone())
                    .unwrap_or_default();
                let photometry_count = s
                    .product
                    .general_definitions
                    .photometries
                    .as_ref()
                    .map(|p| p.photometry.len())
                    .unwrap_or(0);
                let spectra_count = s
                    .product
                    .general_definitions
                    .spectrums
                    .as_ref()
                    .map(|sp| sp.spectrum.len())
                    .unwrap_or(0);
                view! {
                    <h2 style="margin-top: 0;">{ name }</h2>
                    <table style="font-size: 0.95em;">
                        <tbody>
                            <tr><th style="text-align: left; padding-right: 14px;">{ "Manufacturer" }</th><td>{ header.manufacturer.clone() }</td></tr>
                            <tr><th style="text-align: left;">{ "Photometries" }</th><td>{ photometry_count }</td></tr>
                            <tr><th style="text-align: left;">{ "Spectra" }</th><td>{ spectra_count }</td></tr>
                            <tr><th style="text-align: left;">{ "Embedded files" }</th><td>{ s.embedded_files.len() }</td></tr>
                            <tr><th style="text-align: left;">{ "Active language" }</th><td>{ s.active_language.clone() }</td></tr>
                            <tr><th style="text-align: left;">{ "Modified" }</th><td>{ if s.is_modified { "yes" } else { "no" } }</td></tr>
                        </tbody>
                    </table>
                }
            })}
        </div>

        // Spectra panel — shows up automatically when the loaded GLDF has
        // <Spectrum> entries. A later slice will move this behind a left-nav
        // tab; for slice 1.3 the inline placement is enough to prove SVG
        // rendering + height-slider reactivity work end-to-end.
        <div style="margin-top: 24px;">
            <h2 style="margin: 0 0 8px 0; font-size: 1.1em; color: #444;">{ "Spectra" }</h2>
            <SpdViewer />
        </div>
    }
}

// ---- File loading ----

/// Drop handler: takes the first file, reads its bytes, dispatches to `try_load`.
fn handle_drop(e: web_sys::DragEvent, state: GldfContext) {
    e.prevent_default();
    let Some(dt) = e.data_transfer() else { return };
    let Some(files) = dt.files() else { return };
    let Some(file) = files.get(0) else { return };
    load_browser_file(file, state);
}

fn handle_file_input(e: web_sys::Event, state: GldfContext) {
    let Some(target) = e.target() else { return };
    let input: web_sys::HtmlInputElement = match target.dyn_into() {
        Ok(i) => i,
        Err(_) => return,
    };
    let Some(files) = input.files() else { return };
    let Some(file) = files.get(0) else { return };
    load_browser_file(file, state);
    // Clear so re-picking the same file fires another change event.
    input.set_value("");
}

fn load_browser_file(file: web_sys::File, state: GldfContext) {
    let name = file.name();
    let gloo_file = gloo::file::File::from(file);
    spawn_local(async move {
        match gloo::file::futures::read_as_bytes(&gloo_file).await {
            Ok(bytes) => {
                if let Err(e) = try_load(state, &name, bytes) {
                    log::error!("load failed: {}", e);
                }
            }
            Err(e) => log::error!("read failed: {}", e),
        }
    });
}

/// Dispatch the raw bytes to the right parser by extension.
fn try_load(state: GldfContext, filename: &str, bytes: Vec<u8>) -> anyhow::Result<()> {
    let lower = filename.to_lowercase();
    if lower.ends_with(".gldf") {
        let fb = GldfProduct::load_gldf_from_buf_all(bytes)?;
        // Reshape into the (product, embedded_files) pair the editor state
        // expects. The lib's recent fix populates BufFile.file_id from the
        // XML, so we can key the map by id here without extra lookups.
        let embedded_files: HashMap<String, Vec<u8>> = fb
            .files
            .into_iter()
            .filter_map(|f| {
                let content = f.content?;
                let key = f.file_id.or(f.path).or(f.name)?;
                Some((key, content))
            })
            .collect();
        state.update(|s| {
            s.apply(GldfAction::LoadWithFiles {
                product: fb.gldf,
                embedded_files,
            });
        });
        Ok(())
    } else if lower.ends_with(".ies") || lower.ends_with(".ldt") {
        let converted = gldf_rs::convert::ldt_to_gldf(&bytes, filename)
            .map_err(|e| anyhow::anyhow!("ldt_to_gldf: {}", e))?;
        let embedded_files: HashMap<String, Vec<u8>> = converted
            .files
            .into_iter()
            .filter_map(|f| {
                let content = f.content?;
                let key = f.file_id.or(f.path).or(f.name)?;
                Some((key, content))
            })
            .collect();
        state.update(|s| {
            s.apply(GldfAction::LoadWithFiles {
                product: converted.gldf,
                embedded_files,
            });
        });
        Ok(())
    } else {
        anyhow::bail!("Unsupported file: {}", filename);
    }
}
