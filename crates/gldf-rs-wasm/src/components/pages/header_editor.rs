//! Header editor — the Edit Mode counterpart to `HeaderDashboardPage`.
//!
//! Six identity fields (author, manufacturer, creation time, application,
//! default language, format version). Each dispatches its dedicated
//! `GldfAction` variant.

use crate::components::edit_util::{event_value, optional_string};
use crate::state::{use_gldf, GldfAction};
use leptos::prelude::*;

/// Normalise the value emitted by `<input type="datetime-local">` to a
/// schema-valid `xs:dateTime` (the type GLDF uses for both
/// `GldfCreationTimeCode` and the optional `ProductDataTimeCode`).
///
/// The browser emits `YYYY-MM-DDTHH:MM` (no seconds, no timezone).
/// `xs:dateTime` requires seconds — without them DIALux and other
/// strict consumers reject the document as invalid XML. We append
/// `:00` when the input has the short form. Already-full values
/// (`…:SS`, `…:SS.fff`, `…Z`, `…+02:00`) pass through unchanged.
///
/// The GLDF spec recommends UTC ("…ss.ss … preferable provided in
/// UTC"). We don't auto-append `Z` though — `datetime-local` reports
/// the user's local wall-clock time, not UTC, and quietly tagging it
/// `Z` would shift the value by hours. The conservative choice is to
/// preserve the local naive form and let any tooling that needs a
/// timezone resolve it explicitly.
///
/// Empty input passes through too — the editor preserves the prior
/// behaviour of clearing the field when the user wipes it.
fn normalize_xs_datetime(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    // Already has seconds (or a fractional / timezone suffix)?
    // Look at the substring AFTER the `T` — if it contains a second
    // colon, the value is at the `HH:MM:SS` level or richer.
    if let Some((_, time_part)) = s.split_once('T') {
        // Take only up to a timezone indicator so `2026-06-11T14:30Z`
        // also gets the seconds insertion — that variant doesn't
        // happen via datetime-local but a user typing manually might
        // produce it.
        let time_head = time_part
            .split(['Z', '+', '-'])
            .next()
            .unwrap_or(time_part);
        if time_head.matches(':').count() >= 2 {
            return s.to_string();
        }
        // Insert `:00` BEFORE the timezone suffix if present, else
        // at the end.
        if let Some(tz_at) = time_part.find(['Z', '+', '-']) {
            return format!("{}T{}{}:00{}", s.split_once('T').unwrap().0, &time_part[..tz_at], "", &time_part[tz_at..]);
        }
        return format!("{}:00", s);
    }
    // No `T` at all — return as-is. Strict consumers will still
    // reject this; we don't try to guess what the user meant.
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::normalize_xs_datetime;

    #[test]
    fn appends_seconds_when_missing() {
        assert_eq!(
            normalize_xs_datetime("2026-06-11T14:30"),
            "2026-06-11T14:30:00"
        );
    }

    #[test]
    fn passes_through_when_seconds_present() {
        assert_eq!(
            normalize_xs_datetime("2026-06-11T14:30:00"),
            "2026-06-11T14:30:00"
        );
    }

    #[test]
    fn passes_through_with_trailing_z() {
        assert_eq!(
            normalize_xs_datetime("2026-06-11T14:30:00Z"),
            "2026-06-11T14:30:00Z"
        );
    }

    #[test]
    fn passes_through_with_offset() {
        assert_eq!(
            normalize_xs_datetime("2026-06-11T14:30:00+02:00"),
            "2026-06-11T14:30:00+02:00"
        );
    }

    #[test]
    fn empty_stays_empty() {
        assert_eq!(normalize_xs_datetime(""), "");
        assert_eq!(normalize_xs_datetime("   "), "");
    }

    #[test]
    fn no_separator_returns_as_is() {
        // We don't guess — pass the value through so a future
        // stricter validator can complain loudly instead.
        assert_eq!(normalize_xs_datetime("garbage"), "garbage");
    }
}

#[component]
pub fn HeaderEditorPage() -> impl IntoView {
    let state = use_gldf();

    let on_author = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = event_value(&e) {
            state.update(|s| s.apply(GldfAction::SetAuthor(v)));
        }
    };
    let on_manufacturer = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = event_value(&e) {
            state.update(|s| s.apply(GldfAction::SetManufacturer(v)));
        }
    };
    let on_creation = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = event_value(&e) {
            // <input type="datetime-local"> emits `YYYY-MM-DDTHH:MM`
            // — seconds are omitted. The GLDF XSD types
            // GldfCreationTimeCode as xs:dateTime, which REQUIRES
            // seconds (regex `…THH:MM:SS(\.ffff)?(Z|±HH:MM)?`).
            // DIALux's importer enforces this strictly: an edited
            // header with `2026-06-11T14:30` (no seconds) parses as
            // invalid and the whole GLDF is rejected with
            // "Luminaire Data could not be imported". Append `:00`
            // when the input is short. Already-full values
            // (`…:SS` or `…:SS.fff` or `…Z`) pass through unchanged.
            let normalized = normalize_xs_datetime(&v);
            state.update(|s| s.apply(GldfAction::SetCreationTimeCode(normalized)));
        }
    };
    let on_app = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = event_value(&e) {
            state.update(|s| s.apply(GldfAction::SetCreatedWithApplication(v)));
        }
    };
    let on_lang = move |e: crate::components::edit_util::EditEvent| {
        if let Some(v) = event_value(&e) {
            state.update(|s| s.apply(GldfAction::SetDefaultLanguage(optional_string(v))));
        }
    };
    let on_format = move |e: crate::components::edit_util::EditEvent| {
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
