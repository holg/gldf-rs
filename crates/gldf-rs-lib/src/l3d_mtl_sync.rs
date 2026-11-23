//! Sync an L3D archive's material library from a variant's
//! `HousingColor`.
//!
//! Available only when the `l3d` feature is enabled (which pulls in
//! `l3d_rs` as a dep). The editor calls [`sync_l3d_mtl_for_housing_color`](crate::l3d_mtl_sync::sync_l3d_mtl_for_housing_color)
//! inside the `UpdateHousingColor` reducer arm; the WASM viewer's
//! three-d-based renderer picks up the new colour automatically when
//! it re-loads the L3D bytes from `embedded_files`.
//!
//! # The chain
//!
//! ```text
//! Variant
//!   └─ Geometry
//!       └─ ModelGeometryReference @geometryId  ──┐
//!                                                ▼
//! GeneralDefinitions.Geometries.ModelGeometry @id  =  geometryId
//!   └─ GeometryFileReference @fileId  ──┐
//!                                       ▼
//! embedded_files[fileId]  =  <bytes of .l3d ZIP>
//!   └─ unzip → MTL → set Kd to RAL sRGB → repack → write back
//! ```
//!
//! # Soft failure
//!
//! Most "missing piece" cases are *not* errors. A variant without a
//! `ModelGeometryReference` (bare emitter binding) returns
//! [`SyncOutcome::NoModelGeometry`](crate::l3d_mtl_sync::SyncOutcome::NoModelGeometry); a variant whose `HousingColor`
//! has no `@ral` (free-text name only) returns
//! [`SyncOutcome::NoRalAvailable`](crate::l3d_mtl_sync::SyncOutcome::NoRalAvailable). The caller is expected to swallow
//! these — they're the common path for variants the user hasn't yet
//! attached a 3D model to.
//!
//! Hard failures (corrupt L3D, MTL parse error, IO failure) propagate
//! as `Err(SyncError::…)` so the editor can surface them via
//! `state.last_error`.

use crate::gldf::GldfProduct;
use crate::ral;
use l3d_rs::l3d::L3dFile;
use l3d_rs::mtl;
use std::collections::HashMap;

/// Outcome of [`sync_l3d_mtl_for_housing_color`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncOutcome {
    /// The MTL was rewritten and the new L3D bytes are now in
    /// `embedded_files`. Carries the file id we touched.
    Updated { file_id: String, mtl_path: String },

    /// We synthesized a brand-new MTL (the L3D had none) and patched
    /// each OBJ to reference it. The new L3D bytes are in
    /// `embedded_files`.
    Synthesized { file_id: String, mtl_path: String },

    /// Variant has no `ModelGeometryReference`. Nothing to do; not an
    /// error.
    NoModelGeometry,

    /// `HousingColor` carries no `@ral` value (the user typed only a
    /// free-text name). Without a RAL number we don't know which RGB
    /// to write, so the sync silently no-ops.
    NoRalAvailable,

    /// `@ral` is set but isn't a known RAL Classic entry (e.g. user
    /// typed `1234`, which isn't a published RAL). Silent no-op.
    UnknownRal { ral: i32 },

    /// The referenced model-geometry id doesn't resolve to a
    /// declared `ModelGeometry`, or the geometry has no file
    /// references. Silent no-op — common in mid-edit states.
    NoModelGeometryBytes,

    /// The variant index is out of range. Silent no-op.
    NoSuchVariant,
}

/// Hard failures that should surface to the user.
#[derive(Debug)]
pub enum SyncError {
    /// The L3D ZIP bytes we found wouldn't parse as a ZIP archive.
    L3dParseFailed(std::io::Error),

    /// One of the MTLs inside the L3D archive failed to parse.
    MtlParseFailed(mtl::MtlParseError),

    /// Serializing the modified L3D back to ZIP bytes failed.
    L3dWriteFailed(std::io::Error),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::L3dParseFailed(e) => write!(f, "L3D archive is not a valid ZIP: {}", e),
            Self::MtlParseFailed(e) => write!(f, "embedded MTL failed to parse: {}", e),
            Self::L3dWriteFailed(e) => write!(f, "failed to repack L3D archive: {}", e),
        }
    }
}

impl std::error::Error for SyncError {}

