//! Labels editor — XSD-enum multi-select (CE, GS, ENEC, …), scope-aware.

use crate::components::ScopePicker;
use crate::state::{use_gldf, GldfAction, Scope};
use gldf_rs::validation::xsd_enums::{is_canonical, LABELS};
use leptos::prelude::*;
use std::collections::HashSet;

#[component]
pub fn LabelsEditorPage() -> impl IntoView {
    let state = use_gldf();
    let scope = RwSignal::new(Scope::Product);

    view! {
        <div class="editor-section labels-editor">
            <h2>{ "Labels" }</h2>
            <p class="section-description">
                { "Quality and regulatory marks (CE, GS, ENEC, …). Closed XSD enumeration — values outside the list are not schema-valid." }
            </p>
            <ScopePicker scope=scope />

            {move || state.with(|s| {
                let current = scope.get();
                let product_labels: Vec<String> = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|d| d.marketing.as_ref())
                    .and_then(|m| m.labels.as_ref())
                    .map(|l| l.label.clone())
                    .unwrap_or_default();
                let (labels, variant_has_explicit): (Vec<String>, bool) = match current {
                    Scope::Product => (product_labels.clone(), true),
                    Scope::Variant(idx) => {
                        let block = s
                            .product
                            .product_definitions
                            .variants
                            .as_ref()
                            .and_then(|vs| vs.variant.get(idx))
                            .and_then(|v| v.descriptive_attributes.as_ref())
                            .and_then(|d| d.marketing.as_ref())
                            .and_then(|m| m.labels.as_ref());
                        match block {
                            Some(b) => (b.label.clone(), true),
                            None => (Vec::new(), false),
                        }
                    }
                };
                let selected: HashSet<String> = labels.iter().cloned().collect();
                let inheritance_hint = matches!(current, Scope::Variant(_)) && !variant_has_explicit;
                let display_list = if inheritance_hint { product_labels.clone() } else { labels.clone() };
                let removable = !inheritance_hint;
                let scope_label = match current {
                    Scope::Product => "Current labels".to_string(),
                    Scope::Variant(_) if variant_has_explicit => "Variant labels (explicit override)".to_string(),
                    Scope::Variant(_) => "Inherited from product (no override)".to_string(),
                };

                view! {
                    {if inheritance_hint && !product_labels.is_empty() {
                        view! {
                            <div class="inheritance-banner" style="padding: 8px; background: var(--bg-secondary); border-radius: 4px; margin-bottom: 8px;">
                                { format!("This variant has no Labels block of its own — it inherits {} from the product default. Toggling a checkbox below creates an explicit override.", product_labels.len()) }
                            </div>
                        }.into_any()
                    } else { ().into_any() }}

                    <div class="applications-list">
                        <h3>{ scope_label }</h3>
                        {if display_list.is_empty() {
                            view! { <p class="empty-message">{ "No labels selected." }</p> }.into_any()
                        } else {
                            view! {
                                <div class="tags-container" style="display: flex; gap: 6px; flex-wrap: wrap;">
                                    {display_list.into_iter().enumerate().map(|(idx, label)| {
                                        let label_text = label.clone();
                                        let canonical = is_canonical(LABELS, &label);
                                        let class = if canonical { "tag application-tag" } else { "tag application-tag non-canonical" };
                                        let on_remove = move |_| {
                                            state.update(|s| s.apply(GldfAction::RemoveLabel(current, idx)));
                                        };
                                        view! {
                                            <span class=class title=label.clone() style="display: inline-block; padding: 4px 8px; background: var(--bg-secondary); border-radius: 4px; font-size: 12px;">
                                                {if !canonical {
                                                    view! { <span class="non-canonical-flag" title="Not in the XSD enumeration; will fail schema validation">{ "⚠ " }</span> }.into_any()
                                                } else { ().into_any() }}
                                                { label_text }
                                                {if removable {
                                                    view! {
                                                        <button type="button" class="tag-remove" on:click=on_remove title="Remove" style="background: none; border: none; cursor: pointer; margin-left: 4px;">{ "×" }</button>
                                                    }.into_any()
                                                } else { ().into_any() }}
                                            </span>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }}
                    </div>

                    <div class="form-group">
                        <label>{ "Add labels" }</label>
                        <div class="application-checkbox-list" style="border: 1px solid var(--border-color); border-radius: 6px; padding: 8px;">
                            {LABELS.iter().map(|canonical| {
                                let canonical_str = canonical.to_string();
                                let is_selected = selected.contains(&canonical_str);
                                let target = canonical_str.clone();
                                let on_change = move |_e: web_sys::Event| {
                                    state.update(|s| {
                                        let scope_now = scope.get();
                                        let cur: Vec<String> = match scope_now {
                                            Scope::Product => s.product.product_definitions.product_meta_data.as_ref().and_then(|m| m.descriptive_attributes.as_ref()).and_then(|d| d.marketing.as_ref()).and_then(|m| m.labels.as_ref()).map(|l| l.label.clone()).unwrap_or_default(),
                                            Scope::Variant(idx) => s.product.product_definitions.variants.as_ref().and_then(|vs| vs.variant.get(idx)).and_then(|v| v.descriptive_attributes.as_ref()).and_then(|d| d.marketing.as_ref()).and_then(|m| m.labels.as_ref()).map(|l| l.label.clone()).unwrap_or_default(),
                                        };
                                        if let Some(idx) = cur.iter().position(|l| l == &target) {
                                            s.apply(GldfAction::RemoveLabel(scope_now, idx));
                                        } else {
                                            s.apply(GldfAction::AddLabel(scope_now, target.clone()));
                                        }
                                    });
                                };
                                view! {
                                    <label class="application-checkbox-row" style="display: flex; align-items: baseline; gap: 8px; padding: 2px 0;">
                                        <input type="checkbox" prop:checked=is_selected on:change=on_change />
                                        <span>{ *canonical }</span>
                                    </label>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                }
            })}
        </div>
    }
}
