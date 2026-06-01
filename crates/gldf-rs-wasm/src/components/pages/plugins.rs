//! Plugins page — embedded WASM plugins discovered in the GLDF.
//!
//! Slice 1 lists the discovered plugins (manifest name / version / description).
//! The interactive PluginViewer (iframe + WASM bridge) lands in slice 3.
//!
//! Plugin discovery walks the embedded files looking for a `plugin.toml`
//! manifest. The lib's `PluginManager::discover_plugins` expects a
//! `&[BufFile]`; we don't carry those on the Leptos side any more, so we
//! synthesize the minimum the discoverer needs from `embedded_files` + the
//! file definitions.

use crate::state::use_gldf;
use gldf_rs::BufFile;
use leptos::prelude::*;

#[component]
pub fn PluginsPage() -> impl IntoView {
    let state = use_gldf();

    view! {
        {move || state.with(|s| {
            // Reconstruct BufFile entries from the (file_id, bytes) pairs +
            // the matching XML <File> definitions. PluginManager only needs
            // each entry's `name` to recognise `plugin.toml` and walk the
            // bundle, plus `content` to read it.
            let bufs: Vec<BufFile> = s.product.general_definitions.files.file.iter()
                .filter_map(|fdef| {
                    let content = s.embedded_files.get(&fdef.id).cloned()?;
                    Some(BufFile {
                        name: Some(fdef.file_name.clone()),
                        content: Some(content),
                        file_id: Some(fdef.id.clone()),
                        path: Some(fdef.file_name.clone()),
                    })
                })
                .collect();
            let plugins = gldf_rs::plugin::PluginManager::discover_plugins(&bufs);
            if plugins.is_empty() {
                view! {
                    <div class="empty-state">
                        <div class="icon">{ "🔌" }</div>
                        <h3>{ "No Plugins Found" }</h3>
                        <p>{ "This GLDF file does not contain any embedded viewer plugins." }</p>
                    </div>
                }.into_any()
            } else {
                let count = plugins.len();
                view! {
                    <div class="plugins-container">
                        <div class="plugins-info">
                            <p>{ format!("Found {} plugin(s) in this GLDF file:", count) }</p>
                            <ul class="plugin-list">
                                {plugins.into_iter().map(|p| view! {
                                    <li class="plugin-item">
                                        <strong>{ p.manifest.name.clone() }</strong>
                                        <span class="plugin-version">{ format!(" v{}", p.manifest.version) }</span>
                                        <p class="plugin-description">{ p.manifest.description.clone() }</p>
                                    </li>
                                }).collect::<Vec<_>>()}
                            </ul>
                        </div>
                        <p style="color: #666; font-style: italic; margin-top: 16px;">
                            { "Interactive plugin viewer (iframe + WASM bridge) lands in slice 3." }
                        </p>
                    </div>
                }.into_any()
            }
        })}
    }
}
