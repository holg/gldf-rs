//! Locale editor components.
//!
//! Two layers:
//! - [`LocaleInput`] — edits a single `Locale { language, value }`. Used as a
//!   building block (e.g. for `Hyperlink`-style fields where each entry is one
//!   language).
//! - [`LocaleFooEditor`] — edits a `LocaleFoo` (a `Vec<Locale>`). Surfaces the
//!   active language from `GldfState` for the compact view, with an expandable
//!   "show all locales" mode for inspect/edit/add/remove across languages.
//!
//! The compact view is what the user sees most of the time: a single input
//! showing the active language's value, with a small badge counting any other
//! locales present. Expanding reveals every locale row plus an "Add language"
//! affordance — that path is the main fidelity win for files like the BIOLUX
//! HCL DL, which carries seven languages on every Description.

use crate::state::use_gldf;
use gldf_rs::gldf::{Locale, LocaleFoo};
use wasm_bindgen::JsCast;
use yew::prelude::*;

/// Properties for [`LocaleInput`] (single-locale editor).
#[derive(Properties, Clone, PartialEq)]
pub struct LocaleInputProps {
    pub label: AttrValue,
    pub value: Locale,
    pub onchange: Callback<Locale>,
    #[prop_or(false)]
    pub required: bool,
}

/// Single-locale editor: language code + value.
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

/// Properties for [`LocaleFooEditor`].
#[derive(Properties, Clone, PartialEq)]
pub struct LocaleFooEditorProps {
    pub label: AttrValue,
    pub value: LocaleFoo,
    pub onchange: Callback<LocaleFoo>,
    /// When `true`, render the value inputs as `<textarea>` instead of `<input>`.
    /// Use this for long-form fields like Description.
    #[prop_or(false)]
    pub multiline: bool,
}

/// Multi-locale editor with active-language compact view + expandable "show all".
#[function_component(LocaleFooEditor)]
pub fn locale_foo_editor(props: &LocaleFooEditorProps) -> Html {
    let gldf = use_gldf();
    let expanded = use_state(|| false);

    let active_lang = gldf.active_language.clone();
    let value = props.value.clone();
    let multiline = props.multiline;

    // Find the index of the active language's locale entry, if present.
    let active_idx = value.locale.iter().position(|l| l.language == active_lang);
    let active_value = active_idx
        .and_then(|i| value.locale.get(i).map(|l| l.value.clone()))
        .unwrap_or_default();

    let other_count = value
        .locale
        .iter()
        .filter(|l| l.language != active_lang)
        .count();

    // Compact-view editor: edit the active language. If the active language
    // isn't present yet, the first edit creates it.
    let on_compact_change = {
        let onchange = props.onchange.clone();
        let value = value.clone();
        let active_lang = active_lang.clone();
        Callback::from(move |e: Event| {
            let target: web_sys::HtmlElement = e.target_unchecked_into();
            let new_text = read_input_value(&target);
            let mut new_value = value.clone();
            match new_value
                .locale
                .iter()
                .position(|l| l.language == active_lang)
            {
                Some(i) => new_value.locale[i].value = new_text,
                None => new_value.locale.push(Locale {
                    language: active_lang.clone(),
                    value: new_text,
                }),
            }
            onchange.emit(new_value);
        })
    };

    let toggle_expanded = {
        let expanded = expanded.clone();
        Callback::from(move |_| expanded.set(!*expanded))
    };

    html! {
        <div class="locale-foo-editor">
            <div class="locale-foo-header">
                <label class="locale-label">
                    { &props.label }
                    <span class="locale-active-tag">{ format!("[{}]", active_lang) }</span>
                </label>
                <button
                    type="button"
                    class="locale-foo-toggle"
                    onclick={toggle_expanded.clone()}
                >
                    {
                        if *expanded {
                            "Hide locales".to_string()
                        } else if other_count == 0 {
                            "Show all".to_string()
                        } else {
                            format!("Show all (+{})", other_count)
                        }
                    }
                </button>
            </div>

            <div class="locale-foo-compact">
                { render_value_input(&active_value, &on_compact_change, multiline, &active_lang) }
            </div>

            if *expanded {
                <div class="locale-foo-expanded">
                    { render_expanded(&value, &props.onchange, multiline) }
                </div>
            }
        </div>
    }
}

