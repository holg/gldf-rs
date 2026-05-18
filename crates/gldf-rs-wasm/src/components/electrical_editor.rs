//! Electrical attributes editor — scope-aware (Product default vs per-Variant override).
//!
//! Electrical lives at TWO levels in the XSD: `ProductMetaData/.../Electrical`
//! is the family-wide default, and each `Variant/.../Electrical` overrides it
//! per SKU. The editor exposes both via a scope picker at the top: pick
//! "Product default" to edit the family value, pick a variant to edit only
//! that SKU's override.
//!
//! When editing a variant, fields the variant doesn't override show the
//! inherited product value as a placeholder — typing into the field creates
//! the override; clearing the field removes it.

use crate::state::{use_gldf, GldfAction, Scope};
use gldf_rs::gldf::product_definitions::Electrical;
use yew::prelude::*;

#[function_component(ElectricalEditor)]
pub fn electrical_editor() -> Html {
    let gldf = use_gldf();
    let scope = use_state(|| Scope::Product);

    // Resolve the *effective* and *explicit* electrical for the current
    // scope. Explicit drives the form's `value=`; effective (after
    // inheritance) drives placeholders.
    let product_electrical: Option<Electrical> = gldf
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.descriptive_attributes.as_ref())
        .and_then(|d| d.electrical.clone());

    let variant_electrical: Option<Electrical> = match *scope {
        Scope::Variant(idx) => gldf
            .product
            .product_definitions
            .variants
            .as_ref()
            .and_then(|vs| vs.variant.get(idx))
            .and_then(|v| v.descriptive_attributes.as_ref())
            .and_then(|d| d.electrical.clone()),
        Scope::Product => None,
    };

    let explicit: Option<&Electrical> = match *scope {
        Scope::Product => product_electrical.as_ref(),
        Scope::Variant(_) => variant_electrical.as_ref(),
    };
    let inherited: Option<&Electrical> = match *scope {
        Scope::Product => None,
        Scope::Variant(_) => product_electrical.as_ref(),
    };

    let safety_class = explicit
        .and_then(|e| e.electrical_safety_class.clone())
        .unwrap_or_default();
    let ip_code = explicit
        .and_then(|e| e.ingress_protection_ip_code.clone())
        .unwrap_or_default();
    let power_factor = explicit.and_then(|e| e.power_factor);
    let constant_light = explicit.and_then(|e| e.constant_light_output);
    let light_distribution = explicit
        .and_then(|e| e.light_distribution.clone())
        .unwrap_or_default();
    let switching_capacity = explicit
        .and_then(|e| e.switching_capacity.clone())
        .unwrap_or_default();

    // Inherited values (for placeholders when editing a variant).
    let inherited_safety = inherited
        .and_then(|e| e.electrical_safety_class.clone())
        .unwrap_or_default();
    let inherited_ip = inherited
        .and_then(|e| e.ingress_protection_ip_code.clone())
        .unwrap_or_default();
    let inherited_pf = inherited
        .and_then(|e| e.power_factor)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let inherited_dist = inherited
        .and_then(|e| e.light_distribution.clone())
        .unwrap_or_default();
    let inherited_sc = inherited
        .and_then(|e| e.switching_capacity.clone())
        .unwrap_or_default();

    let current_scope = *scope;

    let on_safety_class_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let value = input.value();
            gldf.dispatch(GldfAction::SetElectricalSafetyClass(
                current_scope,
                if value.is_empty() { None } else { Some(value) },
            ));
        })
    };

    let on_ip_code_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            // The IP-code input is now a <select>, so the target is an
            // HtmlSelectElement. Both element types expose `.value()` but
            // the cast must match the actual DOM node.
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let value = select.value();
            gldf.dispatch(GldfAction::SetIngressProtectionIPCode(
                current_scope,
                if value.is_empty() { None } else { Some(value) },
            ));
        })
    };

    let on_power_factor_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let value = input.value();
            gldf.dispatch(GldfAction::SetPowerFactor(
                current_scope,
                value.parse::<f64>().ok().filter(|&v| v > 0.0),
            ));
        })
    };

    let on_constant_light_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            gldf.dispatch(GldfAction::SetConstantLightOutput(
                current_scope,
                Some(input.checked()),
            ));
        })
    };

    let on_light_distribution_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let value = input.value();
            gldf.dispatch(GldfAction::SetLightDistribution(
                current_scope,
                if value.is_empty() { None } else { Some(value) },
            ));
        })
    };

    let on_switching_capacity_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let value = input.value();
            gldf.dispatch(GldfAction::SetSwitchingCapacity(
                current_scope,
                if value.is_empty() { None } else { Some(value) },
            ));
        })
    };

    // ip_placeholder removed: IP code is now a <select>; the inherited
    // value renders inline as the empty-state option text instead.
    let pf_placeholder = if inherited_pf.is_empty() {
        "e.g., 0.95".to_string()
    } else {
        format!("inherits: {}", inherited_pf)
    };
    let sc_placeholder = if inherited_sc.is_empty() {
        "e.g., 50000 cycles".to_string()
    } else {
        format!("inherits: {}", inherited_sc)
    };

    html! {
        <div class="editor-section electrical-editor">
            <h2>{ "Electrical Attributes" }</h2>
            <ScopePicker scope={current_scope} onchange={
                let scope = scope.clone();
                Callback::from(move |s: Scope| scope.set(s))
            } />

            <div class="form-group">
                <label for="safety-class">{ "Electrical Safety Class" }</label>
                <select
                    id="safety-class"
                    onchange={on_safety_class_change}
                >
                    <option value="" selected={safety_class.is_empty()}>
                        { if inherited_safety.is_empty() {
                            "-- Select --".to_string()
                        } else {
                            format!("inherits: Class {}", inherited_safety)
                        } }
                    </option>
                    // All five XSD values: 0 (no protection), I (earth-only),
                    // 0I (basic insulation + earth), II (double insulation),
                    // III (SELV).
                    { for crate::i18n::xsd_enums::ELECTRICAL_SAFETY_CLASSES.iter().map(|v| {
                        let selected = safety_class == *v;
                        let label = match *v {
                            "0" => "Class 0 (no earth, no double insulation)",
                            "I" => "Class I (protective earth)",
                            "0I" => "Class 0I (basic insulation + earth)",
                            "II" => "Class II (double insulation)",
                            "III" => "Class III (SELV)",
                            other => other,
                        };
                        html! { <option value={(*v).to_string()} selected={selected}>{ label }</option> }
                    }) }
                    if !safety_class.is_empty()
                        && !crate::i18n::xsd_enums::is_canonical(
                            crate::i18n::xsd_enums::ELECTRICAL_SAFETY_CLASSES,
                            &safety_class,
                        )
                    {
                        // Legacy/non-canonical value already in the file.
                        // Show it as a flagged option so the user can keep
                        // it (the editor stays open for a fix) without the
                        // dropdown silently swapping to "-- Select --".
                        <option value={safety_class.clone()} selected={true}>
                            { format!("⚠ {} (not in XSD enum)", safety_class) }
                        </option>
                    }
                </select>
                <small class="form-help">{ "IEC 61140 electrical safety classification" }</small>
            </div>

            <div class="form-group">
                <label for="ip-code">{ "IP Code (Ingress Protection)" }</label>
                <select id="ip-code" onchange={on_ip_code_change}>
                    <option value="" selected={ip_code.is_empty()}>
                        { if inherited_ip.is_empty() {
                            "-- Select --".to_string()
                        } else {
                            format!("inherits: {}", inherited_ip)
                        } }
                    </option>
                    { for crate::i18n::xsd_enums::IP_CODES.iter().map(|v| {
                        let selected = ip_code == *v;
                        html! { <option value={(*v).to_string()} selected={selected}>{ *v }</option> }
                    }) }
                    if !ip_code.is_empty()
                        && !crate::i18n::xsd_enums::is_canonical(
                            crate::i18n::xsd_enums::IP_CODES,
                            &ip_code,
                        )
                    {
                        <option value={ip_code.clone()} selected={true}>
                            { format!("⚠ {} (not in XSD enum)", ip_code) }
                        </option>
                    }
                </select>
                <small class="form-help">{ "IEC 60529 ingress protection (IP20–IP69K)" }</small>
            </div>

            <div class="form-group">
                <label for="power-factor">{ "Power Factor" }</label>
                <input
                    type="number"
                    id="power-factor"
                    value={power_factor.map(|v| v.to_string()).unwrap_or_default()}
                    onchange={on_power_factor_change}
                    placeholder={pf_placeholder}
                    step="0.01"
                    min="0"
                    max="1"
                />
                <small class="form-help">{ "Ratio of real power to apparent power (0-1)" }</small>
            </div>

            <div class="form-group checkbox-group">
                <label>
                    <input
                        type="checkbox"
                        checked={constant_light.unwrap_or(false)}
                        onchange={on_constant_light_change}
                    />
                    { " Constant Light Output (CLO)" }
                </label>
                <small class="form-help">{ "Luminaire maintains constant lumen output over lifetime" }</small>
            </div>

            <div class="form-group">
                <label for="light-distribution">{ "Light Distribution" }</label>
                <select
                    id="light-distribution"
                    onchange={on_light_distribution_change}
                >
                    // The 15 values come straight from the XSD enum
                    // (gldf-1.0.0-rc-3.xsd:4670–4692). Option text is
                    // localized via the i18n table; `value=` always carries
                    // the canonical XSD string so the saved GLDF stays
                    // schema-valid regardless of the UI language.
                    <option value="" selected={light_distribution.is_empty()}>
                        { if inherited_dist.is_empty() {
                            "— Select —".to_string()
                        } else {
                            format!("inherits: {}", crate::i18n::light_distribution::display_label(&inherited_dist, "en"))
                        } }
                    </option>
                    { for crate::i18n::light_distribution::canonical_values().iter().map(|v| {
                        let selected = light_distribution == *v;
                        let label = crate::i18n::light_distribution::display_label(v, "en");
                        html! {
                            <option value={(*v).to_string()} selected={selected}>{ label }</option>
                        }
                    }) }
                </select>
                <small class="hint">
                    { "Values are XSD-defined; the saved GLDF carries the canonical English string." }
                </small>
            </div>

            <div class="form-group">
                <label for="switching-capacity">{ "Switching Capacity" }</label>
                <input
                    type="text"
                    id="switching-capacity"
                    value={switching_capacity}
                    onchange={on_switching_capacity_change}
                    placeholder={sc_placeholder}
                />
            </div>
        </div>
    }
}

