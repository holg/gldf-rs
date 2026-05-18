//! Mechanical attributes editor — scope-aware (Product default vs per-Variant override).
//!
//! Mirrors `ElectricalEditor`: pick "Product default" or a variant from the
//! scope picker; the form below targets the selected scope. Inherited
//! product values appear as placeholder text when editing a variant.
//!
//! XSD fields covered: IKRating (free string per spec; common values IK00–IK10),
//! ProductForm (closed enumeration), ProductSize (Length / Width / Height in mm),
//! Weight (kg), SealingMaterial (LocaleFoo).

use crate::components::LocaleFooEditor;
use crate::components::ScopePicker;
use crate::state::{use_gldf, GldfAction, Scope};
use gldf_rs::gldf::product_definitions::Mechanical;
use yew::prelude::*;

/// XSD `ProductForm` enumeration (line 4461 of gldf-1.0.0-rc-3.xsd).
const PRODUCT_FORMS: &[&str] = &[
    "Round", "Rounded", "Square", "Linear", "Areal", "Sphere", "Cuboid", "Cylinder", "Cone",
    "Special",
];

#[function_component(MechanicalEditor)]
pub fn mechanical_editor() -> Html {
    let gldf = use_gldf();
    let scope = use_state(|| Scope::Product);

    let product_mech: Option<Mechanical> = gldf
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.descriptive_attributes.as_ref())
        .and_then(|a| a.mechanical.clone());

    let variant_mech: Option<Mechanical> = match *scope {
        Scope::Variant(idx) => gldf
            .product
            .product_definitions
            .variants
            .as_ref()
            .and_then(|vs| vs.variant.get(idx))
            .and_then(|v| v.descriptive_attributes.as_ref())
            .and_then(|a| a.mechanical.clone()),
        Scope::Product => None,
    };

    let explicit: Option<&Mechanical> = match *scope {
        Scope::Product => product_mech.as_ref(),
        Scope::Variant(_) => variant_mech.as_ref(),
    };
    let inherited: Option<&Mechanical> = match *scope {
        Scope::Product => None,
        Scope::Variant(_) => product_mech.as_ref(),
    };

    let ik_rating = explicit
        .and_then(|m| m.ik_rating.clone())
        .unwrap_or_default();
    let product_form = explicit
        .and_then(|m| m.product_form.clone())
        .unwrap_or_default();
    let weight = explicit
        .and_then(|m| m.weight)
        .map(|w| format!("{}", w))
        .unwrap_or_default();
    let size = explicit.and_then(|m| m.product_size.as_ref());
    let length = size.map(|s| s.length).unwrap_or(0);
    let width = size.map(|s| s.width).unwrap_or(0);
    let height = size.map(|s| s.height).unwrap_or(0);
    let sealing = explicit
        .and_then(|m| m.sealing_material.clone())
        .unwrap_or_default();

    let inherited_ik = inherited
        .and_then(|m| m.ik_rating.clone())
        .unwrap_or_default();
    let inherited_form = inherited
        .and_then(|m| m.product_form.clone())
        .unwrap_or_default();
    let inherited_weight = inherited
        .and_then(|m| m.weight)
        .map(|w| format!("{}", w))
        .unwrap_or_default();

    // ik_placeholder removed: IK rating is now a <select>; the inherited
    // value renders as the empty-state option text instead.
    let weight_placeholder = if inherited_weight.is_empty() {
        String::new()
    } else {
        format!("inherits: {}", inherited_weight)
    };

    let current_scope = *scope;

    let on_ik = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            // IK rating is now a <select>; cast accordingly.
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let v = select.value();
            gldf.dispatch(GldfAction::SetMechanicalIKRating(
                current_scope,
                if v.is_empty() { None } else { Some(v) },
            ));
        })
    };

    let on_form = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let v = select.value();
            gldf.dispatch(GldfAction::SetMechanicalProductForm(
                current_scope,
                if v.is_empty() { None } else { Some(v) },
            ));
        })
    };

    let on_weight = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let parsed: Option<f64> = input.value().trim().parse().ok();
            gldf.dispatch(GldfAction::SetMechanicalWeight(current_scope, parsed));
        })
    };

    let on_int = |action: fn(Scope, Option<i32>) -> GldfAction,
                  scope: Scope,
                  gldf: crate::state::GldfContext| {
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let parsed: Option<i32> = input.value().trim().parse().ok();
            gldf.dispatch(action(scope, parsed));
        })
    };

    let on_sealing = {
        let gldf = gldf.clone();
        Callback::from(move |v: gldf_rs::gldf::LocaleFoo| {
            gldf.dispatch(GldfAction::SetMechanicalSealingMaterial(current_scope, v))
        })
    };

    html! {
        <div class="editor-section mechanical-editor">
            <h2>{ "Mechanical" }</h2>
            <ScopePicker scope={current_scope} onchange={
                let scope = scope.clone();
                Callback::from(move |s: Scope| scope.set(s))
            } />

            <div class="form-group">
                <label for="ik-rating">{ "IK Rating" }</label>
                <select id="ik-rating" onchange={on_ik}>
                    <option value="" selected={ik_rating.is_empty()}>
                        { if inherited_ik.is_empty() {
                            "-- Select --".to_string()
                        } else {
                            format!("inherits: {}", inherited_ik)
                        } }
                    </option>
                    { for crate::i18n::xsd_enums::IK_RATINGS.iter().map(|v| {
                        let selected = ik_rating == *v;
                        html! { <option value={(*v).to_string()} selected={selected}>{ *v }</option> }
                    }) }
                    if !ik_rating.is_empty()
                        && !crate::i18n::xsd_enums::is_canonical(
                            crate::i18n::xsd_enums::IK_RATINGS,
                            &ik_rating,
                        )
                    {
                        // Same legacy-value pattern as the Electrical
                        // editor: surface the existing non-canonical value
                        // with ⚠ so the dropdown doesn't silently drop it.
                        <option value={ik_rating.clone()} selected={true}>
                            { format!("⚠ {} (not in XSD enum)", ik_rating) }
                        </option>
                    }
                </select>
                <span class="hint">{ "Impact protection per IEC 62262 (IK00–IK10, IK10+)." }</span>
            </div>

            <div class="form-group">
                <label for="product-form">{ "Product Form" }</label>
                <select id="product-form" onchange={on_form} value={product_form.clone()}>
                    <option value="" selected={product_form.is_empty()}>
                        { if inherited_form.is_empty() {
                            "(unset)".to_string()
                        } else {
                            format!("inherits: {}", inherited_form)
                        } }
                    </option>
                    { for PRODUCT_FORMS.iter().map(|f| {
                        let selected = *f == product_form.as_str();
                        html! { <option value={(*f).to_string()} selected={selected}>{ *f }</option> }
                    }) }
                </select>
            </div>

            <div class="form-group">
                <label>{ "Product Size (mm)" }</label>
                <div class="size-grid">
                    <label>
                        { "Length:" }
                        <input
                            type="number"
                            value={length.to_string()}
                            onchange={on_int(GldfAction::SetMechanicalLength, current_scope, gldf.clone())}
                        />
                    </label>
                    <label>
                        { "Width:" }
                        <input
                            type="number"
                            value={width.to_string()}
                            onchange={on_int(GldfAction::SetMechanicalWidth, current_scope, gldf.clone())}
                        />
                    </label>
                    <label>
                        { "Height:" }
                        <input
                            type="number"
                            value={height.to_string()}
                            onchange={on_int(GldfAction::SetMechanicalHeight, current_scope, gldf.clone())}
                        />
                    </label>
                </div>
            </div>

            <div class="form-group">
                <label for="weight">{ "Weight (kg)" }</label>
                <input
                    type="number"
                    step="0.001"
                    id="weight"
                    value={weight}
                    onchange={on_weight}
                    placeholder={weight_placeholder}
                />
            </div>

            <div class="form-group">
                <LocaleFooEditor
                    label="Sealing Material"
                    value={sealing}
                    onchange={on_sealing}
                />
            </div>
        </div>
    }
}
