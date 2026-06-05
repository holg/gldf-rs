//! Translation entry point for the GLDF viewer's UI chrome.
//!
//! Every visible English string the viewer hardcodes (sidebar nav, toolbar
//! buttons, welcome screen, headings, hints, empty-state messages…) is
//! translated by looking up a namespaced `gldf.*` key. The translation
//! store lives in the `gldf-rs-i18n` crate (which owns the JSON locale
//! files and exposes a `gldf_rs::i18n::Translator` impl); this module
//! is the thin facade the viewer's pages call.
//!
//! Lookup order:
//! 1. The target language's `gldf-rs-i18n` table
//! 2. English fallback (handled inside `GldfStaticTranslator`)
//! 3. The key itself (so a missing translation produces a visible
//!    string that the user can flag)
//!
//! The translator is cached in a `OnceLock` so locale-JSON parsing
//! happens exactly once per page load.

use gldf_rs::i18n::Translator;
use gldf_rs_i18n::GldfStaticTranslator;
use std::sync::OnceLock;

/// Shared static translator. Cheap to call; the underlying JSON parse
/// runs on the first lookup and is reused for every subsequent call.
fn translator() -> &'static GldfStaticTranslator {
    static TRANSLATOR: OnceLock<GldfStaticTranslator> = OnceLock::new();
    TRANSLATOR.get_or_init(GldfStaticTranslator::new)
}

/// Translate a `gldf.*` key in the active UI language. Falls back to
/// English, then to the key itself if nothing is registered.
pub fn t(key: &str, ui_language: &str) -> String {
    translator()
        .t(key, ui_language)
        .unwrap_or_else(|| key.to_string())
}
