//! Translation table for `Electrical/LightDistribution` enum values.
//!
//! Canonical values come from `gldf-1.0.0-rc-3.xsd` (lines 4670–4692). The
//! 15 entries below are the literal `xs:enumeration` strings the schema
//! accepts. Translations cover the supported UI languages from
//! [`crate::state::SUPPORTED_UI_LANGUAGES`]; English is canonical and
//! therefore never needs a separate row.
//!
//! Use [`display_label`] to render an enum value in the active UI language.
//! Use [`canonical_values`] to drive the editor dropdown — the option text
//! comes from `display_label`, while `value=` stays the canonical XSD
//! string so saves remain schema-valid in any UI language.

/// The 15 canonical XSD enum values, in spec order.
pub const CANONICAL_VALUES: &[&str] = &[
    "Laterally symmetrical narrow",
    "Laterally symmetrical medium",
    "Laterally symmetrical wide",
    "Symmetrical in each quadrant",
    "Symmetric about 0-180 plane",
    "Symmetric about 90-270 plane",
    "Asymmetrical",
    "Asymmetrical flood",
    "Asymmetrical wide flood",
    "Diffuse half spherical",
    "Diffuse full spherical",
    "Direct",
    "Indirect",
    "Direct indirect",
    "Other",
];

/// Borrow the canonical list (used by the editor dropdown).
pub fn canonical_values() -> &'static [&'static str] {
    CANONICAL_VALUES
}

/// Resolve the human-readable label for a canonical XSD value in the
/// chosen UI language. Falls back to the canonical (English) form when the
/// language code isn't in the supported set or has no entry for this
/// particular value.
pub fn display_label(canonical: &str, ui_language: &str) -> String {
    if ui_language == "en" {
        return canonical.to_string();
    }
    let table = match ui_language {
        "de" => DE,
        "es" => ES,
        "fr" => FR,
        "it" => IT,
        "ru" => RU,
        "zh" => ZH,
        _ => return canonical.to_string(),
    };
    for (k, v) in table {
        if *k == canonical {
            return v.to_string();
        }
    }
    canonical.to_string()
}

// Translation tables. Each entry: (canonical_xsd_value, localised_label).
// Order matches CANONICAL_VALUES so they're easy to maintain side-by-side.

const DE: &[(&str, &str)] = &[
    ("Laterally symmetrical narrow", "Lateral symmetrisch eng"),
    ("Laterally symmetrical medium", "Lateral symmetrisch mittel"),
    ("Laterally symmetrical wide", "Lateral symmetrisch breit"),
    (
        "Symmetrical in each quadrant",
        "In jedem Quadranten symmetrisch",
    ),
    (
        "Symmetric about 0-180 plane",
        "Symmetrisch zur 0–180°-Ebene",
    ),
    (
        "Symmetric about 90-270 plane",
        "Symmetrisch zur 90–270°-Ebene",
    ),
    ("Asymmetrical", "Asymmetrisch"),
    ("Asymmetrical flood", "Asymmetrisch breitstrahlend"),
    (
        "Asymmetrical wide flood",
        "Asymmetrisch sehr breitstrahlend",
    ),
    ("Diffuse half spherical", "Diffus halbkugelförmig"),
    ("Diffuse full spherical", "Diffus kugelförmig"),
    ("Direct", "Direkt"),
    ("Indirect", "Indirekt"),
    ("Direct indirect", "Direkt/Indirekt"),
    ("Other", "Andere"),
];

const ES: &[(&str, &str)] = &[
    (
        "Laterally symmetrical narrow",
        "Lateralmente simétrico estrecho",
    ),
    (
        "Laterally symmetrical medium",
        "Lateralmente simétrico medio",
    ),
    (
        "Laterally symmetrical wide",
        "Lateralmente simétrico amplio",
    ),
    (
        "Symmetrical in each quadrant",
        "Simétrico en cada cuadrante",
    ),
    (
        "Symmetric about 0-180 plane",
        "Simétrico respecto al plano 0–180°",
    ),
    (
        "Symmetric about 90-270 plane",
        "Simétrico respecto al plano 90–270°",
    ),
    ("Asymmetrical", "Asimétrico"),
    ("Asymmetrical flood", "Asimétrico de inundación"),
    ("Asymmetrical wide flood", "Asimétrico de inundación amplia"),
    ("Diffuse half spherical", "Difuso semiesférico"),
    ("Diffuse full spherical", "Difuso esférico"),
    ("Direct", "Directo"),
    ("Indirect", "Indirecto"),
    ("Direct indirect", "Directo/Indirecto"),
    ("Other", "Otro"),
];

const FR: &[(&str, &str)] = &[
    (
        "Laterally symmetrical narrow",
        "Latéralement symétrique étroit",
    ),
    (
        "Laterally symmetrical medium",
        "Latéralement symétrique moyen",
    ),
    (
        "Laterally symmetrical wide",
        "Latéralement symétrique large",
    ),
    (
        "Symmetrical in each quadrant",
        "Symétrique dans chaque quadrant",
    ),
    ("Symmetric about 0-180 plane", "Symétrique au plan 0–180°"),
    ("Symmetric about 90-270 plane", "Symétrique au plan 90–270°"),
    ("Asymmetrical", "Asymétrique"),
    ("Asymmetrical flood", "Asymétrique inondation"),
    ("Asymmetrical wide flood", "Asymétrique inondation large"),
    ("Diffuse half spherical", "Diffus hémisphérique"),
    ("Diffuse full spherical", "Diffus sphérique"),
    ("Direct", "Direct"),
    ("Indirect", "Indirect"),
    ("Direct indirect", "Direct/Indirect"),
    ("Other", "Autre"),
];

