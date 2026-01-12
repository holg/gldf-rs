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
//! ## Example
//!
//! ```rust,ignore
//! use gldf_rs::ifc::GldfToIfc;
//!
//! let gldf = GldfProduct::from_file("luminaire.gldf")?;
//! let ifc_content = GldfToIfc::export(&gldf)?;
//! std::fs::write("luminaire.ifc", ifc_content)?;
//! ```

mod step_writer;
mod types;

pub use step_writer::StepWriter;
pub use types::*;

use crate::gldf::GldfProduct;
use anyhow::Result;

/// Extracted light source data for IFC export
struct LightSourceData {
    name: String,
    luminous_flux: Option<f64>,
    color_temperature: Option<f64>,
    rated_power: Option<f64>,
    emission_source: LightEmissionSourceEnum,
    photometry_file: Option<String>,
}

/// Export GLDF to IFC STEP format
pub struct GldfToIfc;

impl GldfToIfc {
    /// Export a GLDF product to IFC STEP format
    ///
    /// Returns the IFC file content as a string
    pub fn export(gldf: &GldfProduct) -> Result<String> {
        let mut writer = StepWriter::new("IFC4");

        // Get product info
        let manufacturer = &gldf.header.manufacturer;
        let product_name = Self::get_product_name(gldf);

        // Extract light source data
        let light_sources = Self::extract_light_sources(gldf);
        let num_sources = light_sources.len() as i32;
        let total_power: f64 = light_sources.iter().filter_map(|ls| ls.rated_power).sum();

        // Create minimal building structure
        let owner_history = writer.add_owner_history(manufacturer);
        let project = writer.add_project("GLDF Export", owner_history);
        let site = writer.add_site("Default Site", owner_history, project);
        let building = writer.add_building("Default Building", owner_history, site);
        let storey = writer.add_storey("Ground Floor", owner_history, building);

        // Determine fixture type from light sources
        let fixture_type_enum = if light_sources.len() > 1 {
            LightFixtureTypeEnum::DirectionSource
        } else {
            LightFixtureTypeEnum::PointSource
        };

        // Create light fixture type
        let fixture_type = writer.add_light_fixture_type(
            &product_name,
            manufacturer,
            fixture_type_enum,
            owner_history,
        );

        // Add light sources as goniometric sources
        for ls in &light_sources {
            let flux = ls.luminous_flux.unwrap_or(1000.0);
            let cct = ls.color_temperature.unwrap_or(4000.0);

            writer.add_light_source_goniometric(
                &ls.name,
                None, // Let IFC viewer determine color from CCT
                cct,
                flux,
                ls.emission_source,
                ls.photometry_file.as_deref(),
            );
        }

        // Add Pset_LightFixtureTypeCommon
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

        // Add electrical properties if available
        Self::add_electrical_properties(gldf, &mut writer, fixture_type, owner_history);

        // Create light fixture occurrence
        let _fixture = writer.add_light_fixture(
            &format!("{}_001", product_name),
            owner_history,
            storey,
            Some(fixture_type),
        );

        Ok(writer.to_step_string())
    }

    /// Extract light source information from GLDF
    fn extract_light_sources(gldf: &GldfProduct) -> Vec<LightSourceData> {
        let mut sources = Vec::new();

        // Get photometry file references
        let photometry_files: std::collections::HashMap<String, String> = gldf
            .general_definitions
            .files
            .file
            .iter()
            .filter(|f| f.content_type.starts_with("ldc"))
            .map(|f| (f.id.clone(), format!("ldc/{}", f.file_name)))
            .collect();

        // Extract from fixed light sources
        if let Some(light_sources) = &gldf.general_definitions.light_sources {
            for ls in &light_sources.fixed_light_source {
                let name = ls
                    .name
                    .locale
                    .first()
                    .map(|l| l.value.clone())
                    .unwrap_or_else(|| ls.id.clone());

                let color_temp = ls
                    .color_information
                    .as_ref()
                    .and_then(|ci| ci.correlated_color_temperature)
                    .map(|t| t as f64);

                sources.push(LightSourceData {
                    name,
                    luminous_flux: None, // Flux comes from emitter
                    color_temperature: color_temp,
                    rated_power: ls.rated_input_power,
                    emission_source: LightEmissionSourceEnum::Led, // Assume LED for now
                    photometry_file: None,
                });
            }
        }

        // Extract from emitters (which have actual photometry references)
        if let Some(emitters) = &gldf.general_definitions.emitters {
            for emitter in &emitters.emitter {
                for fixed_emitter in &emitter.fixed_light_emitter {
                    // LocaleFoo has locale: Vec<Locale>, Locale has value: String
                    let name = fixed_emitter
                        .name
                        .as_ref()
                        .and_then(|n| n.locale.first())
                        .map(|l| l.value.clone())
                        .unwrap_or_else(|| emitter.id.clone());

                    let photometry_file = photometry_files
                        .get(&fixed_emitter.photometry_reference.photometry_id)
                        .cloned();

                    sources.push(LightSourceData {
                        name,
                        luminous_flux: fixed_emitter.rated_luminous_flux.map(|f| f as f64),
                        color_temperature: None,
                        rated_power: None,
                        emission_source: LightEmissionSourceEnum::Led,
                        photometry_file,
                    });
                }

                for changeable_emitter in &emitter.changeable_light_emitter {
                    // Locale has direct value field
                    let name = changeable_emitter
                        .name
                        .as_ref()
                        .map(|n| n.value.clone())
                        .unwrap_or_else(|| emitter.id.clone());

                    let photometry_file = photometry_files
                        .get(&changeable_emitter.photometry_reference.photometry_id)
                        .cloned();

                    sources.push(LightSourceData {
                        name,
                        luminous_flux: None,
                        color_temperature: None,
                        rated_power: None,
                        emission_source: LightEmissionSourceEnum::NotDefined,
                        photometry_file,
                    });
                }
            }
        }

        // If no sources found, create a default one
        if sources.is_empty() {
            sources.push(LightSourceData {
                name: "Light Source".to_string(),
                luminous_flux: Some(1000.0),
                color_temperature: Some(4000.0),
                rated_power: None,
                emission_source: LightEmissionSourceEnum::NotDefined,
                photometry_file: None,
            });
        }

        sources
    }

