//! Electrical attributes editor component for GLDF files

use crate::state::{use_gldf, GldfAction};
use yew::prelude::*;

/// Electrical editor component
#[function_component(ElectricalEditor)]
pub fn electrical_editor() -> Html {
    let gldf = use_gldf();

    // Get current electrical values
    let electrical = gldf
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.descriptive_attributes.as_ref())
        .and_then(|d| d.electrical.as_ref());

    let safety_class = electrical
        .and_then(|e| e.electrical_safety_class.clone())
        .unwrap_or_default();
    let ip_code = electrical
        .and_then(|e| e.ingress_protection_ip_code.clone())
        .unwrap_or_default();
    let power_factor = electrical.and_then(|e| e.power_factor);
    let constant_light = electrical.and_then(|e| e.constant_light_output);
    let light_distribution = electrical
        .and_then(|e| e.light_distribution.clone())
        .unwrap_or_default();
    let switching_capacity = electrical
        .and_then(|e| e.switching_capacity.clone())
        .unwrap_or_default();

    let on_safety_class_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let value = input.value();
            gldf.dispatch(GldfAction::SetElectricalSafetyClass(if value.is_empty() {
                None
            } else {
                Some(value)
            }));
        })
    };

    let on_ip_code_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let value = input.value();
            gldf.dispatch(GldfAction::SetIngressProtectionIPCode(if value.is_empty() {
                None
            } else {
                Some(value)
            }));
        })
    };

    let on_power_factor_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let value = input.value();
            gldf.dispatch(GldfAction::SetPowerFactor(
                value.parse::<f64>().ok().filter(|&v| v > 0.0),
            ));
        })
    };

    let on_constant_light_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            gldf.dispatch(GldfAction::SetConstantLightOutput(Some(input.checked())));
        })
    };

    let on_light_distribution_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let value = input.value();
            gldf.dispatch(GldfAction::SetLightDistribution(if value.is_empty() {
                None
            } else {
                Some(value)
            }));
        })
    };

    let on_switching_capacity_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let value = input.value();
            gldf.dispatch(GldfAction::SetSwitchingCapacity(if value.is_empty() {
                None
            } else {
                Some(value)
            }));
        })
    };

    html! {
        <div class="editor-section electrical-editor">
            <h2>{ "Electrical Attributes" }</h2>

            <div class="form-group">
                <label for="safety-class">{ "Electrical Safety Class" }</label>
                <select
                    id="safety-class"
                    onchange={on_safety_class_change}
                >
                    <option value="" selected={safety_class.is_empty()}>{ "-- Select --" }</option>
                    <option value="I" selected={safety_class == "I"}>{ "Class I" }</option>
                    <option value="II" selected={safety_class == "II"}>{ "Class II" }</option>
                    <option value="III" selected={safety_class == "III"}>{ "Class III" }</option>
                </select>
                <small class="form-help">{ "IEC electrical safety classification" }</small>
            </div>

            <div class="form-group">
                <label for="ip-code">{ "IP Code (Ingress Protection)" }</label>
                <input
                    type="text"
                    id="ip-code"
                    value={ip_code}
                    onchange={on_ip_code_change}
                    placeholder="e.g., IP65, IP20"
                    maxlength="10"
                />
                <small class="form-help">{ "Protection against solids and liquids (IEC 60529)" }</small>
            </div>

            <div class="form-group">
                <label for="power-factor">{ "Power Factor" }</label>
                <input
                    type="number"
                    id="power-factor"
                    value={power_factor.map(|v| v.to_string()).unwrap_or_default()}
                    onchange={on_power_factor_change}
                    placeholder="e.g., 0.95"
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
                    <option value="" selected={light_distribution.is_empty()}>{ "-- Select --" }</option>
                    <option value="Direct" selected={light_distribution == "Direct"}>{ "Direct" }</option>
                    <option value="Indirect" selected={light_distribution == "Indirect"}>{ "Indirect" }</option>
                    <option value="DirectIndirect" selected={light_distribution == "DirectIndirect"}>{ "Direct/Indirect" }</option>
                    <option value="Symmetric" selected={light_distribution == "Symmetric"}>{ "Symmetric" }</option>
                    <option value="Asymmetric" selected={light_distribution == "Asymmetric"}>{ "Asymmetric" }</option>
                    <option value="Narrow" selected={light_distribution == "Narrow"}>{ "Narrow" }</option>
                    <option value="Medium" selected={light_distribution == "Medium"}>{ "Medium" }</option>
                    <option value="Wide" selected={light_distribution == "Wide"}>{ "Wide" }</option>
                </select>
            </div>

            <div class="form-group">
                <label for="switching-capacity">{ "Switching Capacity" }</label>
                <input
                    type="text"
                    id="switching-capacity"
                    value={switching_capacity}
                    onchange={on_switching_capacity_change}
                    placeholder="e.g., 50000 cycles"
                />
            </div>
        </div>
    }
}
