//! Display-only view of a `LocaleFoo` — the GLDF localized-string complex type.
//!
//! Renders the entry matching the active UI language if present, otherwise the
//! first entry, otherwise an em-dash. The editor variant (slice 2) will add
//! per-locale add/remove + language selector; this component is read-only.

use crate::state::use_gldf;
use gldf_rs::gldf::LocaleFoo;
use leptos::prelude::*;

#[component]
pub fn LocaleFooView(
    /// The locale-foo struct to render. `None` shows the em-dash.
    value: LocaleFoo,
    /// Compact rendering — single-line, no language hint.
    #[prop(default = false)]
    compact: bool,
) -> impl IntoView {
    let state = use_gldf();
    let active = move || state.with(|s| s.active_language.clone());
    let label = move || {
        let lang = active();
        let chosen = value
            .locale
            .iter()
            .find(|l| l.language == lang)
            .or_else(|| value.locale.first());
        match chosen {
            Some(l) if !l.value.is_empty() => l.value.clone(),
            _ => "—".to_string(),
        }
    };
    if compact {
        view! { <span>{ label }</span> }.into_any()
    } else {
        view! { <span class="locale-view">{ label }</span> }.into_any()
    }
}
