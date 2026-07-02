//! Hyperlinks editor — one row per Hyperlink, full UpdateHyperlink dispatches.
//! Scope-aware: edits either the product-wide `Marketing.Hyperlinks` or a
//! specific variant's override via the shared `ScopePicker`.

use crate::components::edit_util::{event_value, optional_string};
use crate::components::ScopePicker;
use crate::state::{use_active_scope, use_gldf, GldfAction, Scope};
use gldf_rs::gldf::Hyperlink;
use leptos::prelude::*;

#[component]
pub fn HyperlinksEditorPage() -> impl IntoView {
    let state = use_gldf();
    let scope = use_active_scope();

    let on_add = move |_| {
        let scp = scope.get();
        state.update(|s| s.apply(GldfAction::AddHyperlink(scp)));
    };

    view! {
        <div class="editor-section hyperlinks-editor">
            <h2>{ "Hyperlinks" }</h2>
            <p class="section-description">
                { "Per-locale URLs (datasheets, product pages, IES files…). The href is required; language/region/country are optional. Variants can override the product-wide list." }
            </p>

            <ScopePicker scope=scope />

            {move || state.with(|s| {
                let cur_scope = scope.get();
                let entries: Vec<Hyperlink> = match cur_scope {
                    Scope::Product => s
                        .product
                        .product_definitions
                        .product_meta_data
                        .as_ref()
                        .and_then(|m| m.descriptive_attributes.as_ref())
                        .and_then(|a| a.marketing.as_ref())
                        .and_then(|m| m.hyperlinks.as_ref())
                        .map(|h| h.hyperlink.clone())
                        .unwrap_or_default(),
                    Scope::Variant(idx) => s
                        .product
                        .product_definitions
                        .variants
                        .as_ref()
                        .and_then(|vs| vs.variant.get(idx))
                        .and_then(|v| v.descriptive_attributes.as_ref())
                        .and_then(|a| a.marketing.as_ref())
                        .and_then(|m| m.hyperlinks.as_ref())
                        .map(|h| h.hyperlink.clone())
                        .unwrap_or_default(),
                };
                if entries.is_empty() {
                    return view! { <p class="empty-message">{ "No hyperlinks." }</p> }.into_any();
                }
                view! {
                    <div class="hyperlinks-list" style="display: flex; flex-direction: column; gap: 12px;">
                        {entries.into_iter().enumerate().map(|(idx, link)| view! {
                            <HyperlinkRow idx=idx link=link scope=scope />
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            })}

            <div class="add-row" style="margin-top: 12px;">
                <button type="button" on:click=on_add>{ "+ Add hyperlink" }</button>
            </div>
        </div>
    }
}

#[component]
fn HyperlinkRow(idx: usize, link: Hyperlink, scope: RwSignal<Scope>) -> impl IntoView {
    let state = use_gldf();

    let snap = link.clone();
    let on_href = {
        let snap = snap.clone();
        move |e: web_sys::Event| {
            let Some(v) = event_value(&e) else { return };
            let scp = scope.get();
            state.update(|s| s.apply(GldfAction::UpdateHyperlink {
                scope: scp,
                index: idx,
                href: v,
                language: snap.language.clone(),
                region: snap.region.clone(),
                country_code: snap.country_code.clone(),
                value: snap.value.clone(),
            }));
        }
    };
    let on_lang = {
        let snap = snap.clone();
        move |e: web_sys::Event| {
            let v = event_value(&e).unwrap_or_default();
            let scp = scope.get();
            state.update(|s| s.apply(GldfAction::UpdateHyperlink {
                scope: scp,
                index: idx,
                href: snap.href.clone(),
                language: optional_string(v),
                region: snap.region.clone(),
                country_code: snap.country_code.clone(),
                value: snap.value.clone(),
            }));
        }
    };
    let on_region = {
        let snap = snap.clone();
        move |e: web_sys::Event| {
            let v = event_value(&e).unwrap_or_default();
            let scp = scope.get();
            state.update(|s| s.apply(GldfAction::UpdateHyperlink {
                scope: scp,
                index: idx,
                href: snap.href.clone(),
                language: snap.language.clone(),
                region: optional_string(v),
                country_code: snap.country_code.clone(),
                value: snap.value.clone(),
            }));
        }
    };
    let on_country = {
        let snap = snap.clone();
        move |e: web_sys::Event| {
            let v = event_value(&e).unwrap_or_default();
            let scp = scope.get();
            state.update(|s| s.apply(GldfAction::UpdateHyperlink {
                scope: scp,
                index: idx,
                href: snap.href.clone(),
                language: snap.language.clone(),
                region: snap.region.clone(),
                country_code: optional_string(v),
                value: snap.value.clone(),
            }));
        }
    };
    let on_label = {
        let snap = snap.clone();
        move |e: web_sys::Event| {
            let Some(v) = event_value(&e) else { return };
            let scp = scope.get();
            state.update(|s| s.apply(GldfAction::UpdateHyperlink {
                scope: scp,
                index: idx,
                href: snap.href.clone(),
                language: snap.language.clone(),
                region: snap.region.clone(),
                country_code: snap.country_code.clone(),
                value: v,
            }));
        }
    };
    let on_remove = move |_| {
        let scp = scope.get();
        state.update(|s| s.apply(GldfAction::RemoveHyperlink { scope: scp, index: idx }));
    };

    let href = link.href.clone();
    let value = link.value.clone();
    let lang = link.language.clone().unwrap_or_default();
    let region = link.region.clone().unwrap_or_default();
    let country = link.country_code.clone().unwrap_or_default();

    view! {
        <div class="hyperlink-row" style="border: 1px solid var(--border-color); border-radius: 6px; padding: 8px;">
            <div class="hyperlink-row-main" style="display: flex; gap: 6px; align-items: center; margin-bottom: 6px;">
                <input type="url" class="hyperlink-href" placeholder="https://…" prop:value=href on:change=on_href style="flex: 1;" />
                <input type="text" class="hyperlink-label" placeholder="Label" prop:value=value on:change=on_label style="flex: 1;" />
                <button type="button" class="hyperlink-remove" title="Remove" on:click=on_remove>{ "✕" }</button>
            </div>
            <div class="hyperlink-row-meta" style="display: flex; gap: 8px; font-size: 11px;">
                <label>{ "lang:" }
                    <input type="text" class="hyperlink-lang" maxlength="5" placeholder="en" prop:value=lang on:change=on_lang style="width: 60px;" />
                </label>
                <label>{ "region:" }
                    <input type="text" class="hyperlink-region" maxlength="5" placeholder="EU" prop:value=region on:change=on_region style="width: 60px;" />
                </label>
                <label>{ "country:" }
                    <input type="text" class="hyperlink-country" maxlength="3" placeholder="DE" prop:value=country on:change=on_country style="width: 60px;" />
                </label>
            </div>
        </div>
    }
}
