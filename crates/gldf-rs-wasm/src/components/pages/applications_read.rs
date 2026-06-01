//! Applications read view — list of application tags under Marketing.
//!
//! Applications is a `Vec<String>` rather than a single scalar, so the
//! aggregation reduces to a per-variant tag set. Slice 2A renders all
//! distinct tags as chips with the variants that carry each.

use crate::components::aggregate::variant_chip_label;
use crate::state::use_gldf;
use leptos::prelude::*;

#[component]
pub fn ApplicationsReadPage() -> impl IntoView {
    let state = use_gldf();

    view! {
        <div class="editor-section applications-readonly">
            <h2>{ "Applications" }</h2>
            <p class="section-description" style="color: var(--text-tertiary);">
                { "Marketing-level intended-use tags. Product defaults plus per-variant overrides." }
            </p>

            {move || state.with(|s| {
                let default_apps: Vec<String> = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|d| d.marketing.as_ref())
                    .and_then(|m| m.applications.as_ref())
                    .map(|a| a.application.clone())
                    .unwrap_or_default();
                let variants_owned = s
                    .product
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|vs| vs.variant.clone())
                    .unwrap_or_default();
                let label_of: std::collections::HashMap<String, String> = variants_owned
                    .iter()
                    .map(|v| (v.id.clone(), variant_chip_label(v)))
                    .collect();

                // Build: tag -> list of variant ids that declare it (plus
                // a synthetic "<default>" bucket for the ProductMetaData
                // applications so the user sees what every variant inherits).
                use std::collections::BTreeMap;
                let mut by_tag: BTreeMap<String, Vec<String>> = BTreeMap::new();
                if !default_apps.is_empty() {
                    for tag in &default_apps {
                        by_tag.entry(tag.clone()).or_default().push("<default>".to_string());
                    }
                }
                for variant in &variants_owned {
                    if let Some(apps) = variant
                        .descriptive_attributes
                        .as_ref()
                        .and_then(|a| a.marketing.as_ref())
                        .and_then(|m| m.applications.as_ref())
                    {
                        for tag in &apps.application {
                            by_tag.entry(tag.clone()).or_default().push(variant.id.clone());
                        }
                    }
                }

                if by_tag.is_empty() {
                    return view! {
                        <p style="color: var(--text-tertiary);">{ "No applications declared." }</p>
                    }.into_any();
                }

                view! {
                    <div class="applications-grid" style="display: flex; flex-direction: column; gap: 8px;">
                        {by_tag.into_iter().map(|(tag, sources)| {
                            let label_of = label_of.clone();
                            view! {
                                <div class="aggregate-row">
                                    <span class="aggregate-row-label">{ tag }</span>
                                    <span class="bucket-chips">
                                        {sources.into_iter().map(|src| {
                                            let display = if src == "<default>" {
                                                "Product default".to_string()
                                            } else {
                                                label_of.get(&src).cloned().unwrap_or_else(|| src.clone())
                                            };
                                            view! {
                                                <span
                                                    class="variant-chip"
                                                    style="display: inline-block; padding: 2px 8px; margin: 2px; background: var(--bg-secondary); border-radius: 4px; font-size: 11px;"
                                                >
                                                    { display }
                                                </span>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </span>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            })}
        </div>
    }
}
