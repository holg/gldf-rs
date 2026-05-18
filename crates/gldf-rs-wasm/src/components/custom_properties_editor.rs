//! Custom properties inspector — read-only v1.
//!
//! GLDF allows arbitrary `<Property>` entries under `DescriptiveAttributes/CustomProperties`,
//! both at `ProductMetaData` level and per-`Variant` level (XSD lines 5517–5567). They
//! carry vendor-specific data the schema cannot natively represent (e.g. Reluxnet's GAV
//! ids, multilingual JSON blobs, internal config). Today the viewer surfaces none of
//! this — for the BIOLUX HCL DL fixture, 29 properties live on the variant and are
//! completely invisible until this tab.
//!
//! v1 is read-only by design: write-back semantics for opaque vendor properties need
//! more thought (do we let the user edit the JSON? validate against vendor schema?).
//! For now, surfacing the data is the win.

use crate::state::use_gldf;
use gldf_rs::gldf::product_definitions::Property;
use yew::prelude::*;

#[function_component(CustomPropertiesEditor)]
pub fn custom_properties_editor() -> Html {
    let gldf = use_gldf();

    let meta_props = gldf
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.descriptive_attributes.as_ref())
        .and_then(|a| a.custom_properties.as_ref())
        .map(|cp| cp.property.as_slice())
        .unwrap_or(&[]);

    let variant_groups: Vec<(String, &[Property])> = gldf
        .product
        .product_definitions
        .variants
        .as_ref()
        .map(|vs| {
            vs.variant
                .iter()
                .filter_map(|v| {
                    let props = v
                        .descriptive_attributes
                        .as_ref()
                        .and_then(|a| a.custom_properties.as_ref())?;
                    Some((v.id.clone(), props.property.as_slice()))
                })
                .collect()
        })
        .unwrap_or_default();

    let total = meta_props.len() + variant_groups.iter().map(|(_, p)| p.len()).sum::<usize>();

    html! {
        <div class="editor-section custom-properties-editor">
            <h2>{ "Custom Properties" }</h2>
            <p class="section-description">
                { format!("Vendor-specific Property entries (read-only, {} total). Per the GLDF spec these carry data the schema cannot represent natively — typically vendor taxonomies or multilingual blobs.", total) }
            </p>

            if !meta_props.is_empty() {
                <h3>{ format!("Product-level ({})", meta_props.len()) }</h3>
                { render_property_list(meta_props) }
            }

            { for variant_groups.iter().map(|(variant_id, props)| html! {
                <>
                    <h3>{ format!("Variant: {} ({})", variant_id, props.len()) }</h3>
                    { render_property_list(props) }
                </>
            }) }

            if total == 0 {
                <p class="empty-message">{ "No custom properties in this file." }</p>
            }
        </div>
    }
}

fn render_property_list(properties: &[Property]) -> Html {
    html! {
        <div class="custom-properties-list">
            { for properties.iter().map(render_property) }
        </div>
    }
}

fn render_property(property: &Property) -> Html {
    // Property.name is XSD-typed Locale (multi-locale). Show all entries
    // joined when more than one is present, falling back to "(unnamed)".
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
    let name = if names.is_empty() {
        "(unnamed)".to_string()
    } else {
        names.join(" / ")
    };
    let name_lang = String::new();

    let body = if let Some(file_ref) = &property.file_reference {
        html! {
            <div class="property-value file-ref">
                <span class="label">{ "File reference:" }</span>
                <span class="value code">{ &file_ref.file_id }</span>
            </div>
        }
    } else {
        render_value(&property.value)
    };

    html! {
        <details class="custom-property" open=false>
            <summary>
                <span class="property-id code">{ &property.id }</span>
                <span class="property-name">{ name }</span>
                <span class="property-name-lang">{ name_lang }</span>
                if !property.property_source.is_empty() {
                    <span class="property-source">{ format!("source: {}", property.property_source) }</span>
                }
            </summary>
            { body }
        </details>
    }
}

/// Render the property value. If it parses as JSON, pretty-print it; otherwise
/// show as plain text.
fn render_value(raw: &str) -> Html {
    let trimmed = raw.trim();
    let pretty = if trimmed.starts_with('{') || trimmed.starts_with('[') {
        serde_json::from_str::<serde_json::Value>(trimmed)
            .ok()
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
    } else {
        None
    };

    match pretty {
        Some(json) => html! {
            <pre class="property-value json">{ json }</pre>
        },
        None => html! {
            <pre class="property-value text">{ raw }</pre>
        },
    }
}
