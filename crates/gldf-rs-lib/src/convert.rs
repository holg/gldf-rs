//! Conversion utilities for creating GLDF from other formats.
//!
//! This module provides functions to convert photometric files (LDT/IES)
//! to GLDF format.
//!
//! ## Feature Flag
//!
//! This module requires the `eulumdat` feature to be enabled:
//!
//! ```toml
//! [dependencies]
//! gldf-rs = { version = "0.3", features = ["eulumdat"] }
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use gldf_rs::convert::ldt_to_gldf;
//!
//! let ldt_data = std::fs::read("luminaire.ldt")?;
//! let gldf = ldt_to_gldf(&ldt_data, "luminaire.ldt")?;
//!
//! // Access the GLDF structure
//! println!("Manufacturer: {}", gldf.gldf.header.manufacturer);
//! ```

#[cfg(feature = "eulumdat")]
use eulumdat::{Eulumdat, IesParser};

use crate::{
    BufFile, FileBufGldf,
    gldf::{
        GldfProduct, Header, FormatVersion, GeneralDefinitions, ProductDefinitions,
        ProductMetaData, Variants, Variant,
        Files, File, Photometries, Photometry, PhotometryFileReference,
        LightSources, FixedLightSource,
        Emitters, Emitter, FixedLightEmitter, LightSourceReference, PhotometryReference,
        Locale, LocaleFoo,
    },
};

/// Metadata extracted from an Eulumdat/IES file.
#[derive(Debug, Clone, Default)]
pub struct LdtMetadata {
    /// Manufacturer/Company (line 1 in EULUMDAT).
    pub manufacturer: Option<String>,
    /// Luminaire name.
    pub name: String,
    /// Luminaire number/ID.
    pub luminaire_number: Option<String>,
    /// Total luminous flux in lumens.
    pub lumens: Option<f64>,
    /// Total wattage in watts.
    pub watts: Option<f64>,
    /// Number of lamps.
    pub lamp_count: Option<i32>,
    /// Lamp type description.
    pub lamp_type: Option<String>,
    /// Luminaire dimensions (length, width, height) in mm.
    pub dimensions: Option<(f64, f64, f64)>,
    /// Symmetry type.
    pub symmetry: Option<String>,
    /// Measurement report number.
    pub measurement_report: Option<String>,
}

#[cfg(feature = "eulumdat")]
impl LdtMetadata {
    /// Extract metadata from a parsed Eulumdat structure.
    pub fn from_eulumdat(ldt: &Eulumdat) -> Self {
        // Get first lamp set info if available
        let lamp_set = ldt.lamp_sets.first();

        Self {
            manufacturer: if !ldt.identification.is_empty() {
                Some(ldt.identification.clone())
            } else {
                None
            },
            name: if !ldt.luminaire_name.is_empty() {
                ldt.luminaire_name.clone()
            } else {
                "Unknown Luminaire".to_string()
            },
            luminaire_number: if !ldt.luminaire_number.is_empty() {
                Some(ldt.luminaire_number.clone())
            } else {
                None
            },
            lumens: {
                let flux = ldt.total_luminous_flux();
                if flux > 0.0 { Some(flux) } else { None }
            },
            watts: {
                let watts = ldt.total_wattage();
                if watts > 0.0 { Some(watts) } else { None }
            },
            lamp_count: lamp_set.map(|ls| ls.num_lamps),
            lamp_type: lamp_set.and_then(|ls| {
                if !ls.lamp_type.is_empty() {
                    Some(ls.lamp_type.clone())
                } else {
                    None
                }
            }),
            dimensions: {
                let l = ldt.length;
                let w = ldt.width;
                let h = ldt.height;
                if l > 0.0 || w > 0.0 || h > 0.0 {
                    Some((l, w, h))
                } else {
                    None
                }
            },
            symmetry: Some(format!("{:?}", ldt.symmetry)),
            measurement_report: if !ldt.measurement_report_number.is_empty() {
                Some(ldt.measurement_report_number.clone())
            } else {
                None
            },
        }
    }
}

/// Convert LDT/IES bytes to a GLDF structure.
///
/// Parses the photometric file and creates a minimal but valid GLDF structure
/// containing the photometry data, light source, and emitter definitions.
///
/// # Arguments
/// * `data` - The raw LDT or IES file bytes
/// * `filename` - Original filename (used to determine format and for file reference)
///
/// # Returns
/// * `Ok(FileBufGldf)` - The converted GLDF structure with embedded photometry
/// * `Err(String)` - Error message if parsing or conversion fails
///
/// # Example
///
/// ```ignore
/// let ldt_bytes = std::fs::read("test.ldt")?;
/// let gldf = ldt_to_gldf(&ldt_bytes, "test.ldt")?;
///
/// // The GLDF now contains:
/// // - Header with manufacturer from LDT line 1
/// // - Files section with the embedded LDT
/// // - Photometries referencing the file
/// // - LightSources with power/flux data
/// // - Emitters linking everything together
/// ```
#[cfg(feature = "eulumdat")]
pub fn ldt_to_gldf(data: &[u8], filename: &str) -> Result<FileBufGldf, String> {
    // Parse the photometric data
    let content = std::str::from_utf8(data)
        .map_err(|e| format!("Invalid UTF-8 encoding: {:?}", e))?;

    let ldt = if content.trim_start().starts_with("IESNA") {
        IesParser::parse(content).map_err(|e| format!("IES parse error: {:?}", e))?
    } else {
        Eulumdat::parse(content).map_err(|e| format!("LDT parse error: {:?}", e))?
    };

    // Extract metadata
    let meta = LdtMetadata::from_eulumdat(&ldt);

    // Build GLDF from metadata
    ldt_metadata_to_gldf(&meta, data, filename)
}

