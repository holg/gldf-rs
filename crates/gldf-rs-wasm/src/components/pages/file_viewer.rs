//! File Viewer page — click a row to preview the embedded file inline.
//!
//! Slice 1B port of `view_file_viewer` + `view_file_content` from the Yew side.
//! Now wired to the dedicated renderers:
//! - `.ldt` / `.ies` → `LdtPanel` (goniometric diagram)
//! - `.spd`           → `SpdPanel` (spectrum + colorimetry)
//! - `.l3d`           → `L3dPanel` (three-d WebGL viewer)
//! - `.tm33.xml` / IESTM33-22 / LuminaireOpticalData XML
//!                    → `Tm33Panel` (photometry + SPD in one)
//! - `.jpg` / `.png`  → `<img>` (base64 data-url)
//! - everything else  → truncated UTF-8 text preview
//!
//! Also surfaces a banner of orphan files — `<File>` entries that no
//! `<Photometry>` block references — so the file count and the
//! Photometry-page count can be reconciled in one place.

use crate::components::{L3dPanel, LdtPanel, SpdPanel, Tm33Panel};
use crate::exports::download_embedded_file;
use crate::i18n::ui::t as ui_t;
use crate::state::use_gldf;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use leptos::prelude::*;

#[component]
pub fn FileViewerPage() -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    let selected = RwSignal::new(None::<String>);

    view! {
        {move || state.with(|s| {
            let files = s.product.general_definitions.files.file.clone();
            if files.is_empty() {
                return view! {
                    <div class="empty-state">
                        <div class="icon">{ "📂" }</div>
                        <h3>{ "No Files" }</h3>
                        <p>{ "Load a GLDF file to browse its embedded files" }</p>
                    </div>
                }.into_any();
            }
            let embedded: std::collections::HashSet<String> = s.embedded_files.keys().cloned().collect();

            // Orphan-file diagnostic: ldc/* and spectrum/* <File> entries
            // that no <Photometry> / <Spectrum> block references. Legal GLDF
            // (a <File> is just a content blob) but easy to miss elsewhere
            // in the editor.
            let referenced_photometry_ids: std::collections::HashSet<String> = s
                .product
                .general_definitions
                .photometries
                .as_ref()
                .map(|ps| {
                    ps.photometry
                        .iter()
                        .filter_map(|p| p.photometry_file_reference.as_ref().map(|r| r.file_id.clone()))
                        .collect()
                })
                .unwrap_or_default();
            let referenced_spectrum_ids: std::collections::HashSet<String> = s
                .product
                .general_definitions
                .spectrums
                .as_ref()
                .map(|ss| {
                    ss.spectrum
                        .iter()
                        .filter_map(|sp| sp.spectrum_file_reference.as_ref().map(|r| r.file_id.clone()))
                        .collect()
                })
                .unwrap_or_default();
            let orphan_photometries: Vec<(String, String, String)> = s
                .product
                .general_definitions
                .files
                .file
                .iter()
                .filter(|f| f.content_type.starts_with("ldc/") && !referenced_photometry_ids.contains(&f.id))
                .map(|f| (f.id.clone(), f.content_type.clone(), f.file_name.clone()))
                .collect();
            let orphan_spectrums: Vec<(String, String, String)> = s
                .product
                .general_definitions
                .files
                .file
                .iter()
                .filter(|f| f.content_type.starts_with("spectrum/") && !referenced_spectrum_ids.contains(&f.id))
                .map(|f| (f.id.clone(), f.content_type.clone(), f.file_name.clone()))
                .collect();

            view! {
                <div class="files-container">
                    <OrphanBanner
                        orphan_photometries=orphan_photometries
                        orphan_spectrums=orphan_spectrums
                    />
                    <div class="card">
                        <table class="data-table files-table-clickable">
                            <thead>
                                <tr>
                                    <th>{ tr("gldf.files.col_id") }</th>
                                    <th>{ tr("gldf.files.col_file_name") }</th>
                                    <th>{ tr("gldf.file_viewer.col_content_type") }</th>
                                </tr>
                            </thead>
                            <tbody>
                                {files.into_iter().map(|f| {
                                    let id = f.id.clone();
                                    let id_for_click = f.id.clone();
                                    let file_name = f.file_name.clone();
                                    let content_type = f.content_type.clone();
                                    let ct_class = crate::components::pages::files_list::content_type_class(&content_type);
                                    let has_content = embedded.contains(&id);
                                    let is_selected = selected.with(|s| s.as_deref() == Some(&id));
                                    let row_class = match (has_content, is_selected) {
                                        (true, true) => "selected clickable",
                                        (true, false) => "clickable",
                                        (false, _) => "",
                                    };
                                    let on_click = move |_| {
                                        if has_content {
                                            selected.set(Some(id_for_click.clone()));
                                        }
                                    };
                                    let cursor = if has_content { "cursor: pointer;" } else { "" };
                                    view! {
                                        <tr class=row_class on:click=on_click style=cursor>
                                            <td class="file-id">{ id }</td>
                                            <td class="file-name">{ file_name }</td>
                                            <td>
                                                <span class=format!("content-type-badge {}", ct_class)>{ content_type }</span>
                                            </td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    </div>
                    <FilePreview selected=selected />
                </div>
            }.into_any()
        })}
    }
}

#[component]
fn FilePreview(selected: RwSignal<Option<String>>) -> impl IntoView {
    let state = use_gldf();
    view! {
        {move || {
            let id = match selected.get() {
                Some(s) => s,
                None => return ().into_any(),
            };
            let lookup = state.with(|st| {
                let bytes = st.embedded_files.get(&id).cloned()?;
                let f = st
                    .product
                    .general_definitions
                    .files
                    .file
                    .iter()
                    .find(|f| f.id == id)?;
                Some((f.file_name.clone(), f.content_type.clone(), bytes))
            });
            let Some((file_name, content_type, bytes)) = lookup else { return ().into_any() };
            let len = bytes.len();
            let close = move |_| selected.set(None);
            let download_id = id.clone();
            let on_download = move |_| {
                if let Err(err) = download_embedded_file(state, &download_id) {
                    log::error!("download {}: {}", download_id, err);
                }
            };
            view! {
                <div class="file-viewer-panel" style="margin-top: 20px; background: var(--bg-card); border-radius: 8px; overflow: hidden;">
                    <div style="display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--border-color); background: var(--bg-sidebar);">
                        <div style="font-weight: 500;">
                            { file_name.clone() }
                            <span style="color: var(--text-tertiary); margin-left: 8px; font-size: 12px;">{ format!("({} bytes)", len) }</span>
                        </div>
                        <div style="display: flex; gap: 8px; align-items: center;">
                            <button
                                on:click=on_download
                                style="background: none; border: 1px solid var(--border-color); border-radius: 4px; padding: 4px 10px; cursor: pointer; color: var(--accent-blue); font-size: 13px;"
                            >
                                { "⬇ Download" }
                            </button>
                            <button
                                on:click=close
                                style="background: none; border: none; cursor: pointer; font-size: 18px; color: var(--text-secondary);"
                            >
                                { "✕" }
                            </button>
                        </div>
                    </div>
                    <div style="padding: 16px;">
                        { render_file_content(&file_name, &content_type, &bytes) }
                    </div>
                </div>
            }.into_any()
        }}
    }
}

/// Pick the renderer by extension + GLDF content_type and feed it the
/// embedded bytes. Each dedicated panel (LDT/IES, SPD, L3D, TM-33) takes
/// `Vec<u8>` so we clone the slice once at the boundary. Images go
/// through a base64 data-url, the rest fall back to a truncated UTF-8
/// text preview.
///
/// TM-33 is checked before the generic `.xml` fallback because TM-33 is
/// itself XML — without the explicit sniff it would render as plain
/// text.
fn render_file_content(file_name: &str, content_type: &str, content: &[u8]) -> impl IntoView {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".l3d") {
        view! { <L3dPanel bytes=content.to_vec() /> }.into_any()
    } else if crate::components::is_tm33(file_name, content_type, content) {
        view! { <Tm33Panel bytes=content.to_vec() /> }.into_any()
    } else if lower.ends_with(".ldt") || lower.ends_with(".ies") {
        view! { <LdtPanel bytes=content.to_vec() /> }.into_any()
    } else if lower.ends_with(".spd") {
        view! { <SpdPanel bytes=content.to_vec() /> }.into_any()
    } else if lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
    {
        let mime = if lower.ends_with(".png") { "png" } else { "jpeg" };
        let data_url = format!("data:image/{};base64,{}", mime, BASE64_STANDARD.encode(content));
        view! {
            <img src=data_url style="max-width: 100%; max-height: 500px; border-radius: 4px;" />
        }.into_any()
    } else {
        let truncated = String::from_utf8_lossy(&content[..content.len().min(20000)]).to_string();
        let truncation_note = if content.len() > 20000 {
            Some(format!("... ({} bytes truncated)", content.len() - 20000))
        } else {
            None
        };
        view! {
            <div class="text-preview" style="font-family: var(--font-mono); font-size: 12px; white-space: pre-wrap; max-height: 500px; overflow: auto; background: var(--bg-secondary); padding: 12px; border-radius: 4px;">
                { truncated }
                {truncation_note.map(|n| view! {
                    <div style="margin-top: 12px; color: var(--text-tertiary); font-style: italic;">{ n }</div>
                })}
            </div>
        }.into_any()
    }
}

/// Top-of-page banner listing `<File>` entries with photometry or spectrum
/// content types that no `<Photometry>` / `<Spectrum>` block references.
/// Hidden if both lists are empty.
#[component]
fn OrphanBanner(
    orphan_photometries: Vec<(String, String, String)>,
    orphan_spectrums: Vec<(String, String, String)>,
) -> impl IntoView {
    if orphan_photometries.is_empty() && orphan_spectrums.is_empty() {
        return ().into_any();
    }
    let total = orphan_photometries.len() + orphan_spectrums.len();
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    view! {
        <details
            class="orphan-files-banner"
            open=true
            style="margin-bottom: 12px; padding: 10px 12px; border: 1px dashed var(--accent-orange); border-radius: 6px; background: var(--bg-secondary);"
        >
            <summary style="cursor: pointer; font-size: 13px; color: var(--accent-orange);">
                { format!("⚠ {} file(s) declared without a Photometry/Spectrum block", total) }
            </summary>
            <p style="margin: 8px 0 6px 0; font-size: 12px; color: var(--text-secondary);">
                { "These embedded files carry a photometry or spectrum content type, but no " }
                <code>{ "<Photometry>" }</code>{ " or " }<code>{ "<Spectrum>" }</code>
                { " block in GeneralDefinitions references them. Legal GLDF — a " }<code>{ "<File>" }</code>
                { " can ship without a wrapper — but they won't appear in the Photometry page or in light-source links until they are wired up." }
            </p>
            {if orphan_photometries.is_empty() {
                ().into_any()
            } else {
                view! {
                    <div style="margin-top: 6px;">
                        <div style="font-size: 12px; font-weight: 500;">{ tr("gldf.file_viewer.orphan_photometry_header") }</div>
                        <ul style="margin: 4px 0 0 0; padding-left: 18px; font-size: 12px;">
                            {orphan_photometries.into_iter().map(|(id, content_type, file_name)| {
                                let ct_class = crate::components::pages::files_list::content_type_class(&content_type);
                                view! {
                                    <li><code>{ id }</code>{ " — " }<span class=format!("content-type-badge {}", ct_class)>{ content_type }</span>{ " — " }<span>{ file_name }</span></li>
                                }
                            }).collect::<Vec<_>>()}
                        </ul>
                    </div>
                }.into_any()
            }}
            {if orphan_spectrums.is_empty() {
                ().into_any()
            } else {
                view! {
                    <div style="margin-top: 6px;">
                        <div style="font-size: 12px; font-weight: 500;">{ tr("gldf.file_viewer.orphan_spectrum_header") }</div>
                        <ul style="margin: 4px 0 0 0; padding-left: 18px; font-size: 12px;">
                            {orphan_spectrums.into_iter().map(|(id, content_type, file_name)| {
                                let ct_class = crate::components::pages::files_list::content_type_class(&content_type);
                                view! {
                                    <li><code>{ id }</code>{ " — " }<span class=format!("content-type-badge {}", ct_class)>{ content_type }</span>{ " — " }<span>{ file_name }</span></li>
                                }
                            }).collect::<Vec<_>>()}
                        </ul>
                    </div>
                }.into_any()
            }}
        </details>
    }.into_any()
}
