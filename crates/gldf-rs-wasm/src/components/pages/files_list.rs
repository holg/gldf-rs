//! Files list page — read view of `<GeneralDefinitions/Files>`, plus an
//! edit mode that adds an "Add file…" drop-zone and per-row Edit / Remove
//! affordances. The drop-zone is the generic primitive for shipping any
//! attachment in the bundle; the typed Builder flows (Photometry / Spectrum)
//! still apply when you want the `<File>` wrapped by a matching
//! `<Photometry>` / `<Spectrum>` block.

use crate::components::edit_util::event_value;
use crate::components::IdRenameChip;
use crate::exports::download_embedded_file;
use crate::i18n::ui::t as ui_t;
use crate::state::{use_gldf, GldfAction};
use gldf_rs::validation::xsd_enums::{is_canonical, CONTENT_TYPES};
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use leptos::task::spawn_local;
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlInputElement;

#[component]
pub fn FilesListPage(#[prop(default = false)] editable: bool) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));

    let on_add_file = move |e: crate::components::edit_util::EditEvent| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(target) = e.target() else { return };
            let input: HtmlInputElement = match target.dyn_into() {
                Ok(i) => i,
                Err(_) => return,
            };
            let Some(files) = input.files() else { return };
            // Process every file the user picked. Each known photometric /
            // geometry / spectral extension takes its specialised import
            // path so we auto-register the matching wrapper element
            // alongside the <File> entry:
            //   - .l3d       → ImportL3dFile (also creates ModelGeometry)
            //   - .ldt/.ies  → AddPhotometryFile (also creates Photometry)
            //   - .spd       → ImportSpectrumFile (also creates Spectrum;
            //                  binding to a LightSource happens later via
            //                  the LightSource Spectrum picker)
            // Everything else (images, custom blobs) falls through to the
            // generic AddFileWithBytes flow that just registers a <File>.
            for i in 0..files.length() {
                let Some(file) = files.get(i) else { continue };
                let filename = file.name();
                let gloo_file = gloo::file::File::from(file);
                spawn_local(async move {
                    match gloo::file::futures::read_as_bytes(&gloo_file).await {
                        Ok(bytes) => {
                            let lower = filename.to_lowercase();
                            let is_l3d = lower.ends_with(".l3d");
                            let is_photometric =
                                lower.ends_with(".ldt") || lower.ends_with(".ies");
                            let is_spectrum = lower.ends_with(".spd");
                            state.update(|s| {
                                if is_l3d {
                                    s.apply(GldfAction::ImportL3dFile { bytes, filename });
                                } else if is_photometric {
                                    s.apply(GldfAction::AddPhotometryFile { bytes, filename });
                                } else if is_spectrum {
                                    s.apply(GldfAction::ImportSpectrumFile { bytes, filename });
                                } else {
                                    s.apply(GldfAction::AddFileWithBytes { bytes, filename });
                                }
                            });
                        }
                        Err(err) => log::error!("read file: {}", err),
                    }
                });
            }
            input.set_value("");
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = e;
    };

    view! {
        {move || state.with(|s| {
            let files = s.product.general_definitions.files.file.clone();
            let embedded_ids: std::collections::HashSet<String> = s.embedded_files.keys().cloned().collect();
            view! {
                <div class="files-container">
                    {if editable {
                        view! {
                            <div class="files-add-zone" style="margin-bottom: 12px; padding: 10px 12px; border: 1px dashed var(--accent-blue); border-radius: 6px; background: var(--bg-secondary);">
                                <label style="display: flex; gap: 12px; align-items: center; font-size: 13px; cursor: pointer;">
                                    <span style="color: var(--accent-blue); font-weight: 500;">{ "Add file(s) to this bundle" }</span>
                                    <input
                                        type="file"
                                        multiple=true
                                        on:change=on_add_file.clone()
                                        data-testid="files-add-input"
                                        style="font-size: 12px;"
                                    />
                                </label>
                                <p style="margin: 6px 0 0; font-size: 11px; color: var(--text-tertiary);">
                                    { tr("gldf.files.content_type_help") }
                                </p>
                            </div>
                        }.into_any()
                    } else { ().into_any() }}
                    {if files.is_empty() {
                        view! {
                            <div class="empty-state">
                                <div class="icon">{ "📁" }</div>
                                <h3>{ "No Files" }</h3>
                                <p>{
                                    if editable { "Drop a file above, or use the Builder for typed photometry/spectrum drops." }
                                    else { "Load a GLDF file to view file definitions" }
                                }</p>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="card">
                                <table class="data-table files-table-clickable">
                                    <thead>
                                        <tr>
                                            <th>{ tr("gldf.files.col_id") }</th>
                                            <th>{ tr("gldf.files.col_file_name") }</th>
                                            <th>{ tr("gldf.files.col_content_type") }</th>
                                            <th>{ tr("gldf.files.col_type_attr") }</th>
                                            <th>{ tr("gldf.files.col_actions") }</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {files.into_iter().map(|f| {
                                            let embedded = embedded_ids.contains(&f.id);
                                            view! {
                                                <FileRow file=f editable=editable embedded=embedded />
                                            }
                                        }).collect::<Vec<_>>()}
                                    </tbody>
                                </table>
                                <p style="text-align: center; color: var(--text-tertiary); margin-top: 16px; font-size: 13px;">
                                    { "Click ⬇ to download. Open File Viewer for inline preview." }
                                </p>
                            </div>
                        }.into_any()
                    }}
                </div>
            }.into_any()
        })}
    }
}

