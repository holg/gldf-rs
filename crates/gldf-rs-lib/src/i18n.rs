//! i18n contract for gldf-rs.
//!
//! This module defines only the [`Translator`] trait — the contract
//! that frontends (gldf-rs-wasm, gldf-rs-tui, gldf-rs-cli, gldf-rs-python)
//! implement to translate the `gldf.*` keys this crate emits. The
//! actual translation data lives in a separate crate
//! (`gldf-rs-i18n`) so this crate stays a pure logic/data crate and
//! doesn't ship JSON locale files.
//!
//! # Why a trait, not an embedded table
//!
//! Keeping data out of the library is deliberate:
//! - Headless callers (the CLI, Python bindings, tests) often don't
//!   need translations; they shouldn't pay for the locale files.
//! - Frontends may already have their own translation backend (the
//!   WASM viewer historically used `eulumdat-i18n`'s `extra` map for
//!   shared keys).
//! - Translation files change shape over time; binding the library's
//!   release cadence to a JSON schema would be friction.
//!
//! # i18n keys this library emits
//!
//! - `gldf.ral.<number>.name` — RAL Classic color name (213 entries,
//!   one per [`crate::ral::RalEntry`]). Default fallbacks: German
//!   (`name_de`) for `de*` locales, English (`name_en`) for everything
//!   else.
//! - Future: validation messages, action error templates, reducer
//!   diagnostics that surface to humans.
//!
//! # Default implementation
//!
//! [`NoOpTranslator`] returns `None` for every key. Useful in tests
//! and as a sentinel when the consumer hasn't wired translations.
//! Callers should treat `None` as "fall back to whatever string the
//! library provides alongside the key" (e.g. `RalEntry::fallback_name`).

/// Translate a key in a target language. Implementors return:
///
/// - `Some(text)` if a translation exists in the requested language
/// - `Some(text)` if only an English fallback is available (the
///   implementor decides — `None` is also acceptable in that case)
/// - `None` if neither the requested language nor any fallback is
///   registered for the key
///
/// `language_code` is a BCP-47 stem (`"de"`, `"en"`, `"fr"`, …). The
/// trait does not enforce a closed list; implementors map unknown
/// codes to English (or whatever default they prefer).
pub trait Translator: Send + Sync {
    /// Look up the translation for `key` in `language_code`.
    fn t(&self, key: &str, language_code: &str) -> Option<String>;
}

/// A no-op [`Translator`] that always returns `None`. Useful as a
/// default when the consumer hasn't installed a real translation
/// backend. The library treats `None` as "use the in-code fallback".
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpTranslator;

impl Translator for NoOpTranslator {
    fn t(&self, _key: &str, _language_code: &str) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_returns_none() {
        let t = NoOpTranslator;
        assert!(t.t("gldf.ral.9016.name", "de").is_none());
        assert!(t.t("any.key.at.all", "en").is_none());
    }

    #[test]
    fn translator_is_object_safe() {
        // Compile-time check: the trait must be usable as a trait
        // object so consumers can store `Arc<dyn Translator>`.
        let _: Box<dyn Translator> = Box::new(NoOpTranslator);
    }
}
