//! Per-emitter IES emission for a variant.
//!
//! Composes `gldf_rs::photometry` calls in the order an Unreal export
//! needs:
//!
//! 1. [`gldf_rs::resolve_variant_photometries`] gives us one
//!    [`VariantPhotometryResolution`] per emitter on the variant, with
//!    resolved `lumens` / `watts`.
//! 2. For each resolution, look up the raw LDT bytes by `photometry_id`
//!    (walk `general_definitions.photometries[].photometry_file_reference`
//!    → `general_definitions.files[].file_id` → `buf.files[].name`).
//! 3. Patch them with [`gldf_rs::patch_ldt_for_variant`].
//! 4. Parse the patched bytes as `eulumdat::Eulumdat` and run
//!    [`gldf_rs::export_photometry`] with `PhotometryExportFormat::Ies`
//!    → IES LM-63 bytes that Unreal imports natively as
//!    `UTextureLightProfile`.
//!
//! Emergency-only emitters are skipped (matches viewer behaviour —
//! emergency flux vs full-load wattage is a meaningless ratio).

use eulumdat::Eulumdat;
use gldf_rs::{
    export_photometry, patch_ldt_for_variant, FileBufGldf, GldfProduct, PhotometryExportFormat,
    VariantPhotometryResolution,
};

use crate::error::{ExportError, Result};

/// One ready-to-write IES output, paired with the resolution it came
/// from so the caller can attach the matching `<ActorLight>`.
pub struct EmitterIes {
    pub photometry_id: String,
    pub light_source_id: Option<String>,
    pub lumens: Option<i32>,
    pub watts: Option<f64>,
    pub emergency_behaviour: Option<String>,
    pub control_gear_id: Option<String>,
    /// IES LM-63 bytes (UTF-8 text). Filename suggestion is
    /// `emitter_<photometry_id>.ies`.
    pub ies_bytes: Vec<u8>,
}

/// Build IES outputs for every non-emergency emitter on `variant_id`.
///
/// Returns an empty Vec when the variant has no emitters; returns an
/// error only when something *should* have worked but didn't (e.g. an
/// LDT file referenced by id is missing from the GLDF bundle, or the
/// resolved LDT bytes can't be parsed as EULUMDAT).
pub fn build_ies_outputs_for_variant(
    buf: &FileBufGldf,
    variant_id: &str,
) -> Result<Vec<EmitterIes>> {
    let variant = find_variant(&buf.gldf, variant_id)?;
    let resolutions = gldf_rs::resolve_variant_photometries(
        &buf.gldf.product_definitions,
        variant,
        &buf.gldf.general_definitions,
    );

    let mut out = Vec::with_capacity(resolutions.len());
    for res in resolutions {
        if res.is_emergency_only() {
            // Emergency-only emitters carry the emergency flux, not the
            // normal-mode flux. Patching with that value vs full-load
            // wattage produces a meaningless ratio; skip the export.
            continue;
        }
        let Some(ldt_bytes) = lookup_ldt_bytes(buf, &res.photometry_id) else {
            return Err(ExportError::Photometry(format!(
                "photometry id {:?} referenced by variant {:?} but no LDT \
                 file is bundled with that reference",
                res.photometry_id, variant_id
            )));
        };

        let patched = patch_ldt_for_variant(&ldt_bytes, res.lumens, res.watts)
            .unwrap_or(ldt_bytes);
        let ldt_str = std::str::from_utf8(&patched).map_err(|e| {
            ExportError::Photometry(format!("LDT for {:?} is not UTF-8: {e}", res.photometry_id))
        })?;
        let ldt = Eulumdat::parse(ldt_str).map_err(|e| {
            ExportError::Photometry(format!(
                "EULUMDAT parse failed for {:?}: {e}",
                res.photometry_id
            ))
        })?;
        let ies_bytes = export_photometry(&ldt, PhotometryExportFormat::Ies).ok_or_else(|| {
            ExportError::Photometry(format!(
                "IES export failed for {:?}",
                res.photometry_id
            ))
        })?;

        out.push(emitter_ies_from(res, ies_bytes));
    }
    Ok(out)
}

fn emitter_ies_from(res: VariantPhotometryResolution, ies_bytes: Vec<u8>) -> EmitterIes {
    EmitterIes {
        photometry_id: res.photometry_id,
        light_source_id: res.light_source_id,
        lumens: res.lumens,
        watts: res.watts,
        emergency_behaviour: res.emergency_behaviour,
        control_gear_id: res.control_gear_id,
        ies_bytes,
    }
}

/// Find a variant by id in the GLDF document.
fn find_variant<'a>(
    gldf: &'a GldfProduct,
    variant_id: &str,
) -> Result<&'a gldf_rs::gldf::product_definitions::Variant> {
    gldf.product_definitions
        .variants
        .as_ref()
        .and_then(|v| v.variant.iter().find(|x| x.id == variant_id))
        .ok_or_else(|| ExportError::VariantNotFound(variant_id.to_string()))
}

/// Resolve `photometry_id` → LDT bytes by walking
/// `<Photometries>` → `<File>` → bundled file content.
fn lookup_ldt_bytes(buf: &FileBufGldf, photometry_id: &str) -> Option<Vec<u8>> {
    let photometries = buf.gldf.general_definitions.photometries.as_ref()?;
    let phot = photometries
        .photometry
        .iter()
        .find(|p| p.id == photometry_id)?;
    let file_ref = phot.photometry_file_reference.as_ref()?;
    let file_id = &file_ref.file_id;

    // Walk <Files> for the file entry whose `id` matches; pick out the
    // `file_name` we'd find in the bundled buf.files.
    let file_entry = buf.gldf.general_definitions.files.file.iter().find(|f| &f.id == file_id)?;
    let file_name = file_entry.file_name.as_str();

    // Match on either the original name or the in-zip path suffix.
    buf.files.iter().find_map(|bf| {
        let name_match = bf.name.as_deref() == Some(file_name);
        let path_match = bf
            .path
            .as_deref()
            .map(|p| p.ends_with(file_name) || p.ends_with(&format!("/{file_name}")))
            .unwrap_or(false);
        if name_match || path_match {
            bf.content.clone()
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Option<PathBuf> {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("tests/data")
            .join(name);
        if p.exists() {
            Some(p.canonicalize().unwrap())
        } else {
            None
        }
    }

    #[test]
    fn alurays_first_variant_produces_at_least_one_ies() {
        let Some(p) = fixture("alurays-3000mm.gldf") else {
            eprintln!("skipping: fixture missing");
            return;
        };
        let exporter = crate::Exporter::from_path(&p).unwrap();
        let buf = exporter.file_buf();
        // Take the first variant id.
        let vid = buf
            .gldf
            .product_definitions
            .variants
            .as_ref()
            .and_then(|v| v.variant.first())
            .map(|v| v.id.clone())
            .expect("alurays has at least one variant");

        let outs = build_ies_outputs_for_variant(buf, &vid).expect("ies build");
        assert!(!outs.is_empty(), "should resolve at least one emitter");
        for o in &outs {
            assert!(!o.ies_bytes.is_empty(), "ies bytes are not empty");
            // LM-63 marker is the most reliable sentinel.
            assert!(
                o.ies_bytes
                    .windows(8)
                    .any(|w| w.starts_with(b"IESNA") || w.starts_with(b"IES:") || w.starts_with(b"TILT=")),
                "IES output looks like LM-63"
            );
        }
    }
}
