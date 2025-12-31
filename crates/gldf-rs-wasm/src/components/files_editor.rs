//! Files editor component for GLDF files

use crate::state::{use_gldf, GldfAction};
use wasm_bindgen::prelude::*;
use yew::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = hasEmbeddedViewer)]
    fn has_embedded_viewer(viewer_type: &str) -> bool;
}

/// File content types available in GLDF
const CONTENT_TYPES: &[(&str, &str)] = &[
    ("ldc/eulumdat", "Eulumdat (LDC)"),
    ("ldc/ies", "IES (LDC)"),
    ("ldc/iesxml", "TM-33 IESXML (LDC)"),
    ("geo/l3d", "L3D Geometry"),
    ("geo/m3d", "M3D Geometry"),
    ("geo/r3d", "R3D Geometry"),
    ("image/png", "PNG Image"),
    ("image/jpg", "JPEG Image"),
    ("image/svg", "SVG Image"),
    ("document/pdf", "PDF Document"),
    ("spectrum/txt", "Spectrum File"),
    ("sensor/sens-ldt", "Sensor LDT"),
    ("other", "Other"),
];

/// File types (local vs URL)
const FILE_TYPES: &[(&str, &str)] = &[("localFileName", "Local File"), ("url", "URL")];

/// Available WASM viewer modules that can be embedded
const WASM_VIEWERS: &[(&str, &str, &str, &str)] = &[
    ("bevy", "Bevy 3D Viewer", "3D scene visualization with lighting simulation", "/bevy/"),
    ("typst", "Typst PDF Export", "Generate PDF reports from photometric data", "/typst/"),
    ("acadlisp", "AcadLISP Engine", "AutoLISP interpreter for CAD integration", "/acadlisp/"),
    ("starsky", "Star Sky Viewer", "2D celestial visualization", "/starsky/"),
];

/// Properties for file row
#[derive(Properties, Clone, PartialEq)]
struct FileRowProps {
    file_id: String,
    content_type: String,
    type_attr: String,
    file_name: String,
    on_update: Callback<(String, String, String, String)>,
    on_remove: Callback<String>,
}

/// Single file row component
#[function_component(FileRow)]
fn file_row(props: &FileRowProps) -> Html {
    let editing = use_state(|| false);
    let edit_content_type = use_state(|| props.content_type.clone());
    let edit_type_attr = use_state(|| props.type_attr.clone());
    let edit_file_name = use_state(|| props.file_name.clone());

    let on_edit_click = {
        let editing = editing.clone();
        let edit_content_type = edit_content_type.clone();
        let edit_type_attr = edit_type_attr.clone();
        let edit_file_name = edit_file_name.clone();
        let content_type = props.content_type.clone();
        let type_attr = props.type_attr.clone();
        let file_name = props.file_name.clone();
        Callback::from(move |_| {
            edit_content_type.set(content_type.clone());
            edit_type_attr.set(type_attr.clone());
            edit_file_name.set(file_name.clone());
            editing.set(true);
        })
    };

    let on_save_click = {
        let editing = editing.clone();
        let on_update = props.on_update.clone();
        let file_id = props.file_id.clone();
        let edit_content_type = edit_content_type.clone();
        let edit_type_attr = edit_type_attr.clone();
        let edit_file_name = edit_file_name.clone();
        Callback::from(move |_| {
            on_update.emit((
                file_id.clone(),
                (*edit_content_type).clone(),
                (*edit_type_attr).clone(),
                (*edit_file_name).clone(),
            ));
            editing.set(false);
        })
    };

    let on_cancel_click = {
        let editing = editing.clone();
        Callback::from(move |_| {
            editing.set(false);
        })
    };

    let on_remove_click = {
        let on_remove = props.on_remove.clone();
        let file_id = props.file_id.clone();
        Callback::from(move |_| {
            on_remove.emit(file_id.clone());
        })
    };

    if *editing {
        let on_content_type_change = {
            let edit_content_type = edit_content_type.clone();
            Callback::from(move |e: Event| {
                let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
                edit_content_type.set(select.value());
            })
        };

        let on_type_change = {
            let edit_type_attr = edit_type_attr.clone();
            Callback::from(move |e: Event| {
                let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
                edit_type_attr.set(select.value());
            })
        };

        let on_filename_change = {
            let edit_file_name = edit_file_name.clone();
            Callback::from(move |e: Event| {
                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                edit_file_name.set(input.value());
            })
        };

        html! {
            <tr class="file-row editing">
                <td>{ &props.file_id }</td>
                <td>
                    <select value={(*edit_content_type).clone()} onchange={on_content_type_change}>
                        { for CONTENT_TYPES.iter().map(|(val, label)| {
                            html! {
                                <option value={*val} selected={*val == *edit_content_type}>{ *label }</option>
                            }
                        })}
                    </select>
                </td>
                <td>
                    <select value={(*edit_type_attr).clone()} onchange={on_type_change}>
                        { for FILE_TYPES.iter().map(|(val, label)| {
                            html! {
                                <option value={*val} selected={*val == *edit_type_attr}>{ *label }</option>
                            }
                        })}
                    </select>
                </td>
                <td>
                    <input
                        type="text"
                        value={(*edit_file_name).clone()}
                        onchange={on_filename_change}
                    />
                </td>
                <td class="actions">
                    <button class="btn-save" onclick={on_save_click}>{ "Save" }</button>
                    <button class="btn-cancel" onclick={on_cancel_click}>{ "Cancel" }</button>
                </td>
            </tr>
        }
    } else {
        html! {
            <tr class="file-row">
                <td>{ &props.file_id }</td>
                <td>{ &props.content_type }</td>
                <td>{ &props.type_attr }</td>
                <td class="filename">{ &props.file_name }</td>
                <td class="actions">
                    <button class="btn-edit" onclick={on_edit_click}>{ "Edit" }</button>
                    <button class="btn-remove" onclick={on_remove_click}>{ "Remove" }</button>
                </td>
            </tr>
        }
    }
}

