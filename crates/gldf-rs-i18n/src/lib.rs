//! Translation store for `gldf-rs`.
//!
//! Implements [`gldf_rs::i18n::Translator`] for a static, JSON-backed
//! locale table covering `de`, `en`, `es`, `fr`, `it`, `ru`, `zh`.
//! Translations are bundled at compile time via `include_str!`, so
//! the WASM bundle contains every language with no runtime IO.
//!
//! # Keys
//!
//! Two kinds of keys are bundled:
//!
//! - **UI chrome** (`gldf.nav.*`, `gldf.toolbar.*`, `gldf.sidebar.*`,
//!   `gldf.welcome.*`, `gldf.app.*`) — 42 keys per locale, migrated
//!   from `eulumdat-i18n`'s `extra` map (where these used to live)
//! - **RAL Classic color names** (`gldf.ral.<number>.name`) — all 213
//!   entries in German and English; other locales fall back to English
//!
//! # Lookup order
//!
//! 1. Requested language's table
//! 2. English fallback (matches the GLDF schema's `xml:lang` policy)
//! 3. `None` — the caller then uses whatever in-code fallback the
//!    library provides (e.g. [`gldf_rs::ral::RalEntry::fallback_name`])
//!
//! # Usage
//!
//! ```
//! use gldf_rs::i18n::Translator;
//! use gldf_rs_i18n::GldfStaticTranslator;
//!
//! let t = GldfStaticTranslator::new();
//! assert_eq!(t.t("gldf.nav.overview", "de"), Some("Übersicht".to_string()));
//! // Unknown key → None (callers fall back to their own string)
//! assert_eq!(t.t("gldf.totally.made.up", "en"), None);
//! ```

use gldf_rs::i18n::Translator;
use std::collections::HashMap;

const LOCALE_EN: &str = include_str!("../locales/en.json");
const LOCALE_DE: &str = include_str!("../locales/de.json");
const LOCALE_ES: &str = include_str!("../locales/es.json");
const LOCALE_FR: &str = include_str!("../locales/fr.json");
const LOCALE_IT: &str = include_str!("../locales/it.json");
const LOCALE_RU: &str = include_str!("../locales/ru.json");
const LOCALE_ZH: &str = include_str!("../locales/zh.json");

/// One language's flat key → value map, parsed once from the bundled JSON.
type LocaleMap = HashMap<String, String>;

/// Static translation store. Parses each locale's JSON file once on
/// construction and serves lookups from an in-memory `HashMap`.
///
/// Cheap to clone (the maps are `Arc`-backed conceptually; the
/// per-locale parse runs at construction time, not per call).
pub struct GldfStaticTranslator {
    en: LocaleMap,
    de: LocaleMap,
    es: LocaleMap,
    fr: LocaleMap,
    it: LocaleMap,
    ru: LocaleMap,
    zh: LocaleMap,
}

impl GldfStaticTranslator {
    /// Construct the translator, parsing every bundled locale.
    ///
    /// # Panics
    /// Panics if any bundled JSON file fails to parse. That can only
    /// happen if the build is broken — the JSON files are part of the
    /// crate's source tree and the bundled-tests verify their shape.
    pub fn new() -> Self {
        Self {
            en: parse_locale("en.json", LOCALE_EN),
            de: parse_locale("de.json", LOCALE_DE),
            es: parse_locale("es.json", LOCALE_ES),
            fr: parse_locale("fr.json", LOCALE_FR),
            it: parse_locale("it.json", LOCALE_IT),
            ru: parse_locale("ru.json", LOCALE_RU),
            zh: parse_locale("zh.json", LOCALE_ZH),
        }
    }

    /// Pick the locale map for a BCP-47 stem. Falls through to
    /// English when the code is unrecognized.
    fn locale_for(&self, code: &str) -> &LocaleMap {
        let stem = code.split('-').next().unwrap_or("");
        match stem {
            "de" => &self.de,
            "es" => &self.es,
            "fr" => &self.fr,
            "it" => &self.it,
            "ru" => &self.ru,
            "zh" => &self.zh,
            _ => &self.en,
        }
    }
}

