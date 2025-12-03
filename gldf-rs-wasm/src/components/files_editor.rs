//! Files editor component for GLDF files

use yew::prelude::*;
use crate::state::{use_gldf, GldfAction};

/// File content types available in GLDF
const CONTENT_TYPES: &[(&str, &str)] = &[
    ("ldc/eulumdat", "Eulumdat (LDC)"),
    ("ldc/ies", "IES (LDC)"),
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
const FILE_TYPES: &[(&str, &str)] = &[
    ("localFileName", "Local File"),
    ("url", "URL"),
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
                                <option value={*val} selected={*val == *edit_content_type}>{ label }</option>
                            }
                        })}
                    </select>
                </td>
                <td>
                    <select value={(*edit_type_attr).clone()} onchange={on_type_change}>
                        { for FILE_TYPES.iter().map(|(val, label)| {
                            html! {
                                <option value={*val} selected={*val == *edit_type_attr}>{ label }</option>
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
        Callback::from(move |(id, content_type, type_attr, file_name): (String, String, String, String)| {
            gldf.dispatch(GldfAction::UpdateFile {
                id,
                content_type,
                type_attr,
                file_name,
            });
        })
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
                                    html! { <option value={*val}>{ label }</option> }
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
                                    html! { <option value={*val}>{ label }</option> }
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
        </div>
    }
}
