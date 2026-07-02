//! Custom properties inspector — read-only.
//!
//! GLDF's `<Property>` entries under DescriptiveAttributes can be at
//! ProductMetaData and per-Variant scope. Edit semantics aren't well-defined
//! for vendor-opaque blobs; the Yew side stayed read-only and so do we.

use crate::i18n::ui::t as ui_t;
use crate::state::use_gldf;
use gldf_rs::gldf::product_definitions::Property;
use leptos::prelude::*;

#[component]
pub fn CustomPropertiesPage() -> impl IntoView {
    let state = use_gldf();

    let title_signal = move || ui_t("gldf.custom_properties.title", &state.with(|s| s.ui_language.clone()));

    view! {
        <div class="editor-section custom-properties-editor">
            <h2>{ title_signal }</h2>
            {move || state.with(|s| {
                let lang = s.ui_language.clone();
                let tr = |key: &str| ui_t(key, &lang);
                let meta_props: Vec<Property> = s
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_ref()
                    .and_then(|m| m.descriptive_attributes.as_ref())
                    .and_then(|a| a.custom_properties.as_ref())
                    .map(|cp| cp.property.clone())
                    .unwrap_or_default();
                let variant_groups: Vec<(String, Vec<Property>)> = s
                    .product
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|vs| vs.variant.iter().filter_map(|v| {
                        let props = v.descriptive_attributes.as_ref().and_then(|a| a.custom_properties.as_ref())?;
                        Some((v.id.clone(), props.property.clone()))
                    }).collect())
                    .unwrap_or_default();
                let total = meta_props.len() + variant_groups.iter().map(|(_, p)| p.len()).sum::<usize>();

                let product_level_label = tr("gldf.custom_properties.product_level_fmt");
                let variant_label = tr("gldf.custom_properties.variant_fmt");
                view! {
                    <p class="section-description">
                        { tr("gldf.custom_properties.description").replace("{n}", &total.to_string()) }
                    </p>

                    {if !meta_props.is_empty() {
                        let header = product_level_label.replace("{n}", &meta_props.len().to_string());
                        view! {
                            <h3>{ header }</h3>
                            { render_property_list(meta_props) }
                        }.into_any()
                    } else { ().into_any() }}

                    {variant_groups.into_iter().map(|(vid, props)| {
                        let count = props.len();
                        let header = variant_label
                            .clone()
                            .replace("{vid}", &vid)
                            .replace("{n}", &count.to_string());
                        view! {
                            <>
                                <h3>{ header }</h3>
                                { render_property_list(props) }
                            </>
                        }
                    }).collect::<Vec<_>>()}

                    {if total == 0 {
                        let msg = tr("gldf.custom_properties.empty");
                        view! { <p class="empty-message">{ msg }</p> }.into_any()
                    } else { ().into_any() }}
                }
            })}
        </div>
    }
}

fn render_property_list(properties: Vec<Property>) -> impl IntoView {
    view! {
        <div class="custom-properties-list">
            {properties.into_iter().map(render_property).collect::<Vec<_>>()}
        </div>
    }
}

fn render_property(property: Property) -> impl IntoView {
    let names: Vec<String> = property
        .name
        .locale
        .iter()
        .filter(|l| !l.value.is_empty())
        .map(|l| {
            if l.language.is_empty() {
                l.value.clone()
            } else {
                format!("{} [{}]", l.value, l.language)
            }
        })
        .collect();
    let name = if names.is_empty() { "(unnamed)".to_string() } else { names.join(" / ") };
    let property_id = property.id.clone();
    let property_source = property.property_source.clone();
    let has_source = !property_source.is_empty();

    let body = if let Some(file_ref) = &property.file_reference {
        let fid = file_ref.file_id.clone();
        view! {
            <div class="property-value file-ref">
                <span class="label">{ "File reference:" }</span>
                <span class="value code">{ fid }</span>
            </div>
        }.into_any()
    } else {
        render_value(&property.value).into_any()
    };

    view! {
        <details class="custom-property" style="margin-bottom: 6px; padding: 6px; background: var(--bg-secondary); border-radius: 4px;">
            <summary style="display: flex; gap: 8px; align-items: baseline; cursor: pointer;">
                <span class="property-id code" style="font-family: var(--font-mono);">{ property_id }</span>
                <span class="property-name">{ name }</span>
                {if has_source {
                    view! {
                        <span class="property-source" style="color: var(--text-tertiary); font-size: 11px;">{ format!("source: {}", property_source) }</span>
                    }.into_any()
                } else { ().into_any() }}
            </summary>
            { body }
        </details>
    }
}

fn render_value(raw: &str) -> impl IntoView {
    let trimmed = raw.trim();
    let pretty = if trimmed.starts_with('{') || trimmed.starts_with('[') {
        serde_json::from_str::<serde_json::Value>(trimmed)
            .ok()
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
    } else { None };
    let raw = raw.to_string();
    match pretty {
        Some(json) => view! { <pre class="property-value json" style="background: var(--bg-primary); padding: 6px; overflow: auto; font-size: 11px;">{ json }</pre> }.into_any(),
        None => view! { <pre class="property-value text" style="background: var(--bg-primary); padding: 6px; overflow: auto; font-size: 11px;">{ raw }</pre> }.into_any(),
    }
}