impl Default for GldfStaticTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl Translator for GldfStaticTranslator {
    fn t(&self, key: &str, language_code: &str) -> Option<String> {
        if let Some(v) = self.locale_for(language_code).get(key) {
            return Some(v.clone());
        }
        // English fallback when the target locale doesn't have the key.
        self.en.get(key).cloned()
    }
}

fn parse_locale(path: &str, json: &str) -> LocaleMap {
    serde_json::from_str(json).unwrap_or_else(|e| {
        panic!("bundled locale file '{}' failed to parse: {}", path, e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_all_locales() {
        let t = GldfStaticTranslator::new();
        // Every locale parsed and has at least the migrated UI chrome.
        assert!(t.en.contains_key("gldf.nav.overview"));
        assert!(t.de.contains_key("gldf.nav.overview"));
        assert!(t.es.contains_key("gldf.nav.overview"));
        assert!(t.fr.contains_key("gldf.nav.overview"));
        assert!(t.it.contains_key("gldf.nav.overview"));
        assert!(t.ru.contains_key("gldf.nav.overview"));
        assert!(t.zh.contains_key("gldf.nav.overview"));
    }

    #[test]
    fn nav_overview_german() {
        let t = GldfStaticTranslator::new();
        assert_eq!(
            t.t("gldf.nav.overview", "de"),
            Some("Übersicht".to_string())
        );
    }

    #[test]
    fn nav_overview_english_fallback_for_unknown_lang() {
        let t = GldfStaticTranslator::new();
        // Unknown language code → English fallback
        assert_eq!(
            t.t("gldf.nav.overview", "ja"),
            Some("Overview".to_string())
        );
    }

    #[test]
    fn ral_9016_traffic_white_en() {
        let t = GldfStaticTranslator::new();
        assert_eq!(
            t.t("gldf.ral.9016.name", "en"),
            Some("Traffic white".to_string())
        );
    }

    #[test]
    fn ral_9016_verkehrsweiss_de() {
        let t = GldfStaticTranslator::new();
        assert_eq!(
            t.t("gldf.ral.9016.name", "de"),
            Some("Verkehrsweiß".to_string())
        );
    }

    #[test]
    fn ral_falls_back_to_en_for_other_locales() {
        let t = GldfStaticTranslator::new();
        // We didn't translate RAL into Italian; should fall back to en
        assert_eq!(
            t.t("gldf.ral.9016.name", "it"),
            Some("Traffic white".to_string())
        );
    }

    #[test]
    fn unknown_key_returns_none() {
        let t = GldfStaticTranslator::new();
        assert!(t.t("gldf.never.heard.of.this", "en").is_none());
        assert!(t.t("gldf.never.heard.of.this", "de").is_none());
    }

    #[test]
    fn bcp47_stem_matching() {
        let t = GldfStaticTranslator::new();
        // de-AT should pick up the German table
        assert_eq!(t.t("gldf.nav.overview", "de-AT"), Some("Übersicht".to_string()));
        assert_eq!(t.t("gldf.nav.overview", "de-CH"), Some("Übersicht".to_string()));
    }

    #[test]
    fn locales_have_no_duplicate_keys() {
        // serde_json::from_str silently keeps the last duplicate. This
        // test detects authoring mistakes where the same key shows up
        // twice in a file.
        for (label, json) in &[
            ("en", LOCALE_EN),
            ("de", LOCALE_DE),
            ("es", LOCALE_ES),
            ("fr", LOCALE_FR),
            ("it", LOCALE_IT),
            ("ru", LOCALE_RU),
            ("zh", LOCALE_ZH),
        ] {
            let mut seen = std::collections::HashSet::new();
            for line in json.lines() {
                if let Some(colon_pos) = line.find('"') {
                    let after = &line[colon_pos + 1..];
                    if let Some(quote_end) = after.find('"') {
                        let key = &after[..quote_end];
                        if key.starts_with("gldf.") {
                            assert!(
                                seen.insert(key.to_string()),
                                "{}.json has duplicate key '{}'",
                                label,
                                key
                            );
                        }
                    }
                }
            }
        }
    }
}
