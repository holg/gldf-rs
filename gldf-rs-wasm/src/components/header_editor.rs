//! Header editor component for GLDF files

use yew::prelude::*;
use crate::state::{use_gldf, GldfAction};

/// Header editor component
#[function_component(HeaderEditor)]
pub fn header_editor() -> Html {
    let gldf = use_gldf();
    let header = &gldf.product.header;

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
            gldf.dispatch(GldfAction::SetDefaultLanguage(
                if value.is_empty() { None } else { Some(value) }
            ));
        })
    };

    let on_major_change = {
        let gldf = gldf.clone();
        let format_version = header.format_version.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let major: i32 = input.value().parse().unwrap_or(1);
            gldf.dispatch(GldfAction::SetFormatVersion {
                major,
                minor: format_version.minor,
                pre_release: format_version.pre_release,
            });
        })
    };

    let on_minor_change = {
        let gldf = gldf.clone();
        let format_version = header.format_version.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let minor: i32 = input.value().parse().unwrap_or(0);
            gldf.dispatch(GldfAction::SetFormatVersion {
                major: format_version.major,
                minor,
                pre_release: format_version.pre_release,
            });
        })
    };

    let on_pre_release_change = {
        let gldf = gldf.clone();
        let format_version = header.format_version.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let pre_release: i32 = input.value().parse().unwrap_or(0);
            gldf.dispatch(GldfAction::SetFormatVersion {
                major: format_version.major,
                minor: format_version.minor,
                pre_release,
            });
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

            <fieldset class="version-fieldset">
                <legend>{ "Format Version" }</legend>
                <div class="version-inputs">
                    <div class="form-group inline">
                        <label for="version-major">{ "Major" }</label>
                        <input
                            type="number"
                            id="version-major"
                            value={header.format_version.major.to_string()}
                            onchange={on_major_change}
                            min="1"
                        />
                    </div>
                    <div class="form-group inline">
                        <label for="version-minor">{ "Minor" }</label>
                        <input
                            type="number"
                            id="version-minor"
                            value={header.format_version.minor.to_string()}
                            onchange={on_minor_change}
                            min="0"
                        />
                    </div>
                    <div class="form-group inline">
                        <label for="version-pre-release">{ "Pre-release" }</label>
                        <input
                            type="number"
                            id="version-pre-release"
                            value={header.format_version.pre_release.to_string()}
                            onchange={on_pre_release_change}
                            min="0"
                        />
                    </div>
                </div>
            </fieldset>
        </div>
    }
}