const IT: &[(&str, &str)] = &[
    (
        "Laterally symmetrical narrow",
        "Lateralmente simmetrico stretto",
    ),
    (
        "Laterally symmetrical medium",
        "Lateralmente simmetrico medio",
    ),
    (
        "Laterally symmetrical wide",
        "Lateralmente simmetrico ampio",
    ),
    (
        "Symmetrical in each quadrant",
        "Simmetrico in ogni quadrante",
    ),
    (
        "Symmetric about 0-180 plane",
        "Simmetrico rispetto al piano 0–180°",
    ),
    (
        "Symmetric about 90-270 plane",
        "Simmetrico rispetto al piano 90–270°",
    ),
    ("Asymmetrical", "Asimmetrico"),
    ("Asymmetrical flood", "Asimmetrico ad ampio fascio"),
    ("Asymmetrical wide flood", "Asimmetrico ad ampissimo fascio"),
    ("Diffuse half spherical", "Diffuso semisferico"),
    ("Diffuse full spherical", "Diffuso sferico"),
    ("Direct", "Diretto"),
    ("Indirect", "Indiretto"),
    ("Direct indirect", "Diretto/Indiretto"),
    ("Other", "Altro"),
];

const RU: &[(&str, &str)] = &[
    (
        "Laterally symmetrical narrow",
        "Латерально-симметричный узкий",
    ),
    (
        "Laterally symmetrical medium",
        "Латерально-симметричный средний",
    ),
    (
        "Laterally symmetrical wide",
        "Латерально-симметричный широкий",
    ),
    (
        "Symmetrical in each quadrant",
        "Симметричный в каждом квадранте",
    ),
    (
        "Symmetric about 0-180 plane",
        "Симметричный относительно плоскости 0–180°",
    ),
    (
        "Symmetric about 90-270 plane",
        "Симметричный относительно плоскости 90–270°",
    ),
    ("Asymmetrical", "Асимметричный"),
    ("Asymmetrical flood", "Асимметричный заливающий"),
    ("Asymmetrical wide flood", "Асимметричный широкозаливающий"),
    ("Diffuse half spherical", "Диффузный полусферический"),
    ("Diffuse full spherical", "Диффузный сферический"),
    ("Direct", "Прямой"),
    ("Indirect", "Непрямой"),
    ("Direct indirect", "Прямой/Непрямой"),
    ("Other", "Другое"),
];

const ZH: &[(&str, &str)] = &[
    ("Laterally symmetrical narrow", "横向对称(窄)"),
    ("Laterally symmetrical medium", "横向对称(中)"),
    ("Laterally symmetrical wide", "横向对称(宽)"),
    ("Symmetrical in each quadrant", "各象限对称"),
    ("Symmetric about 0-180 plane", "关于0–180°平面对称"),
    ("Symmetric about 90-270 plane", "关于90–270°平面对称"),
    ("Asymmetrical", "非对称"),
    ("Asymmetrical flood", "非对称泛光"),
    ("Asymmetrical wide flood", "非对称广角泛光"),
    ("Diffuse half spherical", "漫射半球形"),
    ("Diffuse full spherical", "漫射全球形"),
    ("Direct", "直接照射"),
    ("Indirect", "间接照射"),
    ("Direct indirect", "直接/间接照射"),
    ("Other", "其他"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_returns_canonical_unchanged() {
        assert_eq!(display_label("Direct", "en"), "Direct");
        assert_eq!(display_label("Asymmetrical", "en"), "Asymmetrical");
    }

    #[test]
    fn unknown_language_falls_back_to_canonical() {
        assert_eq!(display_label("Direct", "ja"), "Direct");
    }

    #[test]
    fn unknown_value_returns_input_unchanged() {
        assert_eq!(display_label("not-a-valid-enum", "de"), "not-a-valid-enum");
    }

    #[test]
    fn known_translations() {
        assert_eq!(display_label("Direct", "de"), "Direkt");
        assert_eq!(display_label("Direct", "fr"), "Direct");
        assert_eq!(display_label("Direct", "zh"), "直接照射");
        assert_eq!(display_label("Asymmetrical", "ru"), "Асимметричный");
    }

    #[test]
    fn every_language_covers_every_canonical_value() {
        // The intent is "no canonical value is missing from a language
        // table". A few legitimate cross-language collisions exist (e.g.
        // French "Direct" == English "Direct"), so the check is now: if a
        // language's `display_label` ever returns the canonical form for
        // a value that *isn't* in the table, that's the bug. We assert by
        // walking the table list directly.
        for (lang, table) in &[
            ("de", DE),
            ("es", ES),
            ("fr", FR),
            ("it", IT),
            ("ru", RU),
            ("zh", ZH),
        ] {
            for canonical in CANONICAL_VALUES {
                let in_table = table.iter().any(|(k, _)| k == canonical);
                assert!(
                    in_table,
                    "language {} missing entry for {:?}",
                    lang, canonical
                );
            }
        }
    }
}
