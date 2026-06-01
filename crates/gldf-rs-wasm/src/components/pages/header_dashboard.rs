//! Header dashboard — read view of the Header nav item.
//!
//! Mirrors the `view_header_editor` Viewer-mode branch from the Yew side: a
//! two-column key/value list (Identity + Stats) plus a Download GLDF action.

use crate::state::{collect_languages, use_gldf};
use leptos::prelude::*;

#[component]
pub fn HeaderDashboardPage() -> impl IntoView {
    let state = use_gldf();

    // The Yew side wired the download button to Msg::ExportGldf. In Leptos the
    // toolbar's Export GLDF button is the same code path; we mirror it here by
    // dispatching the same save-to-buf-and-download flow. That flow now lives
    // in `crate::exports` (see `main.rs`'s `export_gldf` helper); we reach it
    // via a context value the toolbar registers, but to keep this page
    // standalone we duplicate the small helper.
    let on_download_gldf = move |_| {
        if let Err(e) = crate::exports::export_gldf(state) {
            log::error!("Export GLDF: {}", e);
        }
    };

    view! {
        {move || state.with(|s| {
            let header = &s.product.header;
            let inner_file_count = s.product.general_definitions.files.file.len();
            let langs = collect_languages(&s.product);
            let lang_summary = if langs.is_empty() {
                "(none)".to_string()
            } else if langs.len() == 1 {
                format!("1 language: {}", langs[0])
            } else {
                format!("{} languages: {}", langs.len(), langs.join(", "))
            };
            let schema_url = if s.product.xsnonamespaceschemalocation.is_empty() {
                None
            } else {
                Some(s.product.xsnonamespaceschemalocation.clone())
            };
            let or_dash = |v: &str| if v.is_empty() { "—".to_string() } else { v.to_string() };

            view! {
                <div class="document-dashboard">
                    <div class="dashboard-card">
                        <div class="dashboard-card-header">
                            <h3>{ "Document Identity" }</h3>
                        </div>
                        <dl class="dashboard-list">
                            <dt>{ "Manufacturer" }</dt>
                            <dd>{ or_dash(&header.manufacturer) }</dd>
                            <dt>{ "Author" }</dt>
                            <dd>{ or_dash(&header.author) }</dd>
                            <dt>{ "Created With" }</dt>
                            <dd>{ or_dash(&header.created_with_application) }</dd>
                            <dt>{ "Creation Time" }</dt>
                            <dd>{ or_dash(&header.creation_time_code) }</dd>
                            <dt>{ "Format Version" }</dt>
                            <dd class="code">{ format!("{:?}", header.format_version) }</dd>
                            <dt>{ "Default Language" }</dt>
                            <dd>{ header.default_language.clone().unwrap_or_else(|| "(unset)".to_string()) }</dd>
                        </dl>
                    </div>

                    <div class="dashboard-card">
                        <div class="dashboard-card-header">
                            <h3>{ "Document Stats" }</h3>
                        </div>
                        <dl class="dashboard-list">
                            <dt>{ "Inner Files" }</dt>
                            <dd>{ format!("{} (declared in GeneralDefinitions/Files)", inner_file_count) }</dd>
                            <dt>{ "Embedded Bytes" }</dt>
                            <dd>{ format!("{} file(s) loaded", s.embedded_files.len()) }</dd>
                            <dt>{ "Locale Coverage" }</dt>
                            <dd>{ lang_summary }</dd>
                            {schema_url.map(|url| {
                                let url_for_href = url.clone();
                                view! {
                                    <dt>{ "Schema" }</dt>
                                    <dd>
                                        <a href=url_for_href target="_blank" rel="noopener noreferrer" class="schema-link">
                                            { url }
                                        </a>
                                    </dd>
                                }
                            })}
                        </dl>
                    </div>

                    <div class="dashboard-actions">
                        <button class="btn btn-primary" on:click=on_download_gldf>
                            { "⬇ Download GLDF" }
                        </button>
                    </div>
                </div>
            }
        })}
    }
}
