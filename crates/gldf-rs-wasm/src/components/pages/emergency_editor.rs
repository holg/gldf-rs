//! Emergency lighting editor: DedicatedEmergencyLightingType + DurationTimeAndFlux table.

use crate::components::edit_util::{event_i32, event_value, optional_string};
use crate::state::{use_gldf, GldfAction};
use gldf_rs::validation::xsd_enums::EMERGENCY_LIGHTING_TYPES;
use leptos::prelude::*;

#[component]
pub fn EmergencyEditorPage() -> impl IntoView {
    let state = use_gldf();

    let on_type = move |e: web_sys::Event| {
        let v = event_value(&e).unwrap_or_default();
        state.update(|s| s.apply(GldfAction::SetDedicatedEmergencyLightingType(optional_string(v))));
    };
    let on_add_flux = move |_| state.update(|s| s.apply(GldfAction::AddEmergencyFlux));

    view! {
        <div class="editor-section emergency-editor">
            <h2>{ "Emergency" }</h2>
            <p class="section-description">
                { "Emergency lighting attributes. Populate when the luminaire is rated for emergency use; leave empty otherwise." }
            </p>

            {move || state.with(|s| {
                let emergency = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|a| a.emergency.clone());
                let dedicated_type = emergency
                    .as_ref()
                    .and_then(|e| e.dedicated_emergency_lighting_type.clone())
                    .unwrap_or_default();
                let entries = emergency
                    .as_ref()
                    .and_then(|e| e.duration_time_and_flux.as_ref())
                    .map(|d| d.flux.clone())
                    .unwrap_or_default();

                view! {
                    <div class="form-group">
                        <label for="dedicated-emergency-type">{ "Dedicated emergency lighting type" }</label>
                        <select id="dedicated-emergency-type" on:change=on_type prop:value=dedicated_type.clone()>
                            <option value="" selected=dedicated_type.is_empty()>{ "(unset)" }</option>
                            {EMERGENCY_LIGHTING_TYPES.iter().map(|t| {
                                let selected = *t == dedicated_type.as_str();
                                view! { <option value=(*t).to_string() selected=selected>{ *t }</option> }
                            }).collect::<Vec<_>>()}
                        </select>
                        <span class="hint">{ "Used for dedicated emergency luminaires only. General-purpose luminaires that switch into emergency mode set this on the LightEmitter via emergencyBehaviour=\"Combined\" instead." }</span>
                    </div>

                    <div class="form-group">
                        <h3>{ "Duration Time and Flux" }</h3>
                        <p class="hint">
                            { "Emitted luminous flux (lumens) at given duration hours. Add one row per measurement point." }
                        </p>
                        {if entries.is_empty() {
                            view! { <p class="empty-message">{ "No duration/flux rows." }</p> }.into_any()
                        } else {
                            view! {
                                <table class="emergency-flux-table" style="width: 100%; max-width: 400px;">
                                    <thead>
                                        <tr>
                                            <th>{ "Hours" }</th>
                                            <th>{ "Flux (lm)" }</th>
                                            <th></th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {entries.into_iter().enumerate().map(|(idx, flux)| {
                                            let snap_hours = flux.hours;
                                            let snap_value = flux.value;
                                            let on_hours = move |e: web_sys::Event| {
                                                let h = event_i32(&e).unwrap_or(0);
                                                state.update(|s| s.apply(GldfAction::UpdateEmergencyFlux { index: idx, hours: h, flux: snap_value }));
                                            };
                                            let on_flux = move |e: web_sys::Event| {
                                                let f = event_i32(&e).unwrap_or(0);
                                                state.update(|s| s.apply(GldfAction::UpdateEmergencyFlux { index: idx, hours: snap_hours, flux: f }));
                                            };
                                            let on_remove = move |_| {
                                                state.update(|s| s.apply(GldfAction::RemoveEmergencyFlux(idx)));
                                            };
                                            view! {
                                                <tr>
                                                    <td><input type="number" min="0" prop:value=flux.hours.to_string() on:change=on_hours /></td>
                                                    <td><input type="number" min="0" prop:value=flux.value.to_string() on:change=on_flux /></td>
                                                    <td><button type="button" class="row-remove" title="Remove row" on:click=on_remove>{ "✕" }</button></td>
                                                </tr>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </tbody>
                                </table>
                            }.into_any()
                        }}
                        <button type="button" on:click=on_add_flux style="margin-top: 8px;">{ "+ Add row" }</button>
                    </div>
                }
            })}
        </div>
    }
}
