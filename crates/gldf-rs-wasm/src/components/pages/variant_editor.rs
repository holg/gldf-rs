//! Variant editor — per-SKU overrides of ProductMetaData fields.
//!
//! Expandable variant cards. Each card edits the four LocaleFoo fields
//! (Name / Description / Product Number / Tender Text) plus shows GTIN,
//! mountings, and geometry references read-only. Full mounting edit is
//! deferred to a follow-on.

use crate::components::LocaleFooEditor;
use crate::state::{use_gldf, GldfAction};
use gldf_rs::gldf::product_definitions::{Ceiling, Ground, Mountings, Variant, Wall};
use gldf_rs::gldf::LocaleFoo;
use leptos::prelude::*;

#[component]
pub fn VariantEditorPage() -> impl IntoView {
    let state = use_gldf();
    let expanded = RwSignal::new(None::<usize>);

    view! {
        <div class="editor-section variant-editor">
            <h2>{ "Variants" }</h2>
            <p class="section-description">
                { "Per-SKU variants. Each entry overrides ProductMetaData for one article number / GTIN. Document-level metadata lives under Product Metadata → Identity." }
            </p>

            {move || state.with(|s| {
                let variants: Vec<Variant> = s
                    .product
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|v| v.variant.clone())
                    .unwrap_or_default();
                let count = variants.len();
                let active_lang = s.active_language.clone();
                view! {
                    <div class="variants-section">
                        <h3>{ format!("Variants ({})", count) }</h3>
                        {if variants.is_empty() {
                            view! { <p class="empty-message">{ "No variants defined." }</p> }.into_any()
                        } else {
                            view! {
                                <div class="variant-cards">
                                    {variants.into_iter().enumerate().map(|(idx, variant)| {
                                        let active_lang = active_lang.clone();
                                        view! {
                                            <VariantCard idx=idx variant=variant active_lang=active_lang expanded=expanded />
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }}
                    </div>
                }
            })}
        </div>
    }
}

#[component]
fn VariantCard(
    idx: usize,
    variant: Variant,
    active_lang: String,
    expanded: RwSignal<Option<usize>>,
) -> impl IntoView {
    let state = use_gldf();

    let on_name = Callback::new(move |v: LocaleFoo| {
        state.update(|s| s.apply(GldfAction::SetVariantName(idx, v)));
    });
    let on_pn = Callback::new(move |v: LocaleFoo| {
        state.update(|s| s.apply(GldfAction::SetVariantProductNumber(idx, v)));
    });
    let on_desc = Callback::new(move |v: LocaleFoo| {
        state.update(|s| s.apply(GldfAction::SetVariantDescription(idx, v)));
    });
    let on_tender = Callback::new(move |v: LocaleFoo| {
        state.update(|s| s.apply(GldfAction::SetVariantTenderText(idx, v)));
    });

    let id_str = variant.id.clone();
    let name_header = variant
        .name
        .as_ref()
        .and_then(|n| n.locale.iter().find(|l| l.language == active_lang).or_else(|| n.locale.first()))
        .map(|l| l.value.clone())
        .unwrap_or_else(|| "(No name)".to_string());
    let sort_order = variant.sort_order;
    let gtin = variant.gtin.clone();
    let name_val = variant.name.clone().unwrap_or_default();
    let pn_val = variant.product_number.clone().unwrap_or_default();
    let desc_val = variant.description.clone().unwrap_or_default();
    let tender_val = variant.tender_text.clone().unwrap_or_default();
    let mountings = variant.mountings.clone();
    let geometry = variant.geometry.clone();

    let on_toggle = move |_| {
        expanded.update(|cur| {
            *cur = if *cur == Some(idx) { None } else { Some(idx) };
        });
    };

    view! {
        <div class="variant-card">
            <div class="card-header" on:click=on_toggle style="cursor: pointer; display: flex; gap: 8px; align-items: center;">
                <span class="card-id">{ id_str.clone() }</span>
                <span class="card-name">{ name_header }</span>
                {sort_order.map(|o| view! { <span class="sort-order">{ format!("#{}", o) }</span> })}
                <span class="expand-icon" style="margin-left: auto;">
                    {move || if expanded.get() == Some(idx) { "▼" } else { "▶" }}
                </span>
            </div>

            {move || {
                let is_expanded = expanded.get() == Some(idx);
                if !is_expanded {
                    return ().into_any();
                }
                let mountings_view = mountings.as_ref().map(render_mountings);
                let geometry_view = geometry.as_ref().map(render_geometry);
                view! {
                    <div class="card-body">
                        <div class="variant-info">
                            <LocaleFooEditor label="Variant Name" value=name_val.clone() on_change=on_name />
                            <LocaleFooEditor label="Product Number" value=pn_val.clone() on_change=on_pn />
                            <LocaleFooEditor
                                label="Description"
                                value=desc_val.clone()
                                on_change=on_desc
                                multiline=true
                            />
                            <LocaleFooEditor
                                label="Tender Text"
                                value=tender_val.clone()
                                on_change=on_tender
                                multiline=true
                            />
                            {gtin.clone().map(|g| view! {
                                <div class="detail">
                                    <span class="label">{ "GTIN:" }</span>
                                    <span class="value code">{ g }</span>
                                </div>
                            })}
                        </div>
                        { mountings_view }
                        { geometry_view }
                    </div>
                }.into_any()
            }}
        </div>
    }
}

fn render_mountings(mountings: &Mountings) -> impl IntoView {
    view! {
        <div class="mountings-details">
            <h5>{ "Mountings" }</h5>
            <div class="mountings-grid">
                { mountings.ceiling.as_ref().map(render_ceiling) }
                { mountings.wall.as_ref().map(render_wall) }
                { mountings.ground.as_ref().map(render_ground) }
                {if mountings.working_plane.is_some() {
                    view! {
                        <div class="mounting-detail">
                            <span class="mounting-type">{ "Working Plane" }</span>
                            <ul class="mounting-subtypes">
                                <li>{ "Free standing" }</li>
                            </ul>
                        </div>
                    }.into_any()
                } else { ().into_any() }}
            </div>
        </div>
    }
}

fn render_ceiling(c: &Ceiling) -> impl IntoView {
    let mut details = Vec::new();
    if let Some(ref recessed) = c.recessed {
        details.push(format!("Recessed (depth: {}mm)", recessed.recessed_depth));
        if recessed.rectangular_cutout.is_some() {
            details.push("Rectangular cutout".to_string());
        }
        if recessed.circular_cutout.is_some() {
            details.push("Circular cutout".to_string());
        }
    }
    if c.surface_mounted.is_some() {
        details.push("Surface mounted".to_string());
    }
    if let Some(ref pendant) = c.pendant {
        details.push(format!("Pendant (length: {:.0}mm)", pendant.pendant_length));
    }
    view! {
        <div class="mounting-detail">
            <span class="mounting-type">{ "Ceiling" }</span>
            <ul class="mounting-subtypes">
                {details.into_iter().map(|d| view! { <li>{ d }</li> }).collect::<Vec<_>>()}
            </ul>
        </div>
    }
}

fn render_wall(w: &Wall) -> impl IntoView {
    let mut details = Vec::new();
    if w.mounting_height > 0 {
        details.push(format!("Mounting height: {}mm", w.mounting_height));
    }
    if w.recessed.is_some() {
        details.push("Recessed".to_string());
    }
    if w.surface_mounted.is_some() {
        details.push("Surface mounted".to_string());
    }
    view! {
        <div class="mounting-detail">
            <span class="mounting-type">{ "Wall" }</span>
            <ul class="mounting-subtypes">
                {details.into_iter().map(|d| view! { <li>{ d }</li> }).collect::<Vec<_>>()}
            </ul>
        </div>
    }
}

fn render_ground(g: &Ground) -> impl IntoView {
    let mut details = Vec::new();
    if let Some(ref pt) = g.pole_top {
        if let Some(h) = pt.get_pole_height() {
            details.push(format!("Pole top (height: {}mm)", h));
        } else {
            details.push("Pole top".to_string());
        }
    }
    if let Some(ref pi) = g.pole_integrated {
        if let Some(h) = pi.get_pole_height() {
            details.push(format!("Pole integrated (height: {}mm)", h));
        } else {
            details.push("Pole integrated".to_string());
        }
    }
    if g.free_standing.is_some() {
        details.push("Free standing".to_string());
    }
    if g.surface_mounted.is_some() {
        details.push("Surface mounted".to_string());
    }
    if g.recessed.is_some() {
        details.push("Recessed".to_string());
    }
    view! {
        <div class="mounting-detail">
            <span class="mounting-type">{ "Ground" }</span>
            <ul class="mounting-subtypes">
                {details.into_iter().map(|d| view! { <li>{ d }</li> }).collect::<Vec<_>>()}
            </ul>
        </div>
    }
}

fn render_geometry(g: &gldf_rs::gldf::product_definitions::Geometry) -> impl IntoView {
    let simple_id = g.simple_geometry_reference.as_ref().map(|s| s.geometry_id.clone());
    let model_id = g.model_geometry_reference.as_ref().map(|m| m.geometry_id.clone());
    view! {
        <div class="geometry-info">
            <h5>{ "Geometry" }</h5>
            {simple_id.map(|id| view! {
                <div class="detail">
                    <span class="label">{ "Simple Geometry:" }</span>
                    <span class="value code">{ id }</span>
                </div>
            })}
            {model_id.map(|id| view! {
                <div class="detail">
                    <span class="label">{ "Model Geometry:" }</span>
                    <span class="value code">{ id }</span>
                </div>
            })}
        </div>
    }
}
