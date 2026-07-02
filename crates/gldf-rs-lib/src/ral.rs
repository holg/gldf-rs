//! RAL Classic color table — the only color system the GLDF XSD knows.
//!
//! `HousingColor/@ral` is restricted to a 4-digit integer matching
//! `[1-9][0-9]{3}` (1000-9999). RAL Classic covers ~213 colors across
//! 9 hue groups:
//!
//! | Range     | Group               |
//! |-----------|---------------------|
//! | 1000–1037 | Yellow              |
//! | 2000–2013 | Orange              |
//! | 3000–3033 | Red                 |
//! | 4000–4012 | Violet              |
//! | 5000–5026 | Blue                |
//! | 6000–6038 | Green               |
//! | 7000–7048 | Grey                |
//! | 8000–8029 | Brown               |
//! | 9000–9023 | White / black       |
//!
//! Each entry carries:
//! - the RAL number (e.g. 9016)
//! - canonical German name ("Verkehrsweiß")
//! - English name ("Traffic white")
//! - sRGB approximation in 0–255
//!
//! The sRGB values are the published RAL Classic approximations. They
//! are reproductions, not measured spectral matches — RAL's official
//! values are calibrated paint chips, not RGB triples. The triples are
//! good enough for an editor preview swatch and a synthesized `.mtl`
//! default color.
//!
//! # Usage
//!
//! ```
//! use gldf_rs::ral;
//! let entry = ral::by_number(9016).unwrap();
//! assert_eq!(entry.name_en, "Traffic white");
//! assert_eq!(entry.rgb, [241, 241, 234]);
//!
//! // Fuzzy lookup by name (case-insensitive, both languages).
//! let entry = ral::by_name("verkehrsweiß").unwrap();
//! assert_eq!(entry.number, 9016);
//! let entry = ral::by_name("Traffic White").unwrap();
//! assert_eq!(entry.number, 9016);
//! ```

/// One RAL Classic entry.
///
/// # i18n
///
/// `name_de` and `name_en` are the **fallback** strings — the library
/// always has them available even when no translation framework is
/// loaded (CLI, headless tests, Python bindings).
///
/// Frontends that ship a translation table should look up [`Self::i18n_key`]
/// first and fall back to `name_en` / `name_de` only if the key isn't
/// registered. The key format is `gldf.ral.<number>.name`. Translations
/// live in the consumer's locale store (this lib bundles only the two
/// canonical strings — RAL itself only publishes them in German +
/// English).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RalEntry {
    /// 4-digit RAL number (e.g. 9016).
    pub number: i32,
    /// i18n lookup key (`gldf.ral.9016.name`). Pass this to your
    /// translation framework; fall back to [`Self::name_en`] /
    /// [`Self::name_de`] if the framework doesn't recognize it.
    pub i18n_key: &'static str,
    /// Canonical German name (e.g. "Verkehrsweiß"). Set by RAL gGmbH;
    /// fallback for German-speaking consumers and for non-i18n callers.
    pub name_de: &'static str,
    /// Official English translation (e.g. "Traffic white"). Published
    /// alongside the German name by RAL gGmbH; fallback for everyone
    /// else.
    pub name_en: &'static str,
    /// sRGB approximation, channels in 0..=255.
    pub rgb: [u8; 3],
}

impl RalEntry {
    /// Pick the best fallback name for a given language code (BCP-47
    /// stem). Returns the German name for `"de"` and any German
    /// regional variant (`"de-AT"`, `"de-CH"`); returns the English
    /// name for everything else. Used as the fallback path when the
    /// i18n framework has no entry for [`Self::i18n_key`].
    pub fn fallback_name(&self, language_code: &str) -> &'static str {
        if language_code.starts_with("de") {
            self.name_de
        } else {
            self.name_en
        }
    }

    /// sRGB as 0.0..=1.0 floats — convenient for writing into MTL
    /// (`Kd 0.x 0.x 0.x`) or for tinting a UI swatch.
    pub fn rgb_f32(&self) -> [f32; 3] {
        [
            self.rgb[0] as f32 / 255.0,
            self.rgb[1] as f32 / 255.0,
            self.rgb[2] as f32 / 255.0,
        ]
    }

    /// `#RRGGBB` lowercase hex string.
    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.rgb[0], self.rgb[1], self.rgb[2])
    }
}

