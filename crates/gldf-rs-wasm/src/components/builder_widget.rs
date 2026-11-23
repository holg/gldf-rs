//! Drop-and-link GLDF builder widget (Leptos slice 1).
//!
//! Three workflows in one panel, all going through the core-lib `EditableGldf`
//! builder via `GldfAction`:
//!
//! - Drop a `.ldt` / `.ies` → embed + create `<File>` / `<Photometry>`.
//! - Drop a `.spd` → embed + create `<Spectrum>` and link it to the chosen
//!   light source via `<SpectrumReference>`.
//! - Add a product variant by name.
//!
//! Download Modified GLDF reuses the toolbar export pipeline so users get a
//! .gldf with the canonical XSD and round-tripped bytes.

use crate::exports::export_gldf;
use crate::state::{use_gldf, GldfAction};
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use leptos::task::spawn_local;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlInputElement;

#[component]
pub fn BuilderWidget() -> impl IntoView {
    let state = use_gldf();

    // ---- Photometry drop ----
    let on_photometry_change = move |e: crate::components::edit_util::EditEvent| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(target) = e.target() else { return };
            let input: HtmlInputElement = match target.dyn_into() {
                Ok(i) => i,
                Err(_) => return,
            };
            let Some(files) = input.files() else { return };
            let Some(file) = files.get(0) else { return };
            let filename = file.name();
            let gloo_file = gloo::file::File::from(file);
            spawn_local(async move {
                match gloo::file::futures::read_as_bytes(&gloo_file).await {
                    Ok(bytes) => {
                        state.update(|s| {
                            s.apply(GldfAction::AddPhotometryFile { bytes, filename });
                        });
                    }
                    Err(e) => log::error!("read photometry: {}", e),
                }
            });
            input.set_value("");
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = e;
    };

    // ---- Spectrum drop + light-source picker ----
    let light_source_id = RwSignal::new(String::new());
    // Seed the picker when the state changes (e.g. just loaded a GLDF, or
    // Builder created the first light source). Tracks the state signal.
    Effect::new(move |_| {
        let first = state.with(|s| {
            s.product
                .general_definitions
                .light_sources
                .as_ref()
                .and_then(|ls| ls.fixed_light_source.first().map(|src| src.id.clone()))
        });
        if let Some(id) = first {
            if light_source_id.with(|cur| cur.is_empty()) {
                light_source_id.set(id);
            }
        }
    });

    let on_light_source_change = move |e: crate::components::edit_util::EditEvent| {
        let Some(v) = crate::components::edit_util::event_value(&e) else {
            return;
        };
        light_source_id.set(v);
    };

    let on_spectrum_change = move |e: crate::components::edit_util::EditEvent| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(target) = e.target() else { return };
            let input: HtmlInputElement = match target.dyn_into() {
                Ok(i) => i,
                Err(_) => return,
            };
            let Some(files) = input.files() else { return };
            let Some(file) = files.get(0) else { return };
            let filename = file.name();
            let ls_id = light_source_id.get();
            if ls_id.is_empty() {
                return;
            }
            let gloo_file = gloo::file::File::from(file);
            spawn_local(async move {
                match gloo::file::futures::read_as_bytes(&gloo_file).await {
                    Ok(bytes) => {
                        state.update(|s| {
                            s.apply(GldfAction::AddSpectrumFile {
                                bytes,
                                filename,
                                light_source_id: ls_id,
                            });
                        });
                    }
                    Err(e) => log::error!("read spectrum: {}", e),
                }
            });
            input.set_value("");
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = e;
    };

    // ---- Variant input ----
    let variant_name = RwSignal::new(String::new());
    let on_variant_input = move |e: crate::components::edit_util::EditEvent| {
        let Some(v) = crate::components::edit_util::event_value(&e) else {
            return;
        };
        variant_name.set(v);
    };
    let on_variant_add = move |_| {
        let name = variant_name.get();
        if !name.trim().is_empty() {
            state.update(|s| s.apply(GldfAction::AddVariantSimple { name }));
            variant_name.set(String::new());
        }
    };

    // ---- Save / download ----
    let on_save = move |_| {
        if let Err(e) = export_gldf(state) {
            log::error!("save_to_buf: {}", e);
        }
    };

    view! {
        <div class="builder-widget" style="border: 1px solid #ccc; padding: 12px; margin: 8px 0;">
            <h3>{ "GLDF Builder" }</h3>
            <p style="color: #666; margin-top: 0;">
                { "Drop a photometry (.ldt/.ies) or spectrum (.spd) to embed it, or add a variant. Each action goes through the core-lib EditableGldf builder so every binding shares one path." }
            </p>

            <div style="margin: 10px 0;">
                <label><strong>{ "Add photometry (.ldt / .ies):" }</strong></label><br/>
                <input type="file" accept=".ldt,.ies" on:change=on_photometry_change />
            </div>

            <div style="margin: 10px 0;">
                <label><strong>{ "Add spectrum (.spd) linked to light source:" }</strong></label><br/>
                {move || state.with(|s| {
                    let ids: Vec<String> = s
                        .product
                        .general_definitions
                        .light_sources
                        .as_ref()
                        .map(|ls| {
                            ls.fixed_light_source
                                .iter()
                                .map(|x| x.id.clone())
                                .chain(ls.changeable_light_source.iter().map(|x| x.id.clone()))
                                .collect()
                        })
                        .unwrap_or_default();
                    if ids.is_empty() {
                        view! {
                            <em style="color: #999;">{ "No light sources yet — add a photometry first (auto-creates a light source)." }</em>
                        }.into_any()
                    } else {
                        let current = light_source_id.get();
                        view! {
                            <select on:change=on_light_source_change prop:value=current>
                                {ids.into_iter().map(|id| {
                                    let id_text = id.clone();
                                    view! { <option value=id>{ id_text }</option> }
                                }).collect::<Vec<_>>()}
                            </select>
                            <input
                                type="file"
                                accept=".spd,.xml,.txt"
                                on:change=on_spectrum_change
                                style="margin-left: 8px;"
                            />
                        }.into_any()
                    }
                })}
            </div>

            <div style="margin: 10px 0;">
                <label><strong>{ "Add variant:" }</strong></label><br/>
                <input
                    type="text"
                    placeholder="Variant name"
                    prop:value=move || variant_name.get()
                    on:input=on_variant_input
                />
                <button on:click=on_variant_add style="margin-left: 8px;">{ "Add" }</button>
            </div>

            <div style="margin: 10px 0; padding: 8px; background: #f5f5f5; font-size: 0.9em;">
                {move || state.with(|s| {
                    let phot = s.product.general_definitions.photometries.as_ref().map(|p| p.photometry.len()).unwrap_or(0);
                    let spec = s.product.general_definitions.spectrums.as_ref().map(|sp| sp.spectrum.len()).unwrap_or(0);
                    let vars = s.product.product_definitions.variants.as_ref().map(|v| v.variant.len()).unwrap_or(0);
                    format!("Photometries: {} · Spectra: {} · Variants: {} · Embedded files: {}", phot, spec, vars, s.embedded_files.len())
                })}
            </div>

            {move || state.with(|s| s.last_error.clone()).map(|err| view! {
                <div style="color: #c00; margin: 8px 0;">{ err }</div>
            })}

            <div style="margin-top: 14px;">
                <button on:click=on_save>{ "Download modified GLDF" }</button>
            </div>
        </div>
    }
}
