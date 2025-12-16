//! Variant editor component for GLDF files

use crate::state::use_gldf;
use gldf_rs::gldf::product_definitions::{Ceiling, Ground, Mountings, Wall};
use yew::prelude::*;

/// Render mounting details for Ceiling type
fn render_ceiling_details(ceiling: &Ceiling) -> Html {
    let mut details = Vec::new();

    if let Some(ref recessed) = ceiling.recessed {
        details.push(format!("Recessed (depth: {}mm)", recessed.recessed_depth));
        if recessed.rectangular_cutout.is_some() {
            details.push("Rectangular cutout".to_string());
        }
        if recessed.circular_cutout.is_some() {
            details.push("Circular cutout".to_string());
        }
    }
    if ceiling.surface_mounted.is_some() {
        details.push("Surface mounted".to_string());
    }
    if let Some(ref pendant) = ceiling.pendant {
        details.push(format!("Pendant (length: {:.0}mm)", pendant.pendant_length));
    }

    html! {
        <div class="mounting-detail">
            <span class="mounting-type">{ "Ceiling" }</span>
            <ul class="mounting-subtypes">
                { for details.iter().map(|d| html! { <li>{ d }</li> }) }
            </ul>
        </div>
    }
}

/// Render mounting details for Wall type
fn render_wall_details(wall: &Wall) -> Html {
    let mut details = Vec::new();

    if wall.mounting_height > 0 {
        details.push(format!("Mounting height: {}mm", wall.mounting_height));
    }
    if wall.recessed.is_some() {
        details.push("Recessed".to_string());
    }
    if wall.surface_mounted.is_some() {
        details.push("Surface mounted".to_string());
    }
    if wall.depth > 0 {
        details.push(format!("Depth: {}mm", wall.depth));
    }

    html! {
        <div class="mounting-detail">
            <span class="mounting-type">{ "Wall" }</span>
            <ul class="mounting-subtypes">
                { for details.iter().map(|d| html! { <li>{ d }</li> }) }
            </ul>
        </div>
    }
}

/// Render mounting details for Ground type
fn render_ground_details(ground: &Ground) -> Html {
    let mut details = Vec::new();

    if let Some(ref pole_top) = ground.pole_top {
        if let Some(height) = pole_top.get_pole_height() {
            details.push(format!("Pole top (height: {}mm)", height));
        } else {
            details.push("Pole top".to_string());
        }
    }
    if let Some(ref pole_int) = ground.pole_integrated {
        if let Some(height) = pole_int.get_pole_height() {
            details.push(format!("Pole integrated (height: {}mm)", height));
        } else {
            details.push("Pole integrated".to_string());
        }
    }
    if ground.free_standing.is_some() {
        details.push("Free standing".to_string());
    }
    if ground.surface_mounted.is_some() {
        details.push("Surface mounted".to_string());
    }
    if ground.recessed.is_some() {
        details.push("Recessed".to_string());
    }

    html! {
        <div class="mounting-detail">
            <span class="mounting-type">{ "Ground" }</span>
            <ul class="mounting-subtypes">
                { for details.iter().map(|d| html! { <li>{ d }</li> }) }
            </ul>
        </div>
    }
}

/// Render full mounting details
fn render_mountings(mountings: &Mountings) -> Html {
    html! {
        <div class="mountings-details">
            <h5>{ "Mountings" }</h5>
            <div class="mountings-grid">
                if let Some(ref ceiling) = mountings.ceiling {
                    { render_ceiling_details(ceiling) }
                }
                if let Some(ref wall) = mountings.wall {
                    { render_wall_details(wall) }
                }
                if let Some(ref ground) = mountings.ground {
                    { render_ground_details(ground) }
                }
                if mountings.working_plane.is_some() {
                    <div class="mounting-detail">
                        <span class="mounting-type">{ "Working Plane" }</span>
                        <ul class="mounting-subtypes">
                            <li>{ "Free standing" }</li>
                        </ul>
                    </div>
                }
            </div>
        </div>
    }
}

