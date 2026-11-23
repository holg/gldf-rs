//! Read-side inspection: walk a `GldfProduct` + its embedded asset
//! bytes and report every DIALux-import constraint violation we
//! know about. Mirrors gldf.net's `SpectrumDialuxCompatibilityRule`.
//!
//! Returns hints, never mutates. Use the [`super::sanitize`] helpers
//! to actually fix anything found.

use crate::GldfProduct;
use std::collections::HashMap;

/// Lower / upper wavelength bound DIALux *prefers* (per GLDF spec
/// recommendation). Empirically DIALux tolerates wavelengths outside
/// this range — it's an advisory hint, not a hard refusal.
pub const DIALUX_PREFERRED_MIN_NM: i32 = 380;
pub const DIALUX_PREFERRED_MAX_NM: i32 = 780;

/// One DIALux-compatibility issue found in a `GldfProduct`.
#[derive(Debug, Clone, PartialEq)]
pub enum DialuxCompatHint {
    /// A `spectrum/text` payload contains one or more `#` lines.
    /// **HARD blocker:** DIALux silently aborts the entire import.
    /// Fix with [`super::sanitize_spd`] on the bytes before zipping.
    SpectrumHasComments {
        spectrum_id: String,
        file_id: String,
        file_name: String,
    },
    /// Wavelengths fall outside the spec-recommended 380–780 nm
    /// range. DIALux currently accepts these (manual import 360–830
    /// confirmed working), but they're flagged for spec
    /// conformance. Listed wavelengths are the offending values.
    SpectrumOutOfRange {
        spectrum_id: String,
        /// `None` if the spectrum is inline, otherwise the file the
        /// out-of-range wavelengths were read from.
        file_id: Option<String>,
        wavelengths_nm: Vec<i32>,
    },
}

/// Run every DIALux-compat check against `product` using the
/// embedded asset bytes keyed by file id (matches the shape used by
/// [`crate::editable::EditableGldf::embedded_files`]).
///
/// File-based spectra whose bytes are not in `embedded_files` are
/// skipped silently (they're typically URL-referenced and we can't
/// inspect them offline). Inline `<Intensity>` spectra are always
/// checked.
pub fn dialux_compat_check(
    product: &GldfProduct,
    embedded_files: &HashMap<String, Vec<u8>>,
) -> Vec<DialuxCompatHint> {
    let mut hints = Vec::new();

    let Some(spectrums) = product.general_definitions.spectrums.as_ref() else {
        return hints;
    };

    for spec in &spectrums.spectrum {
        // Inline branch
        if !spec.intensity.is_empty() {
            let bad: Vec<i32> = spec
                .intensity
                .iter()
                .filter_map(|i| i.wavelength)
                .filter(|w| out_of_range(*w))
                .collect();
            let bad = dedup_sorted(bad);
            if !bad.is_empty() {
                hints.push(DialuxCompatHint::SpectrumOutOfRange {
                    spectrum_id: spec.id.clone(),
                    file_id: None,
                    wavelengths_nm: bad,
                });
            }
            continue;
        }

        // File branch
        let Some(file_ref) = spec.spectrum_file_reference.as_ref() else {
            continue;
        };
        let Some(bytes) = embedded_files.get(&file_ref.file_id) else {
            // Bytes not loaded (URL-only, or stripped) → can't check.
            continue;
        };
        // Resolve the human file name via the File table for the
        // hint payload — purely for diagnostics; matching is by id.
        let file_name = product
            .general_definitions
            .files
            .file
            .iter()
            .find(|f| f.id == file_ref.file_id)
            .map(|f| f.file_name.clone())
            .unwrap_or_else(|| file_ref.file_id.clone());

        let text = String::from_utf8_lossy(bytes);

        if text.lines().any(|l| l.trim_start().starts_with('#')) {
            hints.push(DialuxCompatHint::SpectrumHasComments {
                spectrum_id: spec.id.clone(),
                file_id: file_ref.file_id.clone(),
                file_name: file_name.clone(),
            });
        }

        let bad: Vec<i32> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| l.split([' ', '\t']).next())
            .filter_map(|tok| tok.parse::<i32>().ok())
            .filter(|w| out_of_range(*w))
            .collect();
        let bad = dedup_sorted(bad);
        if !bad.is_empty() {
            hints.push(DialuxCompatHint::SpectrumOutOfRange {
                spectrum_id: spec.id.clone(),
                file_id: Some(file_ref.file_id.clone()),
                wavelengths_nm: bad,
            });
        }
    }

    hints
}

