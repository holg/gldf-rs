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
use eulumdat::{Eulumdat, GldfPhotometricData, IesData, IesMetadata, IesParser};

use crate::{
    gldf::{
        DescriptivePhotometry, Emitter, Emitters, File, Files, FixedLightEmitter,
        FixedLightSource, FormatVersion, GeneralDefinitions, GldfProduct, HalfPeakDivergence,
        Header, LightSourceReference, LightSources, Locale, LocaleFoo, Photometries, Photometry,
        PhotometryFileReference, PhotometryReference, ProductDefinitions, ProductMetaData,
        TenthPeakDivergence, UGR4H8H705020LQ, Variant, Variants,
    },
    BufFile, FileBufGldf,
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

    // === IES LM-63-19 Specific Fields ===
    /// IES-specific metadata (test lab, file generation type, etc.)
    #[cfg(feature = "eulumdat")]
    pub ies_metadata: Option<IesMetadata>,
    /// Calculated photometric properties for GLDF DescriptivePhotometry
    #[cfg(feature = "eulumdat")]
    pub photometric_data: Option<GldfPhotometricData>,
}

#[cfg(feature = "eulumdat")]
impl LdtMetadata {
    /// Extract metadata from a parsed Eulumdat structure.
    pub fn from_eulumdat(ldt: &Eulumdat) -> Self {
        // Get first lamp set info if available
        let lamp_set = ldt.lamp_sets.first();

        // Calculate GLDF photometric data
        let photometric_data = Some(GldfPhotometricData::from_eulumdat(ldt));

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
                if flux > 0.0 {
                    Some(flux)
                } else {
                    None
                }
            },
            watts: {
                let watts = ldt.total_wattage();
                if watts > 0.0 {
                    Some(watts)
                } else {
                    None
                }
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
            ies_metadata: None, // Populated by from_ies_data
            photometric_data,
        }
    }

    /// Extract metadata from parsed IES data, including LM-63-19 specific fields.
    ///
    /// This method extracts additional metadata available in IES files that isn't
    /// present in EULUMDAT format, such as:
    /// - File generation type (accredited lab, simulation, etc.)
    /// - Luminous opening shape and dimensions
    /// - TILT data
    /// - Test lab and issue date
    pub fn from_ies_data(ies: &IesData, ldt: &Eulumdat) -> Self {
        let mut meta = Self::from_eulumdat(ldt);

        // Override manufacturer from IES if available
        if !ies.manufacturer.is_empty() {
            meta.manufacturer = Some(ies.manufacturer.clone());
        }

        // Override name from IES LUMINAIRE if available
        if !ies.luminaire.is_empty() {
            meta.name = ies.luminaire.clone();
        }

        // Use IES catalog number if available
        if !ies.luminaire_catalog.is_empty() {
            meta.luminaire_number = Some(ies.luminaire_catalog.clone());
        }

        // Add IES-specific metadata
        meta.ies_metadata = Some(IesMetadata::from_ies_data(ies));

        meta
    }
}

