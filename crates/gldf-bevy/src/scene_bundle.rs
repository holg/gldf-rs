//! Native scene loading: build [`GldfSceneData`] from a `.gldf` on disk.
//!
//! On WASM the editor pushes the current variant's L3D / LDT / emitter
//! config into `localStorage` and the Bevy app polls it (see the
//! `load_*_from_storage` fns in `lib.rs`). Natively there is no
//! `localStorage` and no `window` bridge — the editor instead saves the
//! current product to a temp `.gldf` and launches `gldf-bevy-viewer
//! <path> --variant <idx>`. This module is the native counterpart of the
//! WASM `build_payload`: it opens that `.gldf`, resolves one variant's
//! geometry + photometry + per-emitter config, and returns a populated
//! [`GldfSceneData`] the existing Bevy systems already know how to render.
//!
//! The variant-resolution logic mirrors
//! `gldf-rs-wasm/.../pages/bevy_viewer.rs::build_payload`; the shared
//! per-emitter mapping comes from `gldf_rs::mapping::get_variant_emitter_data`
//! so the two paths agree on CCT / flux / emergency behaviour.

use std::collections::HashMap;

use gldf_rs::gldf::product_definitions::Variant;
use gldf_rs::GldfProduct;

use crate::{EmitterConfig, GldfSceneData, MountingConfig, SceneType};

/// Load a `.gldf` file and build the scene data for `variant_idx`.
///
/// `variant_idx` is clamped: an out-of-range index (or a product with no
/// variants) falls back to variant 0 / a bare scene, so the viewer always
/// opens *something* rather than erroring.
pub fn load_scene_from_gldf(
    path: &std::path::Path,
    variant_idx: usize,
) -> anyhow::Result<GldfSceneData> {
    let bytes = std::fs::read(path)?;
    let loaded = GldfProduct::load_gldf_from_buf_all(bytes)?;
    let product = loaded.gldf;

    // Same embedded-files map the editor builds (file_id / path / name →
    // bytes), so the resolvers below can look up by whichever key the
    // GLDF's <File> entries use.
    let embedded: HashMap<String, Vec<u8>> = loaded
        .files
        .into_iter()
        .filter_map(|f| {
            let content = f.content?;
            let key = f.file_id.or(f.path).or(f.name)?;
            Some((key, content))
        })
        .collect();

    Ok(build_scene_data(&product, &embedded, variant_idx))
}

/// Build [`GldfSceneData`] from an already-parsed product + embedded files.
///
/// Split out from [`load_scene_from_gldf`] so it can be unit-tested and so
/// a future in-process caller (if Bevy ever shares the editor's loaded
/// state) can skip the disk round trip.
pub fn build_scene_data(
    product: &GldfProduct,
    embedded: &HashMap<String, Vec<u8>>,
    variant_idx: usize,
) -> GldfSceneData {
    let mut data = GldfSceneData {
        scene_type: SceneType::Room,
        ..Default::default()
    };

    let Some(variants) = product.product_definitions.variants.as_ref() else {
        return data;
    };
    // Clamp: always render some variant.
    let variant = match variants.variant.get(variant_idx) {
        Some(v) => v,
        None => match variants.variant.first() {
            Some(v) => v,
            None => return data,
        },
    };

    data.l3d_data = resolve_variant_l3d_bytes(product, embedded, variant);
    data.ldt_data = resolve_variant_ldt(product, embedded, variant);
    data.emitter_config = resolve_emitter_config(product, variant);
    data.mounting = resolve_mounting(variant);
    data.housing_color_rgb = resolve_housing_color(variant);

    data
}

// ---- Resolvers (ported from the WASM build_payload; gldf-rs types only) ----

