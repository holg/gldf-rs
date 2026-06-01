//! Hyperlinks read view — clickable list grouped per locale.
//!
//! Hyperlinks live only at ProductMetaData level (no per-variant scope),
//! so no aggregation is needed. Each entry is rendered as a clickable
//! anchor with the locale metadata (lang/region/country) as a small caption.

use crate::state::use_gldf;
use leptos::prelude::*;

#[component]
pub fn HyperlinksReadPage() -> impl IntoView {
    let state = use_gldf();

    view! {
        <div class="editor-section hyperlinks-readonly">
            <h2>{ "Hyperlinks" }</h2>
            <p class="section-description" style="color: var(--text-tertiary);">
                { "Per-locale URLs (datasheets, product pages, IES files…)." }
            </p>

            {move || state.with(|s| {
                let entries = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|a| a.marketing.as_ref())
                    .and_then(|m| m.hyperlinks.as_ref())
                    .map(|h| h.hyperlink.clone())
                    .unwrap_or_default();
                if entries.is_empty() {
                    return view! { <p style="color: var(--text-tertiary);">{ "No hyperlinks." }</p> }.into_any();
                }
                view! {
                    <div class="hyperlinks-list" style="display: flex; flex-direction: column; gap: 8px;">
                        {entries.into_iter().map(|link| {
                            let href = link.href.clone();
                            let href_for_text = link.href.clone();
                            let label_text = if link.value.is_empty() {
                                href_for_text
                            } else {
                                link.value.clone()
                            };
                            let mut meta_parts: Vec<String> = Vec::new();
                            if let Some(lang) = link.language.clone() {
                                if !lang.is_empty() { meta_parts.push(format!("lang: {lang}")); }
                            }
                            if let Some(region) = link.region.clone() {
                                if !region.is_empty() { meta_parts.push(format!("region: {region}")); }
                            }
                            if let Some(cc) = link.country_code.clone() {
                                if !cc.is_empty() { meta_parts.push(format!("country: {cc}")); }
                            }
                            let meta_line = meta_parts.join(" · ");
                            let show_meta = !meta_line.is_empty();
                            view! {
                                <div class="hyperlink-row-view" style="padding: 8px; border: 1px solid var(--border-color); border-radius: 6px;">
                                    <a href=href target="_blank" rel="noopener noreferrer" style="color: var(--accent-blue); text-decoration: none;">
                                        { label_text }
                                    </a>
                                    {if show_meta {
                                        view! {
                                            <div style="font-size: 11px; color: var(--text-tertiary); margin-top: 2px;">
                                                { meta_line }
                                            </div>
                                        }.into_any()
                                    } else { ().into_any() }}
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            })}
        </div>
    }
}
