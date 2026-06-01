//! Mechanical read view — same aggregate pattern as Electrical.

use crate::components::aggregate::{aggregate_field, render_aggregate_row, variant_chip_label};
use crate::state::use_gldf;
use gldf_rs::gldf::product_definitions::Mechanical;
use leptos::prelude::*;

#[component]
pub fn MechanicalReadPage() -> impl IntoView {
    let state = use_gldf();

    view! {
        <div class="editor-section mechanical-readonly">
            <h2>{ "Mechanical Attributes" }</h2>
            {move || state.with(|s| {
                let default_m: Option<Mechanical> = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|d| d.mechanical.clone());
                let variants_owned = s
                    .product
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|vs| vs.variant.clone())
                    .unwrap_or_default();

                if default_m.is_none() && variants_owned.is_empty() {
                    return view! {
                        <p style="color: var(--text-tertiary);">{ "No mechanical attributes defined." }</p>
                    }.into_any();
                }

                let label_of: std::collections::HashMap<String, String> = variants_owned
                    .iter()
                    .map(|v| (v.id.clone(), variant_chip_label(v)))
                    .collect();

                let gather = |access: &dyn Fn(&Mechanical) -> Option<String>| -> Vec<(String, Option<String>)> {
                    variants_owned
                        .iter()
                        .map(|v| {
                            let val = v
                                .descriptive_attributes
                                .as_ref()
                                .and_then(|a| a.mechanical.as_ref())
                                .and_then(access);
                            (v.id.clone(), val)
                        })
                        .collect()
                };

                let ik = aggregate_field(
                    default_m.as_ref().and_then(|m| m.ik_rating.clone()),
                    gather(&|m| m.ik_rating.clone()),
                );
                let form = aggregate_field(
                    default_m.as_ref().and_then(|m| m.product_form.clone()),
                    gather(&|m| m.product_form.clone()),
                );
                let weight = aggregate_field(
                    default_m.as_ref().and_then(|m| m.weight).map(|w| format!("{} kg", w)),
                    gather(&|m| m.weight.map(|w| format!("{} kg", w))),
                );
                let size = aggregate_field(
                    default_m
                        .as_ref()
                        .and_then(|m| m.product_size.as_ref())
                        .map(|s| format!("L {} × W {} × H {} mm", s.length, s.width, s.height)),
                    gather(&|m| {
                        m.product_size
                            .as_ref()
                            .map(|s| format!("L {} × W {} × H {} mm", s.length, s.width, s.height))
                    }),
                );
                let sealing = aggregate_field(
                    default_m.as_ref().and_then(|m| m.sealing_material.as_ref().and_then(|lf| lf.locale.first().map(|l| l.value.clone()))),
                    gather(&|m| m.sealing_material.as_ref().and_then(|lf| lf.locale.first().map(|l| l.value.clone()))),
                );

                view! {
                    <div class="aggregate-rows" style="display: flex; flex-direction: column; gap: 8px;">
                        { render_aggregate_row("IK Rating", &ik, &label_of) }
                        { render_aggregate_row("Product Form", &form, &label_of) }
                        { render_aggregate_row("Product Size", &size, &label_of) }
                        { render_aggregate_row("Weight", &weight, &label_of) }
                        { render_aggregate_row("Sealing Material", &sealing, &label_of) }
                    </div>
                }.into_any()
            })}
        </div>
    }
}
