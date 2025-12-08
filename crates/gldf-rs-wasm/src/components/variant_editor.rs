//! Variant editor component for GLDF files

use crate::state::use_gldf;
use yew::prelude::*;

/// Variant editor component
#[function_component(VariantEditor)]
pub fn variant_editor() -> Html {
    let gldf = use_gldf();
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
                { "Define product metadata and variants. Variants represent different configurations of your luminaire product." }
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
                        { for variants.iter().map(|variant| {
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

                            html! {
                                <div class="variant-card" key={variant.id.clone()}>
                                    <div class="card-header">
                                        <span class="card-id">{ &variant.id }</span>
                                        if let Some(order) = &variant.sort_order {
                                            <span class="sort-order">{ format!("#{}", order) }</span>
                                        }
                                    </div>
                                    <div class="card-body">
                                        <h4>{ name }</h4>
                                        if let Some(pn) = product_number {
                                            <div class="product-number">{ pn }</div>
                                        }
                                        if let Some(desc) = description {
                                            <p class="description">{ desc }</p>
                                        }
                                        if let Some(gtin) = &variant.gtin {
                                            <div class="detail">
                                                <span class="label">{ "GTIN:" }</span>
                                                <span class="value">{ gtin }</span>
                                            </div>
                                        }

                                        // Mountings info
                                        if let Some(mountings) = &variant.mountings {
                                            <div class="mountings-info">
                                                <span class="label">{ "Mountings:" }</span>
                                                <div class="mounting-tags">
                                                    if mountings.ceiling.is_some() {
                                                        <span class="tag">{ "Ceiling" }</span>
                                                    }
                                                    if mountings.wall.is_some() {
                                                        <span class="tag">{ "Wall" }</span>
                                                    }
                                                    if mountings.ground.is_some() {
                                                        <span class="tag">{ "Ground" }</span>
                                                    }
                                                    if mountings.working_plane.is_some() {
                                                        <span class="tag">{ "Working Plane" }</span>
                                                    }
                                                </div>
                                            </div>
                                        }

                                        // Geometry reference
                                        if let Some(geometry) = &variant.geometry {
                                            <div class="detail">
                                                <span class="label">{ "Geometry:" }</span>
                                                <span class="value">
                                                    if let Some(simple) = &geometry.simple_geometry_reference {
                                                        { &simple.geometry_id }
                                                    }
                                                    if let Some(model) = &geometry.model_geometry_reference {
                                                        { &model.geometry_id }
                                                    }
                                                </span>
                                            </div>
                                        }
                                    </div>
                                    <div class="card-actions">
                                        <button class="btn-edit" disabled=true title="Editing coming soon">{ "Edit" }</button>
                                    </div>
                                </div>
                            }
                        })}
                    </div>
                }

                <button class="btn-add" disabled=true title="Adding coming soon">{ "+ Add Variant" }</button>
            </div>
        </div>
    }
}
