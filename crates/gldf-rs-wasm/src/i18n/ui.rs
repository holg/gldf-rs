//! Thin wrapper around [`eulumdat_i18n::t`] for the GLDF viewer's UI chrome.
//!
//! Every visible English string the viewer hardcodes (sidebar nav, toolbar
//! buttons, welcome screen, headings, hints, empty-state messages…) is
//! translated by looking up a namespaced key in the family-shared
//! `eulumdat-i18n` `extra` map. Keys are prefixed `gldf.*` so they don't
//! collide with eulumdat-rs's own UI strings.
//!
//! Lookup order (provided by `eulumdat_i18n::t`): target language's `extra`
//! → English's `extra` → the key itself (so a missing translation
//! produces a visible string). For perf, the underlying API loads the
//! locale JSON on every call; in this viewer that's tens of calls per
//! render, sub-millisecond — fine until proven otherwise.

use eulumdat_i18n::{t as ei18n_t, Language};

/// Translate a `gldf.*` key in the active UI language. Falls back to
/// English, then to the key itself.
pub fn t(key: &str, ui_language: &str) -> String {
    let lang = lang_from_code(ui_language);
    ei18n_t(key, lang)
}

/// Map our supported UI-language code to `eulumdat_i18n::Language`.
fn lang_from_code(code: &str) -> Language {
    match code {
        "de" => Language::German,
        "en" => Language::English,
        "es" => Language::Spanish,
        "fr" => Language::French,
        "it" => Language::Italian,
        "ru" => Language::Russian,
        "zh" => Language::Chinese,
        _ => Language::English,
    }
}
