//! Header editor component for GLDF files

use crate::state::{use_gldf, GldfAction};
use yew::prelude::*;

/// Available emitter view types for the dropdown
const EMITTER_VIEW_OPTIONS: &[(&str, &str, &str)] = &[
    ("", "(default: Polar)", "Use the default view type"),
    ("polar", "Polar", "Traditional polar candela distribution diagram"),
    ("cartesian", "Cartesian", "Cartesian intensity plot"),
    ("heatmap", "Heatmap", "Heat map visualization of light distribution"),
    ("butterfly", "3D Butterfly", "3D butterfly diagram"),
    ("bug", "BUG Rating", "Backlight/Uplight/Glare outdoor rating"),
    ("lcs", "LCS", "Luminaire Classification System"),
    ("spectral", "Spectral (SPD)", "Spectral Power Distribution - best for TM-33 files with color data"),
];

/// Header editor component
#[function_component(HeaderEditor)]
pub fn header_editor() -> Html {
    let gldf = use_gldf();
    let header = &gldf.product.header;

    // Get current default_emitter_view from custom properties
    let current_emitter_view = gldf
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.descriptive_attributes.as_ref())
        .and_then(|d| d.custom_properties.as_ref())
        .and_then(|cp| cp.property.iter().find(|p| p.id == "default_emitter_view"))
        .map(|p| p.value.clone())
        .unwrap_or_default();

    let on_author_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            gldf.dispatch(GldfAction::SetAuthor(input.value()));
        })
    };

    let on_manufacturer_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            gldf.dispatch(GldfAction::SetManufacturer(input.value()));
        })
    };

    let on_creation_time_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            gldf.dispatch(GldfAction::SetCreationTimeCode(input.value()));
        })
    };

    let on_app_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            gldf.dispatch(GldfAction::SetCreatedWithApplication(input.value()));
        })
    };

    let on_default_language_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let value = input.value();
            gldf.dispatch(GldfAction::SetDefaultLanguage(if value.is_empty() {
                None
            } else {
                Some(value)
            }));
        })
    };

    let on_emitter_view_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let value = select.value();
            gldf.dispatch(GldfAction::SetCustomProperty {
                id: "default_emitter_view".to_string(),
                value: if value.is_empty() { None } else { Some(value) },
            });
        })
    };

    let on_format_version_change = {
        let gldf = gldf.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            gldf.dispatch(GldfAction::SetFormatVersion(input.value()));
        })
    };

    html! {
        <div class="editor-section header-editor">
            <h2>{ "Header Information" }</h2>

            <div class="form-group">
                <label for="author">{ "Author" }</label>
                <input
                    type="text"
                    id="author"
                    value={header.author.clone()}
                    onchange={on_author_change}
                    placeholder="Enter author name"
                />
            </div>

            <div class="form-group">
                <label for="manufacturer">{ "Manufacturer" }</label>
                <input
                    type="text"
                    id="manufacturer"
                    value={header.manufacturer.clone()}
                    onchange={on_manufacturer_change}
                    placeholder="Enter manufacturer name"
                />
            </div>

            <div class="form-group">
                <label for="creation-time">{ "Creation Time Code" }</label>
                <input
                    type="datetime-local"
                    id="creation-time"
                    value={header.creation_time_code.clone()}
                    onchange={on_creation_time_change}
                />
            </div>

            <div class="form-group">
                <label for="created-with">{ "Created With Application" }</label>
                <input
                    type="text"
                    id="created-with"
                    value={header.created_with_application.clone()}
                    onchange={on_app_change}
                    placeholder="Enter application name"
                />
            </div>

            <div class="form-group">
                <label for="default-language">{ "Default Language" }</label>
                <input
                    type="text"
                    id="default-language"
                    value={header.default_language.clone().unwrap_or_default()}
                    onchange={on_default_language_change}
                    placeholder="e.g., en, de, fr"
                    maxlength="5"
                />
            </div>

            <div class="form-group">
                <label for="format-version">{ "Format Version" }</label>
                <input
                    type="text"
                    id="format-version"
                    value={header.format_version.to_version_string()}
                    onchange={on_format_version_change}
                    placeholder="e.g., 1.0.0-rc.3"
                />
            </div>

            <hr style="margin: 24px 0; border-color: var(--border-color);" />

            <h3 style="margin-bottom: 16px; font-size: 14px; color: var(--text-secondary);">
                { "Display Settings" }
            </h3>

            <div class="form-group">
                <label for="default-emitter-view">
                    { "Default Emitter View" }
                    <span
                        class="help-icon"
                        title="Sets the default diagram type when viewing emitter photometry.\nThis is stored as a custom property in the GLDF."
                        style="margin-left: 6px; cursor: help; color: var(--accent-blue);"
                    >
                        { "ⓘ" }
                    </span>
                </label>
                <select
                    id="default-emitter-view"
                    value={current_emitter_view.clone()}
                    onchange={on_emitter_view_change}
                    style="width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; background: var(--bg-secondary); color: var(--text-primary);"
                >
                    { for EMITTER_VIEW_OPTIONS.iter().map(|(value, label, _desc)| {
                        html! {
                            <option
                                value={*value}
                                selected={current_emitter_view == *value}
                            >
                                { *label }
                            </option>
                        }
                    })}
                </select>
                <div class="help-text" style="margin-top: 8px; font-size: 11px; color: var(--text-tertiary);">
                    { EMITTER_VIEW_OPTIONS.iter()
                        .find(|(v, _, _)| *v == current_emitter_view.as_str())
                        .map(|(_, _, desc)| *desc)
                        .unwrap_or("Select a view type to see its description")
                    }
                </div>
            </div>
        </div>
    }
}