    /// Get mounting type from GLDF descriptive attributes
    fn get_mounting_type(gldf: &GldfProduct) -> Option<&'static str> {
        if let Some(meta) = &gldf.product_definitions.product_meta_data {
            if let Some(attrs) = &meta.descriptive_attributes {
                if let Some(mechanical) = &attrs.mechanical {
                    // Check product form for mounting hints
                    if mechanical.product_form.is_some() {
                        return Some("SURFACE");
                    }
                }
            }
        }
        None
    }

    /// Add electrical properties from GLDF
    fn add_electrical_properties(
        gldf: &GldfProduct,
        writer: &mut StepWriter,
        fixture_type: EntityRef,
        owner_history: EntityRef,
    ) {
        let mut voltage: Option<f64> = None;
        let mut ip_code: Option<String> = None;

        // Extract from descriptive attributes
        if let Some(meta) = &gldf.product_definitions.product_meta_data {
            if let Some(attrs) = &meta.descriptive_attributes {
                if let Some(electrical) = &attrs.electrical {
                    // Get IP code
                    if let Some(ip) = &electrical.ingress_protection_ip_code {
                        ip_code = Some(format!("IP{}", ip));
                    }
                }
            }
        }

        // Get voltage from control gears
        if let Some(control_gears) = &gldf.general_definitions.control_gears {
            for cg in &control_gears.control_gear {
                if let Some(nominal) = &cg.nominal_voltage {
                    // Get the fixed voltage value directly
                    if let Some(v) = nominal.fixed_voltage {
                        voltage = Some(v);
                        break;
                    }
                    // Or get from voltage range
                    if let Some(range) = &nominal.voltage_range {
                        voltage = Some((range.min + range.max) / 2.0);
                        break;
                    }
                }
            }
        }

        // Only add property set if we have at least one value
        if voltage.is_some() || ip_code.is_some() {
            writer.add_electrical_pset(
                fixture_type,
                owner_history,
                voltage,
                None,
                None,
                ip_code.as_deref(),
            );
        }
    }

    fn get_product_name(gldf: &GldfProduct) -> String {
        // Try to get name from ProductMetaData
        if let Some(meta) = &gldf.product_definitions.product_meta_data {
            if let Some(name) = &meta.name {
                if let Some(locale) = name.locale.first() {
                    if !locale.value.is_empty() {
                        return locale.value.clone();
                    }
                }
            }
        }
        // Fallback to manufacturer name
        gldf.header.manufacturer.clone()
    }
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
        // Load a test GLDF file
        let gldf = GldfProduct::load_gldf("tests/data/test.gldf");
        if let Ok(gldf) = gldf {
            let ifc_result = GldfToIfc::export(&gldf);
            assert!(ifc_result.is_ok(), "IFC export should succeed");

            let ifc_content = ifc_result.unwrap();

            // Verify basic IFC structure
            assert!(ifc_content.contains("ISO-10303-21"));
            assert!(ifc_content.contains("IFC4"));
            assert!(ifc_content.contains("IFCPROJECT"));
            assert!(ifc_content.contains("IFCLIGHTFIXTURETYPE"));
            assert!(ifc_content.contains("IFCLIGHTFIXTURE"));

            // Verify the manufacturer is included
            assert!(ifc_content.contains(&gldf.header.manufacturer));

            println!("=== GLDF to IFC Export ===");
            println!("{}", ifc_content);
        } else {
            // Skip test if file not found
            println!("Test GLDF file not found, skipping test");
        }
    }
}
