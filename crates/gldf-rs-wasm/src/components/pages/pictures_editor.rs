//! Pictures editor — `Image` references with `@fileId` + `@imageType`.

use crate::components::edit_util::event_value;
use crate::state::{use_gldf, GldfAction};
use gldf_rs::validation::xsd_enums::IMAGE_TYPES;
use leptos::prelude::*;

#[component]
pub fn PicturesEditorPage() -> impl IntoView {
    let state = use_gldf();
    let pending_file_id = RwSignal::new(String::new());
    let pending_image_type = RwSignal::new("Product Picture".to_string());

    let on_file_change = move |e: web_sys::Event| {
        if let Some(v) = event_value(&e) {
            pending_file_id.set(v);
        }
    };
    let on_type_change = move |e: web_sys::Event| {
        if let Some(v) = event_value(&e) {
            pending_image_type.set(v);
        }
    };
    let on_add = move |_| {
        let fid = pending_file_id.with(|v| v.trim().to_string());
        if fid.is_empty() { return; }
        let img_type = pending_image_type.get();
        state.update(|s| {
            s.apply(GldfAction::AddProductMetaPicture { file_id: fid, image_type: img_type });
        });
        pending_file_id.set(String::new());
    };

    view! {
        <div class="editor-section pictures-editor">
            <h2>{ "Pictures" }</h2>
            <p class="section-description">
                { "Image references from ProductMetaData. Each entry points to a File defined in General Definitions and tags it with an XSD-enumerated image type." }
            </p>

            {move || state.with(|s| {
                let image_files: Vec<String> = s
                    .product
                    .general_definitions
                    .files
                    .file
                    .iter()
                    .filter(|f| f.content_type.starts_with("image/"))
                    .map(|f| f.id.clone())
                    .collect();
                let pictures = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.pictures.as_ref())
                    .map(|p| p.image.clone())
                    .unwrap_or_default();

                view! {
                    {if pictures.is_empty() {
                        view! { <p class="empty-message">{ "No pictures referenced." }</p> }.into_any()
                    } else {
                        view! {
                            <div class="pictures-list" style="display: flex; flex-direction: column; gap: 6px;">
                                {pictures.into_iter().enumerate().map(|(idx, img)| {
                                    let file_id = img.file_id.clone();
                                    let file_id_for_action = file_id.clone();
                                    let current_type = img.image_type.clone();
                                    let on_type = move |e: web_sys::Event| {
                                        let Some(v) = event_value(&e) else { return };
                                        let fid = file_id_for_action.clone();
                                        state.update(|s| s.apply(GldfAction::UpdateProductMetaPicture {
                                            index: idx, file_id: fid, image_type: v
                                        }));
                                    };
                                    let on_remove = move |_| {
                                        state.update(|s| s.apply(GldfAction::RemoveProductMetaPicture(idx)));
                                    };
                                    view! {
                                        <div class="picture-row" style="display: flex; gap: 6px; align-items: center;">
                                            <span class="picture-file-id code" style="font-family: var(--font-mono); flex: 1;">{ file_id }</span>
                                            <select on:change=on_type prop:value=current_type.clone()>
                                                {IMAGE_TYPES.iter().map(|t| {
                                                    let selected = *t == current_type.as_str();
                                                    view! { <option value=(*t).to_string() selected=selected>{ *t }</option> }
                                                }).collect::<Vec<_>>()}
                                            </select>
                                            <button type="button" class="picture-remove" title="Remove" on:click=on_remove>{ "✕" }</button>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }}

                    <div class="add-picture-row" style="margin-top: 12px;">
                        <h4>{ "Add picture" }</h4>
                        {if image_files.is_empty() {
                            view! {
                                <p class="hint">
                                    { "No image-typed files in General Definitions yet. Add a File with contentType starting with image/ first." }
                                </p>
                            }.into_any()
                        } else {
                            view! {
                                <div class="form-row" style="display: flex; gap: 8px; align-items: center;">
                                    <label>{ "File:" }
                                        <select on:change=on_file_change prop:value=pending_file_id.get()>
                                            <option value="" selected=pending_file_id.with(|v| v.is_empty())>{ "(select a file)" }</option>
                                            {image_files.into_iter().map(|id| {
                                                let id_text = id.clone();
                                                let selected = pending_file_id.with(|cur| cur == &id);
                                                view! { <option value=id selected=selected>{ id_text }</option> }
                                            }).collect::<Vec<_>>()}
                                        </select>
                                    </label>
                                    <label>{ "Type:" }
                                        <select on:change=on_type_change prop:value=pending_image_type.get()>
                                            {IMAGE_TYPES.iter().map(|t| {
                                                let selected = pending_image_type.with(|cur| *t == cur.as_str());
                                                view! { <option value=(*t).to_string() selected=selected>{ *t }</option> }
                                            }).collect::<Vec<_>>()}
                                        </select>
                                    </label>
                                    <button type="button" on:click=on_add prop:disabled=move || pending_file_id.with(|v| v.is_empty())>
                                        { "+ Add" }
                                    </button>
                                </div>
                            }.into_any()
                        }}
                    </div>
                }
            })}
        </div>
    }
}
