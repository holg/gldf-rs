//! Applications editor — XSD-enum multi-select, scope-aware.
//!
//! GLDF's `Marketing/Applications/Application` is a **closed enumeration**
//! of 79 hierarchical strings (see `crate::i18n::applications`). Free
//! text would produce schema-violating files, so this editor renders a
//! checkbox per canonical value with the active UI language's label.
//!
//! The Applications block exists at both ProductMetaData and per-Variant
//! scope. The scope picker at the top selects which one is edited; the
//! "Current applications" section and the checkbox state both reflect the
//! selected scope. When editing a variant that has no explicit Applications
//! block, the inherited product list is shown for reference but is not
//! committed unless the user toggles a checkbox.

use crate::components::ScopePicker;
use crate::state::{use_gldf, GldfAction, Scope};
use std::collections::HashSet;
use yew::prelude::*;

#[function_component(ApplicationsEditor)]
pub fn applications_editor() -> Html {
    let gldf = use_gldf();
    let filter = use_state(String::new);
    let scope = use_state(|| Scope::Product);

    // Read the user's chosen UI language directly from localStorage so the
    // labels match the toolbar picker without threading App state through
    // the GldfProvider context. Falls back to "en" silently.
    let ui_lang = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item("gldf.ui_language").ok().flatten())
        .unwrap_or_else(|| "en".to_string());

    let product_apps: Vec<String> = gldf
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.descriptive_attributes.as_ref())
        .and_then(|d| d.marketing.as_ref())
        .and_then(|m| m.applications.as_ref())
        .map(|a| a.application.clone())
        .unwrap_or_default();

    // Variant scope: if the variant has an Applications block, that is the
    // explicit list (may be empty — meaning "no applications"). If the
    // block is absent, the variant inherits the product list.
    let (applications, variant_has_explicit): (Vec<String>, bool) = match *scope {
        Scope::Product => (product_apps.clone(), true),
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
                .and_then(|m| m.applications.as_ref());
            match block {
                Some(b) => (b.application.clone(), true),
                None => (Vec::new(), false),
            }
        }
    };

    let selected: HashSet<String> = applications.iter().cloned().collect();

    let canonical_set: HashSet<&'static str> = crate::i18n::applications::CANONICAL_VALUES
        .iter()
        .copied()
        .collect();

    // Sort canonical values: depth-first (shallow before deep), then
    // alphabetical within each depth. Same ordering as the viewer.
    let mut sorted_canonical: Vec<&'static str> =
        crate::i18n::applications::CANONICAL_VALUES.to_vec();
    sorted_canonical.sort_by(|a, b| {
        let da = a.matches(':').count();
        let db = b.matches(':').count();
        da.cmp(&db).then_with(|| a.cmp(b))
    });

    let filter_value = (*filter).to_lowercase();
    let filter_lang = ui_lang.clone();
    let filter_matches = |canonical: &str| -> bool {
        if filter_value.is_empty() {
            return true;
        }
        if canonical.to_lowercase().contains(&filter_value) {
            return true;
        }
        let translated = crate::i18n::applications::display_label(canonical, &filter_lang);
        translated.to_lowercase().contains(&filter_value)
    };

    let on_filter_input = {
        let filter = filter.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            filter.set(input.value());
        })
    };

    let current_scope = *scope;

    let on_toggle = {
        let gldf = gldf.clone();
        let applications = applications.clone();
        Callback::from(move |canonical: String| {
            // Toggle: remove if present, otherwise append.
            if let Some(idx) = applications.iter().position(|a| a == &canonical) {
                gldf.dispatch(GldfAction::RemoveApplication(current_scope, idx));
            } else {
                gldf.dispatch(GldfAction::AddApplication(current_scope, canonical));
            }
        })
    };

    let inheritance_hint = matches!(current_scope, Scope::Variant(_)) && !variant_has_explicit;

    // Variant + no explicit override → show product list as read-only chips
    // so the user sees what's inherited.
    let display_list: Vec<String> =
        if matches!(current_scope, Scope::Variant(_)) && !variant_has_explicit {
            product_apps.clone()
        } else {
            applications.clone()
        };
    let removable = !(matches!(current_scope, Scope::Variant(_)) && !variant_has_explicit);

    html! {
        <div class="editor-section applications-editor">
            <h2>{ "Applications" }</h2>
            <p class="section-description">
                { "Application areas the luminaire is suitable for. Pick from the XSD-defined list — values outside that list are not schema-valid." }
            </p>
            <ScopePicker scope={current_scope} onchange={
                let scope = scope.clone();
                Callback::from(move |s: Scope| scope.set(s))
            } />

            if inheritance_hint && !product_apps.is_empty() {
                <div class="inheritance-banner">
                    { format!("This variant has no Applications block of its own — it inherits {} from the product default. Toggling a checkbox below creates an explicit override.", product_apps.len()) }
                </div>
            }

            // Current selections — chips with × to remove. Non-canonical
            // entries flagged so the user can see legacy data sins.
            <div class="applications-list">
                <h3>{
                    match current_scope {
                        Scope::Product => "Current applications".to_string(),
                        Scope::Variant(_) if variant_has_explicit => "Variant applications (explicit override)".to_string(),
                        Scope::Variant(_) => "Inherited from product (no override)".to_string(),
                    }
                }</h3>
                if display_list.is_empty() {
                    <p class="empty-message">{ "No applications selected." }</p>
                } else {
                    <div class="tags-container">
                        { for display_list.iter().enumerate().map(|(index, app)| {
                            let gldf = gldf.clone();
                            let on_remove = {
                                let scope = current_scope;
                                Callback::from(move |_: MouseEvent| {
                                    gldf.dispatch(GldfAction::RemoveApplication(scope, index));
                                })
                            };
                            let is_canonical = canonical_set.contains(app.as_str());
                            let display = if is_canonical {
                                crate::i18n::applications::display_label(app, &ui_lang)
                            } else {
                                app.clone()
                            };
                            let class = if is_canonical {
                                classes!("tag", "application-tag")
                            } else {
                                classes!("tag", "application-tag", "non-canonical")
                            };
                            html! {
                                <span class={class} title={app.clone()}>
                                    if !is_canonical {
                                        <span class="non-canonical-flag" title="Not in the XSD enumeration; will fail schema validation">
                                            { "⚠ " }
                                        </span>
                                    }
                                    { display }
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
                        })}
                    </div>
                }
            </div>

            // Picker — XSD canonical values as checkboxes, filtered.
            <div class="form-group">
                <label for="application-filter">{ "Add applications" }</label>
                <input
                    id="application-filter"
                    type="search"
                    placeholder="Filter…"
                    value={(*filter).clone()}
                    oninput={on_filter_input}
                    style="margin-bottom: 8px;"
                />
                <div class="application-checkbox-list" style="max-height: 360px; overflow-y: auto; border: 1px solid var(--border-color); border-radius: 6px; padding: 8px;">
                    { for sorted_canonical.iter().filter(|c| filter_matches(c)).map(|canonical| {
                        let canonical_str = (*canonical).to_string();
                        let label = crate::i18n::applications::display_label(canonical, &ui_lang);
                        let is_selected = selected.contains(&canonical_str);
                        let depth = canonical.matches(':').count();
                        let on_toggle = on_toggle.clone();
                        let target = canonical_str.clone();
                        let on_change = Callback::from(move |_: Event| on_toggle.emit(target.clone()));

                        html! {
                            <label
                                class={classes!("application-checkbox-row", format!("depth-{}", depth.min(3)))}
                                style={format!("padding-left: {}px; display: flex; align-items: baseline; gap: 8px; padding-top: 2px; padding-bottom: 2px;", depth * 16)}
                            >
                                <input type="checkbox" checked={is_selected} onchange={on_change} />
                                <span>{ label }</span>
                            </label>
                        }
                    }) }
                </div>
            </div>
        </div>
    }
}