/// Convert LDT metadata to a GLDF structure.
///
/// This is useful when you've already parsed the LDT and extracted metadata,
/// or when you want to customize the metadata before conversion.
///
/// # Arguments
/// * `meta` - The extracted LDT metadata
/// * `data` - The raw LDT/IES file bytes to embed
/// * `filename` - Filename for the embedded file
pub fn ldt_metadata_to_gldf(meta: &LdtMetadata, data: &[u8], filename: &str) -> Result<FileBufGldf, String> {
    // Determine content type from filename
    let is_ies = filename.to_lowercase().ends_with(".ies");
    let content_type = if is_ies { "ldc/ies" } else { "ldc/eulumdat" };

    // Simple timestamp (could be improved with chrono if available)
    let timestamp = "2024-01-01T00:00:00Z".to_string();

    // Files
    let file_id = "photometry_file";
    let files = Files {
        file: vec![File {
            id: file_id.to_string(),
            content_type: content_type.to_string(),
            type_attr: "localFileName".to_string(),
            language: String::new(),
            file_name: filename.to_string(),
        }],
    };

    // Photometries
    let photometries = Some(Photometries {
        photometry: vec![Photometry {
            id: "photometry".to_string(),
            photometry_file_reference: Some(PhotometryFileReference {
                file_id: file_id.to_string(),
            }),
            descriptive_photometry: None,
        }],
    });

    // LightSources - create from metadata
    let light_source_id = "lightsource_1";
    let light_sources = Some(LightSources {
        changeable_light_source: vec![],
        fixed_light_source: vec![FixedLightSource {
            id: light_source_id.to_string(),
            name: LocaleFoo {
                locale: vec![Locale {
                    language: "en".to_string(),
                    value: meta.lamp_type.clone().unwrap_or_else(|| meta.name.clone()),
                }],
            },
            description: meta.luminaire_number.as_ref().map(|n| LocaleFoo {
                locale: vec![Locale {
                    language: "en".to_string(),
                    value: n.clone(),
                }],
            }),
            manufacturer: meta.manufacturer.clone(),
            rated_input_power: meta.watts,
            color_information: None,
            ..Default::default()
        }],
    });

    // Emitters
    let emitter_id = "emitter_1";
    let emitters = Some(Emitters {
        emitter: vec![Emitter {
            id: emitter_id.to_string(),
            changeable_light_emitter: vec![],
            fixed_light_emitter: vec![FixedLightEmitter {
                name: Some(LocaleFoo {
                    locale: vec![Locale {
                        language: "en".to_string(),
                        value: "Main Emitter".to_string(),
                    }],
                }),
                photometry_reference: PhotometryReference {
                    photometry_id: "photometry".to_string(),
                },
                light_source_reference: LightSourceReference {
                    fixed_light_source_id: Some(light_source_id.to_string()),
                    changeable_light_source_id: None,
                    light_source_count: meta.lamp_count,
                },
                rated_luminous_flux: meta.lumens.map(|l| l as i32),
                ..Default::default()
            }],
            sensor: vec![],
        }],
    });

    // Variant
    let variant = Variant {
        id: "variant_1".to_string(),
        name: Some(LocaleFoo {
            locale: vec![Locale {
                language: "en".to_string(),
                value: meta.name.clone(),
            }],
        }),
        description: meta.measurement_report.as_ref().map(|r| LocaleFoo {
            locale: vec![Locale {
                language: "en".to_string(),
                value: format!("Report: {}", r),
            }],
        }),
        ..Default::default()
    };

    // Build GldfProduct
    let gldf_product = GldfProduct {
        path: String::new(),
        xmlns_xsi: "http://www.w3.org/2001/XMLSchema-instance".to_string(),
        xsnonamespaceschemalocation: "https://gldf.io/xsd/gldf/1.0.0/gldf.xsd".to_string(),
        header: Header {
            manufacturer: meta.manufacturer.clone().unwrap_or_else(|| "Unknown".to_string()),
            creation_time_code: timestamp,
            created_with_application: "gldf-rs".to_string(),
            format_version: FormatVersion {
                major: 1,
                minor: 0,
                pre_release: 0,
            },
            ..Default::default()
        },
        general_definitions: GeneralDefinitions {
            files,
            photometries,
            light_sources,
            emitters,
            ..Default::default()
        },
        product_definitions: ProductDefinitions {
            product_meta_data: Some(ProductMetaData {
                name: Some(LocaleFoo {
                    locale: vec![Locale {
                        language: "en".to_string(),
                        value: meta.name.clone(),
                    }],
                }),
                description: meta.lamp_type.as_ref().map(|t| LocaleFoo {
                    locale: vec![Locale {
                        language: "en".to_string(),
                        value: t.clone(),
                    }],
                }),
                ..Default::default()
            }),
            variants: Some(Variants {
                variant: vec![variant],
            }),
        },
    };

    // Create the file path for the embedded LDT
    let zip_path = format!("ldc/{}", filename);

    // Create FileBufGldf
    let buf_files = vec![BufFile {
        name: Some(zip_path),
        content: Some(data.to_vec()),
        file_id: Some(file_id.to_string()),
        path: None,
    }];

    Ok(FileBufGldf {
        files: buf_files,
        gldf: gldf_product,
    })
}

#[cfg(test)]
#[cfg(feature = "eulumdat")]
mod tests {
    use super::*;

    #[test]
    fn test_ldt_metadata_default() {
        let meta = LdtMetadata::default();
        assert_eq!(meta.name, "");
        assert!(meta.manufacturer.is_none());
    }
}
