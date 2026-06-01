//! Mechanical attributes editor — scope-aware (Product default vs per-Variant).
//!
//! Covers IKRating (XSD enum), ProductForm (XSD enum), ProductSize
//! (Length/Width/Height in mm), Weight (kg), SealingMaterial (LocaleFoo).
//! When editing a Variant scope, inherited Product values appear as
//! placeholder text so the user sees what they'd fall back to.

use crate::components::edit_util::{event_f64, event_i32, event_value, optional_string};
use crate::components::{LocaleFooEditor, ScopePicker};
use crate::state::{use_gldf, GldfAction, Scope};
use gldf_rs::gldf::product_definitions::Mechanical;
use gldf_rs::validation::xsd_enums;
use leptos::prelude::*;

/// XSD `ProductForm` enumeration.
const PRODUCT_FORMS: &[&str] = &[
    "Round", "Rounded", "Square", "Linear", "Areal", "Sphere", "Cuboid", "Cylinder", "Cone",
    "Special",
];

#[component]
pub fn MechanicalEditorPage() -> impl IntoView {
    let state = use_gldf();
    let scope = RwSignal::new(Scope::Product);

    // Action dispatchers, all bound to the current scope.
    let on_ik = move |e: web_sys::Event| {
        let v = event_value(&e).unwrap_or_default();
        state.update(|s| {
            s.apply(GldfAction::SetMechanicalIKRating(scope.get(), optional_string(v)));
        });
    };
    let on_form = move |e: web_sys::Event| {
        let v = event_value(&e).unwrap_or_default();
        state.update(|s| {
            s.apply(GldfAction::SetMechanicalProductForm(scope.get(), optional_string(v)));
        });
    };
    let on_length = move |e: web_sys::Event| {
        let v = event_i32(&e);
        state.update(|s| s.apply(GldfAction::SetMechanicalLength(scope.get(), v)));
    };
    let on_width = move |e: web_sys::Event| {
        let v = event_i32(&e);
        state.update(|s| s.apply(GldfAction::SetMechanicalWidth(scope.get(), v)));
    };
    let on_height = move |e: web_sys::Event| {
        let v = event_i32(&e);
        state.update(|s| s.apply(GldfAction::SetMechanicalHeight(scope.get(), v)));
    };
    let on_weight = move |e: web_sys::Event| {
        let v = event_f64(&e);
        state.update(|s| s.apply(GldfAction::SetMechanicalWeight(scope.get(), v)));
    };
    let on_sealing = Callback::new(move |v: gldf_rs::gldf::LocaleFoo| {
        state.update(|s| s.apply(GldfAction::SetMechanicalSealingMaterial(scope.get(), v)));
    });

    view! {
        <div class="editor-section mechanical-editor">
            <h2>{ "Mechanical" }</h2>
            <ScopePicker scope=scope />

            {move || state.with(|s| {
                let current = scope.get();
                let product_mech: Option<Mechanical> = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|a| a.mechanical.clone());
                let variant_mech: Option<Mechanical> = match current {
                    Scope::Variant(idx) => s
                        .product
                        .product_definitions
                        .variants
                        .as_ref()
                        .and_then(|vs| vs.variant.get(idx))
                        .and_then(|v| v.descriptive_attributes.as_ref())
                        .and_then(|a| a.mechanical.clone()),
                    Scope::Product => None,
                };
                let (explicit, inherited): (Option<Mechanical>, Option<Mechanical>) = match current {
                    Scope::Product => (product_mech, None),
                    Scope::Variant(_) => (variant_mech, product_mech),
                };

                let ik_rating = explicit.as_ref().and_then(|m| m.ik_rating.clone()).unwrap_or_default();
                let product_form = explicit.as_ref().and_then(|m| m.product_form.clone()).unwrap_or_default();
                let weight = explicit.as_ref().and_then(|m| m.weight).map(|w| format!("{}", w)).unwrap_or_default();
                let size = explicit.as_ref().and_then(|m| m.product_size.clone());
                let length = size.as_ref().map(|s| s.length).unwrap_or(0);
                let width = size.as_ref().map(|s| s.width).unwrap_or(0);
                let height = size.as_ref().map(|s| s.height).unwrap_or(0);
                let sealing = explicit.as_ref().and_then(|m| m.sealing_material.clone()).unwrap_or_default();

                let inherited_ik = inherited.as_ref().and_then(|m| m.ik_rating.clone()).unwrap_or_default();
                let inherited_form = inherited.as_ref().and_then(|m| m.product_form.clone()).unwrap_or_default();
                let inherited_weight = inherited.as_ref().and_then(|m| m.weight).map(|w| format!("{}", w)).unwrap_or_default();

                let weight_placeholder = if inherited_weight.is_empty() {
                    String::new()
                } else {
                    format!("inherits: {}", inherited_weight)
                };

                let ik_legacy_pill = if !ik_rating.is_empty()
                    && !xsd_enums::is_canonical(xsd_enums::IK_RATINGS, &ik_rating)
                {
                    Some(view! {
                        <option value=ik_rating.clone() selected=true>
                            { format!("⚠ {} (not in XSD enum)", ik_rating) }
                        </option>
                    })
                } else { None };

                view! {
                    <div class="form-group">
                        <label for="ik-rating">{ "IK Rating" }</label>
                        <select id="ik-rating" on:change=on_ik>
                            <option value="" selected=ik_rating.is_empty()>
                                { if inherited_ik.is_empty() {
                                    "-- Select --".to_string()
                                } else {
                                    format!("inherits: {}", inherited_ik)
                                } }
                            </option>
                            {xsd_enums::IK_RATINGS.iter().map(|v| {
                                let selected = ik_rating == *v;
                                view! { <option value=(*v).to_string() selected=selected>{ *v }</option> }
                            }).collect::<Vec<_>>()}
                            { ik_legacy_pill }
                        </select>
                        <span class="hint">{ "Impact protection per IEC 62262 (IK00–IK10, IK10+)." }</span>
                    </div>

                    <div class="form-group">
                        <label for="product-form">{ "Product Form" }</label>
                        <select id="product-form" on:change=on_form prop:value=product_form.clone()>
                            <option value="" selected=product_form.is_empty()>
                                { if inherited_form.is_empty() {
                                    "(unset)".to_string()
                                } else {
                                    format!("inherits: {}", inherited_form)
                                } }
                            </option>
                            {PRODUCT_FORMS.iter().map(|f| {
                                let selected = *f == product_form.as_str();
                                view! { <option value=(*f).to_string() selected=selected>{ *f }</option> }
                            }).collect::<Vec<_>>()}
                        </select>
                    </div>

                    <div class="form-group">
                        <label>{ "Product Size (mm)" }</label>
                        <div class="size-grid" style="display: flex; gap: 8px;">
                            <label>{ "Length:" }
                                <input type="number" prop:value=length.to_string() on:change=on_length />
                            </label>
                            <label>{ "Width:" }
                                <input type="number" prop:value=width.to_string() on:change=on_width />
                            </label>
                            <label>{ "Height:" }
                                <input type="number" prop:value=height.to_string() on:change=on_height />
                            </label>
                        </div>
                    </div>

                    <div class="form-group">
                        <label for="weight">{ "Weight (kg)" }</label>
                        <input
                            type="number"
                            step="0.001"
                            id="weight"
                            prop:value=weight
                            on:change=on_weight
                            placeholder=weight_placeholder
                        />
                    </div>

                    <div class="form-group">
                        <LocaleFooEditor
                            label="Sealing Material"
                            value=sealing
                            on_change=on_sealing
                        />
                    </div>
                }
            })}
        </div>
    }
}
