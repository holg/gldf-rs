//! Pictures read view — thumbnail grid with type captions.
//!
//! Renders the inline image via base64 data URL when the referenced file is
//! embedded (jpg / jpeg / png), otherwise just lists the file id + image type.

use crate::state::use_gldf;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use leptos::prelude::*;

#[component]
pub fn PicturesReadPage() -> impl IntoView {
    let state = use_gldf();

    view! {
        <div class="editor-section pictures-readonly">
            <h2>{ "Pictures" }</h2>
            <p class="section-description" style="color: var(--text-tertiary);">
                { "Image references from ProductMetaData." }
            </p>

            {move || state.with(|s| {
                let pictures = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.pictures.as_ref())
                    .map(|p| p.image.clone())
                    .unwrap_or_default();
                if pictures.is_empty() {
                    return view! { <p style="color: var(--text-tertiary);">{ "No pictures referenced." }</p> }.into_any();
                }

                view! {
                    <div class="pictures-grid" style="display: flex; gap: 12px; flex-wrap: wrap;">
                        {pictures.into_iter().map(|img| {
                            let file_id_text = img.file_id.clone();
                            let image_type_text = img.image_type.clone();
                            // Look up the file in GeneralDefinitions/Files and try to
                            // pull bytes from embedded_files. The data URL trick from
                            // FileViewer covers .jpg/.jpeg/.png.
                            let file_def = s
                                .product
                                .general_definitions
                                .files
                                .file
                                .iter()
                                .find(|f| f.id == img.file_id);
                            let preview = file_def.and_then(|f| {
                                let lower = f.file_name.to_lowercase();
                                let mime = if lower.ends_with(".png") {
                                    Some("png")
                                } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
                                    Some("jpeg")
                                } else {
                                    None
                                }?;
                                let bytes = s.embedded_files.get(&f.id)?;
                                Some(format!("data:image/{};base64,{}", mime, BASE64_STANDARD.encode(bytes)))
                            });
                            view! {
                                <div class="picture-card" style="border: 1px solid var(--border-color); border-radius: 6px; padding: 8px; max-width: 220px;">
                                    {match preview {
                                        Some(data_url) => view! {
                                            <img
                                                src=data_url
                                                style="width: 100%; height: 160px; object-fit: contain; background: var(--bg-secondary); border-radius: 4px;"
                                            />
                                        }.into_any(),
                                        None => view! {
                                            <div style="width: 100%; height: 160px; background: var(--bg-secondary); border-radius: 4px; display: flex; align-items: center; justify-content: center; color: var(--text-tertiary); font-size: 28px;">
                                                { "🖼️" }
                                            </div>
                                        }.into_any(),
                                    }}
                                    <div style="margin-top: 6px; font-size: 11px;">
                                        <div class="code" style="font-family: var(--font-mono); color: var(--text-secondary);">{ file_id_text }</div>
                                        <div style="color: var(--text-tertiary);">{ image_type_text }</div>
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            })}
        </div>
    }
}
