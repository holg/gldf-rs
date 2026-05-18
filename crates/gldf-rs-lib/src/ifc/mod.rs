//! IFC (Industry Foundation Classes) Integration
//!
//! This module provides GLDF to IFC export functionality for BIM integration.
//!
//! ## Overview
//!
//! IFC is the open standard for Building Information Modeling (BIM).
//! GLDF luminaires can be exported to IFC for use in architectural software.
//!
//! ## Implementation
//!
//! We generate IFC STEP files directly without external dependencies,
//! since existing Rust IFC libraries (like ifc_rs) don't yet support
//! lighting entities (IfcLightFixture, IfcLightSourceGoniometric, etc.).
//!
//! ## Variant Handling
//!
//! Each GLDF variant is exported as a separate `IFCLIGHTFIXTURETYPE`.
//! This allows BIM software to correctly:
//! - Display different variants as separate luminaire types
//! - Include variant-specific photometric data
//! - Enable quantity takeoffs per variant
//!
//! ## Example
//!
//! ```rust,ignore
//! use gldf_rs::ifc::GldfToIfc;
//!
//! let gldf = GldfProduct::from_file("luminaire.gldf")?;
//! let ifc_content = GldfToIfc::export(&gldf)?;
//! std::fs::write("luminaire.ifc", ifc_content)?;
//! ```

mod gldf_generator;
mod ifc_import;
mod step_parser;
mod step_writer;
mod types;

pub use gldf_generator::GldfGenerator;
pub use ifc_import::{
    DistributionPlane, EmbeddedFile, IfcImporter, ImportedDistribution, ImportedGeometry,
    ImportedLightSource, ImportedLuminaire, ImportedPhotometry, ImportedProperties,
    ImportedVariant, ProductDescriptiveAttributes,
};
pub use step_parser::{StepEntity, StepParser, StepValue};
pub use step_writer::{MeshData, StepWriter};
pub use types::*;

use crate::gldf::GldfProduct;
use crate::BufFile;
use anyhow::Result;
use std::collections::HashMap;

/// Data extracted for a single variant's light emitter
#[allow(dead_code)]
struct VariantEmitterData {
    emitter_id: String,
    name: String,
    luminous_flux: Option<f64>,
    color_temperature: Option<i32>,
    color_rendering_index: Option<i32>,
    rated_power: Option<f64>,
    photometry_id: Option<String>,
    photometry_file: Option<String>,
}

/// Data extracted for a complete variant
#[allow(dead_code)]
struct VariantData {
    id: String,
    name: String,
    description: Option<String>,
    emitters: Vec<VariantEmitterData>,
    mounting_type: Option<String>,
    weight: Option<f64>,
    recessed_depth: Option<f64>,
}

/// LDT parsing result with distribution data and metadata
#[cfg(feature = "eulumdat")]
struct LdtParseResult {
    distribution: Vec<(f64, f64, f64)>,
    metadata: LdtMetadataExport,
}

/// LDT metadata for export to IFC
#[cfg(feature = "eulumdat")]
#[derive(Debug, Clone, Default)]
struct LdtMetadataExport {
    symmetry: i32,
    num_c_planes: i32,
    num_g_angles: i32,
    dc: f64,
    dg: f64,
    total_flux: f64,
    dr: [f64; 10],
    luminaire_name: String,
    /// Complete LDT file content for exact roundtrip preservation
    raw_content: String,
}

/// Stub structs when eulumdat feature is not enabled
#[cfg(not(feature = "eulumdat"))]
struct LdtParseResult {
    distribution: Vec<(f64, f64, f64)>,
    metadata: LdtMetadataExport,
}

#[cfg(not(feature = "eulumdat"))]
#[derive(Debug, Clone, Default)]
struct LdtMetadataExport {
    symmetry: i32,
    num_c_planes: i32,
    num_g_angles: i32,
    dc: f64,
    dg: f64,
    total_flux: f64,
    dr: [f64; 10],
    luminaire_name: String,
    raw_content: String,
}

/// Export GLDF to IFC STEP format
pub struct GldfToIfc;

impl GldfToIfc {
    /// Export a GLDF product to IFC STEP format
    ///
    /// Creates one `IFCLIGHTFIXTURETYPE` per GLDF variant, with proper
    /// photometric data and property sets.
    ///
    /// Returns the IFC file content as a string.
    pub fn export(gldf: &GldfProduct) -> Result<String> {
        Self::export_with_mesh(gldf, None)
    }

    /// Export a GLDF product to IFC STEP format with optional L3D mesh data
    ///
    /// # Arguments
    /// * `gldf` - The GLDF product data
    /// * `mesh_data` - Optional triangulated mesh from L3D. If provided, creates
    ///   tessellated geometry (IFCTRIANGULATEDFACESET). If None,
    ///   creates a default box geometry (IFCEXTRUDEDAREASOLID).
    ///
    /// # Returns
    /// The IFC file content as a string
    pub fn export_with_mesh(gldf: &GldfProduct, mesh_data: Option<&MeshData>) -> Result<String> {
        Self::export_with_mesh_and_files(gldf, mesh_data, &[])
    }

    /// Export a GLDF product to IFC STEP format with mesh data and embedded files
    ///
    /// # Arguments
    /// * `gldf` - The GLDF product data
    /// * `mesh_data` - Optional triangulated mesh from L3D
    /// * `files` - BufFile array containing LDT photometry files to embed in IFC
    ///
    /// # Returns
    /// The IFC file content as a string
    pub fn export_with_mesh_and_files(
        gldf: &GldfProduct,
        mesh_data: Option<&MeshData>,
        files: &[BufFile],
    ) -> Result<String> {
        let mut writer = StepWriter::new("IFC4");

        // Get product info
        let manufacturer = &gldf.header.manufacturer;
        let product_name = Self::get_product_name(gldf);

        // Create minimal building structure
        let owner_history = writer.add_owner_history(manufacturer);
        let project = writer.add_project("GLDF Export", owner_history);
        let site = writer.add_site("Default Site", owner_history, project);
        let building = writer.add_building("Default Building", owner_history, site);
        let storey = writer.add_storey("Ground Floor", owner_history, building);

        // Build lookup tables
        let photometry_files = Self::build_photometry_file_map(gldf);
        let light_sources = Self::build_light_source_map(gldf);
        let emitter_map = Self::build_emitter_map(gldf);

        // Extract variants
        let variants =
            Self::extract_variants(gldf, &photometry_files, &light_sources, &emitter_map);

        if variants.is_empty() {
            // No variants defined - create a single type from emitters
            return Self::export_legacy(gldf, mesh_data);
        }

        // Create shared geometry ONCE for all variants
        // This is the correct IFC pattern - geometry is defined once and referenced by all types
        let representation_map = writer.create_representation_map(mesh_data);

        // Pre-parse all unique LDT files and create shared distribution references
        // This avoids duplicating distribution data for emitters that share photometry
        // Also cache metadata for roundtrip preservation
        let mut distribution_cache: HashMap<String, Option<types::EntityRef>> = HashMap::new();
        let mut metadata_cache: HashMap<String, LdtMetadataExport> = HashMap::new();
        for variant in &variants {
            for emitter in &variant.emitters {
                if let Some(ref path) = emitter.photometry_file {
                    if !distribution_cache.contains_key(path) {
                        let ldt_result = Self::find_and_parse_ldt(files, path);
                        let dist_ref = ldt_result.as_ref().map(|result| {
                            writer.add_light_intensity_distribution("TYPE_C", &result.distribution)
                        });
                        // Cache metadata for later use
                        if let Some(ref result) = ldt_result {
                            metadata_cache.insert(path.clone(), result.metadata.clone());
                        }
                        distribution_cache.insert(path.clone(), dist_ref);
                    }
                }
            }
        }

        // Create one IFCLIGHTFIXTURETYPE per variant, all sharing the same geometry
        let mut fixture_instance_count = 0;
        for variant in &variants {
            // Reset light source tracking for this variant
            writer.clear_light_sources();

            let type_name = format!("{} - {}", product_name, variant.name);

            // Determine fixture type based on emitter count
            let fixture_type_enum = if variant.emitters.len() > 1 {
                LightFixtureTypeEnum::DirectionSource
            } else {
                LightFixtureTypeEnum::PointSource
            };

            // Create fixture type for this variant with shared geometry
            let fixture_type = writer.add_light_fixture_type_with_geometry(
                &type_name,
                manufacturer,
                fixture_type_enum,
                owner_history,
                Some(representation_map),
            );

            // Add light sources for this variant's emitters
            let mut total_flux = 0.0;
            let mut total_power = 0.0;
            let mut num_sources = 0;

            // Collect photometry filenames for this variant
            let mut photometry_filenames: Vec<String> = Vec::new();

            for emitter in &variant.emitters {
                let flux = emitter.luminous_flux.unwrap_or(1000.0);
                let cct = emitter.color_temperature.unwrap_or(4000) as f64;

                total_flux += flux;
                if let Some(power) = emitter.rated_power {
                    total_power += power;
                }
                num_sources += 1;

                // Look up cached distribution reference for this photometry file
                let dist_ref = emitter
                    .photometry_file
                    .as_ref()
                    .and_then(|path| distribution_cache.get(path))
                    .and_then(|opt| *opt);

                // Collect original filename for preservation through roundtrip
                if let Some(ref path) = emitter.photometry_file {
                    let filename = path.rsplit('/').next().unwrap_or(path);
                    photometry_filenames.push(filename.to_string());
                }

                if let Some(dist_ref) = dist_ref {
                    // Use shared distribution reference
                    writer.add_light_source_goniometric_with_distribution_ref(
                        &emitter.name,
                        None,
                        cct,
                        flux,
                        LightEmissionSourceEnum::Led,
                        dist_ref,
                    );
                } else {
                    // Fall back to external reference
                    writer.add_light_source_goniometric(
                        &emitter.name,
                        None,
                        cct,
                        flux,
                        LightEmissionSourceEnum::Led,
                        emitter.photometry_file.as_deref(),
                    );
                }
            }

            // Store photometry filenames in a property for roundtrip preservation
            if !photometry_filenames.is_empty() {
                writer.add_photometry_filenames_pset(
                    fixture_type,
                    owner_history,
                    &photometry_filenames,
                );
            }

            // Store LDT raw content and metadata for each photometry file (for roundtrip preservation)
            for (idx, emitter) in variant.emitters.iter().enumerate() {
                if let Some(ref path) = emitter.photometry_file {
                    if let Some(meta) = metadata_cache.get(path) {
                        // Store the complete raw LDT content for exact roundtrip
                        let filename = path.rsplit('/').next().unwrap_or(path);
                        if !meta.raw_content.is_empty() {
                            writer.add_ldt_raw_content_pset(
                                fixture_type,
                                owner_history,
                                idx + 1,
                                filename,
                                &meta.raw_content,
                            );
                        }
                        // Also store metadata as a fallback
                        writer.add_ldt_metadata_pset(
                            fixture_type,
                            owner_history,
                            idx + 1,
                            meta.symmetry,
                            meta.num_c_planes,
                            meta.num_g_angles,
                            meta.dc,
                            meta.dg,
                            meta.total_flux,
                            &meta.dr,
                            &meta.luminaire_name,
                        );
                    }
                }
            }

            // Add Pset_LightFixtureTypeCommon for this variant
            writer.add_light_fixture_common_pset(
                fixture_type,
                owner_history,
                Some(num_sources),
                if total_power > 0.0 {
                    Some(total_power)
                } else {
                    None
                },
                variant.mounting_type.as_deref(),
                None,
            );

            // Add variant-specific property set
            Self::add_variant_property_set(
                &mut writer,
                fixture_type,
                owner_history,
                &variant.id,
                variant.emitters.first().and_then(|e| e.color_temperature),
                Some(total_flux),
                if total_power > 0.0 {
                    Some(total_power)
                } else {
                    None
                },
                variant
                    .emitters
                    .first()
                    .and_then(|e| e.color_rendering_index),
                variant.weight,
            );

            // Add electrical properties
            Self::add_electrical_properties(gldf, &mut writer, fixture_type, owner_history);

            // Add GLDF descriptive attributes (safety class, maintenance, etc.)
            Self::add_descriptive_attributes(gldf, &mut writer, fixture_type, owner_history);

            // Add variant-level attributes (mounting, recessed depth)
            if variant.mounting_type.is_some() || variant.recessed_depth.is_some() {
                writer.add_gldf_descriptive_pset(
                    fixture_type,
                    owner_history,
                    None, // safety_class already added above
                    None, // median_useful_life already added above
                    variant.mounting_type.as_deref(),
                    variant.recessed_depth,
                );
            }

            // Create one fixture instance for this type
            // Use mapped item to reference the shared geometry via representation map
            fixture_instance_count += 1;
            let _fixture = writer.add_light_fixture_with_map(
                &format!("{}_{:03}", type_name, fixture_instance_count),
                owner_history,
                storey,
                Some(fixture_type),
                Some(representation_map),
                None, // No inline mesh - use the shared rep map
            );
        }

        // Store additional GLDF files (images, sensors) for roundtrip preservation
        Self::store_additional_files(gldf, files, &mut writer, project, owner_history);

        Ok(writer.to_step_string())
    }

