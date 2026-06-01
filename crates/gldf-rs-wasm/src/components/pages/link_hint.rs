//! Small reusable "where do I link this?" callout used on the Photometry
//! and Spectra pages. The GLDF schema only allows photometry↔spectrum↔
//! light-source linkage on the Light Sources / Emitters nodes, not on the
//! `<Photometry>` or `<Spectrum>` blocks themselves — newbies hit this
//! every time, so we point them at the right page in-place.

use leptos::prelude::*;

/// Collapsible info banner. `title` is the short summary line shown when
/// the `<details>` is closed; `body` is the explanation revealed on open.
#[component]
pub fn LinkHint(title: String, body: String) -> impl IntoView {
    view! {
        <details
            class="link-hint"
            style="margin: 0 0 12px 0; padding: 8px 10px; border: 1px dashed var(--accent-blue); border-radius: 6px; background: var(--bg-secondary); font-size: 12px;"
        >
            <summary style="cursor: pointer; color: var(--accent-blue);">
                { "ℹ " } { title }
            </summary>
            <p style="margin: 6px 0 0 0; color: var(--text-secondary);">
                { body }
            </p>
        </details>
    }
}
