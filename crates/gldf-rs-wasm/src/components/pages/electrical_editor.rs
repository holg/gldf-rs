//! Electrical attributes editor — scope-aware (Product default vs per-Variant).
//!
//! Fields: ElectricalSafetyClass (XSD enum), IP code (XSD enum), Power Factor,
//! Constant Light Output (CLO), Light Distribution (XSD enum), Switching
//! Capacity. Same scope semantics as Mechanical: editing a variant shows the
//! inherited product value as placeholder; typing creates the override.

use crate::components::edit_util::{event_checked, event_f64, event_value, optional_string};
use crate::components::ScopePicker;
use crate::i18n::ui::t as ui_t;
use crate::state::{use_active_scope, use_gldf, GldfAction, Scope};
use gldf_rs::gldf::product_definitions::Electrical;
use gldf_rs::validation::xsd_enums;
use leptos::prelude::*;

#[component]
pub fn ElectricalEditorPage() -> impl IntoView {
    let state = use_gldf();
    let scope = use_active_scope();

    let on_safety = move |e: web_sys::Event| {
        let v = event_value(&e).unwrap_or_default();
        state.update(|s| {
            s.apply(GldfAction::SetElectricalSafetyClass(scope.get(), optional_string(v)));
        });
    };
    let on_ip = move |e: web_sys::Event| {
        let v = event_value(&e).unwrap_or_default();
        state.update(|s| {
            s.apply(GldfAction::SetIngressProtectionIPCode(scope.get(), optional_string(v)));
        });
    };
    let on_pf = move |e: web_sys::Event| {
        let v = event_f64(&e).filter(|&x| x > 0.0);
        state.update(|s| s.apply(GldfAction::SetPowerFactor(scope.get(), v)));
    };
    let on_clo = move |e: web_sys::Event| {
        let v = event_checked(&e).unwrap_or(false);
        state.update(|s| s.apply(GldfAction::SetConstantLightOutput(scope.get(), Some(v))));
    };
    let on_dist = move |e: web_sys::Event| {
        let v = event_value(&e).unwrap_or_default();
        state.update(|s| s.apply(GldfAction::SetLightDistribution(scope.get(), optional_string(v))));
    };
    let on_sc = move |e: web_sys::Event| {
        let v = event_value(&e).unwrap_or_default();
        state.update(|s| s.apply(GldfAction::SetSwitchingCapacity(scope.get(), optional_string(v))));
    };

    fn safety_label(v: &str) -> &str {
        match v {
            "0" => "Class 0 (no earth, no double insulation)",
            "I" => "Class I (protective earth)",
            "0I" => "Class 0I (basic insulation + earth)",
            "II" => "Class II (double insulation)",
            "III" => "Class III (SELV)",
            other => other,
        }
    }

    let title_signal = move || ui_t("gldf.electrical.title", &state.with(|s| s.ui_language.clone()));

    view! {
        <div class="editor-section electrical-editor">
            <h2>{ title_signal }</h2>
            <ScopePicker scope=scope />

            {move || state.with(|s| {
                let current = scope.get();
                let lang = s.ui_language.clone();
                let tr = |key: &str| ui_t(key, &lang);
                let product_e: Option<Electrical> = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|d| d.electrical.clone());
                let variant_e: Option<Electrical> = match current {
                    Scope::Variant(idx) => s
                        .product
                        .product_definitions
                        .variants
                        .as_ref()
                        .and_then(|vs| vs.variant.get(idx))
                        .and_then(|v| v.descriptive_attributes.as_ref())
                        .and_then(|d| d.electrical.clone()),
                    Scope::Product => None,
                };
                let (explicit, inherited): (Option<Electrical>, Option<Electrical>) = match current {
                    Scope::Product => (product_e, None),
                    Scope::Variant(_) => (variant_e, product_e),
                };

                let safety_class = explicit.as_ref().and_then(|e| e.electrical_safety_class.clone()).unwrap_or_default();
                let ip_code = explicit.as_ref().and_then(|e| e.ingress_protection_ip_code.clone()).unwrap_or_default();
                let power_factor = explicit.as_ref().and_then(|e| e.power_factor);
                let constant_light = explicit.as_ref().and_then(|e| e.constant_light_output);
                let light_dist = explicit.as_ref().and_then(|e| e.light_distribution.clone()).unwrap_or_default();
                let sc = explicit.as_ref().and_then(|e| e.switching_capacity.clone()).unwrap_or_default();

                let inherited_safety = inherited.as_ref().and_then(|e| e.electrical_safety_class.clone()).unwrap_or_default();
                let inherited_ip = inherited.as_ref().and_then(|e| e.ingress_protection_ip_code.clone()).unwrap_or_default();
                let inherited_pf = inherited.as_ref().and_then(|e| e.power_factor).map(|v| v.to_string()).unwrap_or_default();
                let inherited_dist = inherited.as_ref().and_then(|e| e.light_distribution.clone()).unwrap_or_default();
                let inherited_sc = inherited.as_ref().and_then(|e| e.switching_capacity.clone()).unwrap_or_default();

                let pf_placeholder = if inherited_pf.is_empty() {
                    tr("gldf.electrical.power_factor_placeholder")
                } else {
                    tr("gldf.electrical.inherits_fmt").replace("{value}", &inherited_pf)
                };
                let sc_placeholder = if inherited_sc.is_empty() {
                    tr("gldf.electrical.switching_capacity_placeholder")
                } else {
                    tr("gldf.electrical.inherits_fmt").replace("{value}", &inherited_sc)
                };
                let not_in_xsd_suffix = tr("gldf.electrical.not_in_xsd");
                let safety_legacy = if !safety_class.is_empty()
                    && !xsd_enums::is_canonical(xsd_enums::ELECTRICAL_SAFETY_CLASSES, &safety_class)
                {
                    let label = format!("⚠ {} ({})", safety_class, not_in_xsd_suffix);
                    Some(view! {
                        <option value=safety_class.clone() selected=true>
                            { label }
                        </option>
                    })
                } else { None };
                let ip_legacy = if !ip_code.is_empty()
                    && !xsd_enums::is_canonical(xsd_enums::IP_CODES, &ip_code)
                {
                    let label = format!("⚠ {} ({})", ip_code, not_in_xsd_suffix);
                    Some(view! {
                        <option value=ip_code.clone() selected=true>
                            { label }
                        </option>
                    })
                } else { None };

                let select_placeholder = tr("gldf.common.select");
                let inherits_class_fmt = tr("gldf.electrical.inherits_class_fmt");
                let inherits_fmt = tr("gldf.electrical.inherits_fmt");

                view! {
                    <div class="form-group">
                        <label for="safety-class">{ tr("gldf.electrical.safety_class") }</label>
                        <select id="safety-class" on:change=on_safety>
                            <option value="" selected=safety_class.is_empty()>
                                { if inherited_safety.is_empty() {
                                    select_placeholder.clone()
                                } else {
                                    inherits_class_fmt.clone().replace("{value}", &inherited_safety)
                                } }
                            </option>
                            {xsd_enums::ELECTRICAL_SAFETY_CLASSES.iter().map(|v| {
                                let selected = safety_class == *v;
                                view! { <option value=(*v).to_string() selected=selected>{ safety_label(v) }</option> }
                            }).collect::<Vec<_>>()}
                            { safety_legacy }
                        </select>
                        <small class="form-help">{ tr("gldf.electrical.safety_class_help") }</small>
                    </div>

                    <div class="form-group">
                        <label for="ip-code">{ tr("gldf.electrical.ip_code") }</label>
                        <select id="ip-code" on:change=on_ip>
                            <option value="" selected=ip_code.is_empty()>
                                { if inherited_ip.is_empty() {
                                    select_placeholder.clone()
                                } else {
                                    inherits_fmt.clone().replace("{value}", &inherited_ip)
                                } }
                            </option>
                            {xsd_enums::IP_CODES.iter().map(|v| {
                                let selected = ip_code == *v;
                                view! { <option value=(*v).to_string() selected=selected>{ *v }</option> }
                            }).collect::<Vec<_>>()}
                            { ip_legacy }
                        </select>
                        <small class="form-help">{ tr("gldf.electrical.ip_code_help") }</small>
                    </div>

                    <div class="form-group">
                        <label for="power-factor">{ tr("gldf.electrical.power_factor") }</label>
                        <input
                            type="number"
                            id="power-factor"
                            prop:value=power_factor.map(|v| v.to_string()).unwrap_or_default()
                            on:change=on_pf
                            placeholder=pf_placeholder
                            step="0.01"
                            min="0"
                            max="1"
                        />
                        <small class="form-help">{ tr("gldf.electrical.power_factor_help") }</small>
                    </div>

                    <div class="form-group checkbox-group">
                        <label>
                            <input
                                type="checkbox"
                                prop:checked=constant_light.unwrap_or(false)
                                on:change=on_clo
                            />
                            { format!(" {}", tr("gldf.electrical.clo")) }
                        </label>
                        <small class="form-help">{ tr("gldf.electrical.clo_help") }</small>
                    </div>

                    <div class="form-group">
                        <label for="light-distribution">{ tr("gldf.electrical.light_distribution") }</label>
                        <select id="light-distribution" on:change=on_dist>
                            <option value="" selected=light_dist.is_empty()>
                                { if inherited_dist.is_empty() {
                                    select_placeholder.clone()
                                } else {
                                    inherits_fmt.clone().replace("{value}", &inherited_dist)
                                } }
                            </option>
                            {xsd_enums::LIGHT_DISTRIBUTIONS.iter().map(|v| {
                                let selected = light_dist == *v;
                                view! { <option value=(*v).to_string() selected=selected>{ *v }</option> }
                            }).collect::<Vec<_>>()}
                        </select>
                        <small class="hint">
                            { tr("gldf.electrical.distribution_hint") }
                        </small>
                    </div>

                    <div class="form-group">
                        <label for="switching-capacity">{ tr("gldf.electrical.switching_capacity") }</label>
                        <input
                            type="text"
                            id="switching-capacity"
                            prop:value=sc
                            on:change=on_sc
                            placeholder=sc_placeholder
                        />
                    </div>
                }
            })}
        </div>
    }
}