#[component]
fn FileRow(
    file: gldf_rs::gldf::general_definitions::files::File,
    editable: bool,
    embedded: bool,
) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    let is_editing = RwSignal::new(false);
    let id = file.id.clone();
    let id_for_dl = file.id.clone();
    let id_for_save = file.id.clone();
    let id_for_remove = file.id.clone();
    let id_for_rename = file.id.clone();
    let on_rename_id: Arc<dyn Fn(String) + Send + Sync> = {
        let old_id = id_for_rename.clone();
        Arc::new(move |new_id| {
            state.update(|s| s.apply(GldfAction::RenameFile {
                old_id: old_id.clone(),
                new_id,
            }));
        })
    };
    let file_name = file.file_name.clone();
    let file_name_for_title = file.file_name.clone();
    let content_type = file.content_type.clone();
    let ct_class = content_type_class(&content_type);
    let type_attr = file.type_attr.clone();

    let s_name = RwSignal::new(file.file_name.clone());
    let s_ct = RwSignal::new(file.content_type.clone());
    let s_type = RwSignal::new(file.type_attr.clone());

    let on_download = move |e: crate::components::edit_util::ClickEvt| {
        crate::components::edit_util::stop_click_propagation(&e);
        if let Err(err) = download_embedded_file(state, &id_for_dl) {
            log::error!("download {}: {}", id_for_dl, err);
        }
    };

    let file_name_for_name_cell = file_name.clone();
    let content_type_for_ct_cell = content_type.clone();
    let type_attr_for_type_cell = type_attr.clone();

    view! {
        <tr>
            <td class="file-id">
                <IdRenameChip current_id=id editable=editable on_rename=on_rename_id />
            </td>
            <td class="file-name">
                {move || if editable && is_editing.get() {
                    view! { <TextField value=s_name /> }.into_any()
                } else {
                    view! { <span>{ file_name_for_name_cell.clone() }</span> }.into_any()
                }}
            </td>
            <td>
                {move || if editable && is_editing.get() {
                    let on_change = move |e: crate::components::edit_util::EditEvent| {
                        if let Some(v) = event_value(&e) {
                            s_ct.set(v);
                        }
                    };
                    let current = s_ct.get();
                    let legacy_pill = if !current.is_empty() && !is_canonical(CONTENT_TYPES, &current) {
                        let suffix = tr("gldf.files.content_type_legacy_suffix");
                        Some(view! {
                            <option value=current.clone() selected=true>
                                { format!("⚠ {} ({})", current, suffix) }
                            </option>
                        })
                    } else { None };
                    view! {
                        <select on:change=on_change>
                            <option value="" selected=current.is_empty()>{ tr("gldf.files.content_type_select") }</option>
                            {CONTENT_TYPES.iter().map(|v| {
                                let selected = current == *v;
                                view! { <option value=(*v).to_string() selected=selected>{ *v }</option> }
                            }).collect::<Vec<_>>()}
                            { legacy_pill }
                        </select>
                    }.into_any()
                } else {
                    view! {
                        <span class=format!("content-type-badge {}", ct_class)>{ content_type_for_ct_cell.clone() }</span>
                    }.into_any()
                }}
            </td>
            <td class="file-type">
                {move || if editable && is_editing.get() {
                    view! { <TextField value=s_type /> }.into_any()
                } else {
                    view! { <span>{ type_attr_for_type_cell.clone() }</span> }.into_any()
                }}
            </td>
            <td class="file-download-cell">
                <span style="display: inline-flex; gap: 6px;">
                    {if embedded {
                        view! {
                            <button
                                class="btn-download-file"
                                on:click=on_download
                                title=tr("gldf.files.download_title").replace("{file_name}", &file_name_for_title)
                                style="background: none; border: 1px solid var(--border-color); border-radius: 4px; padding: 4px 8px; cursor: pointer; color: var(--accent-blue);"
                            >{ "⬇" }</button>
                        }.into_any()
                    } else {
                        view! { <span style="color: var(--text-tertiary); font-size: 11px;">{ "—" }</span> }.into_any()
                    }}
                    {if editable {
                        // Defining the handlers here (inside the editable branch
                        // which evaluates exactly once per row, not per reactive
                        // re-render) keeps them as plain Fn closures.
                        let id_for_save = id_for_save.clone();
                        let id_for_remove = id_for_remove.clone();
                        let on_edit = move |_| is_editing.set(true);
                        let on_cancel = move |_| is_editing.set(false);
                        let on_save = move |_| {
                            let id = id_for_save.clone();
                            let file_name = s_name.get();
                            let content_type = s_ct.get();
                            let type_attr = s_type.get();
                            state.update(|s| {
                                s.apply(GldfAction::UpdateFile { id, content_type, type_attr, file_name });
                            });
                            is_editing.set(false);
                        };
                        let on_remove = move |_| {
                            let id = id_for_remove.clone();
                            let template = ui_t(
                                "gldf.files.remove_confirm",
                                &state.with(|s| s.ui_language.clone()),
                            );
                            let msg = template.replace("{id}", &id);
                            if crate::components::edit_util::confirm(&msg)
                            {
                                state.update(|s| s.apply(GldfAction::RemoveFile(id)));
                            }
                        };
                        view! {
                            <ActionCellButtons
                                is_editing=is_editing
                                on_edit=on_edit
                                on_cancel=on_cancel
                                on_save=on_save
                                on_remove=on_remove
                            />
                        }.into_any()
                    } else { ().into_any() }}
                </span>
            </td>
        </tr>
    }
}