fn resolve_variant_l3d_bytes(
    product: &GldfProduct,
    embedded: &HashMap<String, Vec<u8>>,
    variant: &Variant,
) -> Option<Vec<u8>> {
    let geometry_id = variant
        .geometry
        .as_ref()?
        .model_geometry_reference
        .as_ref()?
        .geometry_id
        .clone();
    let model_geom = product
        .general_definitions
        .geometries
        .as_ref()?
        .model_geometry
        .iter()
        .find(|g| g.id == geometry_id)?;
    let file_id = &model_geom.geometry_file_reference.first()?.file_id;
    embedded.get(file_id).cloned()
}

fn resolve_variant_ldt(
    product: &GldfProduct,
    embedded: &HashMap<String, Vec<u8>>,
    variant: &Variant,
) -> Option<eulumdat::Eulumdat> {
    let mappings = gldf_rs::mapping::get_l3d_ldt_mappings(product);
    let mapping = mappings.iter().find(|m| m.variant_id == variant.id)?;
    let ldt_file_id = mapping.ldt_file_id.as_ref()?;
    let bytes = embedded.get(ldt_file_id).or_else(|| {
        mapping.ldt_file_name.as_ref().and_then(|n| embedded.get(n))
    })?;
    let text = String::from_utf8_lossy(bytes).into_owned();
    eulumdat::Eulumdat::parse(&text).ok()
}

fn resolve_emitter_config(product: &GldfProduct, variant: &Variant) -> Vec<EmitterConfig> {
    let data = gldf_rs::mapping::get_variant_emitter_data(product, &variant.id);
    data.emitters
        .iter()
        .filter(|e| !e.leo_name.is_empty())
        .map(|e| EmitterConfig {
            leo_name: e.leo_name.clone(),
            external_name: Some(e.leo_name.clone()),
            luminous_flux: e.luminous_flux,
            color_temperature: e.color_temperature,
            emergency_behavior: e.emergency_behavior.clone(),
        })
        .collect()
}

fn resolve_mounting(variant: &Variant) -> MountingConfig {
    let Some(m) = variant.mountings.as_ref() else {
        return MountingConfig::default();
    };
    if let Some(c) = m.ceiling.as_ref() {
        MountingConfig {
            mounting_type: Some("ceiling".into()),
            recessed_depth_mm: c.recessed.as_ref().map(|r| r.recessed_depth),
            pendant_length_mm: c.pendant.as_ref().map(|p| p.pendant_length),
            is_surface_mounted: c.surface_mounted.is_some(),
            ..Default::default()
        }
    } else if let Some(w) = m.wall.as_ref() {
        MountingConfig {
            mounting_type: Some("wall".into()),
            recessed_depth_mm: w.recessed.as_ref().map(|r| r.recessed_depth),
            mounting_height_mm: Some(w.mounting_height),
            is_surface_mounted: w.surface_mounted.is_some(),
            ..Default::default()
        }
    } else if let Some(g) = m.ground.as_ref() {
        use gldf_rs::gldf::product_definitions::{PoleIntegrated, PoleTop};
        let pole = g
            .pole_top
            .as_ref()
            .and_then(PoleTop::get_pole_height)
            .or_else(|| g.pole_integrated.as_ref().and_then(PoleIntegrated::get_pole_height));
        MountingConfig {
            mounting_type: Some("ground".into()),
            pole_height_mm: pole,
            is_surface_mounted: g.surface_mounted.is_some(),
            is_free_standing: g.free_standing.is_some(),
            ..Default::default()
        }
    } else if m.working_plane.is_some() {
        MountingConfig {
            mounting_type: Some("working_plane".into()),
            ..Default::default()
        }
    } else {
        MountingConfig::default()
    }
}

fn resolve_housing_color(variant: &Variant) -> Option<[f32; 3]> {
    variant
        .descriptive_attributes
        .as_ref()
        .and_then(|a| a.marketing.as_ref())
        .and_then(|m| m.housing_colors.as_ref())
        .and_then(|hc| hc.housing_color.first())
        .and_then(|c| c.ral)
        .and_then(gldf_rs::ral::by_number)
        .map(|e| e.rgb_f32())
}