/// Variant editor component
#[function_component(VariantEditor)]
pub fn variant_editor() -> Html {
    let gldf = use_gldf();
    let selected_variant = use_state(|| None::<usize>);

    let variants = gldf
        .product
        .product_definitions
        .variants
        .as_ref()
        .map(|v| &v.variant)
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    let product_meta = &gldf.product.product_definitions.product_meta_data;

    html! {
        <div class="editor-section variant-editor">
            <h2>{ "Product & Variants" }</h2>
            <p class="section-description">
                { "Product variants with mounting configurations." }
            </p>

            // Product Metadata Section
            if let Some(meta) = product_meta {
                <div class="product-metadata-card">
                    <h3>{ "Product Metadata" }</h3>
                    <div class="metadata-grid">
                        if let Some(name) = &meta.name {
                            if let Some(locale) = name.locale.first() {
                                <div class="meta-item">
                                    <span class="label">{ "Product Name:" }</span>
                                    <span class="value">{ &locale.value }</span>
                                </div>
                            }
                        }
                        if let Some(number) = &meta.product_number {
                            if let Some(locale) = number.locale.first() {
                                <div class="meta-item">
                                    <span class="label">{ "Product Number:" }</span>
                                    <span class="value">{ &locale.value }</span>
                                </div>
                            }
                        }
                        if let Some(desc) = &meta.description {
                            if let Some(locale) = desc.locale.first() {
                                <div class="meta-item wide">
                                    <span class="label">{ "Description:" }</span>
                                    <span class="value">{ &locale.value }</span>
                                </div>
                            }
                        }
                    </div>
                </div>
            }

            // Variants Section
            <div class="variants-section">
                <h3>{ format!("Variants ({})", variants.len()) }</h3>

                if variants.is_empty() {
                    <p class="empty-message">{ "No variants defined." }</p>
                } else {
                    <div class="variant-cards">
                        { for variants.iter().enumerate().map(|(idx, variant)| {
                            let name = variant.name.as_ref()
                                .and_then(|n| n.locale.first())
                                .map(|l| l.value.clone())
                                .unwrap_or_else(|| "(No name)".to_string());
                            let product_number = variant.product_number.as_ref()
                                .and_then(|n| n.locale.first())
                                .map(|l| l.value.clone());
                            let description = variant.description.as_ref()
                                .and_then(|d| d.locale.first())
                                .map(|l| l.value.clone());

                            let is_expanded = *selected_variant == Some(idx);
                            let selected_variant = selected_variant.clone();
                            let on_toggle = Callback::from(move |_: MouseEvent| {
                                if is_expanded {
                                    selected_variant.set(None);
                                } else {
                                    selected_variant.set(Some(idx));
                                }
                            });

                            html! {
                                <div class={classes!("variant-card", is_expanded.then_some("expanded"))} key={variant.id.clone()}>
                                    <div class="card-header" onclick={on_toggle.clone()}>
                                        <span class="card-id">{ &variant.id }</span>
                                        <span class="card-name">{ &name }</span>
                                        if let Some(order) = &variant.sort_order {
                                            <span class="sort-order">{ format!("#{}", order) }</span>
                                        }
                                        <span class="expand-icon">{ if is_expanded { "▼" } else { "▶" } }</span>
                                    </div>

                                    if is_expanded {
                                        <div class="card-body">
                                            // Basic info
                                            <div class="variant-info">
                                                if let Some(pn) = product_number {
                                                    <div class="detail">
                                                        <span class="label">{ "Product Number:" }</span>
                                                        <span class="value">{ pn }</span>
                                                    </div>
                                                }
                                                if let Some(desc) = description {
                                                    <div class="detail wide">
                                                        <span class="label">{ "Description:" }</span>
                                                        <span class="value">{ desc }</span>
                                                    </div>
                                                }
                                                if let Some(gtin) = &variant.gtin {
                                                    <div class="detail">
                                                        <span class="label">{ "GTIN:" }</span>
                                                        <span class="value code">{ gtin }</span>
                                                    </div>
                                                }
                                            </div>

                                            // Mountings section with full details
                                            if let Some(ref mountings) = variant.mountings {
                                                { render_mountings(mountings) }
                                            } else {
                                                <div class="mountings-details empty">
                                                    <h5>{ "Mountings" }</h5>
                                                    <p class="empty-message">{ "No mounting types defined" }</p>
                                                </div>
                                            }

                                            // Geometry reference
                                            if let Some(geometry) = &variant.geometry {
                                                <div class="geometry-info">
                                                    <h5>{ "Geometry" }</h5>
                                                    if let Some(simple) = &geometry.simple_geometry_reference {
                                                        <div class="detail">
                                                            <span class="label">{ "Simple Geometry:" }</span>
                                                            <span class="value code">{ &simple.geometry_id }</span>
                                                        </div>
                                                    }
                                                    if let Some(model) = &geometry.model_geometry_reference {
                                                        <div class="detail">
                                                            <span class="label">{ "Model Geometry:" }</span>
                                                            <span class="value code">{ &model.geometry_id }</span>
                                                        </div>
                                                    }
                                                </div>
                                            }

                                            // Emitter references (from geometry)
                                            if let Some(geometry) = &variant.geometry {
                                                if let Some(ref model) = geometry.model_geometry_reference {
                                                    if !model.emitter_reference.is_empty() {
                                                        <div class="emitter-refs">
                                                            <h5>{ "Emitter References" }</h5>
                                                            <ul>
                                                                { for model.emitter_reference.iter().map(|er| html! {
                                                                    <li>{ &er.emitter_id }</li>
                                                                })}
                                                            </ul>
                                                        </div>
                                                    }
                                                }
                                            }
                                        </div>
                                    }
                                </div>
                            }
                        })}
                    </div>
                }
            </div>
        </div>
    }
}