/// Renders the per-row action buttons (Edit/Remove or Save/Cancel) based
/// on the `is_editing` signal. Keeping this inside its own component lets
/// us pass per-row callbacks as plain typed props without fighting
/// Leptos's reactive-closure trait bounds.
#[component]
fn ActionCellButtons(
    is_editing: RwSignal<bool>,
    on_edit: impl Fn(crate::components::edit_util::ClickEvt) + 'static + Send + Sync + Clone,
    on_cancel: impl Fn(crate::components::edit_util::ClickEvt) + 'static + Send + Sync + Clone,
    on_save: impl Fn(crate::components::edit_util::ClickEvt) + 'static + Send + Sync + Clone,
    on_remove: impl Fn(crate::components::edit_util::ClickEvt) + 'static + Send + Sync + Clone,
) -> impl IntoView {
    view! {
        {move || if is_editing.get() {
            let on_save = on_save.clone();
            let on_cancel = on_cancel.clone();
            view! {
                <>
                    <button
                        on:click=on_save
                        style="padding: 2px 8px; font-size: 11px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                    >{ "Save" }</button>
                    <button
                        on:click=on_cancel
                        style="padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                    >{ "Cancel" }</button>
                </>
            }.into_any()
        } else {
            let on_edit = on_edit.clone();
            let on_remove = on_remove.clone();
            view! {
                <>
                    <button
                        on:click=on_edit
                        style="padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                    >{ "✎" }</button>
                    <button
                        on:click=on_remove
                        style="padding: 2px 8px; font-size: 11px; background: none; border: 1px solid var(--accent-orange); border-radius: 4px; cursor: pointer; color: var(--accent-orange);"
                    >{ "Remove" }</button>
                </>
            }.into_any()
        }}
    }
}

#[component]
fn TextField(value: RwSignal<String>) -> impl IntoView {
    let on_input = move |e: crate::components::edit_util::EditEvent| {
        let Some(v) = crate::components::edit_util::event_value(&e) else { return };
        value.set(v);
    };
    view! {
        <input
            type="text"
            prop:value=move || value.get()
            on:input=on_input
            style="width: 100%; padding: 3px 6px; font-size: 12px; background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 4px; color: var(--text-primary);"
        />
    }
}

pub fn content_type_class(ct: &str) -> &'static str {
    if ct.starts_with("ldc/") {
        "content-type-ldc"
    } else if ct.starts_with("geo/") {
        "content-type-geo"
    } else if ct.starts_with("image/") {
        "content-type-image"
    } else if ct.starts_with("sensor/") {
        "content-type-sensor"
    } else if ct.starts_with("document/") {
        "content-type-doc"
    } else if ct.starts_with("spectrum/") {
        "content-type-spectrum"
    } else {
        "content-type-other"
    }
}
