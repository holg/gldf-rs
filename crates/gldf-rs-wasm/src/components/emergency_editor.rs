//! Emergency lighting editor (ProductMetaData → DescriptiveAttributes → Emergency).
//!
//! Two parts per XSD lines 4697–4744:
//! - `DurationTimeAndFlux`: a table of `<Flux hours="N">lumens</Flux>` rows
//!   describing how the emitted flux varies over the emergency lighting
//!   period.
//! - `DedicatedEmergencyLightingType`: a single value from the closed XSD
//!   enumeration (Exit Light / Guide Light / Evacuation Light / Reference
//!   Light / For Signage / For Lighting). Meaningful only for luminaires
//!   intended for dedicated emergency use.
//!
//! Most general-purpose luminaires (the BIOLUX HCL DL fixture among them) do
//! not populate either of these. Emergency-rated luminaires do.

use crate::state::{use_gldf, GldfAction};
use yew::prelude::*;

/// XSD `DedicatedEmergencyLightingType` enumeration. Routed through the
/// shared canonical table in [`crate::i18n::xsd_enums`].
use crate::i18n::xsd_enums::EMERGENCY_LIGHTING_TYPES;

#[function_component(EmergencyEditor)]
pub fn emergency_editor() -> Html {
    let gldf = use_gldf();

    let emergency = gldf
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.descriptive_attributes.as_ref())
        .and_then(|a| a.emergency.as_ref());

    let dedicated_type = emergency
        .and_then(|e| e.dedicated_emergency_lighting_type.clone())
        .unwrap_or_default();

    let on_type_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let v = select.value();
            gldf.dispatch(GldfAction::SetDedicatedEmergencyLightingType(
                if v.is_empty() { None } else { Some(v) },
            ));
        })
    };

    let on_add_flux = {
        let gldf = gldf.clone();
        Callback::from(move |_| gldf.dispatch(GldfAction::AddEmergencyFlux))
    };

    html! {
        <div class="editor-section emergency-editor">
            <h2>{ "Emergency" }</h2>
            <p class="section-description">
                { "Emergency lighting attributes. Populate when the luminaire is rated for emergency use; leave empty otherwise." }
            </p>

            <div class="form-group">
                <label for="dedicated-emergency-type">{ "Dedicated emergency lighting type" }</label>
                <select id="dedicated-emergency-type" onchange={on_type_change} value={dedicated_type.clone()}>
                    <option value="" selected={dedicated_type.is_empty()}>{ "(unset)" }</option>
                    { for EMERGENCY_LIGHTING_TYPES.iter().map(|t| {
                        let selected = *t == dedicated_type.as_str();
                        html! { <option value={(*t).to_string()} selected={selected}>{ *t }</option> }
                    }) }
                </select>
                <span class="hint">{ "Used for dedicated emergency luminaires only. General-purpose luminaires that switch into emergency mode set this on the LightEmitter via emergencyBehaviour=\"Combined\" instead." }</span>
            </div>

            <div class="form-group">
                <h3>{ "Duration Time and Flux" }</h3>
                <p class="hint">
                    { "Emitted luminous flux (lumens) at given duration hours. Add one row per measurement point." }
                </p>
                { render_flux_table(emergency, &gldf) }
                <button type="button" onclick={on_add_flux}>{ "+ Add row" }</button>
            </div>
        </div>
    }
}

fn render_flux_table(
    emergency: Option<&gldf_rs::gldf::product_definitions::Emergency>,
    gldf: &crate::state::GldfContext,
) -> Html {
    let entries: &[gldf_rs::gldf::product_definitions::Flux] = emergency
        .and_then(|e| e.duration_time_and_flux.as_ref())
        .map(|d| d.flux.as_slice())
        .unwrap_or(&[]);

    if entries.is_empty() {
        return html! {
            <p class="empty-message">{ "No duration/flux rows." }</p>
        };
    }

    html! {
        <table class="emergency-flux-table">
            <thead>
                <tr>
                    <th>{ "Hours" }</th>
                    <th>{ "Flux (lm)" }</th>
                    <th></th>
                </tr>
            </thead>
            <tbody>
                { for entries.iter().enumerate().map(|(idx, flux)| render_flux_row(idx, flux, gldf)) }
            </tbody>
        </table>
    }
}

fn render_flux_row(
    idx: usize,
    flux: &gldf_rs::gldf::product_definitions::Flux,
    gldf: &crate::state::GldfContext,
) -> Html {
    let snap_hours = flux.hours;
    let snap_value = flux.value;

    let on_hours = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let hours: i32 = input.value().trim().parse().unwrap_or(0);
            gldf.dispatch(GldfAction::UpdateEmergencyFlux {
                index: idx,
                hours,
                flux: snap_value,
            });
        })
    };

    let on_flux = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let flux_v: i32 = input.value().trim().parse().unwrap_or(0);
            gldf.dispatch(GldfAction::UpdateEmergencyFlux {
                index: idx,
                hours: snap_hours,
                flux: flux_v,
            });
        })
    };

    let on_remove = {
        let gldf = gldf.clone();
        Callback::from(move |_| gldf.dispatch(GldfAction::RemoveEmergencyFlux(idx)))
    };

    html! {
        <tr>
            <td>
                <input
                    type="number"
                    min="0"
                    value={flux.hours.to_string()}
                    onchange={on_hours}
                />
            </td>
            <td>
                <input
                    type="number"
                    min="0"
                    value={flux.value.to_string()}
                    onchange={on_flux}
                />
            </td>
            <td>
                <button type="button" class="row-remove" title="Remove row" onclick={on_remove}>
                    { "✕" }
                </button>
            </td>
        </tr>
    }
}