fn render_expanded(value: &LocaleFoo, onchange: &Callback<LocaleFoo>, multiline: bool) -> Html {
    let rows: Vec<Html> = value
        .locale
        .iter()
        .enumerate()
        .map(|(idx, locale)| {
            let on_lang = {
                let onchange = onchange.clone();
                let value = value.clone();
                Callback::from(move |e: Event| {
                    let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                    let mut new_value = value.clone();
                    if let Some(l) = new_value.locale.get_mut(idx) {
                        l.language = input.value();
                    }
                    onchange.emit(new_value);
                })
            };
            let on_value = {
                let onchange = onchange.clone();
                let value = value.clone();
                Callback::from(move |e: Event| {
                    let target: web_sys::HtmlElement = e.target_unchecked_into();
                    let new_text = read_input_value(&target);
                    let mut new_value = value.clone();
                    if let Some(l) = new_value.locale.get_mut(idx) {
                        l.value = new_text;
                    }
                    onchange.emit(new_value);
                })
            };
            let on_remove = {
                let onchange = onchange.clone();
                let value = value.clone();
                Callback::from(move |_| {
                    let mut new_value = value.clone();
                    if idx < new_value.locale.len() {
                        new_value.locale.remove(idx);
                        onchange.emit(new_value);
                    }
                })
            };

            html! {
                <div class="locale-foo-row">
                    <input
                        type="text"
                        class="locale-language"
                        value={locale.language.clone()}
                        onchange={on_lang}
                        maxlength="5"
                        placeholder="lang"
                    />
                    { render_value_input(&locale.value, &on_value, multiline, &locale.language) }
                    <button
                        type="button"
                        class="locale-foo-remove"
                        title="Remove this locale"
                        onclick={on_remove}
                    >
                        { "✕" }
                    </button>
                </div>
            }
        })
        .collect();

    let on_add = {
        let onchange = onchange.clone();
        let value = value.clone();
        Callback::from(move |_| {
            let mut new_value = value.clone();
            new_value.locale.push(Locale::default());
            onchange.emit(new_value);
        })
    };

    html! {
        <>
            { for rows }
            <div class="locale-foo-add">
                <button type="button" onclick={on_add}>{ "+ Add language" }</button>
            </div>
        </>
    }
}

/// Render either an `<input>` or a `<textarea>` depending on `multiline`.
fn render_value_input(
    current: &str,
    onchange: &Callback<Event>,
    multiline: bool,
    lang: &str,
) -> Html {
    let placeholder = if lang.is_empty() {
        "Value".to_string()
    } else {
        format!("Value ({})", lang)
    };
    if multiline {
        html! {
            <textarea
                class="locale-value locale-value-multiline"
                value={current.to_string()}
                onchange={onchange.clone()}
                placeholder={placeholder}
                rows="4"
            />
        }
    } else {
        html! {
            <input
                type="text"
                class="locale-value"
                value={current.to_string()}
                onchange={onchange.clone()}
                placeholder={placeholder}
            />
        }
    }
}

/// Read the string value out of either an `<input>` or `<textarea>` event target.
fn read_input_value(target: &web_sys::HtmlElement) -> String {
    if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
        return input.value();
    }
    if let Some(textarea) = target.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        return textarea.value();
    }
    String::new()
}

/// Properties for [`LocaleFooView`] — read-only display.
#[derive(Properties, Clone, PartialEq)]
pub struct LocaleFooViewProps {
    pub value: LocaleFoo,
    /// Optional label rendered above the value. Skipped when empty.
    #[prop_or_default]
    pub label: AttrValue,
    /// Reserved for future styling tweaks (e.g. compact mode for tight cards).
    #[prop_or(false)]
    pub compact: bool,
}

/// Read-only view of a `LocaleFoo`.
///
/// Renders only the value for the currently active language (from
/// [`crate::state::GldfState::active_language`]) — clean text, no language
/// tag, no "show all" toggle. The user picks their language once at file
/// load via the inline banner; once chosen, the viewer should stay quiet.
///
/// If the active language has no entry, falls back to the first available
/// locale silently — a missing translation shouldn't make the field appear
/// empty or broken.
#[function_component(LocaleFooView)]
pub fn locale_foo_view(props: &LocaleFooViewProps) -> Html {
    let gldf = use_gldf();
    let active_lang = gldf.active_language.clone();

    if props.value.locale.is_empty() {
        return html! {
            <div class="locale-foo-view empty">
                if !props.label.is_empty() {
                    <span class="locale-label">{ &props.label }</span>
                }
                <span class="locale-empty">{ "—" }</span>
            </div>
        };
    }

    let displayed = props
        .value
        .locale
        .iter()
        .find(|l| l.language == active_lang)
        .or_else(|| props.value.locale.first())
        .expect("locale list non-empty");

    let class = if props.compact {
        classes!("locale-foo-view", "compact")
    } else {
        classes!("locale-foo-view")
    };

    html! {
        <div class={class}>
            if !props.label.is_empty() {
                <span class="locale-label">{ &props.label }</span>
            }
            <span class="locale-foo-text">{ &displayed.value }</span>
        </div>
    }
}
