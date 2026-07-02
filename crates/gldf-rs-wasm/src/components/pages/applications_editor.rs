//! Applications editor — closed XSD enumeration multi-select, scope-aware.
//!
//! Mirrors the Yew side: tag chips for current selections + checkbox list
//! sorted depth-first (the XSD applications are hierarchical, joined by ':').
//! Filter input narrows the checkbox list by canonical-or-translated label.

use crate::components::edit_util::event_value;
use crate::components::ScopePicker;
use crate::i18n;
use crate::i18n::ui::t as ui_t;
use crate::state::{use_active_scope, use_gldf, GldfAction, Scope};
use leptos::prelude::*;
use std::collections::HashSet;

#[component]
pub fn ApplicationsEditorPage() -> impl IntoView {
    let state = use_gldf();
    let scope = use_active_scope();
    let filter = RwSignal::new(String::new());

    let on_filter = move |e: web_sys::Event| {
        if let Some(v) = event_value(&e) {
            filter.set(v);
        }
    };

    let title_signal = move || ui_t("gldf.applications.title", &state.with(|s| s.ui_language.clone()));
    let desc_signal = move || ui_t("gldf.applications.description", &state.with(|s| s.ui_language.clone()));

    view! {
        <div class="editor-section applications-editor">
            <h2>{ title_signal }</h2>
            <p class="section-description">
                { desc_signal }
            </p>
            <ScopePicker scope=scope />

            {move || state.with(|s| {
                let current = scope.get();
                let ui_lang = s.ui_language.clone();
                let tr = |key: &str| ui_t(key, &ui_lang);
                let product_apps: Vec<String> = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|d| d.marketing.as_ref())
                    .and_then(|m| m.applications.as_ref())
                    .map(|a| a.application.clone())
                    .unwrap_or_default();

                let (applications, variant_has_explicit): (Vec<String>, bool) = match current {
                    Scope::Product => (product_apps.clone(), true),
                    Scope::Variant(idx) => {
                        let block = s
                            .product
                            .product_definitions
                            .variants
                            .as_ref()
                            .and_then(|vs| vs.variant.get(idx))
                            .and_then(|v| v.descriptive_attributes.as_ref())
                            .and_then(|d| d.marketing.as_ref())
                            .and_then(|m| m.applications.as_ref());
                        match block {
                            Some(b) => (b.application.clone(), true),
                            None => (Vec::new(), false),
                        }
                    }
                };

                let selected: HashSet<String> = applications.iter().cloned().collect();
                let canonical_set: HashSet<&'static str> = i18n::applications::CANONICAL_VALUES.iter().copied().collect();
                let mut sorted: Vec<&'static str> = i18n::applications::CANONICAL_VALUES.to_vec();
                sorted.sort_by(|a, b| {
                    let da = a.matches(':').count();
                    let db = b.matches(':').count();
                    da.cmp(&db).then_with(|| a.cmp(b))
                });
                let filter_value = filter.get().to_lowercase();
                let inheritance_hint = matches!(current, Scope::Variant(_)) && !variant_has_explicit;
                let display_list = if inheritance_hint { product_apps.clone() } else { applications.clone() };
                let removable = !inheritance_hint;

                let scope_label = match current {
                    Scope::Product => tr("gldf.applications.current"),
                    Scope::Variant(_) if variant_has_explicit => tr("gldf.applications.variant_explicit"),
                    Scope::Variant(_) => tr("gldf.applications.inherited"),
                };

                view! {
                    {if inheritance_hint && !product_apps.is_empty() {
                        view! {
                            <div class="inheritance-banner" style="padding: 8px; background: var(--bg-secondary); border-radius: 4px; margin-bottom: 8px;">
                                { tr("gldf.applications.inheritance_banner").replace("{n}", &product_apps.len().to_string()) }
                            </div>
                        }.into_any()
                    } else { ().into_any() }}

                    <div class="applications-list">
                        <h3>{ scope_label }</h3>
                        {if display_list.is_empty() {
                            view! { <p class="empty-message">{ tr("gldf.applications.empty_selection") }</p> }.into_any()
                        } else {
                            view! {
                                <div class="tags-container" style="display: flex; gap: 6px; flex-wrap: wrap;">
                                    {display_list.into_iter().enumerate().map(|(idx, app)| {
                                        let app_for_label = app.clone();
                                        let app_for_title = app.clone();
                                        let is_canonical = canonical_set.contains(app.as_str());
                                        let display = if is_canonical {
                                            i18n::applications::display_label(&app_for_label, &ui_lang)
                                        } else { app_for_label };
                                        let class = if is_canonical { "tag application-tag" } else { "tag application-tag non-canonical" };
                                        let on_remove = move |_| {
                                            state.update(|s| s.apply(GldfAction::RemoveApplication(current, idx)));
                                        };
                                        view! {
                                            <span class=class title=app_for_title style="display: inline-block; padding: 4px 8px; background: var(--bg-secondary); border-radius: 4px; font-size: 12px;">
                                                {if !is_canonical {
                                                    view! { <span class="non-canonical-flag" title="Not in the XSD enumeration; will fail schema validation">{ "⚠ " }</span> }.into_any()
                                                } else { ().into_any() }}
                                                { display }
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
                        <label for="application-filter">{ tr("gldf.applications.add") }</label>
                        <input
                            id="application-filter"
                            type="search"
                            placeholder="Filter…"
                            prop:value=filter.get()
                            on:input=on_filter
                            style="margin-bottom: 8px; width: 100%; max-width: 400px;"
                        />
                        <div class="application-checkbox-list" style="max-height: 360px; overflow-y: auto; border: 1px solid var(--border-color); border-radius: 6px; padding: 8px;">
                            {sorted.into_iter().filter(|c| {
                                let cl = c.to_lowercase();
                                let tl = i18n::applications::display_label(c, &ui_lang).to_lowercase();
                                filter_value.is_empty() || cl.contains(&filter_value) || tl.contains(&filter_value)
                            }).map(|canonical| {
                                let canonical_str = canonical.to_string();
                                let label = i18n::applications::display_label(canonical, &ui_lang);
                                let is_selected = selected.contains(&canonical_str);
                                let depth = canonical.matches(':').count();
                                let target = canonical_str.clone();
                                let row_style = format!(
                                    "padding-left: {}px; display: flex; align-items: baseline; gap: 8px; padding-top: 2px; padding-bottom: 2px;",
                                    depth * 16
                                );
                                let on_change = move |_e: web_sys::Event| {
                                    state.update(|s| {
                                        // Re-check current selection at click time —
                                        // applications may have changed since render.
                                        let scope_now = scope.get();
                                        let cur: Vec<String> = match scope_now {
                                            Scope::Product => s.product.product_definitions.product_meta_data.as_ref().and_then(|m| m.descriptive_attributes.as_ref()).and_then(|d| d.marketing.as_ref()).and_then(|m| m.applications.as_ref()).map(|a| a.application.clone()).unwrap_or_default(),
                                            Scope::Variant(idx) => s.product.product_definitions.variants.as_ref().and_then(|vs| vs.variant.get(idx)).and_then(|v| v.descriptive_attributes.as_ref()).and_then(|d| d.marketing.as_ref()).and_then(|m| m.applications.as_ref()).map(|a| a.application.clone()).unwrap_or_default(),
                                        };
                                        if let Some(idx) = cur.iter().position(|a| a == &target) {
                                            s.apply(GldfAction::RemoveApplication(scope_now, idx));
                                        } else {
                                            s.apply(GldfAction::AddApplication(scope_now, target.clone()));
                                        }
                                    });
                                };
                                view! {
                                    <label class=format!("application-checkbox-row depth-{}", depth.min(3)) style=row_style>
                                        <input type="checkbox" prop:checked=is_selected on:change=on_change />
                                        <span>{ label }</span>
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
