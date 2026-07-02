//! TM-33 merge transform.
//!
//! Walks every `FixedLightSource` (and similar) that carries BOTH a
//! `PhotometryReference` (resolved via emitter or direct) AND a
//! `SpectrumReference`, and produces a single TM-33 (`ldc/iesxml`)
//! file that combines the two. The original `ldc/eulumdat`/`ldc/ies`
//! photometry payload and the `spectrum/text` spectrum payload are
//! removed from the archive; the `<Spectrum>` entry is dropped;
//! `<PhotometryFileReference>` is rewritten to point at the new
//! TM-33 file.
//!
//! Photometries without an associated spectrum are deliberately left
//! alone — bare `.ldt`/`.ies` is the canonical shape for them.
//!
//! Driven by [`crate::editable::ExportOptions::merge_into_tm33`].
//! Gated behind `feature = "eulumdat"` because TM-33 generation
//! delegates to the existing per-LightSource writer in
//! [`crate::export::lightsource::export_tm33_xml`].

use crate::gldf::general_definitions::files::File as GldfFile;
use crate::GldfProduct;
use anyhow::{Context, Result};
use std::collections::HashMap;

/// What [`apply_tm33_merge`] actually did.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Tm33MergeOutcome {
    /// `(light_source_id, new_tm33_file_id)` for each merge that ran.
    pub merged: Vec<(String, String)>,
    /// File ids removed: original photometry .ldt/.ies + original spectrum .spd.
    pub files_removed: Vec<String>,
    /// Spectrum ids whose `<Spectrum>` entry was dropped.
    pub spectrums_removed: Vec<String>,
}

impl Tm33MergeOutcome {
    pub fn is_noop(&self) -> bool {
        self.merged.is_empty()
    }
}

/// Replace each photometry+spectrum pair with a single TM-33 file.
///
/// Walks `general_definitions.light_sources.fixed_light_source` —
/// changeable + multi-channel sources are out of scope for slice 3
/// (they have different photometry plumbing). Each FixedLightSource
/// that has a `spectrum_reference` is the merge candidate; its
/// photometry id is resolved via the emitter walker, then we build
/// the TM-33 XML using the existing per-LS writer.
pub fn apply_tm33_merge(
    product: &mut GldfProduct,
    embedded_files: &mut HashMap<String, Vec<u8>>,
) -> Result<Tm33MergeOutcome> {
    let mut outcome = Tm33MergeOutcome::default();

    // Phase 1: enumerate candidates. We CAN'T mutate the product while
    // walking it (TM-33 generation needs read-only access to the full
    // graph), so collect (ls_id, photometry_id, spectrum_id) tuples up
    // front, then mutate in phase 2.
    let candidates = enumerate_merge_candidates(product);

    for cand in &candidates {
        // Build TM-33 XML for this LS via the existing writer.
        let tm33_xml = crate::export::lightsource::export_tm33_xml(
            product,
            embedded_files,
            &cand.light_source_id,
        )
        .with_context(|| {
            format!(
                "build TM-33 XML for light source {}",
                cand.light_source_id
            )
        })?;

        // Pick the photometry file's original name, swapping its
        // extension to `.xml` so DIALux + others recognise the
        // content type by extension.
        let original_phot = product
            .general_definitions
            .files
            .file
            .iter()
            .find(|f| f.id == cand.phot_file_id)
            .cloned();
        let tm33_file_name = original_phot
            .as_ref()
            .map(|f| swap_extension(&f.file_name, "xml"))
            .unwrap_or_else(|| format!("{}.xml", cand.light_source_id));
        let tm33_file_id = format!("{}_tm33", cand.phot_file_id);

        // Update the Photometry's file reference to point at the new
        // TM-33 file id. PhotometryFileReference is optional on the
        // model; merge only applies when the candidate already had
        // one, so the unwrap-set here is safe.
        if let Some(phot_block) = product.general_definitions.photometries.as_mut() {
            for phot in phot_block.photometry.iter_mut() {
                if phot.id == cand.photometry_id {
                    if let Some(ref_mut) = phot.photometry_file_reference.as_mut() {
                        ref_mut.file_id = tm33_file_id.clone();
                    }
                }
            }
        }

        // Drop the SpectrumReference from the FixedLightSource — the
        // SPD now lives inside TM-33.
        if let Some(lss) = product.general_definitions.light_sources.as_mut() {
            for fls in lss.fixed_light_source.iter_mut() {
                if fls.id == cand.light_source_id {
                    fls.spectrum_reference = None;
                }
            }
        }

        // Drop the <Spectrum> entry (its bytes leave with the SPD file).
        if let Some(spectrums) = product.general_definitions.spectrums.as_mut() {
            spectrums
                .spectrum
                .retain(|s| s.id != cand.spectrum_id);
        }
        outcome.spectrums_removed.push(cand.spectrum_id.clone());

        // Remove old File entries and their bytes.
        product
            .general_definitions
            .files
            .file
            .retain(|f| f.id != cand.phot_file_id && f.id != cand.spd_file_id);
        embedded_files.remove(&cand.phot_file_id);
        embedded_files.remove(&cand.spd_file_id);
        outcome
            .files_removed
            .push(cand.phot_file_id.clone());
        outcome.files_removed.push(cand.spd_file_id.clone());

        // Add the new TM-33 File entry + bytes.
        product.general_definitions.files.file.push(GldfFile {
            id: tm33_file_id.clone(),
            content_type: "ldc/iesxml".to_string(),
            type_attr: "localFileName".to_string(),
            language: String::new(),
            file_name: tm33_file_name,
        });
        embedded_files.insert(tm33_file_id.clone(), tm33_xml.into_bytes());

        outcome
            .merged
            .push((cand.light_source_id.clone(), tm33_file_id));
    }

    // If the <Spectrums> block is now empty, drop it entirely (XSD
    // requires <Spectrum minOccurs=1> inside it — same rule as the
    // <Sensors> wrapper).
    if let Some(spectrums) = product.general_definitions.spectrums.as_ref() {
        if spectrums.spectrum.is_empty() {
            product.general_definitions.spectrums = None;
        }
    }

    Ok(outcome)
}