/// Convert LDT/IES bytes to a GLDF structure.
///
/// Parses the photometric file and creates a minimal but valid GLDF structure
/// containing the photometry data, light source, and emitter definitions.
///
/// For IES files, this now extracts LM-63-2019 specific metadata including:
/// - File generation type (accredited lab, simulation, etc.)
/// - Luminous opening shape and dimensions
/// - Test lab and issue date
/// - Photometric data for DescriptivePhotometry
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
/// // - Photometries referencing the file with DescriptivePhotometry
/// // - LightSources with power/flux data
/// // - Emitters linking everything together
/// ```
#[cfg(feature = "eulumdat")]
pub fn ldt_to_gldf(data: &[u8], filename: &str) -> Result<FileBufGldf, String> {
    // Parse the photometric data
    let content =
        std::str::from_utf8(data).map_err(|e| format!("Invalid UTF-8 encoding: {:?}", e))?;

    let is_ies = filename.to_lowercase().ends_with(".ies")
        || content.trim_start().starts_with("IES")
        || content.trim_start().starts_with("IESNA");

    let (ldt, ies_data) = if is_ies {
        // Parse as IES and get both Eulumdat conversion and raw IES data
        let ies = IesParser::parse_to_ies_data(content)
            .map_err(|e| format!("IES parse error: {:?}", e))?;
        let ldt = IesParser::parse(content).map_err(|e| format!("IES parse error: {:?}", e))?;
        (ldt, Some(ies))
    } else {
        let ldt = Eulumdat::parse(content).map_err(|e| format!("LDT parse error: {:?}", e))?;
        (ldt, None)
    };

    // Extract metadata with IES-specific fields if available
    let meta = if let Some(ies) = &ies_data {
        LdtMetadata::from_ies_data(ies, &ldt)
    } else {
        LdtMetadata::from_eulumdat(&ldt)
    };

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
pub fn ldt_metadata_to_gldf(
    meta: &LdtMetadata,
    data: &[u8],
    filename: &str,
) -> Result<FileBufGldf, String> {
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

    // Build DescriptivePhotometry from calculated photometric data
    #[cfg(feature = "eulumdat")]
    let descriptive_photometry = meta.photometric_data.as_ref().map(|pd| DescriptivePhotometry {
        luminaire_luminance: if pd.luminaire_luminance > 0.0 {
            Some(pd.luminaire_luminance.round() as i32)
        } else {
            None
        },
        light_output_ratio: if pd.light_output_ratio > 0.0 {
            Some(pd.light_output_ratio)
        } else {
            None
        },
        luminous_efficacy: if pd.luminous_efficacy > 0.0 {
            Some(pd.luminous_efficacy)
        } else {
            None
        },
        downward_flux_fraction: if pd.downward_flux_fraction > 0.0 {
            Some(pd.downward_flux_fraction)
        } else {
            None
        },
        downward_light_output_ratio: if pd.downward_light_output_ratio > 0.0 {
            Some(pd.downward_light_output_ratio)
        } else {
            None
        },
        upward_light_output_ratio: if pd.upward_light_output_ratio > 0.0 {
            Some(pd.upward_light_output_ratio)
        } else {
            None
        },
        tenth_peak_divergence: Some(TenthPeakDivergence {
            c0_c180: Some(pd.tenth_peak_divergence.0),
            c90_c270: Some(pd.tenth_peak_divergence.1),
        }),
        half_peak_divergence: Some(HalfPeakDivergence {
            c0_c180: Some(pd.half_peak_divergence.0),
            c90_c270: Some(pd.half_peak_divergence.1),
        }),
        photometric_code: if !pd.photometric_code.is_empty() {
            Some(pd.photometric_code.clone())
        } else {
            None
        },
        cie_flux_code: if !pd.cie_flux_code.is_empty() {
            Some(pd.cie_flux_code.clone())
        } else {
            None
        },
        cut_off_angle: if pd.cut_off_angle > 0.0 {
            Some(pd.cut_off_angle)
        } else {
            None
        },
        ugr4_h8_h705020_lq: pd.ugr_4h_8h_705020.as_ref().map(|ugr| UGR4H8H705020LQ {
            x: Some(ugr.crosswise),
            y: Some(ugr.endwise),
        }),
        iesna_light_distribution_definition: None, // Could be populated from IES keywords
        light_distribution_bug_rating: if !pd.light_distribution_bug_rating.is_empty() {
            Some(pd.light_distribution_bug_rating.clone())
        } else {
            None
        },
    });

    #[cfg(not(feature = "eulumdat"))]
    let descriptive_photometry: Option<DescriptivePhotometry> = None;

    // Photometries
    let photometries = Some(Photometries {
        photometry: vec![Photometry {
            id: "photometry".to_string(),
            photometry_file_reference: Some(PhotometryFileReference {
                file_id: file_id.to_string(),
            }),
            descriptive_photometry,
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
            sensor_emitter: vec![],
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
        xsnonamespaceschemalocation: "https://gldf.io/xsd/gldf/1.0.0-rc.3/gldf.xsd".to_string(),
        header: Header {
            manufacturer: meta
                .manufacturer
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
            creation_time_code: timestamp,
            created_with_application: "gldf-rs".to_string(),
            format_version: FormatVersion::default(),
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

    #[test]
    fn test_ldt_to_gldf_with_photometric_data() {
        // Sample LDT content
        let ldt_content = r#"Test Manufacturer
1
1
1
0
3
5
0
Test Luminaire
LUM-001
test.ldt
2024-01-01
100
50
30
80
40
0
0
0
0
100
85
1.0
0
1
1
LED
1000
3000K
80
10
0.5
0.55
0.6
0.65
0.7
0.75
0.8
0.82
0.85
0.88
0
0
45
90
100
80
50
"#;

        let result = ldt_to_gldf(ldt_content.as_bytes(), "test.ldt");
        assert!(result.is_ok());

        let gldf = result.unwrap();

        // Check that manufacturer is set
        assert_eq!(gldf.gldf.header.manufacturer, "Test Manufacturer");

        // Check that photometry has descriptive_photometry populated
        let photometries = gldf.gldf.general_definitions.photometries.unwrap();
        assert!(!photometries.photometry.is_empty());

        let photometry = &photometries.photometry[0];
        assert!(photometry.descriptive_photometry.is_some());

        let dp = photometry.descriptive_photometry.as_ref().unwrap();
        // Check that CIE flux code is populated
        assert!(dp.cie_flux_code.is_some());
        // Check that light output ratio is populated
        assert!(dp.light_output_ratio.is_some());
        assert_eq!(dp.light_output_ratio.unwrap(), 85.0);
        // Check beam angles are populated
        assert!(dp.half_peak_divergence.is_some());
        assert!(dp.tenth_peak_divergence.is_some());
    }

    #[test]
    fn test_ies_to_gldf_with_metadata() {
        // Sample IES content with LM-63-2019 format
        let ies_content = r#"IES:LM-63-2019
[TEST] TEST-001
[TESTLAB] Acme Testing Labs
[ISSUEDATE] 2024-01-15
[MANUFAC] Light Corp
[LUMCAT] LC-100
[LUMINAIRE] LED Downlight
[LAMP] LED Module 3000K
TILT=NONE
1 1000.0 1.0 3 1 1 2 0.1 0.1 0.0
1.0 1.10000 10.0
0 45 90
0
100.0
80.0
50.0
"#;

        let result = ldt_to_gldf(ies_content.as_bytes(), "test.ies");
        assert!(result.is_ok());

        let gldf = result.unwrap();

        // Check manufacturer from IES [MANUFAC]
        assert_eq!(gldf.gldf.header.manufacturer, "Light Corp");

        // Check metadata has IES-specific data
        let photometries = gldf.gldf.general_definitions.photometries.unwrap();
        let photometry = &photometries.photometry[0];
        assert!(photometry.descriptive_photometry.is_some());
    }

    #[test]
    fn test_ies_metadata_from_ies_data() {
        let ies_content = r#"IES:LM-63-2019
[TEST] RPT-12345
[TESTLAB] IES Test Lab
[ISSUEDATE] 2024-06-01
[MANUFAC] Test Manufacturer
[LUMCAT] CAT-001
TILT=NONE
1 1000.0 1.0 3 1 1 2 0.15 0.15 0.0
1.0 1.10000 15.0
0 45 90
0
200.0
150.0
100.0
"#;

        // Parse and extract IES data
        let ies_data = IesParser::parse_to_ies_data(ies_content).unwrap();
        let ldt = IesParser::parse(ies_content).unwrap();

        // Create metadata using IES-specific method
        let meta = LdtMetadata::from_ies_data(&ies_data, &ldt);

        // Verify IES metadata is populated
        assert!(meta.ies_metadata.is_some());
        let ies_meta = meta.ies_metadata.as_ref().unwrap();

        assert_eq!(ies_meta.test_report, "RPT-12345");
        assert_eq!(ies_meta.test_lab, "IES Test Lab");
        assert_eq!(ies_meta.issue_date, "2024-06-01");
        assert!(ies_meta.is_accredited); // 1.10000 = accredited lab
        assert!(!ies_meta.is_simulation);
        assert!(!ies_meta.is_scaled);
        assert!(!ies_meta.is_interpolated);

        // Verify luminous shape (rectangular from positive dimensions)
        assert!(ies_meta.is_rectangular);
        assert!(!ies_meta.is_circular);

        // Verify photometric data is also populated
        assert!(meta.photometric_data.is_some());
    }
}