    /// Legacy export for GLDF files without variants (backward compatibility)
    fn export_legacy(gldf: &GldfProduct, mesh_data: Option<&MeshData>) -> Result<String> {
        let mut writer = StepWriter::new("IFC4");

        let manufacturer = &gldf.header.manufacturer;
        let product_name = Self::get_product_name(gldf);

        let owner_history = writer.add_owner_history(manufacturer);
        let project = writer.add_project("GLDF Export", owner_history);
        let site = writer.add_site("Default Site", owner_history, project);
        let building = writer.add_building("Default Building", owner_history, site);
        let storey = writer.add_storey("Ground Floor", owner_history, building);

        let light_sources = Self::extract_light_sources_legacy(gldf);
        let num_sources = light_sources.len() as i32;
        let total_power: f64 = light_sources.iter().filter_map(|ls| ls.2).sum();

        let fixture_type_enum = if light_sources.len() > 1 {
            LightFixtureTypeEnum::DirectionSource
        } else {
            LightFixtureTypeEnum::PointSource
        };

        let fixture_type = writer.add_light_fixture_type(
            &product_name,
            manufacturer,
            fixture_type_enum,
            owner_history,
        );

        for (name, flux, _power, cct, photometry_file) in &light_sources {
            writer.add_light_source_goniometric(
                name,
                None,
                *cct,
                *flux,
                LightEmissionSourceEnum::Led,
                photometry_file.as_deref(),
            );
        }

        if num_sources > 0 || total_power > 0.0 {
            writer.add_light_fixture_common_pset(
                fixture_type,
                owner_history,
                if num_sources > 0 {
                    Some(num_sources)
                } else {
                    None
                },
                if total_power > 0.0 {
                    Some(total_power)
                } else {
                    None
                },
                Self::get_mounting_type(gldf),
                None,
            );
        }

        Self::add_electrical_properties(gldf, &mut writer, fixture_type, owner_history);

        let _fixture = writer.add_light_fixture(
            &format!("{}_001", product_name),
            owner_history,
            storey,
            Some(fixture_type),
            mesh_data,
        );

        Ok(writer.to_step_string())
    }

    /// Find and parse an LDT file from the BufFile array
    ///
    /// Returns distribution data in format: Vec<(c_angle, g_angle, intensity)>
    #[cfg(feature = "eulumdat")]
    fn find_and_parse_ldt(files: &[BufFile], path: &str) -> Option<LdtParseResult> {
        // Extract filename from path (e.g., "ldc/variant1_1.ldt" -> "variant1_1.ldt")
        let filename = path.rsplit('/').next().unwrap_or(path);

        // Find the file in the BufFile array
        let file = files.iter().find(|f| {
            f.name
                .as_ref()
                .map(|n| {
                    let stored_name = n.rsplit('/').next().unwrap_or(n);
                    stored_name.eq_ignore_ascii_case(filename)
                })
                .unwrap_or(false)
        })?;

        let content = file.content.as_ref()?;

        // Parse as UTF-8 or Latin-1
        let text = std::str::from_utf8(content)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| content.iter().map(|&b| b as char).collect());

        // Parse with eulumdat
        let ldt = eulumdat::Eulumdat::parse(&text).ok()?;

        // Extract metadata for roundtrip preservation
        let symmetry = match ldt.symmetry {
            eulumdat::Symmetry::None => 0,
            eulumdat::Symmetry::VerticalAxis => 1,
            eulumdat::Symmetry::PlaneC0C180 => 2,
            eulumdat::Symmetry::PlaneC90C270 => 3,
            eulumdat::Symmetry::BothPlanes => 4,
        };

        let metadata = LdtMetadataExport {
            symmetry,
            num_c_planes: ldt.num_c_planes as i32,
            num_g_angles: ldt.num_g_planes as i32,
            dc: ldt.c_plane_distance,
            dg: ldt.g_plane_distance,
            total_flux: ldt.total_luminous_flux(),
            dr: ldt.direct_ratios,
            luminaire_name: ldt.luminaire_name.clone(),
            // Store complete raw LDT content for exact roundtrip
            raw_content: text.clone(),
        };

        // Convert to IFC format: (main_plane_angle, secondary_plane_angle, intensity)
        // The eulumdat crate correctly handles symmetry - for symmetric files,
        // intensities contains only the stored data (e.g., 1 row for vertical axis symmetry)
        let mut distribution_data = Vec::new();

        for (c_idx, c_angle) in ldt.c_angles.iter().enumerate() {
            if let Some(intensities) = ldt.intensities.get(c_idx) {
                for (g_idx, &intensity) in intensities.iter().enumerate() {
                    if let Some(&g_angle) = ldt.g_angles.get(g_idx) {
                        distribution_data.push((*c_angle, g_angle, intensity));
                    }
                }
            }
        }

