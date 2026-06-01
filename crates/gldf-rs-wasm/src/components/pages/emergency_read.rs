//! Emergency read view — dedicated type + Duration/Flux table.

use crate::state::use_gldf;
use leptos::prelude::*;

#[component]
pub fn EmergencyReadPage() -> impl IntoView {
    let state = use_gldf();

    view! {
        <div class="editor-section emergency-readonly">
            <h2>{ "Emergency" }</h2>
            <p class="section-description" style="color: var(--text-tertiary);">
                { "Emergency lighting attributes." }
            </p>

            {move || state.with(|s| {
                let emergency = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|a| a.emergency.clone());
                let Some(emergency) = emergency else {
                    return view! {
                        <p style="color: var(--text-tertiary);">{ "No emergency block declared." }</p>
                    }.into_any();
                };
                let dedicated_type = emergency
                    .dedicated_emergency_lighting_type
                    .clone()
                    .unwrap_or_else(|| "—".to_string());
                let entries = emergency
                    .duration_time_and_flux
                    .as_ref()
                    .map(|d| d.flux.clone())
                    .unwrap_or_default();
                view! {
                    <>
                    <div class="aggregate-row" style="margin-bottom: 12px;">
                        <span class="aggregate-row-label">{ "Dedicated emergency lighting type" }</span>
                        <span class="aggregate-row-value">{ dedicated_type }</span>
                    </div>
                    {if entries.is_empty() {
                        view! {
                            <p style="color: var(--text-tertiary); font-size: 12px;">
                                { "No duration / flux measurements declared." }
                            </p>
                        }.into_any()
                    } else {
                        view! {
                            <div style="margin-top: 8px;">
                                <h3 style="margin: 0 0 6px 0; font-size: 14px;">{ "Duration / Flux" }</h3>
                                <table class="data-table" style="max-width: 320px;">
                                    <thead>
                                        <tr><th>{ "Hours" }</th><th>{ "Flux (lm)" }</th></tr>
                                    </thead>
                                    <tbody>
                                        {entries.into_iter().map(|f| view! {
                                            <tr><td>{ f.hours }</td><td>{ f.value }</td></tr>
                                        }).collect::<Vec<_>>()}
                                    </tbody>
                                </table>
                            </div>
                        }.into_any()
                    }}
                    </>
                }.into_any()
            })}
        </div>
    }
}
