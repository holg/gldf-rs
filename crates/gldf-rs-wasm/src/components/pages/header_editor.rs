//! Header editor — the Edit Mode counterpart to `HeaderDashboardPage`.
//!
//! Six identity fields (author, manufacturer, creation time, application,
//! default language, format version). Each dispatches its dedicated
//! `GldfAction` variant.

use crate::components::edit_util::{event_value, optional_string};
use crate::state::{use_gldf, GldfAction};
use leptos::prelude::*;

#[component]
pub fn HeaderEditorPage() -> impl IntoView {
    let state = use_gldf();

    let on_author = move |e: web_sys::Event| {
        if let Some(v) = event_value(&e) {
            state.update(|s| s.apply(GldfAction::SetAuthor(v)));
        }
    };
    let on_manufacturer = move |e: web_sys::Event| {
        if let Some(v) = event_value(&e) {
            state.update(|s| s.apply(GldfAction::SetManufacturer(v)));
        }
    };
    let on_creation = move |e: web_sys::Event| {
        if let Some(v) = event_value(&e) {
            state.update(|s| s.apply(GldfAction::SetCreationTimeCode(v)));
        }
    };
    let on_app = move |e: web_sys::Event| {
        if let Some(v) = event_value(&e) {
            state.update(|s| s.apply(GldfAction::SetCreatedWithApplication(v)));
        }
    };
    let on_lang = move |e: web_sys::Event| {
        if let Some(v) = event_value(&e) {
            state.update(|s| s.apply(GldfAction::SetDefaultLanguage(optional_string(v))));
        }
    };
    let on_format = move |e: web_sys::Event| {
        if let Some(v) = event_value(&e) {
            state.update(|s| s.apply(GldfAction::SetFormatVersion(v)));
        }
    };

    view! {
        <div class="editor-section header-editor">
            <h2>{ "Header Information" }</h2>

            {move || state.with(|s| {
                let header = &s.product.header;
                let author = header.author.clone();
                let manufacturer = header.manufacturer.clone();
                let creation = header.creation_time_code.clone();
                let app = header.created_with_application.clone();
                let lang = header.default_language.clone().unwrap_or_default();
                let format_v = format!("{:?}", header.format_version);
                view! {
                    <div class="form-group">
                        <label for="author">{ "Author" }</label>
                        <input
                            type="text"
                            id="author"
                            prop:value=author
                            on:change=on_author
                            placeholder="Enter author name"
                        />
                    </div>
                    <div class="form-group">
                        <label for="manufacturer">{ "Manufacturer" }</label>
                        <input
                            type="text"
                            id="manufacturer"
                            prop:value=manufacturer
                            on:change=on_manufacturer
                            placeholder="Enter manufacturer name"
                        />
                    </div>
                    <div class="form-group">
                        <label for="creation-time">{ "Creation Time Code" }</label>
                        <input
                            type="datetime-local"
                            id="creation-time"
                            prop:value=creation
                            on:change=on_creation
                        />
                    </div>
                    <div class="form-group">
                        <label for="created-with">{ "Created With Application" }</label>
                        <input
                            type="text"
                            id="created-with"
                            prop:value=app
                            on:change=on_app
                            placeholder="Enter application name"
                        />
                    </div>
                    <div class="form-group">
                        <label for="default-language">{ "Default Language" }</label>
                        <input
                            type="text"
                            id="default-language"
                            prop:value=lang
                            on:change=on_lang
                            placeholder="e.g., en, de, fr"
                            maxlength="5"
                        />
                    </div>
                    <div class="form-group">
                        <label for="format-version">{ "Format Version" }</label>
                        <input
                            type="text"
                            id="format-version"
                            prop:value=format_v
                            on:change=on_format
                            placeholder="e.g., 1.0.0-rc.3"
                        />
                    </div>
                }
            })}
        </div>
    }
}
