//! Approval Marks page — single sidebar home for **two** XSD slots
//! that vendors use interchangeably:
//!
//! * `Marketing/Labels/Label`           — closed XSD enum (CE, GS, ENEC, RoHS, WEEE, …)
//! * `Marketing/ApprovalMarks/ApprovalMark` — open free-text list
//!
//! Both slots round-trip through GLDF independently, but in practice
//! you'll see CE / ENEC in both, and the spec doesn't make the
//! distinction sharp. We surface them together so users don't have to
//! understand the historical reason GLDF carries both. Each list still
//! writes to its own XSD slot on save (no auto-mirror), so existing
//! vendor files keep their shape.
//!
//! Scope-aware: per-product default + per-variant override.

use crate::components::edit_util::event_value;
use crate::components::ScopePicker;
use crate::i18n::ui::t as ui_t;
use crate::state::{use_active_scope, use_gldf, GldfAction, Scope};
use gldf_rs::validation::xsd_enums::{is_canonical, LABELS};
use leptos::prelude::*;
use std::collections::HashSet;

#[component]
pub fn LabelsEditorPage() -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    view! {
        <div class="editor-section labels-editor">
            <h2>{ move || tr("gldf.labels.title") }</h2>
            <p class="section-description">
                { move || tr("gldf.labels.section_description") }
            </p>
            <ScopePicker scope=use_active_scope() />
            <LabelsPanel />
            <ApprovalMarksPanel />
        </div>
    }
}

/// Closed-enum Labels list — XSD `Marketing/Labels/Label`. Renders a
/// checkbox grid of canonical values plus a remove-able tag list of
/// what's currently set.
#[component]
fn LabelsPanel() -> impl IntoView {
    let state = use_gldf();
    let scope = use_active_scope();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));

    view! {
        <div style="margin-top: 16px;">
            <h3 style="margin-bottom: 4px;">{ move || tr("gldf.labels.closed_enum_title") }</h3>
            <p style="font-size: 11px; color: var(--text-secondary); margin: 0 0 8px;">
                { move || tr("gldf.labels.closed_enum_description") }
            </p>
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

                view! {
                    {if inheritance_hint && !product_labels.is_empty() {
                        let hint = tr("gldf.labels.inheritance_hint")
                            .replace("{count}", &product_labels.len().to_string());
                        view! {
                            <div style="padding: 8px; background: var(--bg-secondary); border-radius: 4px; margin-bottom: 8px; font-size: 11px;">
                                { hint }
                            </div>
                        }.into_any()
                    } else { ().into_any() }}

                    <div class="tags-container" style="display: flex; gap: 6px; flex-wrap: wrap; min-height: 22px;">
                        {if display_list.is_empty() {
                            view! { <span style="font-size: 11px; color: var(--text-tertiary); font-style: italic;">{ tr("gldf.labels.no_labels") }</span> }.into_any()
                        } else {
                            display_list.into_iter().enumerate().map(|(idx, label)| {
                                let label_text = label.clone();
                                let canonical = is_canonical(LABELS, &label);
                                let class = if canonical { "tag" } else { "tag non-canonical" };
                                let on_remove = move |_| {
                                    state.update(|s| s.apply(GldfAction::RemoveLabel(current, idx)));
                                };
                                let non_can_warn = tr("gldf.labels.non_canonical_warning");
                                let remove_title = tr("gldf.common.remove");
                                view! {
                                    <span class=class style="display: inline-block; padding: 4px 8px; background: var(--bg-secondary); border-radius: 4px; font-size: 12px;">
                                        {if !canonical {
                                            view! { <span class="non-canonical-flag" title=non_can_warn>{ "⚠ " }</span> }.into_any()
                                        } else { ().into_any() }}
                                        { label_text }
                                        {if removable {
                                            view! {
                                                <button type="button" class="tag-remove" on:click=on_remove title=remove_title style="background: none; border: none; cursor: pointer; margin-left: 4px;">{ "×" }</button>
                                            }.into_any()
                                        } else { ().into_any() }}
                                    </span>
                                }
                            }).collect::<Vec<_>>().into_any()
                        }}
                    </div>

                    <div class="form-group" style="margin-top: 8px;">
                        <details>
                            <summary style="cursor: pointer; font-size: 12px; color: var(--text-secondary);">{ tr("gldf.labels.add_canonical") }</summary>
                            <div style="border: 1px solid var(--border-color); border-radius: 6px; padding: 8px; margin-top: 4px; max-height: 200px; overflow: auto;">
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
                                        <label class="application-checkbox-row">
                                            <input type="checkbox" prop:checked=is_selected on:change=on_change />
                                            <span>{ *canonical }</span>
                                        </label>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </details>
                    </div>
                }
            })}
        </div>
    }
}

/// Free-text ApprovalMarks list — XSD `Marketing/ApprovalMarks/ApprovalMark`.
/// Open string list for marks that aren't in the closed Labels enum
/// (UL, RoHS-China, country-specific certs).
#[component]
fn ApprovalMarksPanel() -> impl IntoView {
    let state = use_gldf();
    let scope = use_active_scope();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    let on_add = move |_| {
        let scp = scope.get();
        state.update(|s| s.apply(GldfAction::AddApprovalMark(scp)));
    };

    view! {
        <div style="margin-top: 24px;">
            <h3 style="margin-bottom: 4px;">{ move || tr("gldf.approval_marks.title") }</h3>
            <p style="font-size: 11px; color: var(--text-secondary); margin: 0 0 8px;">
                { move || tr("gldf.approval_marks.description") }
            </p>
            {move || state.with(|s| {
                let entries = crate::components::pages::marketing_editor::read_approval_marks(s, scope.get());
                if entries.is_empty() {
                    view! { <p class="empty-message" style="font-size: 11px; font-style: italic; color: var(--text-tertiary); margin: 0;">{ tr("gldf.approval_marks.empty") }</p> }.into_any()
                } else {
                    view! {
                        <div style="display: flex; flex-direction: column; gap: 6px;">
                            {entries.into_iter().enumerate().map(|(idx, value)| view! {
                                <ApprovalMarkRow idx=idx value=value scope=scope />
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            })}
            <button type="button" on:click=on_add data-testid="approval-marks-add" style="margin-top: 8px;">{ move || tr("gldf.approval_marks.add") }</button>
        </div>
    }
}

#[component]
fn ApprovalMarkRow(idx: usize, value: String, scope: RwSignal<Scope>) -> impl IntoView {
    let state = use_gldf();
    let tr = move |key: &str| ui_t(key, &state.with(|s| s.ui_language.clone()));
    let on_change = move |e: web_sys::Event| {
        let v = event_value(&e).unwrap_or_default();
        let scp = scope.get();
        state.update(|s| s.apply(GldfAction::UpdateApprovalMark { scope: scp, index: idx, value: v }));
    };
    let on_remove = move |_| {
        let scp = scope.get();
        state.update(|s| s.apply(GldfAction::RemoveApprovalMark { scope: scp, index: idx }));
    };
    view! {
        <div style="display: flex; gap: 6px; align-items: center;">
            <input type="text" placeholder=tr("gldf.approval_marks.placeholder") prop:value=value on:change=on_change style="flex: 1; padding: 4px 6px;" />
            <button type="button" on:click=on_remove title=tr("gldf.approval_marks.remove_title") style="background: none; border: 1px solid var(--accent-orange); border-radius: 4px; padding: 2px 8px; cursor: pointer; color: var(--accent-orange);">{ "✕" }</button>
        </div>
    }
}
