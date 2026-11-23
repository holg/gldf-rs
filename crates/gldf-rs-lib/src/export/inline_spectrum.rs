//! Inline-spectrum transform.
//!
//! Rewrites a `GldfProduct` in place so that every `<Spectrum>` that
//! references an external `spectrum/text` file becomes a list of
//! inline `<Intensity wavelength=…>value</Intensity>` rows. The
//! corresponding `<File>` entry is dropped from `general_definitions.files`
//! and its bytes are removed from the `embedded_files` map.
//!
//! Driven by [`crate::editable::ExportOptions::inline_spectrum`].
//! Pure transform: takes `&mut GldfProduct` + `&mut HashMap<String, Vec<u8>>`,
//! returns a tiny `Outcome` so the caller can surface a hint to the
//! user when there was nothing to do (every spectrum already inline /
//! no spectra at all).

use crate::gldf::general_definitions::photometries::Intensity;
use crate::GldfProduct;
use std::collections::HashMap;

/// What [`apply_inline_spectrum`] actually did.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineSpectrumOutcome {
    /// Spectrums converted from file-based to inline this pass.
    pub inlined: usize,
    /// File ids dropped from `general_definitions.files` (and their
    /// bytes removed from the `embedded_files` map). One per inlined
    /// spectrum, modulo files referenced by something else.
    pub files_removed: Vec<String>,
}

impl InlineSpectrumOutcome {
    /// `true` if there was nothing to do — caller should clear the
    /// flag + note that all spectra were already inline.
    pub fn is_noop(&self) -> bool {
        self.inlined == 0 && self.files_removed.is_empty()
    }
}

/// Replace every external-file spectrum with its inline equivalent.
///
/// Behaviour rules:
///   * `<Spectrum>` with `intensity` already non-empty → left alone.
///   * `<Spectrum>` with `SpectrumFileReference` AND the linked file's
///     bytes parseable as a `wavelength<ws>value` table → inlined.
///   * `<Spectrum>` whose linked file is missing from `embedded_files`
///     or can't be parsed → left alone (we don't risk losing data;
///     the caller can surface a warning).
///
/// SPD parsing tolerates `#` comment lines (it's a producer-level
/// convention) and either tab or whitespace separators. Lines that
/// don't parse as `i32 f64` are silently skipped, matching how
/// `dialux_compat::check` reads them.
pub fn apply_inline_spectrum(
    product: &mut GldfProduct,
    embedded_files: &mut HashMap<String, Vec<u8>>,
) -> InlineSpectrumOutcome {
    let Some(spectrums) = product.general_definitions.spectrums.as_mut() else {
        return InlineSpectrumOutcome {
            inlined: 0,
            files_removed: vec![],
        };
    };

    let mut files_to_remove: Vec<String> = Vec::new();
    let mut inlined: usize = 0;

    for spec in spectrums.spectrum.iter_mut() {
        if !spec.intensity.is_empty() {
            continue;
        }
        let Some(file_ref) = spec.spectrum_file_reference.as_ref() else {
            continue;
        };
        let Some(bytes) = embedded_files.get(&file_ref.file_id) else {
            continue;
        };
        let parsed = parse_spd(&String::from_utf8_lossy(bytes));
        if parsed.is_empty() {
            continue;
        }
        spec.intensity = parsed
            .into_iter()
            .map(|(nm, v)| Intensity {
                wavelength: Some(nm),
                value: Some(v),
            })
            .collect();
        files_to_remove.push(file_ref.file_id.clone());
        spec.spectrum_file_reference = None;
        inlined += 1;
    }

    // Drop the now-orphaned File entries + their bytes. We only remove
    // a file if NOTHING else references it (other spectra, photometries,
    // images, …) — there's no cascade-reference check here yet, so we
    // play conservative: only the spectrum we just inlined.
    for fid in &files_to_remove {
        product
            .general_definitions
            .files
            .file
            .retain(|f| &f.id != fid);
        embedded_files.remove(fid);
    }

    InlineSpectrumOutcome {
        inlined,
        files_removed: files_to_remove,
    }
}

