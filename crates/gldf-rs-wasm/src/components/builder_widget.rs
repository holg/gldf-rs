//! Drop-and-link GLDF builder widget.
//!
//! Three workflows in one panel — every one goes through the core-lib
//! `EditableGldf` builder API via `GldfAction`, so the same logic backs any
//! future FFI / Unreal binding:
//!
//! - Drop a `.ldt` / `.ies` file → embed it + create `<File>` / `<Photometry>`.
//! - Drop a `.spd` file → embed it + create `<Spectrum>` and link it to the
//!   chosen light source via `<SpectrumReference>`.
//! - Add a product variant by name.
//!
//! Also offers a "Download modified GLDF" button that calls
//! `GldfState::save_to_buf()` (which delegates to `EditableGldf::save_to_buf`)
//! — the canonical online XSD and embedded-bytes round-trip come for free.

use crate::state::{use_gldf, GldfAction};
use gloo::file::callbacks::FileReader;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

/// Trigger a browser download of the given bytes with the given filename.
fn download_bytes(bytes: &[u8], filename: &str, mime: &str) {
    let uint8arr =
        js_sys::Uint8Array::new(&unsafe { js_sys::Uint8Array::view(bytes) }.into());
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

/// Read the first file from a file input and feed its bytes to `on_bytes`.
/// Keeps the `FileReader` alive in a `RefCell` because gloo drops the
/// callback otherwise.
fn read_first_file<F>(input: &HtmlInputElement, reader_slot: Rc<RefCell<Option<FileReader>>>, on_bytes: F)
where
    F: FnOnce(String, Vec<u8>) + 'static,
{
    let Some(files) = input.files() else { return };
    let Some(file) = files.get(0) else { return };
    let filename = file.name();
    let gloo_file = gloo::file::File::from(file);
    let reader = gloo::file::callbacks::read_as_bytes(&gloo_file, move |res| {
        if let Ok(bytes) = res {
            on_bytes(filename, bytes);
        }
    });
    *reader_slot.borrow_mut() = Some(reader);
}

#[function_component(BuilderWidget)]
pub fn builder_widget() -> Html {
    let gldf = use_gldf();

    // ----- Identity inputs -----
    // Pull the current product name in the active language, the manufacturer,
    // and the first fixed-light-source id. These are the three placeholders
    // the auto-conversion (`convert::ldt_to_gldf`) tends to invent from weak
    // IES metadata ("TEST", "DEMOLAMPSET", "lightsource_1") — surface them so
    // the user can correct in place without going to other tabs.
    let active_lang = gldf.active_language.clone();
    let current_name = gldf
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.name.as_ref())
        .and_then(|lf| {
            lf.locale
                .iter()
                .find(|l| l.language == active_lang)
                .or_else(|| lf.locale.first())
        })
        .map(|l| l.value.clone())
        .unwrap_or_default();
    let current_manufacturer = gldf.product.header.manufacturer.clone();
    let first_fls_id = gldf
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .and_then(|ls| ls.fixed_light_source.first().map(|s| s.id.clone()));

    let name_input = use_state(|| current_name.clone());
    let manufacturer_input = use_state(|| current_manufacturer.clone());
    let rename_target = use_state(|| first_fls_id.clone().unwrap_or_default());

    // Refresh local state when the underlying product changes (e.g. after a
    // sync round-trip following another action). Otherwise the inputs would
    // stay stuck on whatever was in the field when the widget first mounted.
    {
        let name_input = name_input.clone();
        let current_name_dep = current_name.clone();
        use_effect_with(current_name_dep.clone(), move |new| {
            name_input.set(new.clone());
            || ()
        });
    }
    {
        let manufacturer_input = manufacturer_input.clone();
        use_effect_with(current_manufacturer.clone(), move |new| {
            manufacturer_input.set(new.clone());
            || ()
        });
    }
    {
        let rename_target = rename_target.clone();
        let dep = first_fls_id.clone().unwrap_or_default();
        use_effect_with(dep.clone(), move |new| {
            rename_target.set(new.clone());
            || ()
        });
    }

    let on_name_input = {
        let name_input = name_input.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            name_input.set(input.value());
        })
    };
    let on_name_commit = {
        let name_input = name_input.clone();
        let gldf = gldf.clone();
        let lang = active_lang.clone();
        Callback::from(move |_: FocusEvent| {
            let value = (*name_input).clone();
            let lf = gldf_rs::gldf::LocaleFoo {
                locale: vec![gldf_rs::gldf::header::Locale {
                    language: lang.clone(),
                    value,
                }],
            };
            gldf.dispatch(GldfAction::SetProductMetaName(lf));
        })
    };

    let on_manufacturer_input = {
        let manufacturer_input = manufacturer_input.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            manufacturer_input.set(input.value());
        })
    };
    let on_manufacturer_commit = {
        let manufacturer_input = manufacturer_input.clone();
        let gldf = gldf.clone();
        Callback::from(move |_: FocusEvent| {
            gldf.dispatch(GldfAction::SetManufacturer((*manufacturer_input).clone()));
        })
    };

    let on_rename_input = {
        let rename_target = rename_target.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            rename_target.set(input.value());
        })
    };
    let on_rename_commit = {
        let rename_target = rename_target.clone();
        let gldf = gldf.clone();
        let old_id = first_fls_id.clone();
        Callback::from(move |_: MouseEvent| {
            let new_id = (*rename_target).clone();
            if let Some(old) = old_id.clone() {
                if !new_id.is_empty() && new_id != old {
                    gldf.dispatch(GldfAction::RenameFixedLightSource {
                        old_id: old,
                        new_id,
                    });
                }
            }
        })
    };

    // ----- Variant input -----
    let variant_name = use_state(String::new);
    let on_variant_input = {
        let variant_name = variant_name.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            variant_name.set(input.value());
        })
    };
    let on_variant_add = {
        let variant_name = variant_name.clone();
        let gldf = gldf.clone();
        Callback::from(move |_: MouseEvent| {
            let name = (*variant_name).clone();
            if !name.trim().is_empty() {
                gldf.dispatch(GldfAction::AddVariantSimple { name });
                variant_name.set(String::new());
            }
        })
    };

    // ----- Photometry drop (Add or Replace) -----
    // Existing photometry ids — the Replace dropdown lets the user swap the
    // file behind any of them without changing the photometry id (so emitter
    // PhotometryReference stays valid). This is the right path for "fix the
    // placeholder photometry from the start-page drop with a curated upload".
    let photometry_ids: Vec<String> = gldf
        .product
        .general_definitions
        .photometries
        .as_ref()
        .map(|p| p.photometry.iter().map(|x| x.id.clone()).collect())
        .unwrap_or_default();
    let replace_target = use_state(|| photometry_ids.first().cloned().unwrap_or_default());

    let phot_reader: Rc<RefCell<Option<FileReader>>> = use_mut_ref(|| None);
    let on_photometry_change = {
        let gldf = gldf.clone();
        let phot_reader = phot_reader.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let gldf = gldf.clone();
            read_first_file(&input, phot_reader.clone(), move |filename, bytes| {
                gldf.dispatch(GldfAction::AddPhotometryFile { bytes, filename });
            });
            input.set_value("");
        })
    };

    let on_replace_target_change = {
        let replace_target = replace_target.clone();
        Callback::from(move |e: Event| {
            let select: HtmlSelectElement = e.target_unchecked_into();
            replace_target.set(select.value());
        })
    };

    let replace_reader: Rc<RefCell<Option<FileReader>>> = use_mut_ref(|| None);
    let on_replace_change = {
        let gldf = gldf.clone();
        let replace_reader = replace_reader.clone();
        let replace_target = replace_target.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let gldf = gldf.clone();
            let target = (*replace_target).clone();
            if target.is_empty() {
                return;
            }
            read_first_file(&input, replace_reader.clone(), move |filename, bytes| {
                gldf.dispatch(GldfAction::ReplacePhotometryFile {
                    photometry_id: target,
                    bytes,
                    filename,
                });
            });
            input.set_value("");
        })
    };

    // ----- Spectrum drop -----
    let spectrum_reader: Rc<RefCell<Option<FileReader>>> = use_mut_ref(|| None);
    let light_source_id = use_state(String::new);

    // Collect candidate light-source ids from the product. Fixed + Changeable
    // both expose an `id`; the widget shows them together because the lib's
    // `add_spectrum_reference_to_light_source` handles either.
    let light_source_ids: Vec<String> = gldf
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .map(|ls| {
            ls.fixed_light_source
                .iter()
                .map(|s| s.id.clone())
                .chain(ls.changeable_light_source.iter().map(|s| s.id.clone()))
                .collect()
        })
        .unwrap_or_default();

    // Auto-seed the picker with the first available id so a one-source product
    // doesn't force a click.
    {
        let light_source_id = light_source_id.clone();
        let ids = light_source_ids.clone();
        use_effect_with(ids.clone(), move |ids| {
            if light_source_id.is_empty() {
                if let Some(first) = ids.first() {
                    light_source_id.set(first.clone());
                }
            }
            || ()
        });
    }

    let on_light_source_change = {
        let light_source_id = light_source_id.clone();
        Callback::from(move |e: Event| {
            let select: HtmlSelectElement = e.target_unchecked_into();
            light_source_id.set(select.value());
        })
    };

    let on_spectrum_change = {
        let gldf = gldf.clone();
        let spectrum_reader = spectrum_reader.clone();
        let light_source_id = light_source_id.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let gldf = gldf.clone();
            let ls_id = (*light_source_id).clone();
            if ls_id.is_empty() {
                // The lib would error anyway, but a tiny early-out keeps the
                // user-facing error explicit.
                return;
            }
            read_first_file(&input, spectrum_reader.clone(), move |filename, bytes| {
                gldf.dispatch(GldfAction::AddSpectrumFile {
                    bytes,
                    filename,
                    light_source_id: ls_id,
                });
            });
            input.set_value("");
        })
    };

    // ----- Save / download -----
    let on_save = {
        let gldf = gldf.clone();
        Callback::from(move |_: MouseEvent| match gldf.save_to_buf() {
            Ok(bytes) => download_bytes(&bytes, "edited.gldf", "application/zip"),
            Err(e) => {
                gloo::console::log!("save_to_buf failed:", e.to_string());
            }
        })
    };

    let last_error = gldf.last_error.clone();
    let phot_count = gldf
        .product
        .general_definitions
        .photometries
        .as_ref()
        .map(|p| p.photometry.len())
        .unwrap_or(0);
    let spectrum_count = gldf
        .product
        .general_definitions
        .spectrums
        .as_ref()
        .map(|s| s.spectrum.len())
        .unwrap_or(0);
    let variant_count = gldf
        .product
        .product_definitions
        .variants
        .as_ref()
        .map(|v| v.variant.len())
        .unwrap_or(0);
    let embedded_count = gldf.embedded_files.len();

    html! {
        <div class="builder-widget" style="border: 1px solid #ccc; padding: 12px; margin: 8px 0;">
            <h3>{ "GLDF Builder" }</h3>
            <p style="color: #666; margin-top: 0;">
                { "Drop a photometry (.ldt/.ies) or spectrum (.spd) to embed it, or add a variant. All operations go through the core-lib EditableGldf builder so every binding shares one path." }
            </p>

            // -------- Identity (fix the placeholders the auto-conversion leaves behind) --------
            <fieldset style="margin: 10px 0; padding: 8px 12px; border: 1px solid #ddd;">
                <legend><strong>{ "Identity" }</strong></legend>
                <p style="color: #666; margin: 4px 0;">
                    { format!("Fix the placeholders the start-page drop bakes in (e.g. \"TEST\" / \"DEMOLAMPSET\" / \"lightsource_1\"). Edits commit on blur. Language: {}", active_lang) }
                </p>
                <div style="margin: 6px 0;">
                    <label>{ "Product name: " }</label>
                    <input
                        type="text"
                        value={(*name_input).clone()}
                        oninput={on_name_input}
                        onblur={on_name_commit}
                        size="40"
                    />
                </div>
                <div style="margin: 6px 0;">
                    <label>{ "Manufacturer: " }</label>
                    <input
                        type="text"
                        value={(*manufacturer_input).clone()}
                        oninput={on_manufacturer_input}
                        onblur={on_manufacturer_commit}
                        size="40"
                    />
                </div>
                if let Some(ref old) = first_fls_id {
                    <div style="margin: 6px 0;">
                        <label>{ format!("First light-source id (was \"{}\"): ", old) }</label>
                        <input
                            type="text"
                            value={(*rename_target).clone()}
                            oninput={on_rename_input}
                            size="30"
                        />
                        <button onclick={on_rename_commit} style="margin-left: 6px;">{ "Rename" }</button>
                    </div>
                }
            </fieldset>

            // -------- Photometry --------
            <div style="margin: 10px 0;">
                <label><strong>{ "Add photometry (.ldt / .ies):" }</strong></label><br/>
                <input
                    type="file"
                    accept=".ldt,.ies"
                    onchange={on_photometry_change}
                />
            </div>

            // -------- Replace photometry --------
            if !photometry_ids.is_empty() {
                <div style="margin: 10px 0;">
                    <label><strong>{ "Replace existing photometry (file swap, id unchanged):" }</strong></label><br/>
                    <select onchange={on_replace_target_change} value={(*replace_target).clone()}>
                        { for photometry_ids.iter().map(|id| {
                            let selected = *id == *replace_target;
                            html! { <option value={id.clone()} selected={selected}>{ id.clone() }</option> }
                        }) }
                    </select>
                    <input
                        type="file"
                        accept=".ldt,.ies"
                        onchange={on_replace_change}
                        style="margin-left: 8px;"
                    />
                </div>
            }

            // -------- Spectrum --------
            <div style="margin: 10px 0;">
                <label><strong>{ "Add spectrum (.spd) linked to light source:" }</strong></label><br/>
                if light_source_ids.is_empty() {
                    <em style="color: #999;">{ "No light sources yet — add a light source first." }</em>
                } else {
                    <select onchange={on_light_source_change} value={(*light_source_id).clone()}>
                        { for light_source_ids.iter().map(|id| {
                            let selected = *id == *light_source_id;
                            html! { <option value={id.clone()} selected={selected}>{ id.clone() }</option> }
                        }) }
                    </select>
                    <input
                        type="file"
                        accept=".spd,.xml,.txt"
                        onchange={on_spectrum_change}
                        style="margin-left: 8px;"
                    />
                }
            </div>

            // -------- Variant --------
            <div style="margin: 10px 0;">
                <label><strong>{ "Add variant:" }</strong></label><br/>
                <input
                    type="text"
                    placeholder="Variant name"
                    value={(*variant_name).clone()}
                    oninput={on_variant_input}
                />
                <button onclick={on_variant_add} style="margin-left: 8px;">{ "Add" }</button>
            </div>

            // -------- Status -------
            <div style="margin: 10px 0; padding: 8px; background: #f5f5f5; font-size: 0.9em;">
                { format!("Photometries: {} · Spectra: {} · Variants: {} · Embedded files: {}",
                    phot_count, spectrum_count, variant_count, embedded_count) }
            </div>

            if let Some(err) = last_error {
                <div style="color: #c00; margin: 8px 0;">{ err }</div>
            }

            // -------- Save --------
            <div style="margin-top: 14px;">
                <button onclick={on_save}>{ "Download modified GLDF" }</button>
            </div>
        </div>
    }
}
