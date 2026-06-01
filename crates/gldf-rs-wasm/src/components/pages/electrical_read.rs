//! Electrical read view — aggregates ProductMetaData default + per-Variant
//! overrides into one row per field. Same shape as the Yew side.

use crate::components::aggregate::{aggregate_field, render_aggregate_row, variant_chip_label};
use crate::state::use_gldf;
use gldf_rs::gldf::product_definitions::Electrical;
use leptos::prelude::*;

#[component]
pub fn ElectricalReadPage() -> impl IntoView {
    let state = use_gldf();

    view! {
        <div class="editor-section electrical-readonly">
            <h2>{ "Electrical Attributes" }</h2>
            {move || state.with(|s| {
                let default_e: Option<Electrical> = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|d| d.electrical.clone());
                let variants_owned = s
                    .product
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|vs| vs.variant.clone())
                    .unwrap_or_default();

                if default_e.is_none() && variants_owned.is_empty() {
                    return view! {
                        <p style="color: var(--text-tertiary);">{ "No electrical attributes defined." }</p>
                    }.into_any();
                }

                let label_of: std::collections::HashMap<String, String> = variants_owned
                    .iter()
                    .map(|v| (v.id.clone(), variant_chip_label(v)))
                    .collect();

                // Helper closures captured per field — pull both default
                // and variant override values for the same accessor.
                let gather = |access: &dyn Fn(&Electrical) -> Option<String>| -> Vec<(String, Option<String>)> {
                    variants_owned
                        .iter()
                        .map(|v| {
                            let val = v
                                .descriptive_attributes
                                .as_ref()
                                .and_then(|a| a.electrical.as_ref())
                                .and_then(access);
                            (v.id.clone(), val)
                        })
                        .collect()
                };

                let safety = aggregate_field(
                    default_e.as_ref().and_then(|e| e.electrical_safety_class.clone()).map(|s| format!("Class {}", s)),
                    gather(&|e| e.electrical_safety_class.clone().map(|s| format!("Class {}", s))),
                );
                let ip = aggregate_field(
                    default_e.as_ref().and_then(|e| e.ingress_protection_ip_code.clone()),
                    gather(&|e| e.ingress_protection_ip_code.clone()),
                );
                let pf = aggregate_field(
                    default_e.as_ref().and_then(|e| e.power_factor).map(|v| format!("{:.2}", v)),
                    gather(&|e| e.power_factor.map(|v| format!("{:.2}", v))),
                );
                let clo = aggregate_field(
                    default_e.as_ref().and_then(|e| e.constant_light_output).map(|b| if b { "Yes".to_string() } else { "No".to_string() }),
                    gather(&|e| e.constant_light_output.map(|b| if b { "Yes".to_string() } else { "No".to_string() })),
                );
                let dist = aggregate_field(
                    default_e.as_ref().and_then(|e| e.light_distribution.clone()),
                    gather(&|e| e.light_distribution.clone()),
                );
                let sc = aggregate_field(
                    default_e.as_ref().and_then(|e| e.switching_capacity.clone()),
                    gather(&|e| e.switching_capacity.clone()),
                );

                view! {
                    <div class="aggregate-rows" style="display: flex; flex-direction: column; gap: 8px;">
                        { render_aggregate_row("Electrical Safety Class", &safety, &label_of) }
                        { render_aggregate_row("IP Code (Ingress Protection)", &ip, &label_of) }
                        { render_aggregate_row("Power Factor", &pf, &label_of) }
                        { render_aggregate_row("Constant Light Output (CLO)", &clo, &label_of) }
                        { render_aggregate_row("Light Distribution", &dist, &label_of) }
                        { render_aggregate_row("Switching Capacity", &sc, &label_of) }
                    </div>
                }.into_any()
            })}
        </div>
    }
}