        if distribution_data.is_empty() {
            None
        } else {
            Some(LdtParseResult {
                distribution: distribution_data,
                metadata,
            })
        }
    }

    /// Stub when eulumdat feature is not enabled
    #[cfg(not(feature = "eulumdat"))]
    fn find_and_parse_ldt(_files: &[BufFile], _path: &str) -> Option<LdtParseResult> {
        None
    }

    /// Store additional GLDF files (images, sensors) in IFC for roundtrip preservation
    fn store_additional_files(
        gldf: &GldfProduct,
        files: &[BufFile],
        writer: &mut StepWriter,
        project: types::EntityRef,
        owner_history: types::EntityRef,
    ) {
        // Find and store image files
        for file_def in &gldf.general_definitions.files.file {
            if file_def.content_type.starts_with("image/") {
                // Find the actual file content
                let search_path = format!("image/{}", file_def.file_name);
                if let Some(buf_file) = files.iter().find(|f| {
                    f.name
                        .as_ref()
                        .map(|n| n.contains(&file_def.file_name) || n == &search_path)
                        .unwrap_or(false)
                }) {
                    if let Some(content) = &buf_file.content {
                        writer.add_gldf_file_pset(
                            project,
                            owner_history,
                            "image",
                            &file_def.file_name,
                            &file_def.content_type,
                            content,
                        );
                    }
                }
            }

            // Find and store sensor files
            if file_def.content_type.starts_with("sensor/") {
                let search_path = format!("sensor/{}", file_def.file_name);
                if let Some(buf_file) = files.iter().find(|f| {
                    f.name
                        .as_ref()
                        .map(|n| n.contains(&file_def.file_name) || n == &search_path)
                        .unwrap_or(false)
                }) {
                    if let Some(content) = &buf_file.content {
                        writer.add_gldf_file_pset(
                            project,
                            owner_history,
                            "sensor",
                            &file_def.file_name,
                            &file_def.content_type,
                            content,
                        );
                    }
                }
            }
        }
    }

    /// Build a map of photometry ID to file path
    fn build_photometry_file_map(gldf: &GldfProduct) -> HashMap<String, String> {
        let mut file_id_to_path: HashMap<String, String> = HashMap::new();

        // Map file IDs to file paths
        for file in &gldf.general_definitions.files.file {
            if file.content_type.starts_with("ldc") {
                file_id_to_path.insert(file.id.clone(), format!("ldc/{}", file.file_name));
            }
        }

        // Map photometry IDs to file paths
        let mut result: HashMap<String, String> = HashMap::new();
        if let Some(photometries) = &gldf.general_definitions.photometries {
            for photometry in &photometries.photometry {
                if let Some(file_ref) = &photometry.photometry_file_reference {
                    if let Some(path) = file_id_to_path.get(&file_ref.file_id) {
                        result.insert(photometry.id.clone(), path.clone());
                    }
                }
            }
        }

        result
    }

    /// Build a map of light source ID to (CCT, CRI, Power)
    #[allow(clippy::type_complexity)]
    fn build_light_source_map(
        gldf: &GldfProduct,
    ) -> HashMap<String, (Option<i32>, Option<i32>, Option<f64>)> {
        let mut result = HashMap::new();

        if let Some(light_sources) = &gldf.general_definitions.light_sources {
            for ls in &light_sources.fixed_light_source {
                let cct = ls
                    .color_information
                    .as_ref()
                    .and_then(|ci| ci.correlated_color_temperature);
                let cri = ls
                    .color_information
                    .as_ref()
                    .and_then(|ci| ci.color_rendering_index);
                let power = ls.rated_input_power;

                result.insert(ls.id.clone(), (cct, cri, power));
            }
        }

        result
    }

    /// Build a map of emitter ID to emitter data
    fn build_emitter_map(
        gldf: &GldfProduct,
    ) -> HashMap<String, (String, Option<f64>, String, Option<String>)> {
        // Maps emitter_id -> (name, rated_flux, photometry_id, light_source_id)
        let mut result = HashMap::new();

        if let Some(emitters) = &gldf.general_definitions.emitters {
            for emitter in &emitters.emitter {
                // Handle fixed light emitters
                for fixed in &emitter.fixed_light_emitter {
                    let name = fixed
                        .name
                        .as_ref()
                        .and_then(|n| n.locale.first())
                        .map(|l| l.value.clone())
                        .unwrap_or_else(|| emitter.id.clone());

                    let photometry_id = fixed.photometry_reference.photometry_id.clone();
                    let light_source_id =
                        fixed.light_source_reference.fixed_light_source_id.clone();
                    let rated_flux = fixed.rated_luminous_flux.map(|f| f as f64);

                    result.insert(
                        emitter.id.clone(),
                        (name, rated_flux, photometry_id, light_source_id),
                    );
                }

                // Handle changeable light emitters
                for changeable in &emitter.changeable_light_emitter {
                    let name = changeable
                        .name
                        .as_ref()
                        .and_then(|n| n.locale.first())
                        .map(|l| l.value.clone())
                        .unwrap_or_else(|| emitter.id.clone());

                    let photometry_id = changeable.photometry_reference.photometry_id.clone();

                    result.insert(emitter.id.clone(), (name, None, photometry_id, None));
                }
            }
        }

        result
    }

    /// Extract variant data from GLDF
    #[allow(clippy::type_complexity)]
    fn extract_variants(
        gldf: &GldfProduct,
        photometry_files: &HashMap<String, String>,
        light_sources: &HashMap<String, (Option<i32>, Option<i32>, Option<f64>)>,
        emitter_map: &HashMap<String, (String, Option<f64>, String, Option<String>)>,
    ) -> Vec<VariantData> {
        let mut variants = Vec::new();

        if let Some(product_variants) = &gldf.product_definitions.variants {
            for variant in &product_variants.variant {
                let name = variant
                    .name
                    .as_ref()
                    .and_then(|n| n.locale.first())
                    .map(|l| l.value.clone())
                    .unwrap_or_else(|| variant.id.clone());

                let description = variant
                    .description
                    .as_ref()
                    .and_then(|d| d.locale.first())
                    .map(|l| l.value.clone());

                // Get mounting type from variant
                let mounting_type = variant
                    .mountings
                    .as_ref()
                    .map(|m| {
                        if m.ceiling.is_some() {
                            "CEILING"
                        } else if m.wall.is_some() {
                            "WALL"
                        } else if m.ground.is_some() {
                            "GROUND"
                        } else if m.working_plane.is_some() {
                            "FREESTANDING"
                        } else {
                            "NOTDEFINED"
                        }
                    })
                    .map(String::from);

                // Get weight from descriptive attributes
                let weight = variant
                    .descriptive_attributes
                    .as_ref()
                    .and_then(|da| da.mechanical.as_ref())
                    .and_then(|m| m.weight);

                // Get recessed depth from mountings
                let recessed_depth = variant
                    .mountings
                    .as_ref()
                    .and_then(|m| m.ceiling.as_ref())
                    .and_then(|c| c.recessed.as_ref())
                    .map(|r| r.recessed_depth as f64);

                // Extract emitters for this variant
                let mut emitters = Vec::new();

                if let Some(geometry) = &variant.geometry {
                    // Try ModelGeometryReference first
                    if let Some(model_ref) = &geometry.model_geometry_reference {
                        for emitter_ref in &model_ref.emitter_reference {
                            if let Some((em_name, flux, phot_id, ls_id)) =
                                emitter_map.get(&emitter_ref.emitter_id)
                            {
                                let (cct, cri, power) = ls_id
                                    .as_ref()
                                    .and_then(|id| light_sources.get(id))
                                    .copied()
                                    .unwrap_or((None, None, None));

                                let photometry_file = photometry_files.get(phot_id).cloned();

                                emitters.push(VariantEmitterData {
                                    emitter_id: emitter_ref.emitter_id.clone(),
                                    name: em_name.clone(),
                                    luminous_flux: *flux,
                                    color_temperature: cct,
                                    color_rendering_index: cri,
                                    rated_power: power,
                                    photometry_id: Some(phot_id.clone()),
                                    photometry_file,
                                });
                            }
                        }
                    }

                    // Try SimpleGeometryReference
                    if let Some(simple_ref) = &geometry.simple_geometry_reference {
                        if let Some((em_name, flux, phot_id, ls_id)) =
                            emitter_map.get(&simple_ref.emitter_id)
                        {
                            let (cct, cri, power) = ls_id
                                .as_ref()
                                .and_then(|id| light_sources.get(id))
                                .copied()
                                .unwrap_or((None, None, None));

                            let photometry_file = photometry_files.get(phot_id).cloned();

                            emitters.push(VariantEmitterData {
                                emitter_id: simple_ref.emitter_id.clone(),
                                name: em_name.clone(),
                                luminous_flux: *flux,
                                color_temperature: cct,
                                color_rendering_index: cri,
                                rated_power: power,
                                photometry_id: Some(phot_id.clone()),
                                photometry_file,
                            });
                        }
                    }
                }

                // Only add variant if it has emitters
                if !emitters.is_empty() {
                    variants.push(VariantData {
                        id: variant.id.clone(),
                        name,
                        description,
                        emitters,
                        mounting_type,
                        weight,
                        recessed_depth,
                    });
                }
            }
        }

        variants
    }

    /// Add variant-specific property set (Pset_LuminaireVariant)
    #[allow(clippy::too_many_arguments)]
    fn add_variant_property_set(
        writer: &mut StepWriter,
        element: EntityRef,
        owner_history: EntityRef,
        variant_id: &str,
        cct: Option<i32>,
        luminous_flux: Option<f64>,
        power: Option<f64>,
        cri: Option<i32>,
        weight: Option<f64>,
    ) {
        writer.add_variant_pset(
            element,
            owner_history,
            variant_id,
            cct,
            luminous_flux,
            power,
            cri,
            weight,
        );
    }

    /// Legacy light source extraction for GLDF without variants
    #[allow(clippy::type_complexity)]
    fn extract_light_sources_legacy(
        gldf: &GldfProduct,
    ) -> Vec<(String, f64, Option<f64>, f64, Option<String>)> {
        // Returns: (name, flux, power, cct, photometry_file)
        let mut sources = Vec::new();

        let photometry_files = Self::build_photometry_file_map(gldf);
        let light_source_map = Self::build_light_source_map(gldf);

        if let Some(emitters) = &gldf.general_definitions.emitters {
            for emitter in &emitters.emitter {
                for fixed in &emitter.fixed_light_emitter {
                    let name = fixed
                        .name
                        .as_ref()
                        .and_then(|n| n.locale.first())
                        .map(|l| l.value.clone())
                        .unwrap_or_else(|| emitter.id.clone());

                    let flux = fixed.rated_luminous_flux.unwrap_or(1000) as f64;
                    let photometry_file = photometry_files
                        .get(&fixed.photometry_reference.photometry_id)
                        .cloned();

                    let (cct, _cri, power) = fixed
                        .light_source_reference
                        .fixed_light_source_id
                        .as_ref()
                        .and_then(|id| light_source_map.get(id))
                        .copied()
                        .unwrap_or((Some(4000), None, None));

                    sources.push((
                        name,
                        flux,
                        power,
                        cct.unwrap_or(4000) as f64,
                        photometry_file,
                    ));
                }
            }
        }

        if sources.is_empty() {
            sources.push(("Light Source".to_string(), 1000.0, None, 4000.0, None));
        }

        sources
    }

    /// Get mounting type from GLDF descriptive attributes
    fn get_mounting_type(gldf: &GldfProduct) -> Option<&'static str> {
        if let Some(meta) = &gldf.product_definitions.product_meta_data {
            if let Some(attrs) = &meta.descriptive_attributes {
                if let Some(mechanical) = &attrs.mechanical {
                    if mechanical.product_form.is_some() {
                        return Some("SURFACE");
                    }
                }
            }
        }
        None
    }

    /// Add electrical properties from GLDF (Electrical MVD compliant)
    ///
    /// Extracts all available electrical data from GLDF and creates a comprehensive
    /// Pset_ElectricalDeviceCommon property set per SPARKIE MVD requirements.
    fn add_electrical_properties(
        gldf: &GldfProduct,
        writer: &mut StepWriter,
        fixture_type: EntityRef,
        owner_history: EntityRef,
    ) {
        use crate::ifc::types::{ElectricalDeviceProperties, InsulationStandardClass};

        let mut props = ElectricalDeviceProperties::default();

        // Extract from descriptive attributes
        if let Some(meta) = &gldf.product_definitions.product_meta_data {
            if let Some(attrs) = &meta.descriptive_attributes {
                if let Some(electrical) = &attrs.electrical {
                    // IP Code (IEC 60529)
                    if let Some(ip) = &electrical.ingress_protection_ip_code {
                        props.ip_code = Some(ip.clone());
                    }
                    // Electrical Safety Class -> InsulationStandardClass
                    if let Some(sc) = &electrical.electrical_safety_class {
                        props.insulation_standard_class =
                            Some(InsulationStandardClass::from_gldf_safety_class(sc));
                        // Class I/II/III determines protective earth requirement
                        props.has_protective_earth = match sc.as_str() {
                            "I" | "1" => Some(true),    // Class I has PE
                            "II" | "2" => Some(false),  // Class II - double insulation, no PE
                            "III" | "3" => Some(false), // Class III - SELV, no PE
                            _ => None,
                        };
                    }
                    // Power Factor (cos φ)
                    if let Some(pf) = electrical.power_factor {
                        props.power_factor = Some(pf);
                    }
                }
            }
        }

        // Extract from control gears
        if let Some(control_gears) = &gldf.general_definitions.control_gears {
            if let Some(cg) = control_gears.control_gear.first() {
                // Nominal Voltage - can be fixed or range
                if let Some(nominal) = &cg.nominal_voltage {
                    if let Some(v) = nominal.fixed_voltage {
                        props.rated_voltage = Some(v);
                    } else if let Some(range) = &nominal.voltage_range {
                        props.rated_voltage = Some(range.min);
                        props.rated_voltage_max = Some(range.max);
                    }
                    // Set standard frequency range (50-60Hz covers most regions)
                    props.nominal_frequency_min = Some(50.0);
                    props.nominal_frequency_max = Some(60.0);
                }
                // Standby power as heat dissipation estimate
                if let Some(standby) = cg.standby_power {
                    props.heat_dissipation = Some(standby);
                }
            }
        }

        // Default number of poles for typical luminaires
        if props.rated_voltage.is_some() {
            props.number_of_poles = Some(1); // Single-phase typical for luminaires
            props.number_of_power_supply_ports = Some(1);
        }

        // Write the comprehensive electrical property set
        let has_props = props.rated_voltage.is_some()
            || props.ip_code.is_some()
            || props.power_factor.is_some()
            || props.insulation_standard_class.is_some();

        if has_props {
            writer.add_electrical_device_pset(fixture_type, owner_history, &props);
        }
    }

    /// Add GLDF-specific descriptive attributes from product metadata
    fn add_descriptive_attributes(
        gldf: &GldfProduct,
        writer: &mut StepWriter,
        fixture_type: EntityRef,
        owner_history: EntityRef,
    ) {
        let mut safety_class: Option<String> = None;
        let mut median_useful_life: Option<String> = None;

        if let Some(meta) = &gldf.product_definitions.product_meta_data {
            if let Some(attrs) = &meta.descriptive_attributes {
                // Electrical safety class
                if let Some(electrical) = &attrs.electrical {
                    if let Some(sc) = &electrical.electrical_safety_class {
                        safety_class = Some(sc.clone());
                    }
                }
                // Operations and maintenance
                if let Some(ops) = &attrs.operations_and_maintenance {
                    if let Some(lifetimes) = &ops.median_useful_life_times {
                        if let Some(first) = lifetimes.median_useful_life.first() {
                            median_useful_life = Some(first.clone());
                        }
                    }
                }
            }
        }

        if safety_class.is_some() || median_useful_life.is_some() {
            writer.add_gldf_descriptive_pset(
                fixture_type,
                owner_history,
                safety_class.as_deref(),
                median_useful_life.as_deref(),
                None,
                None,
            );
        }
    }

    fn get_product_name(gldf: &GldfProduct) -> String {
        if let Some(meta) = &gldf.product_definitions.product_meta_data {
            if let Some(name) = &meta.name {
                if let Some(locale) = name.locale.first() {
                    if !locale.value.is_empty() {
                        return locale.value.clone();
                    }
                }
            }
        }
        gldf.header.manufacturer.clone()
    }

    /// Extract L3D mesh data from GLDF files for IFC geometry export
    ///
    /// Searches through the provided files for L3D geometry files,
    /// parses OBJ content, and combines all parts into a single mesh.
    ///
    /// # Arguments
    /// * `gldf` - The GLDF product (to find geometry file references)
    /// * `files` - The embedded files from the GLDF archive
    ///
    /// # Returns
    /// `Some(MeshData)` if L3D mesh was successfully extracted, `None` otherwise
    #[cfg(feature = "l3d")]
    pub fn extract_mesh_from_files(gldf: &GldfProduct, files: &[BufFile]) -> Option<MeshData> {
        let geo_files: Vec<_> = gldf
            .general_definitions
            .files
            .file
            .iter()
            .filter(|f| f.content_type == "geo/l3d")
            .collect();

        if geo_files.is_empty() {
            return None;
        }

        let l3d_ref = geo_files.first()?;

        let l3d_buf = files.iter().find(|f| {
            f.name
                .as_ref()
                .map(|n| n.contains(&l3d_ref.file_name) || n.ends_with(".l3d"))
                .unwrap_or(false)
        })?;

        let l3d_content = l3d_buf.content.as_ref()?;
        let l3d = l3d_rs::from_buffer(l3d_content);

        if l3d.model.parts.is_empty() {
            return None;
        }

        let mut all_vertices: Vec<(f64, f64, f64)> = Vec::new();
        let mut all_triangles: Vec<(u32, u32, u32)> = Vec::new();

        for part in &l3d.model.parts {
            let asset = match l3d.file.assets.iter().find(|a| a.name == part.path) {
                Some(a) => a,
                None => continue,
            };

            let obj_content = match std::str::from_utf8(&asset.content) {
                Ok(s) => s,
                Err(_) => continue,
            };

            if let Some((vertices, triangles)) =
                Self::parse_obj_for_ifc(obj_content, &part.mat, all_vertices.len())
            {
                all_vertices.extend(vertices);
                all_triangles.extend(triangles);
            }
        }

        if all_vertices.is_empty() || all_triangles.is_empty() {
            return None;
        }

        Some(MeshData {
            vertices: all_vertices,
            triangles: all_triangles,
        })
    }

    /// Stub for when l3d feature is not enabled
    #[cfg(not(feature = "l3d"))]
    pub fn extract_mesh_from_files(_gldf: &GldfProduct, _files: &[BufFile]) -> Option<MeshData> {
        None
    }

    /// Parse OBJ content and transform vertices for IFC export
    #[cfg(feature = "l3d")]
    #[allow(clippy::type_complexity)]
    fn parse_obj_for_ifc(
        content: &str,
        transform: &[f32; 16],
        vertex_offset: usize,
    ) -> Option<(Vec<(f64, f64, f64)>, Vec<(u32, u32, u32)>)> {
        let mut temp_positions: Vec<[f32; 3]> = Vec::new();
        let mut vertices: Vec<(f64, f64, f64)> = Vec::new();
        let mut triangles: Vec<(u32, u32, u32)> = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" if parts.len() >= 4 => {
                    let x: f32 = parts[1].parse().unwrap_or(0.0);
                    let y: f32 = parts[2].parse().unwrap_or(0.0);
                    let z: f32 = parts[3].parse().unwrap_or(0.0);
                    temp_positions.push([x, y, z]);
                }
                "f" if parts.len() >= 4 => {
                    let mut face_indices: Vec<usize> = Vec::new();

                    for part in parts.iter().skip(1) {
                        let vertex_parts: Vec<&str> = part.split('/').collect();
                        let pos_idx: usize = vertex_parts[0].parse::<usize>().unwrap_or(1) - 1;
                        face_indices.push(pos_idx);
                    }

                    for i in 1..face_indices.len() - 1 {
                        let i0 = (face_indices[0] + vertex_offset + 1) as u32;
                        let i1 = (face_indices[i] + vertex_offset + 1) as u32;
                        let i2 = (face_indices[i + 1] + vertex_offset + 1) as u32;
                        triangles.push((i0, i1, i2));
                    }
                }
                _ => {}
            }
        }

        if temp_positions.is_empty() {
            return None;
        }

        for pos in &temp_positions {
            let x = pos[0] * transform[0]
                + pos[1] * transform[4]
                + pos[2] * transform[8]
                + transform[12];
            let y = pos[0] * transform[1]
                + pos[1] * transform[5]
                + pos[2] * transform[9]
                + transform[13];
            let z = pos[0] * transform[2]
                + pos[1] * transform[6]
                + pos[2] * transform[10]
                + transform[14];

            // The transform matrix already includes unit conversion (scale 0.001 = mm to m)
            // so we don't divide by 1000 again
            vertices.push((x as f64, z as f64, (-y) as f64));
        }

        Some((vertices, triangles))
    }
}

