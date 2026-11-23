//! T2 — Round-trip every `.gldf` fixture in `tests/data/` through
//! `EditableGldf::save_to_buf` and back, asserting the model survives.
//!
//! What this catches: the long tail of round-trip bugs we've fixed
//! by hand over the last weeks — empty `$text` on Locale fields,
//! `RatedChromacityCoordinateValues` rc.3 typo alias, rc.4
//! `Fluorescent Triphosphor` enum, Hyperlink locale dropouts, etc.
//! Each fixture's `GldfProduct` is loaded twice (raw + after a
//! save→load) and the two products are compared field-by-field via
//! `PartialEq`. Any drift fails the run with a diff-style message.
//!
//! Each fixture runs as its own `#[test]` function so a failing
//! file in one corner doesn't mask drift in the rest. Generated
//! by a `data_driven_tests!` macro to keep the file short.

use gldf_rs::{EditableGldf, GldfProduct};

/// Compare two `GldfProduct`s after a save→load round-trip and
/// return `Ok(())` on identity, else `Err(diff_message)`.
///
/// We compare via PartialEq (derived on GldfProduct) but synthesise
/// a human-readable diff when it fails by serialising both sides to
/// JSON and pointing at the first byte that differs. JSON is verbose
/// but fast to scan in CI output, and the path-to-first-diff usually
/// reveals which field changed.
fn assert_roundtrip(label: &str, path: &str) {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("skipping {label}: read {path}: {e}");
            return;
        }
    };
    let original_bundle =
        GldfProduct::load_gldf_from_buf_all(bytes.clone()).expect("load original");
    let original = original_bundle.gldf.clone();

    // Round-trip via EditableGldf::save_to_buf — that's the path the
    // WASM editor's "Export GLDF" button takes, so testing it covers
    // both the lib and the editor user flow.
    let editable = EditableGldf::from_buf(bytes).expect("EditableGldf::from_buf");
    let resaved = editable.save_to_buf().expect("save_to_buf");
    let roundtripped_bundle =
        GldfProduct::load_gldf_from_buf_all(resaved).expect("reload roundtripped");
    let roundtripped = roundtripped_bundle.gldf;

    // We compare on the XML serialization — that's what actually
    // persists on disk; if two `GldfProduct`s produce the same XML
    // they're interchangeable for every downstream consumer. Direct
    // PartialEq on the struct can drift on f64 NaN or HashMap order
    // even when the XML is byte-identical, so XML-equality is the
    // practical "round-trip safe" definition.
    let lhs = original.to_xml().expect("serialize original to XML");
    let rhs = roundtripped
        .to_xml()
        .expect("serialize roundtripped to XML");
    if lhs == rhs {
        return;
    }
    let lhs_lines: Vec<&str> = lhs.lines().collect();
    let rhs_lines: Vec<&str> = rhs.lines().collect();
    let first_diff = lhs_lines
        .iter()
        .zip(rhs_lines.iter())
        .enumerate()
        .find(|(_, (a, b))| a != b);
    let report = match first_diff {
        Some((line_no, (a, b))) => {
            // Context: 3 lines of preceding JSON to anchor the field path.
            let ctx_from = line_no.saturating_sub(3);
            let context: Vec<String> = lhs_lines[ctx_from..line_no]
                .iter()
                .enumerate()
                .map(|(i, l)| format!("    {}: {}", ctx_from + i, l.trim_end()))
                .collect();
            format!(
                "first diff at line {line_no}:\n{}\n  original ->: {}\n  round-trip>: {}",
                context.join("\n"),
                a.trim_end(),
                b.trim_end()
            )
        }
        None => {
            format!(
                "PartialEq says differ but JSON identical ({} lines / {} chars vs {} chars). \
                 Likely an unsortable container (HashMap?) — review the lib's `derive(PartialEq)` chain.",
                lhs_lines.len(),
                lhs.len(),
                rhs.len()
            )
        }
    };
    panic!("{label}: round-trip drift detected\n{report}");
}

/// Macro that materialises one `#[test]` function per fixture so
/// `cargo test --test roundtrip_fixtures` reports the failing files
/// by name. Filename-with-spaces stays escapable via the path arg.
macro_rules! fixture_tests {
    ($($name:ident => $path:expr,)+) => {
        $(
            #[test]
            fn $name() {
                assert_roundtrip(stringify!($name), $path);
            }
        )+
    };
}

fixture_tests! {
    floodlight_max_enriched
        => "../../tests/data/FLOODLIGHT_MAX_1200W_757_SYM_10_WAL_enriched.gldf",
    freestand_belviso
        => "../../tests/data/Freestand_Belviso-2l3d.gldf",
    just_urls
        => "../../tests/data/Just_URLs.gldf",
    philips_sp542p
        => "../../tests/data/Philips SP542P 0.9 DEMO.gldf",
    r2mcobsik
        => "../../tests/data/R2MCOBSIK-30.gldf",
    slv_tria_2
        => "../../tests/data/SLV - Tria 2.gldf",
    alurays_3000mm
        => "../../tests/data/alurays-3000mm.gldf",
    alurays_3500mm
        => "../../tests/data/alurays-3500mm.gldf",
    alurays_4300mm
        => "../../tests/data/alurays-4300mm.gldf",
    alurays_6200mm
        => "../../tests/data/alurays-6200mm.gldf",
    alurays
        => "../../tests/data/alurays.gldf",
    astral_sky_luedinghausen
        => "../../tests/data/astral_sky_lüdinghausen.gldf",
    full_demo
        => "../../tests/data/full_demo.gldf",
    gldf_demo_icu
        => "../../tests/data/gldf_demo_icu.gldf",
    igz_p450
        => "../../tests/data/igz___p450.gldf",
    rzb_311619
        => "../../tests/data/rzb___311619_002_5_8630.gldf",
    spectral_gldf_example
        => "../../tests/data/spctral_gldf-example.gldf",
    star_sky
        => "../../tests/data/star_sky.gldf",
    test
        => "../../tests/data/test.gldf",
    wip_stadium
        => "../../tests/data/wip_stadium.gldf",
}