struct MergeCandidate {
    light_source_id: String,
    photometry_id: String,
    phot_file_id: String,
    spectrum_id: String,
    spd_file_id: String,
}

fn enumerate_merge_candidates(product: &GldfProduct) -> Vec<MergeCandidate> {
    let mut out = Vec::new();

    let Some(lss) = product.general_definitions.light_sources.as_ref() else {
        return out;
    };

    for fls in &lss.fixed_light_source {
        let Some(spec_ref) = fls.spectrum_reference.as_ref() else {
            continue;
        };

        // Photometry binding lives on the emitter — find it.
        let Some(phot_id) = find_photometry_for_fixed_ls(product, &fls.id) else {
            continue;
        };

        // Photometry → file id
        let Some(phot_file_id) =
            photometry_file_id(product, &phot_id)
        else {
            continue;
        };

        // Spectrum → file id (only file-backed spectra; inline ones
        // can't be merged — TM-33 needs the raw SPD bytes anyway).
        let Some(spd_file_id) = spectrum_file_id(product, &spec_ref.spectrum_id)
        else {
            continue;
        };

        out.push(MergeCandidate {
            light_source_id: fls.id.clone(),
            photometry_id: phot_id,
            phot_file_id,
            spectrum_id: spec_ref.spectrum_id.clone(),
            spd_file_id,
        });
    }

    out
}

fn find_photometry_for_fixed_ls(product: &GldfProduct, ls_id: &str) -> Option<String> {
    let emitters = product.general_definitions.emitters.as_ref()?;
    for em in &emitters.emitter {
        for fle in &em.fixed_light_emitter {
            if fle.light_source_reference.fixed_light_source_id.as_deref() == Some(ls_id) {
                return Some(fle.photometry_reference.photometry_id.clone());
            }
        }
    }
    None
}

fn photometry_file_id(product: &GldfProduct, phot_id: &str) -> Option<String> {
    let p = product.general_definitions.photometries.as_ref()?;
    p.photometry
        .iter()
        .find(|p| p.id == phot_id)
        .and_then(|p| p.photometry_file_reference.as_ref())
        .map(|r| r.file_id.clone())
}

fn spectrum_file_id(product: &GldfProduct, spec_id: &str) -> Option<String> {
    let s = product.general_definitions.spectrums.as_ref()?;
    s.spectrum
        .iter()
        .find(|s| s.id == spec_id)
        .and_then(|s| s.spectrum_file_reference.as_ref())
        .map(|r| r.file_id.clone())
}

fn swap_extension(file_name: &str, new_ext: &str) -> String {
    if let Some(dot) = file_name.rfind('.') {
        format!("{}.{}", &file_name[..dot], new_ext)
    } else {
        format!("{}.{}", file_name, new_ext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// When there are no spectra at all (the wizard wasn't told
    /// about any SPDs), the transform is a no-op.
    #[test]
    fn no_spectra_is_noop() {
        let mut prod = GldfProduct::default();
        let mut em = HashMap::new();
        let outcome = apply_tm33_merge(&mut prod, &mut em).unwrap();
        assert!(outcome.is_noop());
    }

    /// FixedLightSource with no SpectrumReference is left alone even
    /// when spectra exist for OTHER light sources.
    #[test]
    fn bare_photometry_left_alone() {
        use crate::gldf::general_definitions::lightsources::{FixedLightSource, LightSources};
        let mut prod = GldfProduct::default();
        prod.general_definitions.light_sources = Some(LightSources {
            fixed_light_source: vec![FixedLightSource {
                id: "ls1".into(),
                ..FixedLightSource::default()
            }],
            ..LightSources::default()
        });
        let mut em = HashMap::new();
        let outcome = apply_tm33_merge(&mut prod, &mut em).unwrap();
        assert!(outcome.is_noop());
    }

    #[test]
    fn swap_extension_basic() {
        assert_eq!(swap_extension("foo.ldt", "xml"), "foo.xml");
        assert_eq!(swap_extension("a.b.ies", "xml"), "a.b.xml");
        assert_eq!(swap_extension("noext", "xml"), "noext.xml");
    }
}
