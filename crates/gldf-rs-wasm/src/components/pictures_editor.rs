//! Pictures editor (ProductMetaData.Pictures).
//!
//! Each `<Image>` references a `<File>` from `GeneralDefinitions/Files` by
//! `@fileId` and tags it with `@imageType` from a closed XSD enum.
//! The BIOLUX HCL DL fixture emits `product.png` as a File but never references
//! it from a Pictures block — surfacing this lets the user wire it up in one
//! click without leaving the editor.

use crate::state::{use_gldf, GldfAction};
use yew::prelude::*;

/// XSD `imageType` enumeration. Sourced via the shared canonical table
/// in [`crate::i18n::xsd_enums`] so a future XSD change only needs one
/// edit. Same 4 values either way, but the indirection makes the
/// dependency explicit.
use crate::i18n::xsd_enums::IMAGE_TYPES;

#[function_component(PicturesEditor)]
pub fn pictures_editor() -> Html {
    let gldf = use_gldf();

    // Collect candidate fileIds whose contentType looks like an image, so the
    // dropdown only offers things the user can actually reference.
    let image_files: Vec<String> = gldf
        .product
        .general_definitions
        .files
        .file
        .iter()
        .filter(|f| f.content_type.starts_with("image/"))
        .map(|f| f.id.clone())
        .collect();

    let pictures = gldf
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.pictures.as_ref());

    let pending_file_id = use_state(String::new);
    let pending_image_type = use_state(|| "Product Picture".to_string());

    let on_pending_file_change = {
        let pending_file_id = pending_file_id.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            pending_file_id.set(select.value());
        })
    };

    let on_pending_type_change = {
        let pending_image_type = pending_image_type.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            pending_image_type.set(select.value());
        })
    };

    let on_add = {
        let gldf = gldf.clone();
        let pending_file_id = pending_file_id.clone();
        let pending_image_type = pending_image_type.clone();
        Callback::from(move |_| {
            let file_id = pending_file_id.trim().to_string();
            if file_id.is_empty() {
                return;
            }
            gldf.dispatch(GldfAction::AddProductMetaPicture {
                file_id,
                image_type: (*pending_image_type).clone(),
            });
            pending_file_id.set(String::new());
        })
    };

    html! {
        <div class="editor-section pictures-editor">
            <h2>{ "Pictures" }</h2>
            <p class="section-description">
                { "Image references from ProductMetaData. Each entry points to a File defined in General Definitions and tags it with an XSD-enumerated image type." }
            </p>

            { render_existing_pictures(pictures, &gldf) }

            <div class="add-picture-row">
                <h4>{ "Add picture" }</h4>
                if image_files.is_empty() {
                    <p class="hint">
                        { "No image-typed files in General Definitions yet. Add a File with contentType starting with image/ first." }
                    </p>
                } else {
                    <div class="form-row">
                        <label>
                            { "File:" }
                            <select onchange={on_pending_file_change} value={(*pending_file_id).clone()}>
                                <option value="" selected={pending_file_id.is_empty()}>{ "(select a file)" }</option>
                                { for image_files.iter().map(|id| {
                                    let selected = id == &*pending_file_id;
                                    html! { <option value={id.clone()} selected={selected}>{ id }</option> }
                                }) }
                            </select>
                        </label>
                        <label>
                            { "Type:" }
                            <select onchange={on_pending_type_change} value={(*pending_image_type).clone()}>
                                { for IMAGE_TYPES.iter().map(|t| {
                                    let selected = *t == pending_image_type.as_str();
                                    html! { <option value={(*t).to_string()} selected={selected}>{ *t }</option> }
                                }) }
                            </select>
                        </label>
                        <button type="button" onclick={on_add} disabled={pending_file_id.is_empty()}>
                            { "+ Add" }
                        </button>
                    </div>
                }
            </div>
        </div>
    }
}

fn render_existing_pictures(
    pictures: Option<&gldf_rs::gldf::general_definitions::Images>,
    gldf: &crate::state::GldfContext,
) -> Html {
    let images = pictures.map(|p| p.image.as_slice()).unwrap_or(&[]);

    if images.is_empty() {
        return html! {
            <p class="empty-message">{ "No pictures referenced." }</p>
        };
    }

    html! {
        <div class="pictures-list">
            { for images.iter().enumerate().map(|(idx, img)| {
                let on_type_change = {
                    let gldf = gldf.clone();
                    let file_id = img.file_id.clone();
                    Callback::from(move |e: Event| {
                        let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
                        gldf.dispatch(GldfAction::UpdateProductMetaPicture {
                            index: idx,
                            file_id: file_id.clone(),
                            image_type: select.value(),
                        });
                    })
                };
                let on_remove = {
                    let gldf = gldf.clone();
                    Callback::from(move |_| {
                        gldf.dispatch(GldfAction::RemoveProductMetaPicture(idx));
                    })
                };
                let current_type = img.image_type.clone();

                html! {
                    <div class="picture-row" key={format!("{}-{}", idx, img.file_id)}>
                        <span class="picture-file-id code">{ &img.file_id }</span>
                        <select onchange={on_type_change} value={current_type.clone()}>
                            { for IMAGE_TYPES.iter().map(|t| {
                                let selected = *t == current_type.as_str();
                                html! { <option value={(*t).to_string()} selected={selected}>{ *t }</option> }
                            }) }
                        </select>
                        <button type="button" class="picture-remove" title="Remove" onclick={on_remove}>
                            { "✕" }
                        </button>
                    </div>
                }
            }) }
        </div>
    }
}
