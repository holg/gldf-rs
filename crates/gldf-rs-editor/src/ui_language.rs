//! UI translation language — picked from the viewer's chrome, independent
//! of the document's locale. Persisted via a frontend-supplied store so
//! the state crate stays free of browser- and filesystem-specific calls.

use std::sync::Arc;

/// Storage adapter for the user's UI language preference. The WASM
/// frontend persists to `localStorage`; the TUI persists to a config file
/// under `$XDG_CONFIG_HOME`; tests use [`NullLangStore`] which does
/// nothing.
pub trait UiLanguageStore: Send + Sync {
    /// Read the last-chosen UI language, if any.
    fn load(&self) -> Option<String>;
    /// Persist the chosen UI language.
    fn store(&self, lang: &str);
}

/// No-op store. Useful in unit tests where the language pick doesn't
/// need to survive across reducer runs.
pub struct NullLangStore;

impl UiLanguageStore for NullLangStore {
    fn load(&self) -> Option<String> {
        None
    }
    fn store(&self, _lang: &str) {}
}

/// Canonical list of supported UI translation codes.
pub const SUPPORTED_UI_LANGUAGES: &[&str] = &["de", "en", "es", "fr", "it", "ru", "zh-CN"];

/// Native-script display name for each supported code.
pub fn ui_language_native_name(code: &str) -> Option<&'static str> {
    match code {
        "de" => Some("Deutsch"),
        "en" => Some("English"),
        "es" => Some("Español"),
        "fr" => Some("Français"),
        "it" => Some("Italiano"),
        "ru" => Some("Русский"),
        "zh-CN" => Some("中文 (简体)"),
        _ => None,
    }
}

/// Map common locale variants to the canonical SUPPORTED_UI_LANGUAGES code.
///
/// Examples:
/// - `"en-US"` → `"en"`
/// - `"de-AT"` → `"de"`
/// - `"zh"` / `"cn"` / `"zh-Hans"` / `"zh-CN"` / `"zh-TW"` → `"zh-CN"`
pub fn normalise_ui_language(input: &str) -> String {
    let lower = input.to_lowercase();
    if lower == "cn" || lower.starts_with("zh") {
        return "zh-CN".to_string();
    }
    let primary = lower.split(['-', '_']).next().unwrap_or("en");
    primary.to_string()
}

/// Resolve the initial UI language at startup: stored value first
/// (validated against [`SUPPORTED_UI_LANGUAGES`]), else `"en"`.
pub fn initial_ui_language(store: &dyn UiLanguageStore) -> String {
    if let Some(stored) = store.load() {
        let normalised = normalise_ui_language(&stored);
        if SUPPORTED_UI_LANGUAGES.iter().any(|l| **l == normalised) {
            return normalised;
        }
    }
    "en".to_string()
}

/// Convenience alias for the shared trait object passed into `GldfState`.
pub type LangStore = Arc<dyn UiLanguageStore>;