/// Parse a `spectrum/text` payload into `(wavelength_nm, value)` pairs.
/// Skips `#` comments and blank lines. Lines whose first two
/// whitespace-separated tokens aren't `i32` and `f64` are silently
/// skipped, matching the rest of the codebase's lenient reading.
fn parse_spd(text: &str) -> Vec<(i32, f64)> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut it = l.split(|c: char| c.is_whitespace());
            let nm = it.next()?.parse::<i32>().ok()?;
            let value = it.next()?.parse::<f64>().ok()?;
            Some((nm, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gldf::general_definitions::files::File as GldfFile;
    use crate::gldf::general_definitions::photometries::{
        Spectrum, SpectrumFileReference, Spectrums,
    };

    fn product_with_one_file_spectrum() -> (GldfProduct, HashMap<String, Vec<u8>>) {
        let mut prod = GldfProduct::default();
        prod.general_definitions.files.file.push(GldfFile {
            id: "spd1".into(),
            content_type: "spectrum/text".into(),
            type_attr: "localFileName".into(),
            language: String::new(),
            file_name: "spec.spd".into(),
        });
        prod.general_definitions.spectrums = Some(Spectrums {
            spectrum: vec![Spectrum {
                id: "s1".into(),
                spectrum_file_reference: Some(SpectrumFileReference {
                    file_id: "spd1".into(),
                }),
                intensity: vec![],
            }],
        });
        let mut em = HashMap::new();
        em.insert(
            "spd1".into(),
            b"# header\n380\t0.5\n385\t0.6\n390 0.7\n".to_vec(),
        );
        (prod, em)
    }

    #[test]
    fn inlines_file_spectrum_and_drops_file() {
        let (mut prod, mut em) = product_with_one_file_spectrum();
        let outcome = apply_inline_spectrum(&mut prod, &mut em);
        assert_eq!(outcome.inlined, 1);
        assert_eq!(outcome.files_removed, vec!["spd1"]);
        let specs = &prod
            .general_definitions
            .spectrums
            .as_ref()
            .unwrap()
            .spectrum;
        assert_eq!(specs[0].intensity.len(), 3);
        assert_eq!(specs[0].intensity[0].wavelength, Some(380));
        assert_eq!(specs[0].intensity[0].value, Some(0.5));
        assert_eq!(specs[0].intensity[2].wavelength, Some(390)); // whitespace-separated row
        assert!(specs[0].spectrum_file_reference.is_none());
        assert!(prod.general_definitions.files.file.is_empty());
        assert!(em.is_empty());
    }

    #[test]
    fn already_inline_is_noop() {
        let mut prod = GldfProduct::default();
        prod.general_definitions.spectrums = Some(Spectrums {
            spectrum: vec![Spectrum {
                id: "s1".into(),
                spectrum_file_reference: None,
                intensity: vec![Intensity {
                    wavelength: Some(380),
                    value: Some(0.5),
                }],
            }],
        });
        let mut em = HashMap::new();
        let outcome = apply_inline_spectrum(&mut prod, &mut em);
        assert!(outcome.is_noop());
    }

    #[test]
    fn no_spectrums_at_all_is_noop() {
        let mut prod = GldfProduct::default();
        let mut em = HashMap::new();
        let outcome = apply_inline_spectrum(&mut prod, &mut em);
        assert!(outcome.is_noop());
    }

    #[test]
    fn missing_file_bytes_leaves_spectrum_alone() {
        let mut prod = GldfProduct::default();
        prod.general_definitions.spectrums = Some(Spectrums {
            spectrum: vec![Spectrum {
                id: "s1".into(),
                spectrum_file_reference: Some(SpectrumFileReference {
                    file_id: "missing".into(),
                }),
                intensity: vec![],
            }],
        });
        let mut em = HashMap::new();
        let outcome = apply_inline_spectrum(&mut prod, &mut em);
        assert!(outcome.is_noop());
        assert!(prod
            .general_definitions
            .spectrums
            .as_ref()
            .unwrap()
            .spectrum[0]
            .spectrum_file_reference
            .is_some());
    }
}