/// Walk the product chain `Variant → ModelGeometryReference →
/// ModelGeometry → File → embedded_files`, read the L3D ZIP, set
/// every `Kd` to the RAL Classic RGB, repack, and write back to
/// `embedded_files`.
///
/// `variant_idx` is the index into `product.product_definitions.variants`.
/// We read the variant's *first* `HousingColor` (most variants have one;
/// when there are several the user gets whichever the array's first slot
/// holds — a deliberate simplification, since the L3D itself only has
/// one housing material to drive).
///
/// Returns [`SyncOutcome`] describing what we did. See the module
/// docs for the failure-mode policy.
pub fn sync_l3d_mtl_for_housing_color(
    embedded_files: &mut HashMap<String, Vec<u8>>,
    product: &GldfProduct,
    variant_idx: usize,
) -> Result<SyncOutcome, SyncError> {
    // Resolve the variant.
    let variant = match product
        .product_definitions
        .variants
        .as_ref()
        .and_then(|vs| vs.variant.get(variant_idx))
    {
        Some(v) => v,
        None => return Ok(SyncOutcome::NoSuchVariant),
    };

    // Pick the first HousingColor and its RAL number.
    let ral = match variant
        .descriptive_attributes
        .as_ref()
        .and_then(|a| a.marketing.as_ref())
        .and_then(|m| m.housing_colors.as_ref())
        .and_then(|hc| hc.housing_color.first())
        .and_then(|h| h.ral)
    {
        Some(r) => r,
        None => return Ok(SyncOutcome::NoRalAvailable),
    };

    // Resolve the RAL Classic entry → sRGB triple.
    let entry = match ral::by_number(ral) {
        Some(e) => e,
        None => return Ok(SyncOutcome::UnknownRal { ral }),
    };
    let rgb = entry.rgb_f32();

    // Resolve the Variant's ModelGeometryReference → ModelGeometry → File.id.
    let geometry_id = match variant
        .geometry
        .as_ref()
        .and_then(|g| g.model_geometry_reference.as_ref())
        .map(|m| m.geometry_id.clone())
    {
        Some(id) => id,
        None => return Ok(SyncOutcome::NoModelGeometry),
    };

    let file_id = match product
        .general_definitions
        .geometries
        .as_ref()
        .and_then(|gs| gs.model_geometry.iter().find(|g| g.id == geometry_id))
        .and_then(|g| g.geometry_file_reference.first())
        .map(|fr| fr.file_id.clone())
    {
        Some(id) => id,
        None => return Ok(SyncOutcome::NoModelGeometryBytes),
    };

    let l3d_bytes = match embedded_files.get(&file_id) {
        Some(b) => b.clone(),
        None => return Ok(SyncOutcome::NoModelGeometryBytes),
    };

    // Parse the L3D archive.
    let mut l3d_file = parse_l3d(&l3d_bytes)?;

    // Find every MTL inside and rewrite every material's Kd.
    let mtl_paths: Vec<String> = l3d_file.mtl_paths().into_iter().map(String::from).collect();
    let outcome = if mtl_paths.is_empty() {
        // No MTL at all → synthesize one with the RAL colour.
        // ensure_mtl writes one MTL per OBJ parent directory so
        // viewers can resolve `mtllib default.mtl` relative to the
        // OBJ that references it. We report the first synthesized
        // path (deterministic — `ensure_mtl` iterates assets in
        // declaration order).
        let default_mat = mtl::Material::flat_color("housing", rgb);
        let synthesized = l3d_file
            .ensure_mtl(default_mat)
            .map_err(SyncError::L3dWriteFailed)?;
        debug_assert!(synthesized, "ensure_mtl should report synthesis");
        let mtl_path = l3d_file
            .mtl_paths()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "default.mtl".to_string());
        SyncOutcome::Synthesized {
            file_id: file_id.clone(),
            mtl_path,
        }
    } else {
        // Rewrite every material's Kd in every MTL. We want the housing
        // colour to win regardless of which material the renderer ends
        // up picking — most L3Ds only have one housing material anyway,
        // and the few with multiple typically want them all to match.
        let primary_path = mtl_paths[0].clone();
        for path in &mtl_paths {
            let mut mtl = l3d_file
                .mtl(path)
                .expect("mtl_paths returned a path that asset() can't find")
                .map_err(SyncError::MtlParseFailed)?;
            for m in mtl.materials.iter_mut() {
                m.kd = Some(rgb);
            }
            l3d_file.replace_mtl(path, &mtl);
        }
        SyncOutcome::Updated {
            file_id: file_id.clone(),
            mtl_path: primary_path,
        }
    };

    // Repack and put the new bytes back.
    let new_bytes = l3d_file.to_buffer().map_err(SyncError::L3dWriteFailed)?;
    embedded_files.insert(file_id, new_bytes);

    Ok(outcome)
}

