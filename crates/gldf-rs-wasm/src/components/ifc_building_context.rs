//! IFC Building Context Component
//!
//! Displays the IFC building hierarchy: Project → Site → Building → Storey → Luminaires

use gldf_rs::ifc::ImportedLuminaire;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct IfcBuildingContextProps {
    pub luminaire: ImportedLuminaire,
    #[prop_or(0)]
    pub selected_variant: usize,
    #[prop_or_default]
    pub on_variant_select: Callback<usize>,
}

#[function_component(IfcBuildingContext)]
pub fn ifc_building_context(props: &IfcBuildingContextProps) -> Html {
    let luminaire = &props.luminaire;
    let selected = props.selected_variant;
    let on_select = props.on_variant_select.clone();

    // IFC building hierarchy (from our GLDF→IFC export structure)
    // Project → Site → Building → Storey → LightFixtureTypes (variants)
    html! {
        <div class="ifc-building-context">
            <div class="context-title">{"IFC Building Context"}</div>

            <div class="hierarchy">
                // Project level
                <div class="hierarchy-node project">
                    <span class="node-icon">{"📁"}</span>
                    <span class="node-label">{"GLDF Export"}</span>
                </div>

                // Site level
                <div class="hierarchy-node site indent-1">
                    <span class="node-icon">{"🏗️"}</span>
                    <span class="node-label">{"Default Site"}</span>
                </div>

                // Building level
                <div class="hierarchy-node building indent-2">
                    <span class="node-icon">{"🏢"}</span>
                    <span class="node-label">{"Default Building"}</span>
                </div>

                // Storey level
                <div class="hierarchy-node storey indent-3">
                    <span class="node-icon">{"📐"}</span>
                    <span class="node-label">{"Ground Floor"}</span>
                </div>

                // Light Fixture Types (one per variant)
                <div class="hierarchy-section indent-4">
                    <div class="section-header">
                        <span class="node-icon">{"💡"}</span>
                        <span class="section-label">
                            {format!("Light Fixtures ({})", luminaire.variants.len())}
                        </span>
                    </div>

                    <div class="variant-list">
                        {for luminaire.variants.iter().enumerate().map(|(i, v)| {
                            let is_selected = i == selected;
                            let on_click = {
                                let on_select = on_select.clone();
                                Callback::from(move |_| on_select.emit(i))
                            };
                            html! {
                                <div
                                    class={classes!(
                                        "variant-item",
                                        is_selected.then_some("selected")
                                    )}
                                    onclick={on_click}
                                >
                                    <span class="variant-icon">
                                        {if is_selected { "●" } else { "○" }}
                                    </span>
                                    <span class="variant-name">{&v.name}</span>
                                    {v.geometry.as_ref().map(|_g| html! {
                                        <span class="variant-badge geometry">
                                            {"3D"}
                                        </span>
                                    })}
                                    {v.properties.color_temperature.map(|cct| html! {
                                        <span class="variant-badge cct">
                                            {format!("{}K", cct as i32)}
                                        </span>
                                    })}
                                </div>
                            }
                        })}
                    </div>
                </div>
            </div>

            // Product summary
            <div class="product-summary">
                <div class="summary-row">
                    <span class="summary-label">{"Product:"}</span>
                    <span class="summary-value">{&luminaire.name}</span>
                </div>
                {luminaire.manufacturer.as_ref().map(|m| html! {
                    <div class="summary-row">
                        <span class="summary-label">{"Manufacturer:"}</span>
                        <span class="summary-value">{m}</span>
                    </div>
                })}
                <div class="summary-row">
                    <span class="summary-label">{"Variants:"}</span>
                    <span class="summary-value">{luminaire.variants.len()}</span>
                </div>
                <div class="summary-row">
                    <span class="summary-label">{"With Geometry:"}</span>
                    <span class="summary-value">
                        {luminaire.variants.iter().filter(|v| v.geometry.is_some()).count()}
                    </span>
                </div>
            </div>
        </div>
    }
}
