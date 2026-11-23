//! Multi-locale text editor.
//!
//! Port of `LocaleFooEditor` from the Yew side. Shows an inline input/textarea
//! for the active language's value plus a "Show all" toggle to expand and
//! edit every locale row (language code + value + remove button + add row).
//!
//! Used by Header, Identity, Variant, LightSource, and the Marketing editors
//! — any place a GLDF `LocaleFoo` field needs editing.

use crate::state::use_gldf;
use gldf_rs::gldf::{Locale, LocaleFoo};
use leptos::prelude::*;
use std::sync::Arc;

/// Multi-locale editor. The `on_change` callback receives the full updated
/// `LocaleFoo` whenever the user types into any field, adds a row, or
/// removes a row.
#[component]
pub fn LocaleFooEditor(
    /// Label rendered above the inputs (e.g. "Name", "Description").
    label: &'static str,
    /// Current value.
    value: LocaleFoo,
    /// Called with the full updated `LocaleFoo` on every edit.
    #[prop(into)]
    on_change: Callback<LocaleFoo>,
    /// When true, render values as `<textarea>` instead of `<input>` for
    /// long-form fields (Description, Tender Text).
    #[prop(default = false)]
    multiline: bool,
) -> impl IntoView {
    let state = use_gldf();
    let expanded = RwSignal::new(false);

    // Snapshot the active language into a signal so the closures can react
    // when the user picks a different active language without re-mounting.
    let active_lang = Signal::derive(move || state.with(|s| s.active_language.clone()));

    // Mirror the prop into a local signal so edits flow through one source.
    // Each on:change rewrites the signal and emits via `on_change`.
    let current = RwSignal::new(value);

    // Compact-view (single active-language) editor. If the active language
    // isn't present yet, the first edit appends it.
    let on_change_compact = {
        move |new_text: String| {
            let lang = active_lang.get();
            let mut new_value = current.get();
            match new_value.locale.iter().position(|l| l.language == lang) {
                Some(i) => new_value.locale[i].value = new_text,
                None => new_value.locale.push(Locale {
                    language: lang,
                    value: new_text,
                }),
            }
            current.set(new_value.clone());
            on_change.run(new_value);
        }
    };
    let on_change_compact = Arc::new(on_change_compact);

    let on_compact_event = {
        let on_change_compact = on_change_compact.clone();
        move |e: crate::components::edit_util::EditEvent| {
            if let Some(text) = read_input_value(&e) {
                on_change_compact(text);
            }
        }
    };

    let toggle_expanded = move |_| expanded.update(|v| *v = !*v);

    view! {
        <div class="locale-foo-editor">
            <div class="locale-foo-header">
                <label class="locale-label">
                    { label }
                    <span class="locale-active-tag">{ move || format!("[{}]", active_lang.get()) }</span>
                </label>
                <button
                    type="button"
                    class="locale-foo-toggle"
                    on:click=toggle_expanded
                >
                    {move || {
                        if expanded.get() {
                            "Hide locales".to_string()
                        } else {
                            let other_count = current.with(|v| {
                                let lang = active_lang.get();
                                v.locale.iter().filter(|l| l.language != lang).count()
                            });
                            if other_count == 0 {
                                "Show all".to_string()
                            } else {
                                format!("Show all (+{})", other_count)
                            }
                        }
                    }}
                </button>
            </div>

            <div class="locale-foo-compact">
                {move || {
                    let lang = active_lang.get();
                    let active_value = current.with(|v| {
                        v.locale
                            .iter()
                            .find(|l| l.language == lang)
                            .map(|l| l.value.clone())
                            .unwrap_or_default()
                    });
                    let on_change_compact = on_compact_event.clone();
                    render_value_input(active_value, on_change_compact, multiline, lang)
                }}
            </div>

            {move || if expanded.get() {
                let on_change = on_change;
                view! {
                    <div class="locale-foo-expanded">
                        <ExpandedRows current=current on_change=on_change multiline=multiline />
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}
        </div>
    }
}

#[component]
fn ExpandedRows(
    current: RwSignal<LocaleFoo>,
    #[prop(into)] on_change: Callback<LocaleFoo>,
    multiline: bool,
) -> impl IntoView {
    let rows = move || {
        let snapshot = current.get();
        let total = snapshot.locale.len();
        let on_change = on_change;
        (0..total).map(|idx| {
            let on_change_lang = on_change;
            let on_change_value = on_change;
            let on_change_remove = on_change;

            let on_lang = move |e: crate::components::edit_util::EditEvent| {
                let Some(new_lang) = read_input_value(&e) else { return };
                let mut v = current.get();
                if let Some(l) = v.locale.get_mut(idx) {
                    l.language = new_lang;
                }
                current.set(v.clone());
                on_change_lang.run(v);
            };
            let on_value = move |e: crate::components::edit_util::EditEvent| {
                let Some(text) = read_input_value(&e) else { return };
                let mut v = current.get();
                if let Some(l) = v.locale.get_mut(idx) {
                    l.value = text;
                }
                current.set(v.clone());
                on_change_value.run(v);
            };
            let on_remove = move |_| {
                let mut v = current.get();
                if idx < v.locale.len() {
                    v.locale.remove(idx);
                    current.set(v.clone());
                    on_change_remove.run(v);
                }
            };

            let locale = snapshot.locale[idx].clone();
            let value_input = render_value_input(
                locale.value.clone(),
                on_value,
                multiline,
                locale.language.clone(),
            );
            view! {
                <div class="locale-foo-row" style="display: flex; gap: 6px; align-items: center; margin-bottom: 4px;">
                    <input
                        type="text"
                        class="locale-language"
                        prop:value=locale.language.clone()
                        on:change=on_lang
                        maxlength="5"
                        placeholder="lang"
                        style="width: 60px;"
                    />
                    { value_input }
                    <button
                        type="button"
                        class="locale-foo-remove"
                        title="Remove this locale"
                        on:click=on_remove
                    >
                        { "✕" }
                    </button>
                </div>
            }
        }).collect::<Vec<_>>()
    };

    let on_add = move |_| {
        let mut v = current.get();
        v.locale.push(Locale::default());
        current.set(v.clone());
        on_change.run(v);
    };

    view! {
        <>
            { rows }
            <div class="locale-foo-add" style="margin-top: 6px;">
                <button type="button" on:click=on_add>{ "+ Add language" }</button>
            </div>
        </>
    }
}

/// Render either `<input>` (single line) or `<textarea>` (multiline).
/// Takes any `FnMut(EditEvent)` so callers can hand it their own
/// closure without sharing the captured signals across two siblings.
fn render_value_input<F>(
    current: String,
    on_change: F,
    multiline: bool,
    lang: impl AsRef<str>,
) -> impl IntoView
where
    F: FnMut(crate::components::edit_util::EditEvent) + 'static,
{
    let placeholder = if lang.as_ref().is_empty() {
        "Value".to_string()
    } else {
        format!("Value ({})", lang.as_ref())
    };
    if multiline {
        view! {
            <textarea
                class="locale-value locale-value-multiline"
                prop:value=current
                on:change=on_change
                placeholder=placeholder
                rows="4"
                style="flex: 1; min-width: 0;"
            />
        }
        .into_any()
    } else {
        view! {
            <input
                type="text"
                class="locale-value"
                prop:value=current
                on:change=on_change
                placeholder=placeholder
                style="flex: 1; min-width: 0;"
            />
        }
        .into_any()
    }
}

/// Read the string from either an `<input>` or `<textarea>` event target.
fn read_input_value(e: &crate::components::edit_util::EditEvent) -> Option<String> {
    crate::components::edit_util::event_value(e)
}
