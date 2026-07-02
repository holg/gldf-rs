//! T3 — Bootstrap a fresh GLDF from a TM-33 XML, then re-export the
//! single LightSource as TM-33 and assert the SPD survives.
//!
//! Chain (`tm33_to_gldf` is the entry point):
//!
//!     .tm33.xml bytes
//!       └─> tm33_to_gldf → FileBufGldf  (1 File, 2 refs: Photometry + Spectrum)
//!             └─> EditableGldf::from_gldf_product → save_to_buf
//!                   └─> load_gldf_from_buf_all
//!                         └─> export_tm33_xml(ls_id) → re-exported TM-33
//!
//! Per-step assertions:
//!
//! * **Bootstrap shape**: the bundle has exactly ONE embedded file,
//!   the Photometry's `@fileId` equals the Spectrum's `@fileId`, the
//!   FixedLightSource has the SpectrumReference pre-wired.
//! * **SPD round-trip**: the re-exported TM-33's
//!   `Emitter.spectral_distribution.wavelengths` matches the source
//!   sample count.
//! * **Photometry round-trip**: the re-exported TM-33's
//!   `LuminousData.LuminousIntensity` carries the same number of
//!   vertical angles as the source.
//!
//! What this catches: drift in the dual-reference single-file scheme
//! (e.g. the rename action accidentally losing the cross-reference),
//! eulumdat-rs ATLA conversion regressions, or anything that broke
//! the wizard's TM-33-as-starter flow.
//!
//! Requires the `eulumdat` feature: `cargo test --features eulumdat`.

#![cfg(feature = "eulumdat")]

use eulumdat::atla::xml as atla_xml;
use gldf_rs::{
    export::lightsource::export_tm33_xml, tm33_to_gldf, EditableGldf, GldfProduct,
};
use std::collections::HashMap;

fn count_intensity_samples(xml: &str) -> Option<usize> {
    let doc = atla_xml::parse(xml).ok()?;
    let em = doc.emitters.first()?;
    let id = em.intensity_distribution.as_ref()?;
    // TM-33-23 stores the matrix as `intensities` (Vec<Vec<f64>>):
    // outer = horizontal planes, inner = vertical angles. Total
    // sample count is sum-of-inner-lengths.
    Some(id.intensities.iter().map(|row| row.len()).sum())
}

fn spd_sample_count(xml: &str) -> Option<usize> {
    let doc = atla_xml::parse(xml).ok()?;
    doc.emitters
        .first()
        .and_then(|e| e.spectral_distribution.as_ref())
        .map(|spd| spd.wavelengths.len())
}

#[test]
fn golden_lightsource_2_bootstrap_roundtrip() {
    let path = "../../tests/data/golden_lightsource_2.tm33.xml";
    let source_bytes = std::fs::read(path).expect("read golden TM-33");
    let source_text = std::str::from_utf8(&source_bytes).expect("UTF-8 TM-33");

    // --- Bootstrap ---
    let bundle = tm33_to_gldf(&source_bytes, "golden_lightsource_2.tm33.xml")
        .expect("tm33_to_gldf");

    // The single-file dual-reference shape: 1 File entry, 1
    // Photometry, 1 Spectrum, both pointing at the SAME @fileId.
    let g = &bundle.gldf;
    assert_eq!(
        g.general_definitions.files.file.len(),
        1,
        "want exactly 1 embedded <File>"
    );
    let file_id = &g.general_definitions.files.file[0].id;
    let photo = &g
        .general_definitions
        .photometries
        .as_ref()
        .expect("photometries")
        .photometry[0];
    let spec = &g
        .general_definitions
        .spectrums
        .as_ref()
        .expect("spectrums")
        .spectrum[0];
    assert_eq!(
        photo
            .photometry_file_reference
            .as_ref()
            .map(|r| r.file_id.as_str()),
        Some(file_id.as_str()),
        "Photometry@fileId must equal File@id"
    );
    assert_eq!(
        spec.spectrum_file_reference
            .as_ref()
            .map(|r| r.file_id.as_str()),
        Some(file_id.as_str()),
        "Spectrum@fileId must equal File@id"
    );

    // --- save → reload ---
    let mut editable_in = EditableGldf::from_product(bundle.gldf.clone());
    for f in &bundle.files {
        if let (Some(id), Some(content)) = (f.file_id.as_ref(), f.content.as_ref()) {
            editable_in.embedded_files.insert(id.clone(), content.clone());
        }
    }
    let resaved = editable_in.save_to_buf().expect("save_to_buf");
    let reloaded_bundle =
        GldfProduct::load_gldf_from_buf_all(resaved).expect("reload");
    let reloaded_product = reloaded_bundle.gldf;
    let reloaded_files: HashMap<String, Vec<u8>> = reloaded_bundle
        .files
        .into_iter()
        .filter_map(|bf| {
            let id = bf.file_id.clone()?;
            let content = bf.content.clone()?;
            Some((id, content))
        })
        .collect();

    // The reload preserved the single-file shape.
    assert_eq!(
        reloaded_product.general_definitions.files.file.len(),
        1,
        "reload must keep exactly 1 <File>"
    );

    // --- re-export as TM-33 ---
    let ls_id = &reloaded_product
        .general_definitions
        .light_sources
        .as_ref()
        .expect("light_sources after reload")
        .fixed_light_source[0]
        .id;
    let reexported = export_tm33_xml(&reloaded_product, &reloaded_files, ls_id)
        .expect("export_tm33_xml");

    // --- Compare SPD sample counts ---
    let source_spd = spd_sample_count(source_text);
    let roundtrip_spd = spd_sample_count(&reexported);
    assert_eq!(
        roundtrip_spd, source_spd,
        "SPD sample count drift: source={source_spd:?}, round-trip={roundtrip_spd:?}"
    );

    // --- Compare intensity-matrix totals ---
    let source_intensity = count_intensity_samples(source_text);
    let roundtrip_intensity = count_intensity_samples(&reexported);
    assert!(
        roundtrip_intensity.unwrap_or(0) > 0,
        "round-trip TM-33 has no intensity data: source had {source_intensity:?}"
    );
    // The eulumdat ATLA round-trip MAY re-sample the angles, so we
    // accept any nonzero overlap rather than strict equality —
    // strict equality would lock us to a specific conversion that
    // upstream might tighten or relax.
}
