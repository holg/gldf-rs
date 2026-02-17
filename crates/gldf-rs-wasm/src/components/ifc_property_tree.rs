//! IFC Property Tree Component
//!
//! Displays IFC luminaire properties in an expandable tree view.

use gldf_rs::ifc::ImportedLuminaire;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct IfcPropertyTreeProps {
    pub luminaire: ImportedLuminaire,
    #[prop_or(0)]
    pub selected_variant: usize,
}

/// Collapsible tree node state
#[derive(Clone, PartialEq)]
enum NodeState {
    Expanded,
    Collapsed,
}

impl NodeState {
    fn toggle(&self) -> Self {
        match self {
            NodeState::Expanded => NodeState::Collapsed,
            NodeState::Collapsed => NodeState::Expanded,
        }
    }
}

#[function_component(IfcPropertyTree)]
pub fn ifc_property_tree(props: &IfcPropertyTreeProps) -> Html {
    // State for expanded/collapsed nodes
    let product_expanded = use_state(|| NodeState::Expanded);
    let variant_expanded = use_state(|| NodeState::Expanded);
    let light_sources_expanded = use_state(|| NodeState::Expanded);
    let electrical_expanded = use_state(|| NodeState::Collapsed);
    let descriptive_expanded = use_state(|| NodeState::Collapsed);

    let luminaire = &props.luminaire;
    let selected_variant = props.selected_variant;

    // Get selected variant
    let variant = luminaire.variants.get(selected_variant);

    // Pre-compute descriptive attributes check
    let has_desc = luminaire
        .descriptive_attributes
        .electrical_safety_class
        .is_some()
        || luminaire.descriptive_attributes.ip_code.is_some()
        || luminaire
            .descriptive_attributes
            .median_useful_life
            .is_some();

    html! {
        <div class="ifc-property-tree">
            // Product Info
            <div class="tree-node">
                <div class="tree-header" onclick={
                    let expanded = product_expanded.clone();
                    Callback::from(move |_| expanded.set(expanded.toggle()))
                }>
                    <span class="tree-icon">{if *product_expanded == NodeState::Expanded { "▼" } else { "▶" }}</span>
                    <span class="tree-label">{"Product Information"}</span>
                </div>
                {if *product_expanded == NodeState::Expanded {
                    html! {
                        <div class="tree-children">
                            <PropertyRow label="Name" value={luminaire.name.clone()} />
                            {luminaire.description.as_ref().map(|d| html! {
                                <PropertyRow label="Description" value={d.clone()} />
                            })}
                            {luminaire.manufacturer.as_ref().map(|m| html! {
                                <PropertyRow label="Manufacturer" value={m.clone()} />
                            })}
                            {luminaire.model_reference.as_ref().map(|m| html! {
                                <PropertyRow label="Model Reference" value={m.clone()} />
                            })}
                        </div>
                    }
                } else {
                    html! {}
                }}
            </div>

            // Variant Properties (if variant selected)
            {variant.map(|v| html! {
                <div class="tree-node">
                    <div class="tree-header" onclick={
                        let expanded = variant_expanded.clone();
                        Callback::from(move |_| expanded.set(expanded.toggle()))
                    }>
                        <span class="tree-icon">{if *variant_expanded == NodeState::Expanded { "▼" } else { "▶" }}</span>
                        <span class="tree-label">{format!("Variant: {}", v.name)}</span>
                    </div>
                    {if *variant_expanded == NodeState::Expanded {
                        html! {
                            <div class="tree-children">
                                {v.description.as_ref().map(|d| html! {
                                    <PropertyRow label="Description" value={d.clone()} />
                                })}
                                {v.gldf_variant_id.as_ref().map(|id| html! {
                                    <PropertyRow label="GLDF Variant ID" value={id.clone()} />
                                })}
                                {v.fixture_type.as_ref().map(|ft| html! {
                                    <PropertyRow label="Fixture Type" value={ft.clone()} />
                                })}
                                {v.properties.color_temperature.map(|cct| html! {
                                    <PropertyRow label="Color Temperature" value={format!("{} K", cct as i32)} />
                                })}
                                {v.properties.luminous_flux.map(|flux| html! {
                                    <PropertyRow label="Luminous Flux" value={format!("{:.0} lm", flux)} />
                                })}
                                {v.properties.power.map(|power| html! {
                                    <PropertyRow label="Power" value={format!("{:.1} W", power)} />
                                })}
                                {v.properties.efficacy.map(|eff| html! {
                                    <PropertyRow label="Efficacy" value={format!("{:.1} lm/W", eff)} />
                                })}
                                {v.properties.cri.map(|cri| html! {
                                    <PropertyRow label="CRI" value={format!("{}", cri)} />
                                })}
                                {v.geometry.as_ref().map(|g| html! {
                                    <PropertyRow label="Geometry" value={format!("{} vertices, {} triangles", g.vertices.len(), g.triangles.len())} />
                                })}
                            </div>
                        }
                    } else {
                        html! {}
                    }}
                </div>
            })}

            // Light Sources
            {variant.map(|v| {
                if !v.light_sources.is_empty() {
                    html! {
                        <div class="tree-node">
                            <div class="tree-header" onclick={
                                let expanded = light_sources_expanded.clone();
                                Callback::from(move |_| expanded.set(expanded.toggle()))
                            }>
                                <span class="tree-icon">{if *light_sources_expanded == NodeState::Expanded { "▼" } else { "▶" }}</span>
                                <span class="tree-label">{format!("Light Sources ({})", v.light_sources.len())}</span>
                            </div>
                            {if *light_sources_expanded == NodeState::Expanded {
                                html! {
                                    <div class="tree-children">
                                        {for v.light_sources.iter().enumerate().map(|(i, ls)| html! {
                                            <div class="tree-leaf light-source">
                                                <span class="light-source-name">{format!("{}. {}", i + 1, ls.name)}</span>
                                                {ls.color_temperature.map(|cct| html! {
                                                    <span class="light-source-detail">{format!("{:.0} K", cct)}</span>
                                                })}
                                                {ls.luminous_flux.map(|flux| html! {
                                                    <span class="light-source-detail">{format!("{:.0} lm", flux)}</span>
                                                })}
                                                {ls.emission_source.as_ref().map(|src| html! {
                                                    <span class="light-source-detail">{src}</span>
                                                })}
                                                {ls.distribution.as_ref().map(|d| html! {
                                                    <span class="light-source-detail">
                                                        {format!("{} ({} planes)", d.distribution_type, d.data.len())}
                                                    </span>
                                                })}
                                            </div>
                                        })}
                                    </div>
                                }
                            } else {
                                html! {}
                            }}
                        </div>
                    }
                } else {
                    html! {}
                }
            })}

            // Electrical Properties
            {variant.map(|v| {
                let has_electrical = v.properties.ip_code.is_some()
                    || v.properties.electrical_safety_class.is_some()
                    || v.properties.rated_voltage.is_some();

                if has_electrical {
                    html! {
                        <div class="tree-node">
                            <div class="tree-header" onclick={
                                let expanded = electrical_expanded.clone();
                                Callback::from(move |_| expanded.set(expanded.toggle()))
                            }>
                                <span class="tree-icon">{if *electrical_expanded == NodeState::Expanded { "▼" } else { "▶" }}</span>
                                <span class="tree-label">{"Electrical Properties"}</span>
                            </div>
                            {if *electrical_expanded == NodeState::Expanded {
                                html! {
                                    <div class="tree-children">
                                        {v.properties.ip_code.as_ref().map(|ip| html! {
                                            <PropertyRow label="IP Code" value={ip.clone()} />
                                        })}
                                        {v.properties.electrical_safety_class.as_ref().map(|sc| html! {
                                            <PropertyRow label="Safety Class" value={sc.clone()} />
                                        })}
                                        {v.properties.rated_voltage.map(|v| html! {
                                            <PropertyRow label="Rated Voltage" value={format!("{:.0} V", v)} />
                                        })}
                                    </div>
                                }
                            } else {
                                html! {}
                            }}
                        </div>
                    }
                } else {
                    html! {}
                }
            })}

            // Descriptive Attributes
            {if has_desc {
                let expanded = descriptive_expanded.clone();
                let on_click = Callback::from(move |_| expanded.set(expanded.toggle()));
                html! {
                    <div class="tree-node">
                        <div class="tree-header" onclick={on_click}>
                            <span class="tree-icon">{if *descriptive_expanded == NodeState::Expanded { "▼" } else { "▶" }}</span>
                            <span class="tree-label">{"Product Attributes"}</span>
                        </div>
                        {if *descriptive_expanded == NodeState::Expanded {
                            html! {
                                <div class="tree-children">
                                    {luminaire.descriptive_attributes.electrical_safety_class.as_ref().map(|sc| html! {
                                        <PropertyRow label="Safety Class" value={sc.clone()} />
                                    })}
                                    {luminaire.descriptive_attributes.ip_code.as_ref().map(|ip| html! {
                                        <PropertyRow label="IP Code" value={ip.clone()} />
                                    })}
                                    {luminaire.descriptive_attributes.median_useful_life.as_ref().map(|life| html! {
                                        <PropertyRow label="Median Useful Life" value={life.clone()} />
                                    })}
                                </div>
                            }
                        } else {
                            html! {}
                        }}
                    </div>
                }
            } else {
                html! {}
            }}
        </div>
    }
}

/// Simple property row component
#[derive(Properties, PartialEq, Clone)]
struct PropertyRowProps {
    label: String,
    value: String,
}

#[function_component(PropertyRow)]
fn property_row(props: &PropertyRowProps) -> Html {
    html! {
        <div class="property-row">
            <span class="property-label">{&props.label}</span>
            <span class="property-value">{&props.value}</span>
        </div>
    }
}
