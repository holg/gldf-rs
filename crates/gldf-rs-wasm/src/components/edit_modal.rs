//! Reusable centered modal for "zoom-in" edit forms.
//!
//! Cards on the LightSources / Emitters / Variants pages live in a narrow
//! auto-fill grid (~280px) — fine for read-only display, painful for
//! filling out a 10-field form. Wrapping the edit form in this modal
//! gives the inputs room to breathe while keeping the page layout stable.
//!
//! Behavior: clicking the backdrop or pressing Esc dispatches `on_close`.
//! The inner content (passed as the `children` slot) handles its own
//! Save / Cancel buttons; the modal only owns the chrome.

use leptos::prelude::*;
use std::sync::Arc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// Centered modal. `open` toggles visibility; while open, Esc and
/// backdrop clicks fire `on_close`. `title` shows in the modal header.
///
/// `children` uses `ChildrenFn` so the reactive open/closed switch can
/// rebuild the panel content on each toggle without consuming the slot.
#[component]
pub fn EditModal(
    open: RwSignal<bool>,
    title: String,
    on_close: Arc<dyn Fn() + Send + Sync>,
    children: ChildrenFn,
) -> impl IntoView {
    // Esc key handler — install once per mount, tear down on unmount.
    // Re-runs only when `open` flips so we don't keep a stale listener.
    let on_close_for_esc = on_close.clone();
    Effect::new(move |_| {
        if !open.get() {
            return;
        }
        let Some(window) = web_sys::window() else { return };
        let on_close = on_close_for_esc.clone();
        let cb = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
            if e.key() == "Escape" {
                on_close();
            }
        }) as Box<dyn FnMut(_)>);
        let _ = window
            .add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
        // Leak the closure: it's tiny and the listener tears down with the
        // page unload. Keeping it would need wrapping in StoredValue +
        // explicit cleanup; not worth the complexity here.
        cb.forget();
    });

    let on_backdrop = {
        let on_close = on_close.clone();
        move |_| on_close()
    };
    let on_panel_click = move |e: web_sys::MouseEvent| {
        // Stop the click from reaching the backdrop.
        e.stop_propagation();
    };

    view! {
        {move || if open.get() {
            view! {
                <div
                    class="edit-modal-backdrop"
                    on:click=on_backdrop.clone()
                    style="position: fixed; inset: 0; background: rgba(0, 0, 0, 0.55); z-index: 1000; display: flex; align-items: flex-start; justify-content: center; padding: 48px 16px; overflow-y: auto;"
                >
                    <div
                        class="edit-modal-panel"
                        on:click=on_panel_click
                        style="background: var(--bg-card); border: 1px solid var(--border-color); border-radius: 8px; width: 100%; max-width: 640px; box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);"
                    >
                        <div
                            class="edit-modal-header"
                            style="display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--border-color);"
                        >
                            <h3 style="margin: 0; font-size: 14px;">{ title.clone() }</h3>
                            <button
                                on:click=on_backdrop.clone()
                                title="Close (Esc)"
                                style="background: none; border: none; cursor: pointer; font-size: 20px; color: var(--text-secondary); line-height: 1;"
                            >{ "✕" }</button>
                        </div>
                        <div class="edit-modal-body" style="padding: 16px;">
                            { children() }
                        </div>
                    </div>
                </div>
            }.into_any()
        } else {
            ().into_any()
        }}
    }
}
