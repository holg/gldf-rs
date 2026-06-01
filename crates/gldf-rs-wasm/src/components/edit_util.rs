//! Shared helpers for editor components.
//!
//! Single-source utilities for "read string from input/textarea event" and
//! "read number from input event" — used across every editor so the same
//! boilerplate doesn't drift between files.

use wasm_bindgen::JsCast;

/// Read the string value out of an `Event` whose target is an `<input>` or
/// `<textarea>`. Returns `None` if the target is neither.
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
pub fn event_f64(e: &web_sys::Event) -> Option<f64> {
    let s = event_value(e)?;
    if s.is_empty() {
        None
    } else {
        s.parse().ok()
    }
}

/// Parse an event's string value as i32. Empty → None.
pub fn event_i32(e: &web_sys::Event) -> Option<i32> {
    let s = event_value(e)?;
    if s.is_empty() {
        None
    } else {
        s.parse().ok()
    }
}

/// Treat the empty string as `None`, otherwise `Some(s)`.
pub fn optional_string(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Read a checkbox's `checked` from an event.
pub fn event_checked(e: &web_sys::Event) -> Option<bool> {
    let target = e.target()?;
    let input = target.dyn_into::<web_sys::HtmlInputElement>().ok()?;
    Some(input.checked())
}
