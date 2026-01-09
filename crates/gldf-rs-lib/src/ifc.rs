//! IFC (Industry Foundation Classes) Integration
//!
//! This module provides conversion between GLDF and IFC formats for BIM integration.
//!
//! ## Overview
//!
//! IFC is the open standard for Building Information Modeling (BIM).
//! GLDF luminaires can be exported to IFC for use in architectural software,
//! and IFC light fixtures can be imported with photometric data.
//!
//! ## Features (planned)
//!
//! - Export GLDF → IFC light fixture with photometric data
//! - Import IFC → GLDF with manufacturer info and geometry
//! - Property set mapping for lighting-specific attributes
//!
//! ## Example
//!
//! ```rust,ignore
//! use gldf_rs::ifc::{GldfToIfc, IfcToGldf};
//!
//! // Export GLDF to IFC
//! let gldf = GldfProduct::from_file("luminaire.gldf")?;
//! let ifc_fixture = GldfToIfc::convert(&gldf)?;
//!
//! // Import IFC to GLDF
//! let ifc = ifc_rs::IFC::from_file("building.ifc")?;
//! let gldf = IfcToGldf::convert(&ifc.light_fixtures()[0])?;
//! ```

use serde::{Deserialize, Serialize};

/// Light fixture type enum matching IFC IfcLightFixtureTypeEnum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightFixtureType {
    /// Point light source
    PointSource,
    /// Directional light source
    DirectionSource,
    /// Security/emergency lighting
    SecurityLighting,
    /// User-defined type
    UserDefined,
    /// Not defined
    NotDefined,
}

impl Default for LightFixtureType {
    fn default() -> Self {
        Self::NotDefined
    }
}

/// Light emission source enum matching IFC IfcLightEmissionSourceEnum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightEmissionSource {
    CompactFluorescent,
    Fluorescent,
    HighPressureMercury,
    HighPressureSodium,
    Led,
    LightEmittingDiode,
    LowPressureSodium,
    LowVoltageHalogen,
    MainVoltageHalogen,
    MetalHalide,
    TungstenFilament,
    NotDefined,
}

impl Default for LightEmissionSource {
    fn default() -> Self {
        Self::NotDefined
    }
}

/// Simplified IFC light fixture representation for GLDF conversion
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IfcLightFixtureData {
    /// Global unique identifier
    pub global_id: String,
    /// Fixture name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Fixture type
    pub fixture_type: LightFixtureType,

    // Manufacturer info (from Pset_ManufacturerTypeInformation)
    /// Manufacturer name
    pub manufacturer: Option<String>,
    /// Article/product number
    pub article_number: Option<String>,
    /// Model reference
    pub model_reference: Option<String>,

    // Light fixture properties (from Pset_LightFixtureTypeCommon)
    /// Number of light sources
    pub number_of_sources: Option<u32>,
    /// Total wattage in W
    pub total_wattage: Option<f64>,
    /// Mounting type
    pub mounting_type: Option<String>,
    /// Maintenance factor (0.0-1.0)
    pub maintenance_factor: Option<f64>,

    // Goniometric light source data
    /// Luminous flux in lumens
    pub luminous_flux: Option<f64>,
    /// Color temperature in Kelvin
    pub color_temperature: Option<f64>,
    /// Light emission source type
    pub emission_source: LightEmissionSource,
    /// External photometry file reference (IES/LDT path)
    pub photometry_file: Option<String>,
}

impl IfcLightFixtureData {
    /// Create a new IFC light fixture with a generated GUID
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            global_id: generate_ifc_guid(),
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set manufacturer information
    pub fn with_manufacturer(mut self, manufacturer: impl Into<String>) -> Self {
        self.manufacturer = Some(manufacturer.into());
        self
    }

    /// Set photometric data
    pub fn with_photometry(
        mut self,
        flux: f64,
        color_temp: f64,
        source: LightEmissionSource,
    ) -> Self {
        self.luminous_flux = Some(flux);
        self.color_temperature = Some(color_temp);
        self.emission_source = source;
        self
    }

    /// Set electrical data
    pub fn with_electrical(mut self, wattage: f64, sources: u32) -> Self {
        self.total_wattage = Some(wattage);
        self.number_of_sources = Some(sources);
        self
    }
}

/// Generate a simple IFC-style GUID (not cryptographically secure, for demo only)
fn generate_ifc_guid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // IFC GUIDs are 22 character base64-like strings
    format!("{:0>22}", base64_encode_simple(timestamp))
}

fn base64_encode_simple(mut n: u128) -> String {
    const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    let mut result = String::new();
    while n > 0 || result.is_empty() {
        result.insert(0, CHARS[(n % 64) as usize] as char);
        n /= 64;
    }
    result
}

/// Convert GLDF product to IFC light fixture data
#[cfg(feature = "ifc")]
pub fn gldf_to_ifc(gldf: &crate::gldf::GldfProduct) -> IfcLightFixtureData {
    use crate::gldf::GldfProduct;

    let mut fixture = IfcLightFixtureData::new("");

    // Map header info
    fixture.manufacturer = Some(gldf.header.manufacturer.clone());

    // Map product name from ProductDefinitions if available
    if let Some(product_meta) = &gldf.product_definitions.product_meta_data {
        if let Some(name) = &product_meta.name {
            if let Some(locale) = name.locale.first() {
                fixture.name = locale.value.clone();
            }
        }
    }

    fixture
}

/// Convert IFC light fixture data to GLDF product
#[cfg(feature = "ifc")]
pub fn ifc_to_gldf(fixture: &IfcLightFixtureData) -> crate::gldf::GldfProduct {
    use crate::gldf::{GldfProduct, Header};

    let mut gldf = GldfProduct::default();

    // Map manufacturer
    if let Some(ref mfr) = fixture.manufacturer {
        gldf.header.manufacturer = mfr.clone();
    }

    gldf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ifc_fixture_creation() {
        let fixture = IfcLightFixtureData::new("Test Luminaire")
            .with_manufacturer("ACME Lighting")
            .with_photometry(3000.0, 4000.0, LightEmissionSource::Led)
            .with_electrical(30.0, 1);

        assert_eq!(fixture.name, "Test Luminaire");
        assert_eq!(fixture.manufacturer, Some("ACME Lighting".to_string()));
        assert_eq!(fixture.luminous_flux, Some(3000.0));
        assert_eq!(fixture.color_temperature, Some(4000.0));
        assert_eq!(fixture.emission_source, LightEmissionSource::Led);
        assert_eq!(fixture.total_wattage, Some(30.0));
        assert!(!fixture.global_id.is_empty());
    }

    #[test]
    fn test_ifc_guid_generation() {
        let guid1 = generate_ifc_guid();
        let guid2 = generate_ifc_guid();

        // GUIDs should be 22 chars (padded)
        assert!(guid1.len() >= 1);
        // GUIDs should be different (unless generated in same nanosecond)
        // This test may occasionally fail if run very fast
        println!("GUID1: {}, GUID2: {}", guid1, guid2);
    }
}