/// Scope picker shared by Electrical / Mechanical / Applications editors.
///
/// Renders a small dropdown listing "Product default" plus every variant by
/// chip label. Selection drives the `Scope` prop the editor uses for both
/// form values and dispatched actions. Hidden when the document has no
/// variants (the only scope is Product anyway).
#[derive(Properties, PartialEq)]
pub struct ScopePickerProps {
    pub scope: Scope,
    pub onchange: Callback<Scope>,
}

#[function_component(ScopePicker)]
pub fn scope_picker(props: &ScopePickerProps) -> Html {
    let gldf = use_gldf();
    let variants: Vec<(usize, String)> = gldf
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
        return html! {};
    }

    let onchange = props.onchange.clone();
    let on_select = Callback::from(move |e: Event| {
        let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
        let v = select.value();
        let new_scope = if v == "product" {
            Scope::Product
        } else if let Ok(idx) = v.parse::<usize>() {
            Scope::Variant(idx)
        } else {
            Scope::Product
        };
        onchange.emit(new_scope);
    });

    let current_value = match props.scope {
        Scope::Product => "product".to_string(),
        Scope::Variant(idx) => idx.to_string(),
    };

    html! {
        <div class="scope-picker">
            <label class="scope-picker-label">{ "Editing scope:" }</label>
            <select class="scope-picker-select" onchange={on_select} value={current_value.clone()}>
                <option value="product" selected={matches!(props.scope, Scope::Product)}>
                    { "Product default" }
                </option>
                { for variants.iter().map(|(idx, label)| {
                    let selected = matches!(props.scope, Scope::Variant(i) if i == *idx);
                    html! {
                        <option value={idx.to_string()} selected={selected}>
                            { format!("Variant: {}", label) }
                        </option>
                    }
                }) }
            </select>
            { match props.scope {
                Scope::Product => html! {
                    <small class="scope-picker-hint">
                        { "Family-wide default. Variants without an override inherit this." }
                    </small>
                },
                Scope::Variant(_) => html! {
                    <small class="scope-picker-hint">
                        { "Per-variant override. Empty fields fall back to the product default." }
                    </small>
                },
            }}
        </div>
    }
}
