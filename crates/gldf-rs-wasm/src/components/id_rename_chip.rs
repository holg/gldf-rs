//! Inline rename affordance for entity ids on card headers.
//!
//! In editor mode, the id shown on a LightSource / Emitter card becomes
//! click-to-edit. The user clicks the ✎ to open an inline input + Save /
//! Cancel buttons. On Save the parent's `on_rename(new_id)` callback fires;
//! it's the parent's job to dispatch the appropriate state action
//! (`RenameFixedLightSource`, `RenameChangeableLightSource`,
//! `RenameEmitter`, …) so the lib's cascade keeps the document consistent.
//!
//! The chip is read-only (renders just the id text) when `editable=false`.

use leptos::prelude::*;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

/// Inline click-to-edit id chip.
///
/// `current_id` is the id to display when not editing. `editable` toggles
/// the affordance — when false the chip renders as a plain span.
/// `on_rename` is invoked with the new id string when the user clicks Save
/// (with a non-empty, changed value). Errors (collisions, invalid ids) are
/// surfaced by the state layer via `last_error`, not by this component.
#[component]
pub fn IdRenameChip(
    current_id: String,
    editable: bool,
    on_rename: Arc<dyn Fn(String) + Send + Sync>,
) -> impl IntoView {
    let is_editing = RwSignal::new(false);
    let draft = RwSignal::new(current_id.clone());
    let display_id = current_id.clone();
    let original_id = current_id.clone();

    let on_edit = move |_| {
        // Re-seed the draft each time we enter edit mode so a previous
        // Cancel doesn't leave a stale value lingering.
        draft.set(original_id.clone());
        is_editing.set(true);
    };
    let on_cancel = move |_| {
        is_editing.set(false);
    };
    let original_for_save = display_id.clone();
    let on_save = move |_| {
        let new_id = draft.get();
        let trimmed = new_id.trim();
        if !trimmed.is_empty() && trimmed != original_for_save {
            on_rename(trimmed.to_string());
        }
        is_editing.set(false);
    };
    let on_input = move |e: web_sys::Event| {
        let Some(target) = e.target() else { return };
        let input: HtmlInputElement = match target.dyn_into() {
            Ok(i) => i,
            Err(_) => return,
        };
        draft.set(input.value());
    };

    view! {
        {move || if editable && is_editing.get() {
            view! {
                <span style="display: inline-flex; gap: 4px; align-items: center;">
                    <input
                        type="text"
                        prop:value=move || draft.get()
                        on:input=on_input.clone()
                        style="font-family: var(--font-mono); font-size: 12px; padding: 2px 4px; background: var(--bg-secondary); border: 1px solid var(--accent-blue); border-radius: 4px; color: var(--text-primary); min-width: 120px;"
                    />
                    <button
                        on:click=on_save.clone()
                        style="padding: 2px 6px; font-size: 11px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer;"
                    >{ "Save" }</button>
                    <button
                        on:click=on_cancel.clone()
                        style="padding: 2px 6px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-secondary);"
                    >{ "Cancel" }</button>
                </span>
            }.into_any()
        } else if editable {
            view! {
                <span style="display: inline-flex; gap: 4px; align-items: center;">
                    <span class="card-id">{ display_id.clone() }</span>
                    <button
                        on:click=on_edit.clone()
                        title="Rename"
                        style="padding: 1px 6px; font-size: 11px; background: none; border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; color: var(--text-tertiary);"
                    >{ "✎" }</button>
                </span>
            }.into_any()
        } else {
            view! { <span class="card-id">{ display_id.clone() }</span> }.into_any()
        }}
    }
}