/// Import IFC luminaire file and convert to GLDF format
///
/// This is the main entry point for IFC to GLDF conversion.
///
/// # Arguments
/// * `ifc_content` - The IFC file content as a string
///
/// # Returns
/// A GLDF package as bytes (ZIP archive) on success
///
/// # Example
/// ```rust,ignore
/// use gldf_rs::ifc::ifc_to_gldf;
///
/// let ifc_content = std::fs::read_to_string("luminaire.ifc")?;
/// let gldf_bytes = ifc_to_gldf(&ifc_content)?;
/// std::fs::write("luminaire.gldf", gldf_bytes)?;
/// ```
pub fn ifc_to_gldf(ifc_content: &str) -> Result<Vec<u8>> {
    let importer = IfcImporter::from_str(ifc_content)?;
    let luminaire = importer.import()?;
    let generator = GldfGenerator::new(luminaire);
    generator.generate()
}

/// Import IFC file and extract luminaire data (without generating GLDF)
///
/// Use this when you want to inspect or modify the imported data
/// before generating a GLDF package.
///
/// # Arguments
/// * `ifc_content` - The IFC file content as a string
///
/// # Returns
/// The extracted luminaire data structure
pub fn import_ifc(ifc_content: &str) -> Result<ImportedLuminaire> {
    let importer = IfcImporter::from_str(ifc_content)?;
    importer.import()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_writer_basic() {
        let mut writer = StepWriter::new("IFC4");
        let oh = writer.add_owner_history("Test Manufacturer");
        let _project = writer.add_project("Test Project", oh);

        let output = writer.to_step_string();
        assert!(output.contains("ISO-10303-21"));
        assert!(output.contains("IFC4"));
        assert!(output.contains("IFCPROJECT"));
    }

    #[test]
    fn test_gldf_to_ifc_export() {
        let gldf = GldfProduct::load_gldf("tests/data/test.gldf");
        if let Ok(gldf) = gldf {
            let ifc_result = GldfToIfc::export(&gldf);
            assert!(ifc_result.is_ok(), "IFC export should succeed");

            let ifc_content = ifc_result.unwrap();

            assert!(ifc_content.contains("ISO-10303-21"));
            assert!(ifc_content.contains("IFC4"));
            assert!(ifc_content.contains("IFCPROJECT"));
            assert!(ifc_content.contains("IFCLIGHTFIXTURETYPE"));
            assert!(ifc_content.contains("IFCLIGHTFIXTURE"));
            assert!(ifc_content.contains(&gldf.header.manufacturer));

            println!("=== GLDF to IFC Export ===");
            println!("{}", ifc_content);
        } else {
            println!("Test GLDF file not found, skipping test");
        }
    }

    #[test]
    fn test_tessellated_geometry_export() {
        let mesh = MeshData {
            vertices: vec![
                (0.0, 0.0, 0.1),
                (0.1, 0.0, 0.1),
                (0.1, 0.1, 0.1),
                (0.0, 0.1, 0.1),
                (0.0, 0.0, 0.0),
                (0.1, 0.0, 0.0),
                (0.1, 0.1, 0.0),
                (0.0, 0.1, 0.0),
            ],
            triangles: vec![
                (1, 2, 3),
                (1, 3, 4),
                (5, 7, 6),
                (5, 8, 7),
                (4, 3, 7),
                (3, 8, 7),
                (1, 6, 2),
                (1, 5, 6),
                (2, 6, 7),
                (2, 7, 3),
                (1, 4, 8),
                (1, 8, 5),
            ],
        };

        let mut writer = StepWriter::new("IFC4");
        let oh = writer.add_owner_history("Test Manufacturer");
        let project = writer.add_project("Test Project", oh);
        let site = writer.add_site("Test Site", oh, project);
        let building = writer.add_building("Test Building", oh, site);
        let storey = writer.add_storey("Ground Floor", oh, building);

        let fixture_type = writer.add_light_fixture_type(
            "LED Panel",
            "Test Corp",
            LightFixtureTypeEnum::DirectionSource,
            oh,
        );

        let _fixture =
            writer.add_light_fixture("LED Panel 001", oh, storey, Some(fixture_type), Some(&mesh));

        let output = writer.to_step_string();

        assert!(
            output.contains("IFCCARTESIANPOINTLIST3D"),
            "Should contain point list"
        );
        assert!(
            output.contains("IFCTRIANGULATEDFACESET"),
            "Should contain triangulated faceset"
        );
        assert!(
            output.contains("'Tessellation'"),
            "Should have Tessellation representation type"
        );
        assert!(
            !output.contains("IFCEXTRUDEDAREASOLID"),
            "Should NOT contain extruded solid"
        );

        println!("=== Tessellated Geometry IFC Export ===");
        println!("{}", output);
    }

    #[test]
    fn test_export_with_mesh() {
        let gldf = GldfProduct::load_gldf("tests/data/test.gldf");
        if let Ok(gldf) = gldf {
            let mesh = MeshData {
                vertices: vec![(0.0, 0.0, 0.0), (0.1, 0.0, 0.0), (0.05, 0.1, 0.0)],
                triangles: vec![(1, 2, 3)],
            };

            let ifc_result = GldfToIfc::export_with_mesh(&gldf, Some(&mesh));
            assert!(ifc_result.is_ok(), "IFC export with mesh should succeed");

            let ifc_content = ifc_result.unwrap();

            assert!(ifc_content.contains("IFCTRIANGULATEDFACESET"));
            assert!(ifc_content.contains("'Tessellation'"));
            assert!(!ifc_content.contains("IFCEXTRUDEDAREASOLID"));

            println!("=== Export with Mesh ===");
            println!("{}", ifc_content);
        } else {
            println!("Test GLDF file not found, skipping test");
        }
    }

    #[test]
    fn test_variant_export() {
        // Test with SLV TRIA 2 which has multiple variants
        // Note: path is relative to workspace root, not crate root
        let paths = [
            "tests/data/SLV - Tria 2.gldf",
            "../../tests/data/SLV - Tria 2.gldf",
        ];
        let gldf = paths.iter().find_map(|p| GldfProduct::load_gldf(p).ok());
        if let Some(gldf) = gldf {
            let ifc_result = GldfToIfc::export(&gldf);
            assert!(ifc_result.is_ok(), "IFC export should succeed");

            let ifc_content = ifc_result.unwrap();

            // Should have multiple IFCLIGHTFIXTURETYPE entries (one per variant)
            let type_count = ifc_content.matches("IFCLIGHTFIXTURETYPE").count();
            assert!(
                type_count >= 3,
                "Should have at least 3 fixture types for SLV TRIA variants, got {}",
                type_count
            );

            // Should have variant property sets
            assert!(
                ifc_content.contains("Pset_LuminaireVariant"),
                "Should contain variant property set"
            );

            // Should have variant IDs in properties
            assert!(
                ifc_content.contains("GLDF_VariantId"),
                "Should contain GLDF variant ID property"
            );

            println!("=== Variant Export ===");
            println!("{}", ifc_content);
        } else {
            println!("SLV TRIA 2 GLDF file not found, skipping test");
        }
    }

    #[test]
    #[cfg(feature = "l3d")]
    fn test_full_export_with_mesh() {
        // Test full export with actual mesh data and save the IFC for inspection
        let paths = [
            "tests/data/SLV - Tria 2.gldf",
            "../../tests/data/SLV - Tria 2.gldf",
        ];

        for path in &paths {
            if let Ok(buf) = std::fs::read(path) {
                if let Ok(gldf_buf) = crate::GldfProduct::load_gldf_from_buf_all(buf) {
                    let mesh = GldfToIfc::extract_mesh_from_files(&gldf_buf.gldf, &gldf_buf.files);
                    if let Some(ref m) = mesh {
                        println!("Mesh: {} vertices", m.vertices.len());
                        // Print first 5 vertices to check scale
                        for (i, v) in m.vertices.iter().take(5).enumerate() {
                            println!("  Vertex {}: ({:.6}, {:.6}, {:.6})", i, v.0, v.1, v.2);
                        }
                        // Find bounding box
                        let mut min = (f64::MAX, f64::MAX, f64::MAX);
                        let mut max = (f64::MIN, f64::MIN, f64::MIN);
                        for v in &m.vertices {
                            min.0 = min.0.min(v.0);
                            min.1 = min.1.min(v.1);
                            min.2 = min.2.min(v.2);
                            max.0 = max.0.max(v.0);
                            max.1 = max.1.max(v.1);
                            max.2 = max.2.max(v.2);
                        }
                        println!(
                            "Bounding box: ({:.4}, {:.4}, {:.4}) to ({:.4}, {:.4}, {:.4})",
                            min.0, min.1, min.2, max.0, max.1, max.2
                        );
                        println!(
                            "Size: {:.4} x {:.4} x {:.4} meters",
                            max.0 - min.0,
                            max.1 - min.1,
                            max.2 - min.2
                        );
                    }

                    let ifc = GldfToIfc::export_with_mesh(&gldf_buf.gldf, mesh.as_ref()).unwrap();

                    // Save for inspection
                    std::fs::write("/tmp/test_export.ifc", &ifc).ok();
                    println!("Saved IFC to /tmp/test_export.ifc ({} bytes)", ifc.len());

                    // Check contents
                    println!(
                        "Has IFCTRIANGULATEDFACESET: {}",
                        ifc.contains("IFCTRIANGULATEDFACESET")
                    );
                    println!(
                        "Has IFCCARTESIANPOINTLIST3D: {}",
                        ifc.contains("IFCCARTESIANPOINTLIST3D")
                    );
                    println!(
                        "Has IFCEXTRUDEDAREASOLID: {}",
                        ifc.contains("IFCEXTRUDEDAREASOLID")
                    );
                    println!("Has IFCMAPPEDITEM: {}", ifc.contains("IFCMAPPEDITEM"));
                    println!(
                        "Has IFCREPRESENTATIONMAP: {}",
                        ifc.contains("IFCREPRESENTATIONMAP")
                    );

                    assert!(
                        ifc.contains("IFCTRIANGULATEDFACESET"),
                        "Should have tessellated geometry"
                    );
                    return;
                }
            }
        }
        println!("SLV Tria not found");
    }

    #[test]
    #[cfg(feature = "l3d")]
    fn test_mesh_extraction() {
        // Test that mesh extraction works on SLV Tria
        let paths = [
            "tests/data/SLV - Tria 2.gldf",
            "../../tests/data/SLV - Tria 2.gldf",
        ];

        for path in &paths {
            if let Ok(buf) = std::fs::read(path) {
                if let Ok(gldf_buf) = crate::GldfProduct::load_gldf_from_buf_all(buf) {
                    println!("Loaded GLDF with {} files", gldf_buf.files.len());
                    for f in &gldf_buf.files {
                        println!(
                            "  File: {:?}, size: {:?}",
                            f.name,
                            f.content.as_ref().map(|c| c.len())
                        );
                    }

                    // Check geo files in definition
                    let geo_files: Vec<_> = gldf_buf
                        .gldf
                        .general_definitions
                        .files
                        .file
                        .iter()
                        .filter(|f| f.content_type == "geo/l3d")
                        .collect();
                    println!(
                        "Geo files in definition: {:?}",
                        geo_files.iter().map(|f| &f.file_name).collect::<Vec<_>>()
                    );

                    // Try to find matching BufFile
                    if let Some(l3d_ref) = geo_files.first() {
                        println!("Looking for L3D file: {}", l3d_ref.file_name);
                        for bf in &gldf_buf.files {
                            if let Some(name) = &bf.name {
                                if name.contains(&l3d_ref.file_name) || name.ends_with(".l3d") {
                                    println!("  Found matching BufFile: {}", name);
                                    if let Some(content) = &bf.content {
                                        println!("  Content size: {} bytes", content.len());
                                        // Try to parse L3D
                                        let l3d = l3d_rs::from_buffer(content);
                                        println!("  L3D parts: {}", l3d.model.parts.len());
                                        println!("  L3D assets: {}", l3d.file.assets.len());
                                        for part in &l3d.model.parts {
                                            println!("    Part: {}", part.path);
                                        }
                                        for asset in &l3d.file.assets {
                                            println!(
                                                "    Asset: {}, size: {}",
                                                asset.name,
                                                asset.content.len()
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let mesh = GldfToIfc::extract_mesh_from_files(&gldf_buf.gldf, &gldf_buf.files);
                    if let Some(ref m) = mesh {
                        println!(
                            "Mesh extracted: {} vertices, {} triangles",
                            m.vertices.len(),
                            m.triangles.len()
                        );
                        assert!(m.vertices.len() > 8, "Should have more than a simple box");
                    } else {
                        panic!("Mesh extraction failed!");
                    }

                    // If mesh exists, export with mesh and check output
                    let ifc = GldfToIfc::export_with_mesh(&gldf_buf.gldf, mesh.as_ref()).unwrap();
                    assert!(
                        ifc.contains("IFCTRIANGULATEDFACESET"),
                        "Should use tessellated geometry"
                    );
                    println!("IFC contains tessellated geometry!");
                    return;
                }
            }
        }
        println!("SLV Tria not found");
    }

    #[test]
    #[cfg(feature = "l3d")]
    fn test_slv_tria_ifc_roundtrip() {
        // Test with SLV TRIA 2 - real-world roundtrip
        let paths = [
            "tests/data/SLV - Tria 2.gldf",
            "../../tests/data/SLV - Tria 2.gldf",
        ];
        let gldf_buf = paths.iter().find_map(|p| {
            std::fs::read(p)
                .ok()
                .and_then(|buf| crate::GldfProduct::load_gldf_from_buf_all(buf).ok())
        });

        if let Some(gldf_buf) = gldf_buf {
            // Extract mesh from GLDF
            let mesh = GldfToIfc::extract_mesh_from_files(&gldf_buf.gldf, &gldf_buf.files);
            println!(
                "Original mesh: {:?} vertices",
                mesh.as_ref().map(|m| m.vertices.len())
            );

            // Export to IFC with mesh and LDT files (for photometry embedding)
            let ifc_content = GldfToIfc::export_with_mesh_and_files(
                &gldf_buf.gldf,
                mesh.as_ref(),
                &gldf_buf.files,
            )
            .expect("IFC export should succeed");
            println!("IFC export size: {} bytes", ifc_content.len());

            // Save IFC for inspection
            std::fs::write("/tmp/slv_tria_exported.ifc", &ifc_content).ok();
            println!("Saved IFC to /tmp/slv_tria_exported.ifc");

            // Check if IFC contains tessellated geometry
            println!(
                "IFC has IFCTRIANGULATEDFACESET: {}",
                ifc_content.contains("IFCTRIANGULATEDFACESET")
            );
            println!(
                "IFC has IFCEXTRUDEDAREASOLID: {}",
                ifc_content.contains("IFCEXTRUDEDAREASOLID")
            );
            println!(
                "IFC has IFCCARTESIANPOINTLIST3D: {}",
                ifc_content.contains("IFCCARTESIANPOINTLIST3D")
            );
            println!(
                "IFC has IFCLIGHTINTENSITYDISTRIBUTION: {}",
                ifc_content.contains("IFCLIGHTINTENSITYDISTRIBUTION")
            );

            // Import back from IFC
            let luminaire = import_ifc(&ifc_content).expect("IFC import should succeed");
            println!("Imported {} variants", luminaire.variants.len());

            // Check imported geometry, distributions, and electrical properties
            for (i, v) in luminaire.variants.iter().enumerate() {
                if let Some(ref geom) = v.geometry {
                    println!(
                        "Variant {} geometry: {} vertices, {} triangles",
                        i,
                        geom.vertices.len(),
                        geom.triangles.len()
                    );
                } else {
                    println!("Variant {} has NO geometry", i);
                }
                // Debug electrical properties
                println!(
                    "  IP Code: {:?}, Safety Class: {:?}",
                    v.properties.ip_code, v.properties.electrical_safety_class
                );
                // Debug distribution data
                for (j, ls) in v.light_sources.iter().enumerate() {
                    if let Some(ref dist) = ls.distribution {
                        println!(
                            "  LS {} '{}': {} C-planes, {} gamma each",
                            j,
                            ls.name,
                            dist.data.len(),
                            dist.data.first().map(|d| d.intensities.len()).unwrap_or(0)
                        );
                        if let Some(first) = dist.data.first() {
                            println!(
                                "    C-angle 0: {}, first 5 gamma: {:?}",
                                first.main_angle,
                                first
                                    .intensities
                                    .iter()
                                    .take(5)
                                    .map(|(a, _)| *a)
                                    .collect::<Vec<_>>()
                            );
                        }
                    } else {
                        println!("  LS {} '{}': NO distribution", j, ls.name);
                    }
                }
            }

            // Generate GLDF from imported data
            let gldf_bytes = ifc_to_gldf(&ifc_content).expect("GLDF generation should succeed");
            println!("GLDF size: {} bytes", gldf_bytes.len());

            // Save GLDF for inspection
            std::fs::write("/tmp/slv_tria_roundtrip.gldf", &gldf_bytes).ok();
            println!("Saved to /tmp/slv_tria_roundtrip.gldf");

            // Extract product.xml for debugging
            let cursor = std::io::Cursor::new(&gldf_bytes);
            let mut archive = zip::ZipArchive::new(cursor).expect("open zip");
            let mut xml = String::new();
            use std::io::Read;
            archive
                .by_name("product.xml")
                .expect("product.xml")
                .read_to_string(&mut xml)
                .expect("read");

            std::fs::write("/tmp/slv_tria_product.xml", &xml).ok();
            println!("product.xml saved to /tmp/slv_tria_product.xml");

            // Try to parse the XML directly
            match GldfProduct::from_xml(&xml) {
                Ok(loaded) => {
                    println!(
                        "XML parsing OK, manufacturer: {}",
                        loaded.header.manufacturer
                    );
                }
                Err(e) => {
                    println!("XML parsing FAILED: {}", e);
                    // Print first 500 chars around the error if possible
                    panic!("Failed to parse product.xml: {}", e);
                }
            }

            // Load the generated GLDF to verify it's valid
            match GldfProduct::load_gldf_from_buf_all(gldf_bytes.clone()) {
                Ok(file_buf) => {
                    println!(
                        "Loaded GLDF successfully, manufacturer: {}",
                        file_buf.gldf.header.manufacturer
                    );
                    println!("Files extracted: {}", file_buf.files.len());
                    for f in &file_buf.files {
                        println!("  - {:?}", f.name);
                    }

                    // Debug: Check L3D internal structure
                    if let Some(l3d_file) = file_buf.files.iter().find(|f| {
                        f.name
                            .as_ref()
                            .map(|n| n.ends_with(".l3d"))
                            .unwrap_or(false)
                    }) {
                        if let Some(l3d_bytes) = &l3d_file.content {
                            let l3d = l3d_rs::from_buffer(l3d_bytes);
                            println!("\n=== L3D Debug ===");
                            println!("Parts ({}):", l3d.model.parts.len());
                            for p in &l3d.model.parts {
                                println!("  Part path: '{}'", p.path);
                            }
                            println!("Assets ({}):", l3d.file.assets.len());
                            for a in &l3d.file.assets {
                                println!("  Asset name: '{}' ({} bytes)", a.name, a.content.len());
                            }
                        }
                    }

                    // Debug: Check LDT files parsing
                    // Also parse original LDT files for comparison
                    println!("\n=== Original LDT Files ===");
                    for orig_name in ["middle.ldt", "wide.ldt", "narrow.ldt"] {
                        let orig_path =
                            format!("tests/data/SLV - Tria 2_extracted/ldc/{}", orig_name);
                        let alt_path =
                            format!("../../tests/data/SLV - Tria 2_extracted/ldc/{}", orig_name);
                        if let Ok(content) = std::fs::read_to_string(&orig_path)
                            .or_else(|_| std::fs::read_to_string(&alt_path))
                        {
                            match eulumdat::Eulumdat::parse(&content) {
                                Ok(ldt) => {
                                    println!(
                                        "  {} - OK: Mc={}, Ng={}, Dc={}, Dg={}, {:?} symmetry",
                                        orig_name,
                                        ldt.num_c_planes,
                                        ldt.num_g_planes,
                                        ldt.c_plane_distance,
                                        ldt.g_plane_distance,
                                        ldt.symmetry
                                    );
                                    println!(
                                        "    C-angles len: {}, G-angles len: {}",
                                        ldt.c_angles.len(),
                                        ldt.g_angles.len()
                                    );
                                    println!(
                                        "    Intensities: {} rows, first row len: {}",
                                        ldt.intensities.len(),
                                        ldt.intensities.first().map(|r| r.len()).unwrap_or(0)
                                    );
                                }
                                Err(e) => println!("  {} - PARSE ERROR: {:?}", orig_name, e),
                            }
                        }
                    }

                    println!("\n=== Generated LDT Files ===");
                    for ldt_file in file_buf.files.iter().filter(|f| {
                        f.name
                            .as_ref()
                            .map(|n| n.ends_with(".ldt"))
                            .unwrap_or(false)
                    }) {
                        if let (Some(name), Some(content)) = (&ldt_file.name, &ldt_file.content) {
                            let text = std::str::from_utf8(content).unwrap_or("[binary]");
                            match eulumdat::Eulumdat::parse(text) {
                                Ok(ldt) => {
                                    println!(
                                        "  {} - OK: Mc={}, Ng={}, Dc={}, Dg={}, {:?} symmetry",
                                        name,
                                        ldt.num_c_planes,
                                        ldt.num_g_planes,
                                        ldt.c_plane_distance,
                                        ldt.g_plane_distance,
                                        ldt.symmetry
                                    );
                                    println!(
                                        "    C-angles len: {}, G-angles len: {}",
                                        ldt.c_angles.len(),
                                        ldt.g_angles.len()
                                    );
                                    println!(
                                        "    Intensities: {} rows, first row len: {}",
                                        ldt.intensities.len(),
                                        ldt.intensities.first().map(|r| r.len()).unwrap_or(0)
                                    );
                                }
                                Err(e) => {
                                    println!("  {} - PARSE ERROR: {:?}", name, e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    panic!("Failed to load generated GLDF: {}", e);
                }
            }
        } else {
            println!("SLV TRIA 2 GLDF file not found, skipping test");
        }
    }

    #[test]
    fn test_ifc_to_gldf_roundtrip() {
        // Test the full IFC import -> GLDF export flow
        let ifc_content = r#"
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('GLDF Export'),'2;1');
FILE_NAME('test.ifc','2024-01-01',(''),(''),'gldf-rs','gldf-rs','');
FILE_SCHEMA(('IFC4'));
ENDSEC;

DATA;
#1=IFCPERSON($,$,'',$,$,$,$,$);
#2=IFCORGANIZATION($,'Test Manufacturer','Testing',$,$);
#3=IFCPERSONANDORGANIZATION(#1,#2,$);
#4=IFCAPPLICATION(#2,'1.0','gldf-rs','gldf-rs');
#5=IFCOWNERHISTORY(#3,#4,.READWRITE.,.ADDED.,$,$,$,0);
#6=IFCLIGHTFIXTURETYPE('guid1',#5,'LED Panel - 4000K','Standard 4000K variant',$,$,$,$,$,.POINTSOURCE.);
#7=IFCLIGHTFIXTURETYPE('guid2',#5,'LED Panel - 3000K','Warm white variant',$,$,$,$,$,.POINTSOURCE.);
#8=IFCPROPERTYSINGLEVALUE('Manufacturer',$,IFCLABEL('Test Manufacturer'),$);
#9=IFCPROPERTYSINGLEVALUE('ModelReference',$,IFCLABEL('LED-PANEL-001'),$);
#10=IFCPROPERTYSET('pset1',#5,'Pset_ManufacturerTypeInformation',$,(#8,#9));
#11=IFCRELDEFINESBYPROPERTIES('rel1',#5,$,$,(#6),#10);
#12=IFCRELDEFINESBYPROPERTIES('rel2',#5,$,$,(#7),#10);
#13=IFCPROPERTYSINGLEVALUE('GLDF_VariantId',$,IFCLABEL('var_4000K'),$);
#14=IFCPROPERTYSINGLEVALUE('ColorTemperature',$,IFCTHERMODYNAMICTEMPERATUREMEASURE(4000.0),$);
#15=IFCPROPERTYSINGLEVALUE('LuminousFlux',$,IFCLUMINOUSFLUXMEASURE(3000.0),$);
#16=IFCPROPERTYSET('pset2',#5,'Pset_LuminaireVariant',$,(#13,#14,#15));
#17=IFCRELDEFINESBYPROPERTIES('rel3',#5,$,$,(#6),#16);
ENDSEC;

END-ISO-10303-21;
"#;

        // Import the IFC content
        let luminaire = import_ifc(ifc_content).unwrap();

        assert_eq!(luminaire.name, "LED Panel");
        assert_eq!(
            luminaire.manufacturer,
            Some("Test Manufacturer".to_string())
        );
        assert_eq!(luminaire.variants.len(), 2);

        // Find the 4000K variant (order may vary due to HashMap)
        let v1 = luminaire
            .variants
            .iter()
            .find(|v| v.name.contains("4000K"))
            .expect("Should have 4000K variant");
        assert_eq!(v1.gldf_variant_id, Some("var_4000K".to_string()));
        assert_eq!(v1.properties.color_temperature, Some(4000.0));
        assert_eq!(v1.properties.luminous_flux, Some(3000.0));

        // Generate GLDF
        let gldf_bytes = ifc_to_gldf(ifc_content).unwrap();

        // Verify it's a valid ZIP
        assert!(gldf_bytes.len() > 100);
        assert_eq!(&gldf_bytes[0..2], &[0x50, 0x4B]); // PK signature

        // Extract and check product.xml
        let cursor = std::io::Cursor::new(&gldf_bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut product_xml = String::new();
        use std::io::Read;
        archive
            .by_name("product.xml")
            .unwrap()
            .read_to_string(&mut product_xml)
            .unwrap();

        assert!(product_xml.contains("LED Panel"));
        assert!(product_xml.contains("Test Manufacturer"));
        // Variant IDs are generated as var_1, var_2, etc.
        assert!(product_xml.contains("var_1"));

        println!("=== IFC to GLDF Roundtrip ===");
        println!("Imported {} variants", luminaire.variants.len());
        println!("GLDF size: {} bytes", gldf_bytes.len());
    }

    #[test]
    fn test_electrical_mvd_roundtrip() {
        // Test the full Electrical MVD properties roundtrip
        // IFC with Pset_ElectricalDeviceCommon properties
        let ifc_content = r#"
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('GLDF Export - Electrical MVD'),'2;1');
FILE_NAME('test_electrical.ifc','2024-01-01',(''),(''),'gldf-rs','gldf-rs','');
FILE_SCHEMA(('IFC4'));
ENDSEC;

DATA;
#1=IFCPERSON($,$,'',$,$,$,$,$);
#2=IFCORGANIZATION($,'Acme Lighting','Test',$,$);
#3=IFCPERSONANDORGANIZATION(#1,#2,$);
#4=IFCAPPLICATION(#2,'1.0','gldf-rs','gldf-rs');
#5=IFCOWNERHISTORY(#3,#4,.READWRITE.,.ADDED.,$,$,$,0);
#6=IFCLIGHTFIXTURETYPE('guid1',#5,'LED Panel - 4000K','Standard 4000K variant',$,$,$,$,$,.POINTSOURCE.);
#7=IFCPROPERTYSINGLEVALUE('Manufacturer',$,IFCLABEL('Acme Lighting'),$);
#8=IFCPROPERTYSINGLEVALUE('ModelReference',$,IFCLABEL('LED-PANEL-001'),$);
#9=IFCPROPERTYSET('pset1',#5,'Pset_ManufacturerTypeInformation',$,(#7,#8));
#10=IFCRELDEFINESBYPROPERTIES('rel1',#5,$,$,(#6),#9);
#11=IFCPROPERTYSINGLEVALUE('IP_Code',$,IFCLABEL('IP65'),$);
#12=IFCPROPERTYSINGLEVALUE('IK_Code',$,IFCLABEL('IK08'),$);
#13=IFCPROPERTYSINGLEVALUE('RatedVoltage',$,IFCELECTRICVOLTAGEMEASURE(230.0),$);
#14=IFCPROPERTYSINGLEVALUE('RatedCurrent',$,IFCELECTRICCURRENTMEASURE(0.15),$);
#15=IFCPROPERTYSINGLEVALUE('PowerFactor',$,IFCNORMALISEDRATIOMEASURE(0.95),$);
#16=IFCPROPERTYSINGLEVALUE('NumberOfPoles',$,IFCINTEGER(2),$);
#17=IFCPROPERTYSINGLEVALUE('HasProtectiveEarth',$,.T.,$);
#18=IFCPROPERTYSINGLEVALUE('InsulationStandardClass',$,.CLASSII.,$);
#19=IFCPROPERTYSINGLEVALUE('Power',$,IFCPOWERMEASURE(35.0),$);
#20=IFCPROPERTYSET('pset2',#5,'Pset_ElectricalDeviceCommon',$,(#11,#12,#13,#14,#15,#16,#17,#18,#19));
#21=IFCRELDEFINESBYPROPERTIES('rel2',#5,$,$,(#6),#20);
#22=IFCPROPERTYSINGLEVALUE('MaintenanceFactor',$,IFCNORMALISEDRATIOMEASURE(0.8),$);
#23=IFCPROPERTYSINGLEVALUE('NumberOfSources',$,IFCINTEGER(1),$);
#24=IFCPROPERTYSINGLEVALUE('TotalWattage',$,IFCPOWERMEASURE(35.0),$);
#25=IFCPROPERTYSET('pset3',#5,'Pset_LightFixtureTypeCommon',$,(#22,#23,#24));
#26=IFCRELDEFINESBYPROPERTIES('rel3',#5,$,$,(#6),#25);
ENDSEC;

END-ISO-10303-21;
"#;

        // Import the IFC content
        let luminaire = import_ifc(ifc_content).unwrap();

        // Verify basic info
        assert_eq!(luminaire.name, "LED Panel");
        assert_eq!(luminaire.manufacturer, Some("Acme Lighting".to_string()));
        assert_eq!(luminaire.variants.len(), 1);

        // Get the variant
        let v = &luminaire.variants[0];

        // Verify electrical properties from Pset_ElectricalDeviceCommon
        assert_eq!(v.properties.ip_code, Some("IP65".to_string()));
        assert_eq!(v.properties.ik_code, Some("IK08".to_string()));
        assert_eq!(v.properties.rated_voltage, Some(230.0));
        assert_eq!(v.properties.rated_current, Some(0.15));
        assert_eq!(v.properties.power_factor, Some(0.95));
        assert_eq!(v.properties.number_of_poles, Some(2));
        assert_eq!(v.properties.has_protective_earth, Some(true));
        assert_eq!(
            v.properties.insulation_standard_class,
            Some("CLASSII".to_string())
        );
        assert_eq!(v.properties.power, Some(35.0));

        // Verify properties from Pset_LightFixtureTypeCommon
        assert_eq!(v.properties.maintenance_factor, Some(0.8));
        assert_eq!(v.properties.number_of_sources, Some(1));
        assert_eq!(v.properties.total_wattage, Some(35.0));

        // Generate GLDF
        let gldf_bytes = ifc_to_gldf(ifc_content).unwrap();

        // Verify it's a valid ZIP
        assert!(gldf_bytes.len() > 100);
        assert_eq!(&gldf_bytes[0..2], &[0x50, 0x4B]); // PK signature

        // Extract and check product.xml
        let cursor = std::io::Cursor::new(&gldf_bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut product_xml = String::new();
        use std::io::Read;
        archive
            .by_name("product.xml")
            .unwrap()
            .read_to_string(&mut product_xml)
            .unwrap();

        assert!(product_xml.contains("LED Panel"));
        assert!(product_xml.contains("Acme Lighting"));
        // IP code should be preserved in GLDF
        assert!(product_xml.contains("IP65") || product_xml.contains("ip-code"));

        println!("=== Electrical MVD Roundtrip ===");
        println!("Imported electrical properties:");
        println!("  IP Code: {:?}", v.properties.ip_code);
        println!("  IK Code: {:?}", v.properties.ik_code);
        println!("  Rated Voltage: {:?}", v.properties.rated_voltage);
        println!("  Power Factor: {:?}", v.properties.power_factor);
        println!(
            "  Insulation Class: {:?}",
            v.properties.insulation_standard_class
        );
        println!(
            "  Has Protective Earth: {:?}",
            v.properties.has_protective_earth
        );
        println!("GLDF size: {} bytes", gldf_bytes.len());
    }

    #[test]
    #[cfg(feature = "l3d")]
    fn test_l3d_parsing_comparison() {
        // Test working L3D
        let working_paths = ["tests/data/test.gldf", "../../tests/data/test.gldf"];

        for path in &working_paths {
            if let Ok(buf) = std::fs::read(path) {
                if let Ok(gldf) = crate::GldfProduct::load_gldf_from_buf_all(buf) {
                    if let Some(l3d_file) = gldf.files.iter().find(|f| {
                        f.name
                            .as_ref()
                            .map(|n| n.ends_with(".l3d"))
                            .unwrap_or(false)
                    }) {
                        if let Some(l3d_bytes) = &l3d_file.content {
                            let l3d = l3d_rs::from_buffer(l3d_bytes);
                            println!("=== Working L3D (test.gldf) ===");
                            println!("Parts: {}", l3d.model.parts.len());
                            for p in &l3d.model.parts {
                                println!("  Part path: '{}'", p.path);
                            }
                            println!("Assets: {}", l3d.file.assets.len());
                            for a in &l3d.file.assets {
                                println!("  Asset: '{}' ({} bytes)", a.name, a.content.len());
                            }
                            return;
                        }
                    }
                }
            }
        }
        println!("Could not find working test L3D");
    }
}
