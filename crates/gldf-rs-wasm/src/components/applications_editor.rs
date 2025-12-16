//! Applications editor component for GLDF files

use crate::state::{use_gldf, GldfAction};
use yew::prelude::*;

/// Common application areas for lighting
const COMMON_APPLICATIONS: &[&str] = &[
    "Office",
    "Industrial",
    "Retail",
    "Hospitality",
    "Healthcare",
    "Education",
    "Residential",
    "Outdoor",
    "Street",
    "Sports",
    "Museum",
    "Warehouse",
    "Parking",
    "Emergency",
    "Accent",
    "Architectural",
    "Facade",
    "Landscape",
];

/// Applications editor component
#[function_component(ApplicationsEditor)]
pub fn applications_editor() -> Html {
    let gldf = use_gldf();
    let new_app = use_state(String::new);

    // Get current applications
    let applications: Vec<String> = gldf
        .product
        .product_definitions
        .product_meta_data
        .as_ref()
        .and_then(|m| m.descriptive_attributes.as_ref())
        .and_then(|d| d.marketing.as_ref())
        .and_then(|m| m.applications.as_ref())
        .map(|a| a.application.clone())
        .unwrap_or_default();

    let on_add_custom = {
        let gldf = gldf.clone();
        let new_app = new_app.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let value = (*new_app).trim().to_string();
            if !value.is_empty() {
                gldf.dispatch(GldfAction::AddApplication(value));
                new_app.set(String::new());
            }
        })
    };

    let on_input_change = {
        let new_app = new_app.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            new_app.set(input.value());
        })
    };

    let on_add_common = {
        let gldf = gldf.clone();
        let applications = applications.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let value = select.value();
            if !value.is_empty() && !applications.contains(&value) {
                gldf.dispatch(GldfAction::AddApplication(value));
            }
            // Reset select
            select.set_value("");
        })
    };

    html! {
        <div class="editor-section applications-editor">
            <h2>{ "Applications" }</h2>
            <p class="section-description">{ "Define where this luminaire can be used" }</p>

            // Current applications list
            <div class="applications-list">
                <h3>{ "Current Applications" }</h3>
                if applications.is_empty() {
                    <p class="empty-message">{ "No applications defined" }</p>
                } else {
                    <div class="tags-container">
                        { for applications.iter().enumerate().map(|(index, app)| {
                            let gldf = gldf.clone();
                            let on_remove = Callback::from(move |_: MouseEvent| {
                                gldf.dispatch(GldfAction::RemoveApplication(index));
                            });
                            html! {
                                <span class="tag application-tag">
                                    { app }
                                    <button
                                        type="button"
                                        class="tag-remove"
                                        onclick={on_remove}
                                        title="Remove application"
                                    >
                                        { "×" }
                                    </button>
                                </span>
                            }
                        })}
                    </div>
                }
            </div>

            // Add common application
            <div class="form-group">
                <label for="common-app">{ "Add Common Application" }</label>
                <select id="common-app" onchange={on_add_common}>
                    <option value="">{ "-- Select application --" }</option>
                    { for COMMON_APPLICATIONS.iter().map(|app| {
                        let disabled = applications.contains(&app.to_string());
                        html! {
                            <option
                                value={*app}
                                disabled={disabled}
                            >
                                { app }
                                if disabled { { " (added)" } }
                            </option>
                        }
                    })}
                </select>
            </div>

            // Add custom application
            <form onsubmit={on_add_custom} class="add-custom-form">
                <div class="form-group">
                    <label for="custom-app">{ "Add Custom Application" }</label>
                    <div class="input-with-button">
                        <input
                            type="text"
                            id="custom-app"
                            value={(*new_app).clone()}
                            oninput={on_input_change}
                            placeholder="Enter custom application"
                        />
                        <button type="submit" class="btn btn-add" disabled={new_app.is_empty()}>
                            { "Add" }
                        </button>
                    </div>
                </div>
            </form>
        </div>
    }
}
