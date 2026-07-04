//! Shared helpers for editor components.
//!
//! Single-source utilities for "read string from input/textarea event" and
//! "read number from input event" — used across every editor so the same
//! boilerplate doesn't drift between files.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// The event type edit handlers receive from `on:input` / `on:change`:
/// `web_sys::Event` in the browser, the blitz typed `InputEvent`
/// natively. Annotate closures with THIS so the same component source
/// compiles for both targets.
#[cfg(target_arch = "wasm32")]
pub type EditEvent = web_sys::Event;
#[cfg(not(target_arch = "wasm32"))]
pub type EditEvent = leptos::tachys::renderer::blitz::typed_events::InputEvent;

/// Drag & drop event alias: `web_sys::DragEvent` in the browser; natively
/// the blitz `GenericEvent` fallback (blitz does not emit drag events, so
/// these listeners attach but never fire — dead drop zones are acceptable).
#[cfg(target_arch = "wasm32")]
pub type DragEvt = web_sys::DragEvent;
#[cfg(not(target_arch = "wasm32"))]
pub type DragEvt = leptos::tachys::renderer::blitz::typed_events::GenericEvent;

/// Click/mouse event alias: `web_sys::MouseEvent` in the browser, the
/// blitz typed `PointerEvent` natively.
#[cfg(target_arch = "wasm32")]
pub type ClickEvt = web_sys::MouseEvent;
#[cfg(not(target_arch = "wasm32"))]
pub type ClickEvt = leptos::tachys::renderer::blitz::typed_events::PointerEvent;

/// Native (blitz) counterpart: `input`/`change` events carry the new
/// value directly in the typed payload — no DOM casting.
#[cfg(not(target_arch = "wasm32"))]
pub use native_impl::*;

#[cfg(not(target_arch = "wasm32"))]
mod native_impl {
    use leptos::tachys::renderer::blitz::typed_events::InputEvent;

    pub fn event_value(e: &InputEvent) -> Option<String> {
        Some(e.event.value.clone())
    }

    pub fn event_f64(e: &InputEvent) -> Option<f64> {
        let s = event_value(e)?;
        if s.is_empty() { None } else { s.parse().ok() }
    }

    pub fn event_i32(e: &InputEvent) -> Option<i32> {
        let s = event_value(e)?;
        if s.is_empty() { None } else { s.parse().ok() }
    }

    /// Checkbox state: blitz reports the toggled value through the input
    /// event's value ("true"/"false" per blitz form-control semantics);
    /// fall back to None when it isn't boolean-shaped.
    pub fn event_checked(e: &InputEvent) -> Option<bool> {
        match e.event.value.as_str() {
            "true" | "on" | "checked" => Some(true),
            "false" | "" => Some(false),
            _ => None,
        }
    }
}

/// Read the string value out of an `Event` whose target is an `<input>` or
/// `<textarea>`. Returns `None` if the target is neither.
#[cfg(target_arch = "wasm32")]
pub fn event_value(e: &web_sys::Event) -> Option<String> {
    let target = e.target()?;
    if let Ok(input) = target.clone().dyn_into::<web_sys::HtmlInputElement>() {
        return Some(input.value());
    }
    if let Ok(textarea) = target.clone().dyn_into::<web_sys::HtmlTextAreaElement>() {
        return Some(textarea.value());
    }
    if let Ok(select) = target.dyn_into::<web_sys::HtmlSelectElement>() {
        return Some(select.value());
    }
    None
}

/// Parse an event's string value as f64. Empty → None.
#[cfg(target_arch = "wasm32")]
pub fn event_f64(e: &web_sys::Event) -> Option<f64> {
    let s = event_value(e)?;
    if s.is_empty() {
        None
    } else {
        s.parse().ok()
    }
}

/// Parse an event's string value as i32. Empty → None.
#[cfg(target_arch = "wasm32")]
pub fn event_i32(e: &web_sys::Event) -> Option<i32> {
    let s = event_value(e)?;
    if s.is_empty() {
        None
    } else {
        s.parse().ok()
    }
}

/// Ask the user to confirm a destructive action. Browser: `window.confirm`.
/// Native: no blocking dialog available in the blitz shell yet — proceed.
#[cfg(target_arch = "wasm32")]
pub fn confirm(msg: &str) -> bool {
    web_sys::window()
        .and_then(|w| w.confirm_with_message(msg).ok())
        .unwrap_or(false)
}

/// Native: proceed without a dialog.
#[cfg(not(target_arch = "wasm32"))]
pub fn confirm(_msg: &str) -> bool {
    true
}

/// Stop a click event from propagating. Native: the blitz typed
/// `PointerEvent` payload has no propagation control — no-op.
#[cfg(target_arch = "wasm32")]
pub fn stop_click_propagation(e: &ClickEvt) {
    e.stop_propagation();
}

/// Native no-op.
#[cfg(not(target_arch = "wasm32"))]
pub fn stop_click_propagation(_e: &ClickEvt) {}

/// Treat the empty string as `None`, otherwise `Some(s)`.
pub fn optional_string(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Read a checkbox's `checked` from an event.
#[cfg(target_arch = "wasm32")]
pub fn event_checked(e: &web_sys::Event) -> Option<bool> {
    let target = e.target()?;
    let input = target.dyn_into::<web_sys::HtmlInputElement>().ok()?;
    Some(input.checked())
}