/// All RAL Classic entries in numeric order.
pub const RAL_CLASSIC: &[RalEntry] = &[
    // --- Yellow (1xxx) ---
    RalEntry { number: 1000, i18n_key: "gldf.ral.1000.name", name_de: "Grünbeige",            name_en: "Green beige",          rgb: [205, 186, 136] },
    RalEntry { number: 1001, i18n_key: "gldf.ral.1001.name", name_de: "Beige",                name_en: "Beige",                rgb: [208, 176, 132] },
    RalEntry { number: 1002, i18n_key: "gldf.ral.1002.name", name_de: "Sandgelb",             name_en: "Sand yellow",          rgb: [210, 170, 109] },
    RalEntry { number: 1003, i18n_key: "gldf.ral.1003.name", name_de: "Signalgelb",           name_en: "Signal yellow",        rgb: [249, 168,   0] },
    RalEntry { number: 1004, i18n_key: "gldf.ral.1004.name", name_de: "Goldgelb",             name_en: "Golden yellow",        rgb: [228, 158,  16] },
    RalEntry { number: 1005, i18n_key: "gldf.ral.1005.name", name_de: "Honiggelb",            name_en: "Honey yellow",         rgb: [203, 142,   0] },
    RalEntry { number: 1006, i18n_key: "gldf.ral.1006.name", name_de: "Maisgelb",             name_en: "Maize yellow",         rgb: [226, 144,   0] },
    RalEntry { number: 1007, i18n_key: "gldf.ral.1007.name", name_de: "Narzissengelb",        name_en: "Daffodil yellow",      rgb: [220, 138,   0] },
    RalEntry { number: 1011, i18n_key: "gldf.ral.1011.name", name_de: "Braunbeige",           name_en: "Brown beige",          rgb: [141, 110,  68] },
    RalEntry { number: 1012, i18n_key: "gldf.ral.1012.name", name_de: "Zitronengelb",         name_en: "Lemon yellow",         rgb: [199, 178,  43] },
    RalEntry { number: 1013, i18n_key: "gldf.ral.1013.name", name_de: "Perlweiß",             name_en: "Oyster white",         rgb: [234, 224, 203] },
    RalEntry { number: 1014, i18n_key: "gldf.ral.1014.name", name_de: "Elfenbein",            name_en: "Ivory",                rgb: [225, 204, 156] },
    RalEntry { number: 1015, i18n_key: "gldf.ral.1015.name", name_de: "Hellelfenbein",        name_en: "Light ivory",          rgb: [230, 214, 178] },
    RalEntry { number: 1016, i18n_key: "gldf.ral.1016.name", name_de: "Schwefelgelb",         name_en: "Sulfur yellow",        rgb: [241, 221,  56] },
    RalEntry { number: 1017, i18n_key: "gldf.ral.1017.name", name_de: "Safrangelb",           name_en: "Saffron yellow",       rgb: [246, 169,  51] },
    RalEntry { number: 1018, i18n_key: "gldf.ral.1018.name", name_de: "Zinkgelb",             name_en: "Zinc yellow",          rgb: [250, 210,  44] },
    RalEntry { number: 1019, i18n_key: "gldf.ral.1019.name", name_de: "Graubeige",            name_en: "Grey beige",           rgb: [164, 143, 122] },
    RalEntry { number: 1020, i18n_key: "gldf.ral.1020.name", name_de: "Olivgelb",             name_en: "Olive yellow",         rgb: [160, 143,  101] },
    RalEntry { number: 1021, i18n_key: "gldf.ral.1021.name", name_de: "Rapsgelb",             name_en: "Rape yellow",          rgb: [246, 182,   0] },
    RalEntry { number: 1023, i18n_key: "gldf.ral.1023.name", name_de: "Verkehrsgelb",         name_en: "Traffic yellow",       rgb: [247, 181,   0] },
    RalEntry { number: 1024, i18n_key: "gldf.ral.1024.name", name_de: "Ockergelb",            name_en: "Ochre yellow",         rgb: [186, 143,  76] },
    RalEntry { number: 1026, i18n_key: "gldf.ral.1026.name", name_de: "Leuchtgelb",           name_en: "Luminous yellow",      rgb: [255, 255,   0] },
    RalEntry { number: 1027, i18n_key: "gldf.ral.1027.name", name_de: "Currygelb",            name_en: "Curry",                rgb: [167, 127,  14] },
    RalEntry { number: 1028, i18n_key: "gldf.ral.1028.name", name_de: "Melonengelb",          name_en: "Melon yellow",         rgb: [255, 154,   0] },
    RalEntry { number: 1032, i18n_key: "gldf.ral.1032.name", name_de: "Ginstergelb",          name_en: "Broom yellow",         rgb: [225, 161,   0] },
    RalEntry { number: 1033, i18n_key: "gldf.ral.1033.name", name_de: "Dahliengelb",          name_en: "Dahlia yellow",        rgb: [248, 156,   0] },
    RalEntry { number: 1034, i18n_key: "gldf.ral.1034.name", name_de: "Pastellgelb",          name_en: "Pastel yellow",        rgb: [235, 156,  82] },
    RalEntry { number: 1035, i18n_key: "gldf.ral.1035.name", name_de: "Perlbeige",            name_en: "Pearl beige",          rgb: [144, 131, 112] },
    RalEntry { number: 1036, i18n_key: "gldf.ral.1036.name", name_de: "Perlgold",             name_en: "Pearl gold",           rgb: [128,  99,  53] },
    RalEntry { number: 1037, i18n_key: "gldf.ral.1037.name", name_de: "Sonnengelb",           name_en: "Sun yellow",           rgb: [240, 146,   0] },

    // --- Orange (2xxx) ---
    RalEntry { number: 2000, i18n_key: "gldf.ral.2000.name", name_de: "Gelborange",           name_en: "Yellow orange",        rgb: [221, 119,   7] },
    RalEntry { number: 2001, i18n_key: "gldf.ral.2001.name", name_de: "Rotorange",            name_en: "Red orange",           rgb: [186,  76,  37] },
    RalEntry { number: 2002, i18n_key: "gldf.ral.2002.name", name_de: "Blutorange",           name_en: "Vermilion",            rgb: [191,  57,  35] },
    RalEntry { number: 2003, i18n_key: "gldf.ral.2003.name", name_de: "Pastellorange",        name_en: "Pastel orange",        rgb: [246, 121,  53] },
    RalEntry { number: 2004, i18n_key: "gldf.ral.2004.name", name_de: "Reinorange",           name_en: "Pure orange",          rgb: [226,  88,  34] },
    RalEntry { number: 2005, i18n_key: "gldf.ral.2005.name", name_de: "Leuchtorange",         name_en: "Luminous orange",      rgb: [255,  77,   6] },
    RalEntry { number: 2007, i18n_key: "gldf.ral.2007.name", name_de: "Leuchthellorange",     name_en: "Luminous bright orange", rgb: [255, 184,   0] },
    RalEntry { number: 2008, i18n_key: "gldf.ral.2008.name", name_de: "Hellrotorange",        name_en: "Bright red orange",    rgb: [237, 109,  37] },
    RalEntry { number: 2009, i18n_key: "gldf.ral.2009.name", name_de: "Verkehrsorange",       name_en: "Traffic orange",       rgb: [222,  83,  16] },
    RalEntry { number: 2010, i18n_key: "gldf.ral.2010.name", name_de: "Signalorange",         name_en: "Signal orange",        rgb: [217,  88,  43] },
    RalEntry { number: 2011, i18n_key: "gldf.ral.2011.name", name_de: "Tieforange",           name_en: "Deep orange",          rgb: [227, 105,  19] },
    RalEntry { number: 2012, i18n_key: "gldf.ral.2012.name", name_de: "Lachsorange",          name_en: "Salmon orange",        rgb: [217,  98,  73] },
    RalEntry { number: 2013, i18n_key: "gldf.ral.2013.name", name_de: "Perlorange",           name_en: "Pearl orange",         rgb: [149,  64,  43] },

    // --- Red (3xxx) ---
    RalEntry { number: 3000, i18n_key: "gldf.ral.3000.name", name_de: "Feuerrot",             name_en: "Flame red",            rgb: [170,  39,  47] },
    RalEntry { number: 3001, i18n_key: "gldf.ral.3001.name", name_de: "Signalrot",            name_en: "Signal red",           rgb: [162,  35,  29] },
    RalEntry { number: 3002, i18n_key: "gldf.ral.3002.name", name_de: "Karminrot",            name_en: "Carmine red",          rgb: [161,  35,  33] },
    RalEntry { number: 3003, i18n_key: "gldf.ral.3003.name", name_de: "Rubinrot",             name_en: "Ruby red",             rgb: [139,  28,  37] },
    RalEntry { number: 3004, i18n_key: "gldf.ral.3004.name", name_de: "Purpurrot",            name_en: "Purple red",           rgb: [108,  30,  38] },
    RalEntry { number: 3005, i18n_key: "gldf.ral.3005.name", name_de: "Weinrot",              name_en: "Wine red",             rgb: [89,   25,  31] },
    RalEntry { number: 3007, i18n_key: "gldf.ral.3007.name", name_de: "Schwarzrot",           name_en: "Black red",            rgb: [62,   27,  29] },
    RalEntry { number: 3009, i18n_key: "gldf.ral.3009.name", name_de: "Oxidrot",              name_en: "Oxide red",            rgb: [109,  52,  44] },
    RalEntry { number: 3011, i18n_key: "gldf.ral.3011.name", name_de: "Braunrot",             name_en: "Brown red",            rgb: [121,  36,  35] },
    RalEntry { number: 3012, i18n_key: "gldf.ral.3012.name", name_de: "Beigerot",             name_en: "Beige red",            rgb: [198, 132, 109] },
    RalEntry { number: 3013, i18n_key: "gldf.ral.3013.name", name_de: "Tomatenrot",           name_en: "Tomato red",           rgb: [151,  46,  37] },
    RalEntry { number: 3014, i18n_key: "gldf.ral.3014.name", name_de: "Altrosa",              name_en: "Antique pink",         rgb: [203, 115, 117] },
    RalEntry { number: 3015, i18n_key: "gldf.ral.3015.name", name_de: "Hellrosa",             name_en: "Light pink",           rgb: [216, 160, 166] },
    RalEntry { number: 3016, i18n_key: "gldf.ral.3016.name", name_de: "Korallenrot",          name_en: "Coral red",            rgb: [168,  57,  44] },
    RalEntry { number: 3017, i18n_key: "gldf.ral.3017.name", name_de: "Rosé",                 name_en: "Rose",                 rgb: [202,  85,  89] },
    RalEntry { number: 3018, i18n_key: "gldf.ral.3018.name", name_de: "Erdbeerrot",           name_en: "Strawberry red",       rgb: [197,  64,  73] },
    RalEntry { number: 3020, i18n_key: "gldf.ral.3020.name", name_de: "Verkehrsrot",          name_en: "Traffic red",          rgb: [187,  30,  16] },
    RalEntry { number: 3022, i18n_key: "gldf.ral.3022.name", name_de: "Lachsrot",             name_en: "Salmon pink",          rgb: [207, 105,  85] },
    RalEntry { number: 3024, i18n_key: "gldf.ral.3024.name", name_de: "Leuchtrot",            name_en: "Luminous red",         rgb: [248,   0,  18] },
    RalEntry { number: 3026, i18n_key: "gldf.ral.3026.name", name_de: "Leuchthellrot",        name_en: "Luminous bright red",  rgb: [254,   0,   0] },
    RalEntry { number: 3027, i18n_key: "gldf.ral.3027.name", name_de: "Himbeerrot",           name_en: "Raspberry red",        rgb: [174,  32,  65] },
    RalEntry { number: 3028, i18n_key: "gldf.ral.3028.name", name_de: "Reinrot",              name_en: "Pure red",             rgb: [203,  40,  33] },
    RalEntry { number: 3031, i18n_key: "gldf.ral.3031.name", name_de: "Orientrot",            name_en: "Orient red",           rgb: [167,  41,  47] },
    RalEntry { number: 3032, i18n_key: "gldf.ral.3032.name", name_de: "Perlrubinrot",         name_en: "Pearl ruby red",       rgb: [112,  29,  35] },
    RalEntry { number: 3033, i18n_key: "gldf.ral.3033.name", name_de: "Perlrosa",             name_en: "Pearl pink",           rgb: [165,  58,  45] },

    // --- Violet (4xxx) ---
    RalEntry { number: 4001, i18n_key: "gldf.ral.4001.name", name_de: "Rotlila",              name_en: "Red lilac",            rgb: [129,  97, 131] },
    RalEntry { number: 4002, i18n_key: "gldf.ral.4002.name", name_de: "Rotviolett",           name_en: "Red violet",           rgb: [142,  62,  82] },
    RalEntry { number: 4003, i18n_key: "gldf.ral.4003.name", name_de: "Erikaviolett",         name_en: "Heather violet",       rgb: [196, 100, 154] },
    RalEntry { number: 4004, i18n_key: "gldf.ral.4004.name", name_de: "Bordeauxviolett",      name_en: "Claret violet",        rgb: [101,  29,  52] },
    RalEntry { number: 4005, i18n_key: "gldf.ral.4005.name", name_de: "Blaulila",             name_en: "Blue lilac",           rgb: [118, 105, 142] },
    RalEntry { number: 4006, i18n_key: "gldf.ral.4006.name", name_de: "Verkehrspurpur",       name_en: "Traffic purple",       rgb: [144,  50, 110] },
    RalEntry { number: 4007, i18n_key: "gldf.ral.4007.name", name_de: "Purpurviolett",        name_en: "Purple violet",        rgb: [73,   27,  53] },
    RalEntry { number: 4008, i18n_key: "gldf.ral.4008.name", name_de: "Signalviolett",        name_en: "Signal violet",        rgb: [131,  63, 113] },
    RalEntry { number: 4009, i18n_key: "gldf.ral.4009.name", name_de: "Pastellviolett",       name_en: "Pastel violet",        rgb: [157, 133, 138] },
    RalEntry { number: 4010, i18n_key: "gldf.ral.4010.name", name_de: "Telemagenta",          name_en: "Telemagenta",          rgb: [187,  64, 119] },
    RalEntry { number: 4011, i18n_key: "gldf.ral.4011.name", name_de: "Perlviolett",          name_en: "Pearl violet",         rgb: [110,  99, 132] },
    RalEntry { number: 4012, i18n_key: "gldf.ral.4012.name", name_de: "Perlbrombeer",         name_en: "Pearl blackberry",     rgb: [106, 103, 120] },

    // --- Blue (5xxx) ---
    RalEntry { number: 5000, i18n_key: "gldf.ral.5000.name", name_de: "Violettblau",          name_en: "Violet blue",          rgb: [49,   76, 110] },
    RalEntry { number: 5001, i18n_key: "gldf.ral.5001.name", name_de: "Grünblau",             name_en: "Green blue",           rgb: [15,   76,  100] },
    RalEntry { number: 5002, i18n_key: "gldf.ral.5002.name", name_de: "Ultramarinblau",       name_en: "Ultramarine blue",     rgb: [0,    56, 122] },
    RalEntry { number: 5003, i18n_key: "gldf.ral.5003.name", name_de: "Saphirblau",           name_en: "Sapphire blue",        rgb: [31,   55,  82] },
    RalEntry { number: 5004, i18n_key: "gldf.ral.5004.name", name_de: "Schwarzblau",          name_en: "Black blue",           rgb: [25,   30,  40] },
    RalEntry { number: 5005, i18n_key: "gldf.ral.5005.name", name_de: "Signalblau",           name_en: "Signal blue",          rgb: [0,    83, 135] },
    RalEntry { number: 5007, i18n_key: "gldf.ral.5007.name", name_de: "Brillantblau",         name_en: "Brilliant blue",       rgb: [55,  103, 142] },
    RalEntry { number: 5008, i18n_key: "gldf.ral.5008.name", name_de: "Graublau",             name_en: "Grey blue",            rgb: [44,   53,  62] },
    RalEntry { number: 5009, i18n_key: "gldf.ral.5009.name", name_de: "Azurblau",             name_en: "Azure blue",           rgb: [37,   88, 113] },
    RalEntry { number: 5010, i18n_key: "gldf.ral.5010.name", name_de: "Enzianblau",           name_en: "Gentian blue",         rgb: [13,   78, 138] },
    RalEntry { number: 5011, i18n_key: "gldf.ral.5011.name", name_de: "Stahlblau",            name_en: "Steel blue",           rgb: [25,   46,  65] },
    RalEntry { number: 5012, i18n_key: "gldf.ral.5012.name", name_de: "Lichtblau",            name_en: "Light blue",           rgb: [62,  138, 184] },
    RalEntry { number: 5013, i18n_key: "gldf.ral.5013.name", name_de: "Kobaltblau",           name_en: "Cobalt blue",          rgb: [25,   42,  77] },
    RalEntry { number: 5014, i18n_key: "gldf.ral.5014.name", name_de: "Taubenblau",           name_en: "Pigeon blue",          rgb: [99,  118, 142] },
    RalEntry { number: 5015, i18n_key: "gldf.ral.5015.name", name_de: "Himmelblau",           name_en: "Sky blue",             rgb: [0,   124, 176] },
    RalEntry { number: 5017, i18n_key: "gldf.ral.5017.name", name_de: "Verkehrsblau",         name_en: "Traffic blue",         rgb: [6,    79, 138] },
    RalEntry { number: 5018, i18n_key: "gldf.ral.5018.name", name_de: "Türkisblau",           name_en: "Turquoise blue",       rgb: [6,   113, 113] },
    RalEntry { number: 5019, i18n_key: "gldf.ral.5019.name", name_de: "Capriblau",            name_en: "Capri blue",           rgb: [22,   85, 117] },
    RalEntry { number: 5020, i18n_key: "gldf.ral.5020.name", name_de: "Ozeanblau",            name_en: "Ocean blue",           rgb: [24,   60,  63] },
    RalEntry { number: 5021, i18n_key: "gldf.ral.5021.name", name_de: "Wasserblau",           name_en: "Water blue",           rgb: [25,  111, 109] },
    RalEntry { number: 5022, i18n_key: "gldf.ral.5022.name", name_de: "Nachtblau",            name_en: "Night blue",           rgb: [38,   37,  64] },
    RalEntry { number: 5023, i18n_key: "gldf.ral.5023.name", name_de: "Fernblau",             name_en: "Distant blue",         rgb: [73,  103, 130] },
    RalEntry { number: 5024, i18n_key: "gldf.ral.5024.name", name_de: "Pastellblau",          name_en: "Pastel blue",          rgb: [101, 142, 167] },
    RalEntry { number: 5025, i18n_key: "gldf.ral.5025.name", name_de: "Perlenzian",           name_en: "Pearl gentian blue",   rgb: [33,  104, 124] },
    RalEntry { number: 5026, i18n_key: "gldf.ral.5026.name", name_de: "Perlnachtblau",        name_en: "Pearl night blue",     rgb: [15,   48,  82] },

    // --- Green (6xxx) ---
    RalEntry { number: 6000, i18n_key: "gldf.ral.6000.name", name_de: "Patinagrün",           name_en: "Patina green",         rgb: [50,  127, 105] },
    RalEntry { number: 6001, i18n_key: "gldf.ral.6001.name", name_de: "Smaragdgrün",          name_en: "Emerald green",        rgb: [40,  114,  51] },
    RalEntry { number: 6002, i18n_key: "gldf.ral.6002.name", name_de: "Laubgrün",             name_en: "Leaf green",           rgb: [49,  102,  29] },
    RalEntry { number: 6003, i18n_key: "gldf.ral.6003.name", name_de: "Olivgrün",             name_en: "Olive green",          rgb: [80,   83,  60] },
    RalEntry { number: 6004, i18n_key: "gldf.ral.6004.name", name_de: "Blaugrün",             name_en: "Blue green",           rgb: [2,    68,  66] },
    RalEntry { number: 6005, i18n_key: "gldf.ral.6005.name", name_de: "Moosgrün",             name_en: "Moss green",           rgb: [17,   66,  50] },
    RalEntry { number: 6006, i18n_key: "gldf.ral.6006.name", name_de: "Grauoliv",             name_en: "Grey olive",           rgb: [62,   59,  50] },
    RalEntry { number: 6007, i18n_key: "gldf.ral.6007.name", name_de: "Flaschengrün",         name_en: "Bottle green",         rgb: [44,   55,  35] },
    RalEntry { number: 6008, i18n_key: "gldf.ral.6008.name", name_de: "Braungrün",            name_en: "Brown green",          rgb: [55,   53,  39] },
    RalEntry { number: 6009, i18n_key: "gldf.ral.6009.name", name_de: "Tannengrün",           name_en: "Fir green",            rgb: [39,   54,  39] },
    RalEntry { number: 6010, i18n_key: "gldf.ral.6010.name", name_de: "Grasgrün",             name_en: "Grass green",          rgb: [77,  111,  57] },
    RalEntry { number: 6011, i18n_key: "gldf.ral.6011.name", name_de: "Resedagrün",           name_en: "Reseda green",         rgb: [108, 124,  89] },
    RalEntry { number: 6012, i18n_key: "gldf.ral.6012.name", name_de: "Schwarzgrün",          name_en: "Black green",          rgb: [48,   60,  57] },
    RalEntry { number: 6013, i18n_key: "gldf.ral.6013.name", name_de: "Schilfgrün",           name_en: "Reed green",           rgb: [125, 118,  90] },
    RalEntry { number: 6014, i18n_key: "gldf.ral.6014.name", name_de: "Gelboliv",             name_en: "Yellow olive",         rgb: [71,   65,  56] },
    RalEntry { number: 6015, i18n_key: "gldf.ral.6015.name", name_de: "Schwarzoliv",          name_en: "Black olive",          rgb: [61,   60,  54] },
    RalEntry { number: 6016, i18n_key: "gldf.ral.6016.name", name_de: "Türkisgrün",           name_en: "Turquoise green",      rgb: [0,   105,  76] },
    RalEntry { number: 6017, i18n_key: "gldf.ral.6017.name", name_de: "Maigrün",              name_en: "May green",            rgb: [88,  127,  64] },
    RalEntry { number: 6018, i18n_key: "gldf.ral.6018.name", name_de: "Gelbgrün",             name_en: "Yellow green",         rgb: [97,  153,  59] },
    RalEntry { number: 6019, i18n_key: "gldf.ral.6019.name", name_de: "Weißgrün",             name_en: "Pastel green",         rgb: [185, 206, 172] },
    RalEntry { number: 6020, i18n_key: "gldf.ral.6020.name", name_de: "Chromoxidgrün",        name_en: "Chrome green",         rgb: [55,   66,  47] },
    RalEntry { number: 6021, i18n_key: "gldf.ral.6021.name", name_de: "Blassgrün",            name_en: "Pale green",           rgb: [138, 153, 119] },
    RalEntry { number: 6022, i18n_key: "gldf.ral.6022.name", name_de: "Braunoliv",            name_en: "Olive drab",           rgb: [54,   51,  45] },
    RalEntry { number: 6024, i18n_key: "gldf.ral.6024.name", name_de: "Verkehrsgrün",         name_en: "Traffic green",        rgb: [0,  131,  81] },
    RalEntry { number: 6025, i18n_key: "gldf.ral.6025.name", name_de: "Farngrün",             name_en: "Fern green",           rgb: [80,  104,  60] },
    RalEntry { number: 6026, i18n_key: "gldf.ral.6026.name", name_de: "Opalgrün",             name_en: "Opal green",           rgb: [1,   93,  82] },
    RalEntry { number: 6027, i18n_key: "gldf.ral.6027.name", name_de: "Lichtgrün",            name_en: "Light green",          rgb: [126, 186, 181] },
    RalEntry { number: 6028, i18n_key: "gldf.ral.6028.name", name_de: "Kieferngrün",          name_en: "Pine green",           rgb: [49,   84,  66] },
    RalEntry { number: 6029, i18n_key: "gldf.ral.6029.name", name_de: "Minzgrün",             name_en: "Mint green",           rgb: [0,  111,  61] },
    RalEntry { number: 6032, i18n_key: "gldf.ral.6032.name", name_de: "Signalgrün",           name_en: "Signal green",         rgb: [35,  127,  82] },
    RalEntry { number: 6033, i18n_key: "gldf.ral.6033.name", name_de: "Minttürkis",           name_en: "Mint turquoise",       rgb: [70,  137, 129] },
    RalEntry { number: 6034, i18n_key: "gldf.ral.6034.name", name_de: "Pastelltürkis",        name_en: "Pastel turquoise",     rgb: [122, 173, 173] },
    RalEntry { number: 6035, i18n_key: "gldf.ral.6035.name", name_de: "Perlgrün",             name_en: "Pearl green",          rgb: [25,   88,  39] },
    RalEntry { number: 6036, i18n_key: "gldf.ral.6036.name", name_de: "Perlopalgrün",         name_en: "Pearl opal green",     rgb: [0,   91,  82] },
    RalEntry { number: 6037, i18n_key: "gldf.ral.6037.name", name_de: "Reingrün",             name_en: "Pure green",           rgb: [0,  140,  53] },
    RalEntry { number: 6038, i18n_key: "gldf.ral.6038.name", name_de: "Leuchtgrün",           name_en: "Luminous green",       rgb: [0,  181,  29] },

    // --- Grey (7xxx) ---
    RalEntry { number: 7000, i18n_key: "gldf.ral.7000.name", name_de: "Fehgrau",              name_en: "Squirrel grey",        rgb: [122, 136, 142] },
    RalEntry { number: 7001, i18n_key: "gldf.ral.7001.name", name_de: "Silbergrau",           name_en: "Silver grey",          rgb: [140, 152, 159] },
    RalEntry { number: 7002, i18n_key: "gldf.ral.7002.name", name_de: "Olivgrau",             name_en: "Olive grey",           rgb: [129, 120,  88] },
    RalEntry { number: 7003, i18n_key: "gldf.ral.7003.name", name_de: "Moosgrau",             name_en: "Moss grey",            rgb: [122, 116,  93] },
    RalEntry { number: 7004, i18n_key: "gldf.ral.7004.name", name_de: "Signalgrau",           name_en: "Signal grey",          rgb: [158, 162, 162] },
    RalEntry { number: 7005, i18n_key: "gldf.ral.7005.name", name_de: "Mausgrau",             name_en: "Mouse grey",           rgb: [108, 110, 107] },
    RalEntry { number: 7006, i18n_key: "gldf.ral.7006.name", name_de: "Beigegrau",            name_en: "Beige grey",           rgb: [118, 106,  94] },
    RalEntry { number: 7008, i18n_key: "gldf.ral.7008.name", name_de: "Khakigrau",            name_en: "Khaki grey",           rgb: [116,  94,  61] },
    RalEntry { number: 7009, i18n_key: "gldf.ral.7009.name", name_de: "Grüngrau",             name_en: "Green grey",           rgb: [93,   96,  88] },
    RalEntry { number: 7010, i18n_key: "gldf.ral.7010.name", name_de: "Zeltgrau",             name_en: "Tarpaulin grey",       rgb: [88,   92,  86] },
    RalEntry { number: 7011, i18n_key: "gldf.ral.7011.name", name_de: "Eisengrau",            name_en: "Iron grey",            rgb: [82,   89,  93] },
    RalEntry { number: 7012, i18n_key: "gldf.ral.7012.name", name_de: "Basaltgrau",           name_en: "Basalt grey",          rgb: [79,   86,  86] },
    RalEntry { number: 7013, i18n_key: "gldf.ral.7013.name", name_de: "Braungrau",            name_en: "Brown grey",           rgb: [80,   75,  61] },
    RalEntry { number: 7015, i18n_key: "gldf.ral.7015.name", name_de: "Schiefergrau",         name_en: "Slate grey",           rgb: [69,   73,  82] },
    RalEntry { number: 7016, i18n_key: "gldf.ral.7016.name", name_de: "Anthrazitgrau",        name_en: "Anthracite grey",      rgb: [56,   62,  66] },
    RalEntry { number: 7021, i18n_key: "gldf.ral.7021.name", name_de: "Schwarzgrau",          name_en: "Black grey",           rgb: [47,   50,  52] },
    RalEntry { number: 7022, i18n_key: "gldf.ral.7022.name", name_de: "Umbragrau",            name_en: "Umbra grey",           rgb: [76,   74,  68] },
    RalEntry { number: 7023, i18n_key: "gldf.ral.7023.name", name_de: "Betongrau",            name_en: "Concrete grey",        rgb: [128, 128, 118] },
    RalEntry { number: 7024, i18n_key: "gldf.ral.7024.name", name_de: "Graphitgrau",          name_en: "Graphite grey",        rgb: [69,   71,  74] },
    RalEntry { number: 7026, i18n_key: "gldf.ral.7026.name", name_de: "Granitgrau",           name_en: "Granite grey",         rgb: [55,   67,  69] },
    RalEntry { number: 7030, i18n_key: "gldf.ral.7030.name", name_de: "Steingrau",            name_en: "Stone grey",           rgb: [146, 142, 133] },
    RalEntry { number: 7031, i18n_key: "gldf.ral.7031.name", name_de: "Blaugrau",             name_en: "Blue grey",            rgb: [91,  104, 109] },
    RalEntry { number: 7032, i18n_key: "gldf.ral.7032.name", name_de: "Kieselgrau",           name_en: "Pebble grey",          rgb: [184, 178, 161] },
    RalEntry { number: 7033, i18n_key: "gldf.ral.7033.name", name_de: "Zementgrau",           name_en: "Cement grey",          rgb: [125, 132, 113] },
    RalEntry { number: 7034, i18n_key: "gldf.ral.7034.name", name_de: "Gelbgrau",             name_en: "Yellow grey",          rgb: [143, 131, 112] },
    RalEntry { number: 7035, i18n_key: "gldf.ral.7035.name", name_de: "Lichtgrau",            name_en: "Light grey",           rgb: [215, 215, 215] },
    RalEntry { number: 7036, i18n_key: "gldf.ral.7036.name", name_de: "Platingrau",           name_en: "Platinum grey",        rgb: [127, 118, 121] },
    RalEntry { number: 7037, i18n_key: "gldf.ral.7037.name", name_de: "Staubgrau",            name_en: "Dusty grey",           rgb: [124, 127, 126] },
    RalEntry { number: 7038, i18n_key: "gldf.ral.7038.name", name_de: "Achatgrau",            name_en: "Agate grey",           rgb: [178, 180, 177] },
    RalEntry { number: 7039, i18n_key: "gldf.ral.7039.name", name_de: "Quarzgrau",            name_en: "Quartz grey",          rgb: [109, 105,  93] },
    RalEntry { number: 7040, i18n_key: "gldf.ral.7040.name", name_de: "Fenstergrau",          name_en: "Window grey",          rgb: [152, 158, 160] },
    RalEntry { number: 7042, i18n_key: "gldf.ral.7042.name", name_de: "Verkehrsgrau A",       name_en: "Traffic grey A",       rgb: [142, 146, 145] },
    RalEntry { number: 7043, i18n_key: "gldf.ral.7043.name", name_de: "Verkehrsgrau B",       name_en: "Traffic grey B",       rgb: [79,   82,  80] },
    RalEntry { number: 7044, i18n_key: "gldf.ral.7044.name", name_de: "Seidengrau",           name_en: "Silk grey",            rgb: [183, 179, 169] },
    RalEntry { number: 7045, i18n_key: "gldf.ral.7045.name", name_de: "Telegrau 1",           name_en: "Telegrey 1",           rgb: [141, 146, 149] },
    RalEntry { number: 7046, i18n_key: "gldf.ral.7046.name", name_de: "Telegrau 2",           name_en: "Telegrey 2",           rgb: [127, 134, 138] },
    RalEntry { number: 7047, i18n_key: "gldf.ral.7047.name", name_de: "Telegrau 4",           name_en: "Telegrey 4",           rgb: [200, 200, 199] },
    RalEntry { number: 7048, i18n_key: "gldf.ral.7048.name", name_de: "Perlmausgrau",         name_en: "Pearl mouse grey",     rgb: [129, 123, 116] },

    // --- Brown (8xxx) ---
    RalEntry { number: 8000, i18n_key: "gldf.ral.8000.name", name_de: "Grünbraun",            name_en: "Green brown",          rgb: [137, 105,  62] },
    RalEntry { number: 8001, i18n_key: "gldf.ral.8001.name", name_de: "Ockerbraun",           name_en: "Ochre brown",          rgb: [157,  98,  43] },
    RalEntry { number: 8002, i18n_key: "gldf.ral.8002.name", name_de: "Signalbraun",          name_en: "Signal brown",         rgb: [121,  77,  62] },
    RalEntry { number: 8003, i18n_key: "gldf.ral.8003.name", name_de: "Lehmbraun",            name_en: "Clay brown",           rgb: [126,  75,  39] },
    RalEntry { number: 8004, i18n_key: "gldf.ral.8004.name", name_de: "Kupferbraun",          name_en: "Copper brown",         rgb: [142,  64,  42] },
    RalEntry { number: 8007, i18n_key: "gldf.ral.8007.name", name_de: "Rehbraun",             name_en: "Fawn brown",           rgb: [112,  69,  42] },
    RalEntry { number: 8008, i18n_key: "gldf.ral.8008.name", name_de: "Olivbraun",            name_en: "Olive brown",          rgb: [114,  74,  37] },
    RalEntry { number: 8011, i18n_key: "gldf.ral.8011.name", name_de: "Nussbraun",            name_en: "Nut brown",            rgb: [90,   56,  38] },
    RalEntry { number: 8012, i18n_key: "gldf.ral.8012.name", name_de: "Rotbraun",             name_en: "Red brown",            rgb: [102,  47,  41] },
    RalEntry { number: 8014, i18n_key: "gldf.ral.8014.name", name_de: "Sepiabraun",           name_en: "Sepia brown",          rgb: [73,   51,  39] },
    RalEntry { number: 8015, i18n_key: "gldf.ral.8015.name", name_de: "Kastanienbraun",       name_en: "Chestnut brown",       rgb: [94,   46,  41] },
    RalEntry { number: 8016, i18n_key: "gldf.ral.8016.name", name_de: "Mahagonibraun",        name_en: "Mahogany brown",       rgb: [78,   42,  35] },
    RalEntry { number: 8017, i18n_key: "gldf.ral.8017.name", name_de: "Schokoladenbraun",     name_en: "Chocolate brown",      rgb: [69,   45,  36] },
    RalEntry { number: 8019, i18n_key: "gldf.ral.8019.name", name_de: "Graubraun",            name_en: "Grey brown",           rgb: [64,   54,  52] },
    RalEntry { number: 8022, i18n_key: "gldf.ral.8022.name", name_de: "Schwarzbraun",         name_en: "Black brown",          rgb: [33,   32,  31] },
    RalEntry { number: 8023, i18n_key: "gldf.ral.8023.name", name_de: "Orangebraun",          name_en: "Orange brown",         rgb: [166,  94,  46] },
    RalEntry { number: 8024, i18n_key: "gldf.ral.8024.name", name_de: "Beigebraun",           name_en: "Beige brown",          rgb: [121,  85,  61] },
    RalEntry { number: 8025, i18n_key: "gldf.ral.8025.name", name_de: "Blassbraun",           name_en: "Pale brown",           rgb: [117,  92,  72] },
    RalEntry { number: 8028, i18n_key: "gldf.ral.8028.name", name_de: "Terrabraun",           name_en: "Terra brown",          rgb: [78,   59,  49] },
    RalEntry { number: 8029, i18n_key: "gldf.ral.8029.name", name_de: "Perlkupfer",           name_en: "Pearl copper",         rgb: [118,  60,  40] },

    // --- White / black (9xxx) ---
    RalEntry { number: 9001, i18n_key: "gldf.ral.9001.name", name_de: "Cremeweiß",            name_en: "Cream",                rgb: [233, 224, 210] },
    RalEntry { number: 9002, i18n_key: "gldf.ral.9002.name", name_de: "Grauweiß",             name_en: "Grey white",           rgb: [215, 213, 203] },
    RalEntry { number: 9003, i18n_key: "gldf.ral.9003.name", name_de: "Signalweiß",           name_en: "Signal white",         rgb: [237, 237, 230] },
    RalEntry { number: 9004, i18n_key: "gldf.ral.9004.name", name_de: "Signalschwarz",        name_en: "Signal black",         rgb: [40,   40,  40] },
    RalEntry { number: 9005, i18n_key: "gldf.ral.9005.name", name_de: "Tiefschwarz",          name_en: "Jet black",            rgb: [14,   14,  16] },
    RalEntry { number: 9006, i18n_key: "gldf.ral.9006.name", name_de: "Weißaluminium",        name_en: "White aluminium",      rgb: [161, 161, 161] },
    RalEntry { number: 9007, i18n_key: "gldf.ral.9007.name", name_de: "Graualuminium",        name_en: "Grey aluminium",       rgb: [135, 133, 129] },
    RalEntry { number: 9010, i18n_key: "gldf.ral.9010.name", name_de: "Reinweiß",             name_en: "Pure white",           rgb: [241, 236, 225] },
    RalEntry { number: 9011, i18n_key: "gldf.ral.9011.name", name_de: "Graphitschwarz",       name_en: "Graphite black",       rgb: [39,   41,  43] },
    RalEntry { number: 9016, i18n_key: "gldf.ral.9016.name", name_de: "Verkehrsweiß",         name_en: "Traffic white",       rgb: [241, 241, 234] },
    RalEntry { number: 9017, i18n_key: "gldf.ral.9017.name", name_de: "Verkehrsschwarz",      name_en: "Traffic black",        rgb: [42,   41,  42] },
    RalEntry { number: 9018, i18n_key: "gldf.ral.9018.name", name_de: "Papyrusweiß",          name_en: "Papyrus white",        rgb: [200, 203, 196] },
    RalEntry { number: 9022, i18n_key: "gldf.ral.9022.name", name_de: "Perlhellgrau",         name_en: "Pearl light grey",     rgb: [133, 133, 131] },
    RalEntry { number: 9023, i18n_key: "gldf.ral.9023.name", name_de: "Perldunkelgrau",       name_en: "Pearl dark grey",      rgb: [121, 123, 122] },
];