/// Files editor component
#[function_component(FilesEditor)]
pub fn files_editor() -> Html {
    let gldf = use_gldf();
    let files = &gldf.product.general_definitions.files.file;

    let show_add_form = use_state(|| false);
    let new_id = use_state(String::new);
    let new_content_type = use_state(|| "ldc/eulumdat".to_string());
    let new_type_attr = use_state(|| "localFileName".to_string());
    let new_file_name = use_state(String::new);

    let on_show_add = {
        let show_add_form = show_add_form.clone();
        Callback::from(move |_| {
            show_add_form.set(true);
        })
    };

    let on_cancel_add = {
        let show_add_form = show_add_form.clone();
        let new_id = new_id.clone();
        let new_file_name = new_file_name.clone();
        Callback::from(move |_| {
            show_add_form.set(false);
            new_id.set(String::new());
            new_file_name.set(String::new());
        })
    };

    let on_add_file = {
        let gldf = gldf.clone();
        let show_add_form = show_add_form.clone();
        let new_id = new_id.clone();
        let new_content_type = new_content_type.clone();
        let new_type_attr = new_type_attr.clone();
        let new_file_name = new_file_name.clone();
        Callback::from(move |_| {
            if !(*new_id).is_empty() && !(*new_file_name).is_empty() {
                gldf.dispatch(GldfAction::AddFile {
                    id: (*new_id).clone(),
                    content_type: (*new_content_type).clone(),
                    type_attr: (*new_type_attr).clone(),
                    file_name: (*new_file_name).clone(),
                    language: None,
                });
                show_add_form.set(false);
                new_id.set(String::new());
                new_file_name.set(String::new());
            }
        })
    };

    let on_update_file = {
        let gldf = gldf.clone();
        Callback::from(
            move |(id, content_type, type_attr, file_name): (String, String, String, String)| {
                gldf.dispatch(GldfAction::UpdateFile {
                    id,
                    content_type,
                    type_attr,
                    file_name,
                });
            },
        )
    };

    let on_remove_file = {
        let gldf = gldf.clone();
        Callback::from(move |id: String| {
            gldf.dispatch(GldfAction::RemoveFile(id));
        })
    };

    html! {
        <div class="editor-section files-editor">
            <h2>{ "Files" }</h2>
            <p class="section-description">
                { "Manage file references in your GLDF package. Files include photometric data (LDC), geometries, images, and documents." }
            </p>

            <table class="files-table">
                <thead>
                    <tr>
                        <th>{ "ID" }</th>
                        <th>{ "Content Type" }</th>
                        <th>{ "Type" }</th>
                        <th>{ "File Name / URL" }</th>
                        <th>{ "Actions" }</th>
                    </tr>
                </thead>
                <tbody>
                    { for files.iter().map(|f| {
                        html! {
                            <FileRow
                                key={f.id.clone()}
                                file_id={f.id.clone()}
                                content_type={f.content_type.clone()}
                                type_attr={f.type_attr.clone()}
                                file_name={f.file_name.clone()}
                                on_update={on_update_file.clone()}
                                on_remove={on_remove_file.clone()}
                            />
                        }
                    })}
                </tbody>
            </table>

            if files.is_empty() {
                <p class="empty-message">{ "No files defined. Click 'Add File' to add one." }</p>
            }

            if *show_add_form {
                <div class="add-file-form">
                    <h3>{ "Add New File" }</h3>
                    <div class="form-row">
                        <div class="form-group">
                            <label>{ "File ID" }</label>
                            <input
                                type="text"
                                value={(*new_id).clone()}
                                onchange={{
                                    let new_id = new_id.clone();
                                    Callback::from(move |e: Event| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        new_id.set(input.value());
                                    })
                                }}
                                placeholder="e.g., photometry-1"
                            />
                        </div>
                        <div class="form-group">
                            <label>{ "Content Type" }</label>
                            <select
                                value={(*new_content_type).clone()}
                                onchange={{
                                    let new_content_type = new_content_type.clone();
                                    Callback::from(move |e: Event| {
                                        let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
                                        new_content_type.set(select.value());
                                    })
                                }}
                            >
                                { for CONTENT_TYPES.iter().map(|(val, label)| {
                                    html! { <option value={*val}>{ *label }</option> }
                                })}
                            </select>
                        </div>
                    </div>
                    <div class="form-row">
                        <div class="form-group">
                            <label>{ "Type" }</label>
                            <select
                                value={(*new_type_attr).clone()}
                                onchange={{
                                    let new_type_attr = new_type_attr.clone();
                                    Callback::from(move |e: Event| {
                                        let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
                                        new_type_attr.set(select.value());
                                    })
                                }}
                            >
                                { for FILE_TYPES.iter().map(|(val, label)| {
                                    html! { <option value={*val}>{ *label }</option> }
                                })}
                            </select>
                        </div>
                        <div class="form-group">
                            <label>{ "File Name / URL" }</label>
                            <input
                                type="text"
                                value={(*new_file_name).clone()}
                                onchange={{
                                    let new_file_name = new_file_name.clone();
                                    Callback::from(move |e: Event| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        new_file_name.set(input.value());
                                    })
                                }}
                                placeholder="e.g., example.ldt or https://..."
                            />
                        </div>
                    </div>
                    <div class="form-actions">
                        <button class="btn-primary" onclick={on_add_file}>{ "Add File" }</button>
                        <button class="btn-secondary" onclick={on_cancel_add}>{ "Cancel" }</button>
                    </div>
                </div>
            } else {
                <button class="btn-add" onclick={on_show_add}>{ "+ Add File" }</button>
            }

            <hr style="margin: 32px 0; border-color: var(--border-color);" />

            // Embedded WASM Viewers section
            <WasmViewersSection />
        </div>
    }
}

