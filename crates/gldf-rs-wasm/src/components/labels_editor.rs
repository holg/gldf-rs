//! Labels editor — XSD-enum multi-select for `Marketing/Labels/Label`.
//!
//! GLDF's `Label` is a closed XSD enumeration of 14 quality / regulatory
//! marks (`CE`, `GS`, `ENEC`, `CCC`, `VDE`, ...). This editor mirrors the
//! Applications editor pattern: a checkbox per canonical value, with a
//! `⚠` chip for any pre-existing non-canonical value (legacy data) so the
//! user can see what would fail XSD validation and decide whether to fix.

use crate::components::ScopePicker;
use crate::i18n::xsd_enums::{is_canonical, LABELS};
use crate::state::{use_gldf, GldfAction, Scope};
use std::collections::HashSet;
use yew::prelude::*;

#[function_component(LabelsEditor)]
pub fn labels_editor() -> Html {
    let gldf = use_gldf();
    let scope = use_state(|| Scope::Product);
    let current_scope = *scope;

    let product_labels: Vec<String> = gldf
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.descriptive_attributes.as_ref())
        .and_then(|d| d.marketing.as_ref())
        .and_then(|m| m.labels.as_ref())
        .map(|l| l.label.clone())
        .unwrap_or_default();

    let (labels, variant_has_explicit): (Vec<String>, bool) = match current_scope {
        Scope::Product => (product_labels.clone(), true),
        Scope::Variant(idx) => {
            let v = gldf
                .product
                .product_definitions
                .variants
                .as_ref()
                .and_then(|vs| vs.variant.get(idx));
            let block = v
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

    let on_toggle = {
        let gldf = gldf.clone();
        let labels = labels.clone();
        Callback::from(move |canonical: String| {
            if let Some(idx) = labels.iter().position(|l| l == &canonical) {
                gldf.dispatch(GldfAction::RemoveLabel(current_scope, idx));
            } else {
                gldf.dispatch(GldfAction::AddLabel(current_scope, canonical));
            }
        })
    };

    let inheritance_hint = matches!(current_scope, Scope::Variant(_))
        && !variant_has_explicit
        && !product_labels.is_empty();
    let display_list: Vec<String> =
        if matches!(current_scope, Scope::Variant(_)) && !variant_has_explicit {
            product_labels.clone()
        } else {
            labels.clone()
        };
    let removable = !(matches!(current_scope, Scope::Variant(_)) && !variant_has_explicit);

    html! {
        <div class="editor-section labels-editor">
            <h2>{ "Labels" }</h2>
            <p class="section-description">
                { "Quality and regulatory marks (CE, GS, ENEC, …). Closed XSD enumeration — values outside the list are not schema-valid." }
            </p>
            <ScopePicker scope={current_scope} onchange={
                let scope = scope.clone();
                Callback::from(move |s: Scope| scope.set(s))
            } />

            if inheritance_hint {
                <div class="inheritance-banner">
                    { format!("This variant has no Labels block of its own — it inherits {} from the product default. Toggling a checkbox below creates an explicit override.", product_labels.len()) }
                </div>
            }

            <div class="applications-list">
                <h3>{
                    match current_scope {
                        Scope::Product => "Current labels".to_string(),
                        Scope::Variant(_) if variant_has_explicit => "Variant labels (explicit override)".to_string(),
                        Scope::Variant(_) => "Inherited from product (no override)".to_string(),
                    }
                }</h3>
                if display_list.is_empty() {
                    <p class="empty-message">{ "No labels selected." }</p>
                } else {
                    <div class="tags-container">
                        { for display_list.iter().enumerate().map(|(index, label)| {
                            let gldf = gldf.clone();
                            let on_remove = {
                                let scope = current_scope;
                                Callback::from(move |_: MouseEvent| {
                                    gldf.dispatch(GldfAction::RemoveLabel(scope, index));
                                })
                            };
                            let canonical = is_canonical(LABELS, label);
                            let class = if canonical {
                                classes!("tag", "application-tag")
                            } else {
                                classes!("tag", "application-tag", "non-canonical")
                            };
                            html! {
                                <span class={class} title={label.clone()}>
                                    if !canonical {
                                        <span class="non-canonical-flag" title="Not in the XSD enumeration; will fail schema validation">
                                            { "⚠ " }
                                        </span>
                                    }
                                    { label }
                                    if removable {
                                        <button
                                            type="button"
                                            class="tag-remove"
                                            onclick={on_remove}
                                            title="Remove"
                                        >
                                            { "×" }
                                        </button>
                                    }
                                </span>
                            }
                        }) }
                    </div>
                }
            </div>

            <div class="form-group">
                <label>{ "Add labels" }</label>
                <div class="application-checkbox-list" style="border: 1px solid var(--border-color); border-radius: 6px; padding: 8px;">
                    { for LABELS.iter().map(|canonical| {
                        let canonical_str = (*canonical).to_string();
                        let is_selected = selected.contains(&canonical_str);
                        let on_toggle = on_toggle.clone();
                        let target = canonical_str.clone();
                        let on_change = Callback::from(move |_: Event| on_toggle.emit(target.clone()));
                        html! {
                            <label class="application-checkbox-row" style="display: flex; align-items: baseline; gap: 8px; padding: 2px 0;">
                                <input type="checkbox" checked={is_selected} onchange={on_change} />
                                <span>{ *canonical }</span>
                            </label>
                        }
                    }) }
                </div>
            </div>
        </div>
    }
}
