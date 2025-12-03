//! Locale input component for handling localized text fields

use yew::prelude::*;
use gldf_rs::gldf::Locale;

/// Properties for LocaleInput component
#[derive(Properties, Clone, PartialEq)]
pub struct LocaleInputProps {
    /// The label for the input field
    pub label: AttrValue,
    /// The current locale value
    pub value: Locale,
    /// Callback when the value changes
    pub onchange: Callback<Locale>,
    /// Whether the field is required
    #[prop_or(false)]
    pub required: bool,
}

/// A component for editing locale-aware text fields
#[function_component(LocaleInput)]
pub fn locale_input(props: &LocaleInputProps) -> Html {
    let value = props.value.clone();
    let onchange = props.onchange.clone();

    let on_language_change = {
        let value = value.clone();
        let onchange = onchange.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            onchange.emit(Locale {
                language: input.value(),
                value: value.value.clone(),
            });
        })
    };

    let on_value_change = {
        let value = value.clone();
        let onchange = onchange.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            onchange.emit(Locale {
                language: value.language.clone(),
                value: input.value(),
            });
        })
    };

    html! {
        <div class="locale-input">
            <label class="locale-label">{ &props.label }</label>
            <div class="locale-fields">
                <input
                    type="text"
                    class="locale-language"
                    placeholder="Language (e.g., en)"
                    value={value.language.clone()}
                    onchange={on_language_change}
                    maxlength="5"
                />
                <input
                    type="text"
                    class="locale-value"
                    placeholder="Value"
                    value={value.value.clone()}
                    onchange={on_value_change}
                    required={props.required}
                />
            </div>
        </div>
    }
}