/// Lookup by RAL number (e.g. 9016 → Traffic white).
pub fn by_number(number: i32) -> Option<&'static RalEntry> {
    RAL_CLASSIC.iter().find(|e| e.number == number)
}

/// Fuzzy lookup by name. Matches against both German and English
/// canonical names, case- and whitespace-insensitive. Returns the
/// first match.
pub fn by_name(name: &str) -> Option<&'static RalEntry> {
    let needle = normalize(name);
    RAL_CLASSIC
        .iter()
        .find(|e| normalize(e.name_de) == needle || normalize(e.name_en) == needle)
}

/// Find all entries whose German or English name *contains* the
/// substring (case-insensitive). Useful for "as you type" filtering
/// in a combobox. Returns at most `limit` matches in numeric order.
pub fn search(query: &str, limit: usize) -> Vec<&'static RalEntry> {
    let needle = query.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    RAL_CLASSIC
        .iter()
        .filter(|e| {
            e.name_de.to_ascii_lowercase().contains(&needle)
                || e.name_en.to_ascii_lowercase().contains(&needle)
        })
        .take(limit)
        .collect()
}

fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_9016_traffic_white() {
        let e = by_number(9016).unwrap();
        assert_eq!(e.name_en, "Traffic white");
        assert_eq!(e.name_de, "Verkehrsweiß");
    }

    #[test]
    fn lookup_9007_grey_aluminium() {
        let e = by_number(9007).unwrap();
        assert_eq!(e.name_en, "Grey aluminium");
        assert_eq!(e.name_de, "Graualuminium");
    }

    #[test]
    fn lookup_by_name_either_language() {
        assert_eq!(by_name("Verkehrsweiß").unwrap().number, 9016);
        assert_eq!(by_name("Traffic white").unwrap().number, 9016);
        // Case + whitespace insensitive
        assert_eq!(by_name("  traffic WHITE  ").unwrap().number, 9016);
        assert_eq!(by_name("verkehrsweiss").is_none(), true); // ß ≠ ss
    }

    #[test]
    fn rgb_f32_round_trip() {
        let e = by_number(7035).unwrap(); // Light grey
        let rgb = e.rgb_f32();
        assert!((rgb[0] - 215.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn hex_format() {
        let e = by_number(9005).unwrap(); // Jet black
        assert_eq!(e.hex(), "#0e0e10");
    }

    #[test]
    fn search_finds_partial() {
        let hits = search("traffic", 10);
        // 9016 + 9017 + 7042 + 7043 + 1023 + 2009 + 3020 + 4006 + 5017 + 6024 …
        assert!(hits.len() > 5);
        assert!(hits.iter().any(|e| e.number == 9016));
    }

    #[test]
    fn unknown_number_returns_none() {
        assert!(by_number(1234).is_none());
        assert!(by_number(0).is_none());
        assert!(by_number(10000).is_none());
    }

    #[test]
    fn all_entries_match_xsd_pattern() {
        // XSD restricts @ral to [1-9][0-9]{3} — 1000..=9999.
        for e in RAL_CLASSIC {
            assert!(
                (1000..=9999).contains(&e.number),
                "RAL {} out of XSD-allowed range",
                e.number
            );
        }
    }

    #[test]
    fn i18n_keys_match_number() {
        // Every entry's i18n_key follows `gldf.ral.<number>.name` so a
        // consumer can synthesize the key from the number without
        // touching the table at all.
        for e in RAL_CLASSIC {
            let expected = format!("gldf.ral.{}.name", e.number);
            assert_eq!(
                e.i18n_key, expected,
                "RAL {} has wrong i18n_key '{}'",
                e.number, e.i18n_key
            );
        }
    }

    #[test]
    fn fallback_name_picks_language() {
        let e = by_number(9016).unwrap();
        assert_eq!(e.fallback_name("de"), "Verkehrsweiß");
        assert_eq!(e.fallback_name("de-AT"), "Verkehrsweiß"); // regional
        assert_eq!(e.fallback_name("en"), "Traffic white");
        assert_eq!(e.fallback_name("en-US"), "Traffic white");
        // Unknown language → English fallback (matches eulumdat-i18n's behaviour)
        assert_eq!(e.fallback_name("ja"), "Traffic white");
        assert_eq!(e.fallback_name(""), "Traffic white");
    }
}
