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
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
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
    // Esc key handler — install ONCE per modal instance and gate by
    // `open.get_untracked()` inside the closure. Previous version
    // re-ran an Effect on every open flip and called `cb.forget()` in
    // each branch, which leaked one listener per open cycle. Doing it
    // once at mount keeps the listener count constant at 1; the
    // forget() leak is bounded to that single instance and torn down
    // with the page.
    #[cfg(target_arch = "wasm32")]
    {
        let on_close_for_esc = on_close.clone();
        if let Some(window) = web_sys::window() {
            let cb = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
                if e.key() == "Escape" && open.get_untracked() {
                    (on_close_for_esc)();
                }
            }) as Box<dyn FnMut(_)>);
            let _ = window.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            cb.forget();
        }
    }

    let on_backdrop = {
        let on_close = on_close.clone();
        move |_| on_close()
    };
    let on_panel_click = move |e: crate::components::edit_util::ClickEvt| {
        // Stop the click from reaching the backdrop.
        crate::components::edit_util::stop_click_propagation(&e);
    };

    // Swallow drag-and-drop on the backdrop. Without this, a file dropped
    // onto the wizard (or any modal) bubbles past it to the Welcome
    // screen's `on:drop`, which then loads the file as a brand-new GLDF
    // and abandons whatever the user was doing. preventDefault stops the
    // browser's "navigate to file://…" fallback when nothing handles it.
    let on_dnd_swallow = move |e: crate::components::edit_util::DragEvt| {
        #[cfg(target_arch = "wasm32")]
        {
            e.prevent_default();
            e.stop_propagation();
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = e;
    };

    view! {
        {move || if open.get() {
            view! {
                <div
                    class="edit-modal-backdrop"
                    on:click=on_backdrop.clone()
                    on:dragenter=on_dnd_swallow
                    on:dragover=on_dnd_swallow
                    on:drop=on_dnd_swallow
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