fn parse_l3d(bytes: &[u8]) -> Result<L3dFile, SyncError> {
    // l3d_rs::from_buffer doesn't expose IO errors — it returns a default
    // L3d on failure. We need to detect "this isn't a ZIP at all" so we
    // can surface a real error. Use a thin direct ZIP read instead.
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor).map_err(|e| {
        SyncError::L3dParseFailed(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    })?;
    let mut file = L3dFile::default();
    use std::io::Read;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| {
            SyncError::L3dParseFailed(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;
        if !entry.is_file() {
            continue;
        }
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(SyncError::L3dParseFailed)?;
        let name = entry.name().to_string();
        if name == "structure.xml" {
            file.structure = String::from_utf8_lossy(&buf).into_owned();
        } else {
            let size = buf.len() as u64;
            file.assets.push(l3d_rs::l3d::BufFile {
                name,
                content: buf,
                size,
            });
        }
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gldf::general_definitions::geometries::{GeometryFileReference, ModelGeometry};
    use crate::gldf::general_definitions::Files;
    use crate::gldf::product_definitions::DescriptiveAttributes;
    use crate::gldf::product_definitions::{
        Geometry, HousingColor, HousingColors, Marketing, ModelGeometryReference,
        ProductDefinitions, Variant, Variants,
    };

    /// Build the smallest L3D archive that exercises ensure_mtl —
    /// one OBJ, no MTL.
    fn make_minimal_l3d_no_mtl() -> Vec<u8> {
        let file = L3dFile {
            structure: "<Structure/>".to_string(),
            assets: vec![l3d_rs::l3d::BufFile {
                name: "geom/luminaire.obj".to_string(),
                content: b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n".to_vec(),
                size: 32,
            }],
        };
        file.to_buffer().unwrap()
    }

    /// Build a minimal L3D archive with an MTL referencing one material.
    fn make_minimal_l3d_with_mtl() -> Vec<u8> {
        let file = L3dFile {
            structure: "<Structure/>".to_string(),
            assets: vec![
                l3d_rs::l3d::BufFile {
                    name: "geom/luminaire.obj".to_string(),
                    content: b"mtllib luminaire.mtl\nusemtl housing\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n".to_vec(),
                    size: 0,
                },
                l3d_rs::l3d::BufFile {
                    name: "geom/luminaire.mtl".to_string(),
                    content: b"newmtl housing\nKa 0.1 0.1 0.1\nKd 0.5 0.5 0.5\nKs 0.2 0.2 0.2\nNs 100\nd 1\nillum 2\n".to_vec(),
                    size: 0,
                },
            ],
        };
        file.to_buffer().unwrap()
    }

    /// Build a `GldfProduct` with one variant pointing at one
    /// ModelGeometry that points at file_id "l3d-1".
    fn make_product(housing_ral: Option<i32>) -> GldfProduct {
        use crate::gldf::general_definitions::files::File as GldfFile;
        use crate::gldf::general_definitions::Geometries;
        use crate::gldf::LocaleFoo;
        let mut p = GldfProduct::default();
        p.general_definitions.files = Files {
            file: vec![GldfFile {
                id: "l3d-1".to_string(),
                content_type: "geo/l3d".to_string(),
                type_attr: "localFileName".to_string(),
                language: String::new(),
                file_name: "luminaire.l3d".to_string(),
            }],
        };
        p.general_definitions.geometries = Some(Geometries {
            simple_geometry: vec![],
            model_geometry: vec![ModelGeometry {
                id: "geom-1".to_string(),
                geometry_file_reference: vec![GeometryFileReference {
                    file_id: "l3d-1".to_string(),
                    level_of_detail: None,
                }],
            }],
        });
        p.product_definitions = ProductDefinitions {
            variants: Some(Variants {
                variant: vec![Variant {
                    id: "v1".to_string(),
                    geometry: Some(Geometry {
                        emitter_reference: vec![],
                        simple_geometry_reference: None,
                        model_geometry_reference: Some(ModelGeometryReference {
                            geometry_id: "geom-1".to_string(),
                            ..Default::default()
                        }),
                    }),
                    descriptive_attributes: Some(DescriptiveAttributes {
                        marketing: Some(Marketing {
                            housing_colors: Some(HousingColors {
                                housing_color: vec![HousingColor {
                                    ral: housing_ral,
                                    locale: LocaleFoo::default(),
                                }],
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };
        p
    }

    #[test]
    fn updates_existing_mtl_with_ral_rgb() {
        let product = make_product(Some(3020)); // Traffic red
        let mut embedded = HashMap::new();
        embedded.insert("l3d-1".to_string(), make_minimal_l3d_with_mtl());

        let outcome = sync_l3d_mtl_for_housing_color(&mut embedded, &product, 0).unwrap();
        match outcome {
            SyncOutcome::Updated { file_id, .. } => assert_eq!(file_id, "l3d-1"),
            other => panic!("expected Updated, got {:?}", other),
        }

        // Re-read the L3D and verify Kd matches RAL 3020.
        let new_bytes = embedded.get("l3d-1").unwrap().clone();
        let file = parse_l3d(&new_bytes).unwrap();
        let mtl = file.mtl("geom/luminaire.mtl").unwrap().unwrap();
        let housing = mtl.find("housing").unwrap();
        let kd = housing.kd.unwrap();
        let expected = ral::by_number(3020).unwrap().rgb_f32();
        for i in 0..3 {
            assert!(
                (kd[i] - expected[i]).abs() < 1e-3,
                "channel {} mismatch: kd={:?}, expected={:?}",
                i,
                kd,
                expected
            );
        }
    }

    #[test]
    fn synthesizes_mtl_when_none_present() {
        let product = make_product(Some(9007)); // Grey aluminium
        let mut embedded = HashMap::new();
        embedded.insert("l3d-1".to_string(), make_minimal_l3d_no_mtl());

        let outcome = sync_l3d_mtl_for_housing_color(&mut embedded, &product, 0).unwrap();
        match outcome {
            SyncOutcome::Synthesized { file_id, mtl_path } => {
                assert_eq!(file_id, "l3d-1");
                // MTL written next to the OBJ ("geom_1/luminaire.obj"
                // → "geom/default.mtl") so three-d / tobj can resolve
                // it relative to the OBJ's parent dir.
                assert_eq!(mtl_path, "geom/default.mtl");
            }
            other => panic!("expected Synthesized, got {:?}", other),
        }

        // Verify the synthesized MTL exists at the OBJ-relative path
        // with the right RAL colour.
        let new_bytes = embedded.get("l3d-1").unwrap().clone();
        let file = parse_l3d(&new_bytes).unwrap();
        assert!(file.mtl_paths().contains(&"geom/default.mtl"));
        let mtl = file.mtl("geom/default.mtl").unwrap().unwrap();
        let housing = mtl.find("housing").unwrap();
        let expected = ral::by_number(9007).unwrap().rgb_f32();
        let kd = housing.kd.unwrap();
        for i in 0..3 {
            assert!((kd[i] - expected[i]).abs() < 1e-3);
        }
    }

    #[test]
    fn no_housing_color_ral_is_silent_noop() {
        let product = make_product(None);
        let mut embedded = HashMap::new();
        let original = make_minimal_l3d_with_mtl();
        embedded.insert("l3d-1".to_string(), original.clone());

        let outcome = sync_l3d_mtl_for_housing_color(&mut embedded, &product, 0).unwrap();
        assert!(matches!(outcome, SyncOutcome::NoRalAvailable));
        // Bytes unchanged.
        assert_eq!(embedded.get("l3d-1").unwrap(), &original);
    }

    #[test]
    fn unknown_ral_is_silent_noop() {
        let product = make_product(Some(1234)); // not in the RAL Classic table
        let mut embedded = HashMap::new();
        let original = make_minimal_l3d_with_mtl();
        embedded.insert("l3d-1".to_string(), original.clone());

        let outcome = sync_l3d_mtl_for_housing_color(&mut embedded, &product, 0).unwrap();
        assert!(matches!(outcome, SyncOutcome::UnknownRal { ral: 1234 }));
        assert_eq!(embedded.get("l3d-1").unwrap(), &original);
    }

    #[test]
    fn variant_without_model_geometry_is_silent_noop() {
        let mut product = make_product(Some(3020));
        // Clear the ModelGeometryReference
        if let Some(vs) = product.product_definitions.variants.as_mut() {
            if let Some(v) = vs.variant.get_mut(0) {
                if let Some(g) = v.geometry.as_mut() {
                    g.model_geometry_reference = None;
                }
            }
        }
        let mut embedded = HashMap::new();
        let original = make_minimal_l3d_with_mtl();
        embedded.insert("l3d-1".to_string(), original.clone());

        let outcome = sync_l3d_mtl_for_housing_color(&mut embedded, &product, 0).unwrap();
        assert!(matches!(outcome, SyncOutcome::NoModelGeometry));
        assert_eq!(embedded.get("l3d-1").unwrap(), &original);
    }
}