fn out_of_range(nm: i32) -> bool {
    !(DIALUX_PREFERRED_MIN_NM..=DIALUX_PREFERRED_MAX_NM).contains(&nm)
}

fn dedup_sorted(mut v: Vec<i32>) -> Vec<i32> {
    v.sort_unstable();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SPD with 3 comment lines + range 360–830 (the chapter-01-slv
    /// case from `docs/_dialux_compat_findings.md`) should produce
    /// both hint kinds.
    #[test]
    fn flags_both_comments_and_range_on_demo_spd() {
        let spd = b"# 2700K reference\n# Synthetic\n# header\n\
                    360\t0.014\n\
                    380\t0.024\n\
                    780\t0.91\n\
                    830\t0.85\n";
        let prod = build_product_with_spectrum_file("spd_warm", "spd1", "spectrum.spd");
        let mut files = HashMap::new();
        files.insert("spd1".to_string(), spd.to_vec());

        let hints = dialux_compat_check(&prod, &files);
        assert!(hints
            .iter()
            .any(|h| matches!(h, DialuxCompatHint::SpectrumHasComments { spectrum_id, .. } if spectrum_id == "spd_warm")));
        assert!(hints
            .iter()
            .any(|h| matches!(h, DialuxCompatHint::SpectrumOutOfRange { wavelengths_nm, .. } if wavelengths_nm == &vec![360, 830])));
    }

    #[test]
    fn clean_spd_produces_no_hints() {
        let spd = b"380\t0.5\n385\t0.6\n780\t0.4\n";
        let prod = build_product_with_spectrum_file("spd_ok", "spd1", "ok.spd");
        let mut files = HashMap::new();
        files.insert("spd1".to_string(), spd.to_vec());
        assert!(dialux_compat_check(&prod, &files).is_empty());
    }

    #[test]
    fn out_of_range_inline_spectrum_is_flagged() {
        use crate::gldf::general_definitions::photometries::{Intensity, Spectrum, Spectrums};
        let mut prod = GldfProduct::default();
        prod.general_definitions.spectrums = Some(Spectrums {
            spectrum: vec![Spectrum {
                id: "inline".to_string(),
                spectrum_file_reference: None,
                intensity: vec![
                    Intensity {
                        wavelength: Some(360),
                        value: Some(0.1),
                    },
                    Intensity {
                        wavelength: Some(400),
                        value: Some(0.5),
                    },
                    Intensity {
                        wavelength: Some(830),
                        value: Some(0.2),
                    },
                ],
            }],
        });
        let hints = dialux_compat_check(&prod, &HashMap::new());
        match hints.as_slice() {
            [DialuxCompatHint::SpectrumOutOfRange { wavelengths_nm, .. }] => {
                assert_eq!(wavelengths_nm, &vec![360, 830]);
            }
            other => panic!("unexpected hints: {other:?}"),
        }
    }

    fn build_product_with_spectrum_file(
        spec_id: &str,
        file_id: &str,
        file_name: &str,
    ) -> GldfProduct {
        use crate::gldf::general_definitions::files::File;
        use crate::gldf::general_definitions::photometries::{
            Spectrum, SpectrumFileReference, Spectrums,
        };
        let mut prod = GldfProduct::default();
        prod.general_definitions.files.file.push(File {
            id: file_id.to_string(),
            content_type: "spectrum/text".to_string(),
            type_attr: "localFileName".to_string(),
            language: String::new(),
            file_name: file_name.to_string(),
        });
        prod.general_definitions.spectrums = Some(Spectrums {
            spectrum: vec![Spectrum {
                id: spec_id.to_string(),
                spectrum_file_reference: Some(SpectrumFileReference {
                    file_id: file_id.to_string(),
                }),
                intensity: vec![],
            }],
        });
        prod
    }
}
