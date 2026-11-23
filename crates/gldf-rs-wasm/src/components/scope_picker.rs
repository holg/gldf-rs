//! Scope picker — Product default vs per-Variant override.
//!
//! Used by the scope-aware editors (Electrical, Mechanical, Applications,
//! Marketing) so a single component handles the family-wide default plus
//! per-SKU overrides. Returns the chosen `Scope` via callback.

use crate::components::edit_util::event_value;
use crate::state::{use_gldf, Scope};
use leptos::prelude::*;

#[component]
pub fn ScopePicker(scope: RwSignal<Scope>) -> impl IntoView {
    let state = use_gldf();

    let on_select = move |e: crate::components::edit_util::EditEvent| {
        let Some(v) = event_value(&e) else { return };
        let new_scope = if v == "product" {
            Scope::Product
        } else if let Ok(idx) = v.parse::<usize>() {
            Scope::Variant(idx)
        } else {
            Scope::Product
        };
        scope.set(new_scope);
    };

    view! {
        {move || state.with(|s| {
            let variants: Vec<(usize, String)> = s
                .product
                .product_definitions
                .variants
                .as_ref()
                .map(|vs| {
                    vs.variant
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            let label = v
                                .product_number
                                .as_ref()
                                .and_then(|pn| pn.locale.iter().find(|l| !l.value.is_empty()))
                                .map(|l| l.value.clone())
                                .unwrap_or_else(|| v.id.clone());
                            (i, label)
                        })
                        .collect()
                })
                .unwrap_or_default();

            if variants.is_empty() {
                return ().into_any();
            }

            let current = scope.get();
            let current_value = match current {
                Scope::Product => "product".to_string(),
                Scope::Variant(idx) => idx.to_string(),
            };
            let hint = match current {
                Scope::Product => "Family-wide default. Variants without an override inherit this.",
                Scope::Variant(_) => "Per-variant override. Empty fields fall back to the product default.",
            };
            let product_selected = matches!(current, Scope::Product);

            view! {
                <div class="scope-picker">
                    <label class="scope-picker-label">{ "Editing scope:" }</label>
                    <select class="scope-picker-select" data-testid="scope-picker-select" on:change=on_select prop:value=current_value>
                        <option value="product" selected=product_selected>
                            { "Product default" }
                        </option>
                        {variants.into_iter().map(|(idx, label)| {
                            let selected = matches!(current, Scope::Variant(i) if i == idx);
                            view! {
                                <option value=idx.to_string() selected=selected>
                                    { format!("Variant: {}", label) }
                                </option>
                            }
                        }).collect::<Vec<_>>()}
                    </select>
                    <small class="scope-picker-hint">{ hint }</small>
                </div>
            }.into_any()
        })}
    }
}