/// Properties for WasmViewersSection
#[derive(Properties, Clone, PartialEq)]
pub struct WasmViewersSectionProps {
    #[prop_or_default]
    pub on_embed: Option<Callback<String>>,
    #[prop_or_default]
    pub on_remove: Option<Callback<String>>,
    #[prop_or_default]
    pub loading_viewer: Option<String>,
    #[prop_or_default]
    pub has_gldf: bool,
}

/// Section for managing embedded WASM viewer modules
#[function_component(WasmViewersSection)]
pub fn wasm_viewers_section(props: &WasmViewersSectionProps) -> Html {
    let error_msg = use_state(|| None::<String>);

    // Check which viewers are currently embedded
    let embedded_viewers: Vec<&str> = WASM_VIEWERS
        .iter()
        .filter(|(id, _, _, _)| has_embedded_viewer(id))
        .map(|(id, _, _, _)| *id)
        .collect();

    html! {
        <div class="wasm-viewers-section">
            <h3 style="margin-bottom: 12px; display: flex; align-items: center; gap: 8px;">
                <span style="font-size: 18px;">{ "🔌" }</span>
                { "Embedded WASM Viewers" }
                <span
                    class="help-icon"
                    title="WASM modules can be embedded in the GLDF file for self-contained operation.\nThey load from other/viewer/<name>/ inside the GLDF ZIP."
                    style="cursor: help; color: var(--accent-blue); font-size: 14px;"
                >
                    { "ⓘ" }
                </span>
            </h3>
            <p class="section-description" style="margin-bottom: 16px; font-size: 12px; color: var(--text-secondary);">
                { "Embed WASM modules for offline viewing. Modules are stored in " }
                <code style="background: var(--bg-tertiary); padding: 2px 6px; border-radius: 3px;">
                    { "other/viewer/<name>/" }
                </code>
            </p>

            <div class="wasm-viewers-grid" style="display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 12px;">
                { for WASM_VIEWERS.iter().map(|(id, name, desc, url_path)| {
                    let is_embedded = embedded_viewers.contains(id);
                    let is_loading = props.loading_viewer.as_deref() == Some(*id);
                    let viewer_id = id.to_string();
                    let viewer_id_remove = id.to_string();

                    let on_embed_click = props.on_embed.as_ref().map(|cb| {
                        let cb = cb.clone();
                        let vid = viewer_id.clone();
                        Callback::from(move |_| cb.emit(vid.clone()))
                    });

                    let on_remove_click = props.on_remove.as_ref().map(|cb| {
                        let cb = cb.clone();
                        let vid = viewer_id_remove.clone();
                        Callback::from(move |_| cb.emit(vid.clone()))
                    });

                    html! {
                        <div
                            class="wasm-viewer-card"
                            style={format!(
                                "background: var(--bg-secondary); border: 1px solid {}; border-radius: 8px; padding: 16px;",
                                if is_embedded { "var(--accent-green)" } else { "var(--border-color)" }
                            )}
                        >
                            <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 8px;">
                                <div>
                                    <strong style="font-size: 14px;">{ *name }</strong>
                                    if is_embedded {
                                        <span style="margin-left: 8px; color: var(--accent-green); font-size: 11px;">
                                            { "✓ Embedded" }
                                        </span>
                                    }
                                </div>
                                <code style="font-size: 10px; color: var(--text-tertiary); background: var(--bg-tertiary); padding: 2px 6px; border-radius: 3px;">
                                    { *id }
                                </code>
                            </div>
                            <p style="font-size: 11px; color: var(--text-secondary); margin-bottom: 12px;">
                                { *desc }
                            </p>
                            <div style="display: flex; gap: 8px; flex-wrap: wrap;">
                                if is_embedded {
                                    <span style="font-size: 11px; color: var(--text-tertiary);">
                                        { "Located in: " }
                                        <code style="background: var(--bg-tertiary); padding: 1px 4px; border-radius: 2px;">
                                            { format!("other/viewer/{}/", id) }
                                        </code>
                                    </span>
                                } else {
                                    <div style="font-size: 11px; color: var(--text-tertiary);">
                                        { "Available from: " }
                                        <code style="background: var(--bg-tertiary); padding: 1px 4px; border-radius: 2px;">
                                            { *url_path }
                                        </code>
                                    </div>
                                }
                            </div>
                            <div style="margin-top: 12px; display: flex; gap: 8px;">
                                if is_embedded {
                                    if let Some(ref on_remove) = on_remove_click {
                                        if props.has_gldf {
                                            <button
                                                class="btn-small"
                                                style="font-size: 11px; padding: 4px 10px; background: var(--accent-red); color: white; border: none; border-radius: 4px; cursor: pointer;"
                                                onclick={on_remove.clone()}
                                                title="Remove this viewer from the GLDF"
                                            >
                                                { "Remove" }
                                            </button>
                                        }
                                    }
                                } else {
                                    if let Some(ref on_embed) = on_embed_click {
                                        if props.has_gldf {
                                            <button
                                                class="btn-small"
                                                style="font-size: 11px; padding: 4px 10px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                                                disabled={is_loading}
                                                onclick={on_embed.clone()}
                                                title={format!("Embed {} from {}", name, url_path)}
                                            >
                                                if is_loading {
                                                    { "Loading..." }
                                                } else {
                                                    { "Embed from URL" }
                                                }
                                            </button>
                                        } else {
                                            <span style="font-size: 11px; color: var(--text-tertiary); font-style: italic;">
                                                { "Load a GLDF file first" }
                                            </span>
                                        }
                                    }
                                }
                            </div>
                        </div>
                    }
                })}
            </div>

            if let Some(ref err) = *error_msg {
                <div style="margin-top: 12px; padding: 12px; background: rgba(255, 100, 100, 0.1); border: 1px solid var(--accent-red); border-radius: 4px; color: var(--accent-red); font-size: 12px;">
                    { err }
                </div>
            }

            <div class="help-box" style="margin-top: 16px; padding: 12px; background: var(--bg-tertiary); border-radius: 6px; font-size: 11px; color: var(--text-secondary);">
                <strong>{ "How it works:" }</strong>
                <ul style="margin: 8px 0 0 16px; padding: 0;">
                    <li>{ "Embedded viewers are stored in " }<code>{ "other/viewer/<name>/" }</code></li>
                    <li>{ "Each viewer has a " }<code>{ "manifest.json" }</code>{ " describing its files" }</li>
                    <li>{ "Viewers include .js loader and .wasm binary files" }</li>
                    <li>{ "Embedded viewers work offline without network access" }</li>
                </ul>
            </div>
        </div>
    }
}
