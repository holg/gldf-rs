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
    gldf::{
        Emitter, Emitters, File, Files, FixedLightEmitter, FixedLightSource, FormatVersion,
        GeneralDefinitions, GldfProduct, Header, LightSourceReference, LightSources, Locale,
        LocaleFoo, Photometries, Photometry, PhotometryFileReference, PhotometryReference,
        ProductDefinitions, ProductMetaData, Variant, Variants,
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
                let flux = ldt.luminous_flux();
                if flux > 0.0 {
                    Some(flux)
                } else {
                    None
                }
            },
            watts: {
                let watts = ldt.wattage();
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
    let meta = ldt_metadata(data)?;
    ldt_metadata_to_gldf(&meta, data, filename)
}

/// Parse LDT/IES bytes and extract their [`LdtMetadata`] without building
/// a GLDF. Sniffs IESNA vs EULUMDAT the same way [`ldt_to_gldf`] does.
/// Useful when a caller (e.g. the batch converter's template placeholder
/// substitution) needs the luminaire name / manufacturer / power before
/// deciding how to assemble the product.
#[cfg(feature = "eulumdat")]
pub fn ldt_metadata(data: &[u8]) -> Result<LdtMetadata, String> {
    let content =
        std::str::from_utf8(data).map_err(|e| format!("Invalid UTF-8 encoding: {:?}", e))?;

    let ldt = if content.trim_start().starts_with("IESNA") {
        IesParser::parse(content).map_err(|e| format!("IES parse error: {:?}", e))?
    } else {
        Eulumdat::parse(content).map_err(|e| format!("LDT parse error: {:?}", e))?
    };

    Ok(LdtMetadata::from_eulumdat(&ldt))
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
        multi_channel_light_source: vec![],
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
            multi_channel_light_emitter: vec![],
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

/// Convert an IES TM-33-23 / ATLA-S001 XML file into a fresh GLDF bundle.
///
/// **Single embedded file, dual GLDF reference.** TM-33 is IES's own
/// XML photometric+spectral format (descended from the IESNA family),
/// so the GLDF stores the TM-33 XML bytes ONCE as a `<File>` with
/// `contentType="ldc/iesxml"` and references that same `@fileId` from
/// BOTH a `<Photometry>` AND a `<Spectrum>` entry. This keeps the
/// source format intact — no LDT/SPD down-conversion that would drop
/// TM-33-only fields (UUID, ColorRendering.Rf/Rg, AngularSpectralData,
/// regulatory flags).
///
/// The GLDF XSD's `contentType` enum doesn't have a dedicated
/// `tm33`-flavoured slug, but: (a) the format is XML, (b) IES-XML is
/// the closest legal slot, (c) the XSD's keyref structure
/// (`Photometry-File` and `Spectrum-File` both target the same
/// `FileId` key set) explicitly permits one `<File>` to be referenced
/// from both Photometry AND Spectrum simultaneously. Parsers detect
/// TM-33 vs IES-XML by content-sniffing the bytes.
///
/// TM-33 header → GLDF mapping:
/// * `<Header><Manufacturer>` → GLDF Header.manufacturer +
///   FixedLightSource.manufacturer
/// * `<Header><Description>` → ProductMetaData.name + Variant.name +
///   FixedLightSource.name
/// * `<Header><CatalogNumber>` → FixedLightSource.description
/// * `<Header><ReportNumber>` → Variant.description
/// * `<Header><UUID>` → Header.created_with_application annotation
/// * `<Luminaire><Dimensions>` → (not yet plumbed — would land in a
///   future GeometryDimensions slot)
/// * `<Emitter><InputWattage|RatedLumens|FixedCCT|Ra>` → FixedLightSource
///   (power/CCT/CRI) + FixedLightEmitter.rated_luminous_flux
///
/// Requires the `eulumdat` feature.
#[cfg(feature = "eulumdat")]
pub fn tm33_to_gldf(data: &[u8], filename: &str) -> Result<FileBufGldf, String> {
    use crate::gldf::{
        ColorInformation, Spectrum, SpectrumFileReference, SpectrumReference, Spectrums,
    };
    use eulumdat::atla::xml as atla_xml;

    let content = std::str::from_utf8(data).map_err(|e| format!("TM-33: not UTF-8 ({})", e))?;
    let doc = atla_xml::parse(content).map_err(|e| format!("TM-33 parse: {:?}", e))?;
    let emitter_opt = doc.emitters.first();

    // Use a stable filename based on the source so the Files page
    // shows something recognisable. Pass the original stem through.
    let stem = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("luminaire");
    let tm33_filename = if filename.to_lowercase().ends_with(".tm33.xml") {
        filename.to_string()
    } else if filename.to_lowercase().ends_with(".xml") {
        // Force a .tm33.xml suffix so downstream tools can sniff by
        // extension alone — the bytes are TM-33 either way.
        format!("{}.tm33.xml", stem.trim_end_matches(".xml"))
    } else {
        format!("{}.tm33.xml", stem)
    };

    let timestamp = "2024-01-01T00:00:00Z".to_string();

    // ── ONE <File> entry, shared between Photometry and Spectrum ──
    //
    // contentType="ldc/iesxml": the XSD-legal landing zone for any
    // XML-shaped photometric data. TM-33 is IESNA's XML photometric
    // format; parsers sniff the bytes to disambiguate IES-XML vs TM-33.
    let file_id = "tm33_file_1";
    let files = Files {
        file: vec![File {
            id: file_id.to_string(),
            content_type: "ldc/iesxml".to_string(),
            type_attr: "localFileName".to_string(),
            language: String::new(),
            file_name: tm33_filename.clone(),
        }],
    };

    // ── <Photometry> referencing the TM-33 file ──
    let photometry_id = "photometry_1";
    let photometries = Some(Photometries {
        photometry: vec![Photometry {
            id: photometry_id.to_string(),
            photometry_file_reference: Some(PhotometryFileReference {
                file_id: file_id.to_string(),
            }),
            descriptive_photometry: None,
        }],
    });

    // ── <Spectrum> referencing the SAME TM-33 file ──
    //
    // The XSD allows this (Photometry-File and Spectrum-File keyrefs
    // both target the FileId key set; nothing forbids overlap). When
    // the TM-33 doesn't carry an SPD, skip the Spectrum entry so we
    // don't register a spurious empty spectrum.
    let has_spd = emitter_opt
        .and_then(|e| e.spectral_distribution.as_ref())
        .map(|spd| !spd.wavelengths.is_empty())
        .unwrap_or(false);
    let spectrum_id = "spectrum_1";
    let spectrums = if has_spd {
        Some(Spectrums {
            spectrum: vec![Spectrum {
                id: spectrum_id.to_string(),
                spectrum_file_reference: Some(SpectrumFileReference {
                    file_id: file_id.to_string(),
                }),
                intensity: vec![],
            }],
        })
    } else {
        None
    };

    // ── FixedLightSource with metadata from TM-33 Header + Emitter ──
    let ls_name = doc
        .header
        .description
        .clone()
        .or_else(|| doc.header.catalog_number.clone())
        .unwrap_or_else(|| "TM-33 LightSource".to_string());
    let ls_description = doc.header.catalog_number.clone();
    let rated_input_power = emitter_opt.and_then(|e| e.input_watts);
    let cct = emitter_opt.and_then(|e| e.cct).map(|c| c.round() as i32);
    let cri = emitter_opt
        .and_then(|e| e.color_rendering.as_ref())
        .and_then(|cr| cr.ra)
        .map(|r| r.round() as i32);
    let color_information = if cct.is_some() || cri.is_some() {
        Some(ColorInformation {
            correlated_color_temperature: cct,
            color_rendering_index: cri,
            ..Default::default()
        })
    } else {
        None
    };

    let light_source_id = "lightsource_1";
    let light_sources = Some(LightSources {
        changeable_light_source: vec![],
        multi_channel_light_source: vec![],
        fixed_light_source: vec![FixedLightSource {
            id: light_source_id.to_string(),
            name: LocaleFoo {
                locale: vec![Locale {
                    language: "en".to_string(),
                    value: ls_name.clone(),
                }],
            },
            description: ls_description.map(|d| LocaleFoo {
                locale: vec![Locale {
                    language: "en".to_string(),
                    value: d,
                }],
            }),
            manufacturer: doc.header.manufacturer.clone(),
            rated_input_power,
            color_information,
            spectrum_reference: if has_spd {
                Some(SpectrumReference {
                    spectrum_id: spectrum_id.to_string(),
                })
            } else {
                None
            },
            ..Default::default()
        }],
    });

    // ── Emitter binding LS↔Photometry, with TM-33 RatedLumens ──
    let emitter_id = "emitter_1";
    let rated_lumens = emitter_opt
        .and_then(|e| e.rated_lumens)
        .map(|l| l.round() as i32);
    let emitters = Some(Emitters {
        emitter: vec![Emitter {
            id: emitter_id.to_string(),
            changeable_light_emitter: vec![],
            multi_channel_light_emitter: vec![],
            sensor_emitter: vec![],
            fixed_light_emitter: vec![FixedLightEmitter {
                name: Some(LocaleFoo {
                    locale: vec![Locale {
                        language: "en".to_string(),
                        value: "Main Emitter".to_string(),
                    }],
                }),
                photometry_reference: PhotometryReference {
                    photometry_id: photometry_id.to_string(),
                },
                light_source_reference: LightSourceReference {
                    fixed_light_source_id: Some(light_source_id.to_string()),
                    changeable_light_source_id: None,
                    light_source_count: None,
                },
                rated_luminous_flux: rated_lumens,
                ..Default::default()
            }],
        }],
    });

    // ── Variant ──
    let variant_name = doc
        .header
        .description
        .clone()
        .unwrap_or_else(|| ls_name.clone());
    let variant_description = doc.header.report_number.clone().map(|rn| LocaleFoo {
        locale: vec![Locale {
            language: "en".to_string(),
            value: format!("Report: {}", rn),
        }],
    });
    let variant = Variant {
        id: "variant_1".to_string(),
        name: Some(LocaleFoo {
            locale: vec![Locale {
                language: "en".to_string(),
                value: variant_name.clone(),
            }],
        }),
        description: variant_description,
        ..Default::default()
    };

    // ── Header — Manufacturer + optional TM-33 UUID hint ──
    let created_with = if let Some(uuid) = doc.header.uuid.as_ref() {
        format!("gldf-rs (TM-33 UUID {})", uuid)
    } else {
        "gldf-rs".to_string()
    };
    let manufacturer = doc
        .header
        .manufacturer
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());

    let gldf_product = GldfProduct {
        path: String::new(),
        xmlns_xsi: "http://www.w3.org/2001/XMLSchema-instance".to_string(),
        xsnonamespaceschemalocation: "https://gldf.io/xsd/gldf/1.0.0-rc.3/gldf.xsd".to_string(),
        header: Header {
            manufacturer,
            creation_time_code: timestamp,
            created_with_application: created_with,
            format_version: FormatVersion::default(),
            ..Default::default()
        },
        general_definitions: GeneralDefinitions {
            files,
            photometries,
            spectrums,
            light_sources,
            emitters,
            ..Default::default()
        },
        product_definitions: ProductDefinitions {
            product_meta_data: Some(ProductMetaData {
                name: Some(LocaleFoo {
                    locale: vec![Locale {
                        language: "en".to_string(),
                        value: variant_name,
                    }],
                }),
                description: doc.header.description.as_ref().map(|d| LocaleFoo {
                    locale: vec![Locale {
                        language: "en".to_string(),
                        value: d.clone(),
                    }],
                }),
                ..Default::default()
            }),
            variants: Some(Variants {
                variant: vec![variant],
            }),
        },
    };

    // The TM-33 bytes land under `ldc/` so the zip layout matches the
    // `ldc/iesxml` content_type. ONE BufFile — no SPD duplicate.
    let zip_path = format!("ldc/{}", tm33_filename);
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

    /// tm33_to_gldf must keep the TM-33 XML as a SINGLE embedded
    /// `<File contentType="ldc/iesxml">` and reference it from BOTH a
    /// `<Photometry>` AND a `<Spectrum>`. The XSD's keyref structure
    /// allows this — splitting into separate .ldt/.spd files was lossy
    /// and dropped TM-33-only fields (UUID, ColorRendering.Rf/Rg, …).
    #[test]
    fn tm33_to_gldf_single_file_dual_reference() {
        let path = "../../tests/data/golden_lightsource_2.tm33.xml";
        let data = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("skipping: {path} missing ({e})");
                return;
            }
        };
        let bundle = tm33_to_gldf(&data, "golden_lightsource_2.tm33.xml").expect("tm33_to_gldf");
        let g = &bundle.gldf;

        // Header manufacturer surfaced.
        assert!(
            !g.header.manufacturer.is_empty() && g.header.manufacturer != "Unknown",
            "header manufacturer not surfaced: {:?}",
            g.header.manufacturer
        );

        // EXACTLY one <File> entry — the TM-33 XML itself, no
        // synthesised .ldt/.spd duplicates.
        let files = &g.general_definitions.files.file;
        assert_eq!(
            files.len(),
            1,
            "want 1 File (the TM-33 itself), got {}: {:?}",
            files.len(),
            files
        );
        let file = &files[0];
        assert_eq!(
            file.content_type, "ldc/iesxml",
            "TM-33 file should be tagged ldc/iesxml"
        );
        assert!(
            file.file_name.to_lowercase().ends_with(".tm33.xml")
                || file.file_name.to_lowercase().ends_with(".xml"),
            "filename should preserve .tm33.xml / .xml: {}",
            file.file_name
        );

        // Photometry + Spectrum entries — BOTH point at the SAME file id.
        let photometry = &g
            .general_definitions
            .photometries
            .as_ref()
            .expect("photometries")
            .photometry[0];
        let spectrum = &g
            .general_definitions
            .spectrums
            .as_ref()
            .expect("spectrums")
            .spectrum[0];
        let ph_fid = &photometry
            .photometry_file_reference
            .as_ref()
            .expect("Photometry.photometry_file_reference")
            .file_id;
        let sp_fid = &spectrum
            .spectrum_file_reference
            .as_ref()
            .expect("Spectrum.spectrum_file_reference")
            .file_id;
        assert_eq!(
            ph_fid, &file.id,
            "Photometry@fileId mismatch (want {}, got {})",
            file.id, ph_fid
        );
        assert_eq!(
            sp_fid, &file.id,
            "Spectrum@fileId mismatch (want {}, got {})",
            file.id, sp_fid
        );

        // FixedLightSource → Spectrum binding pre-wired.
        let fls = &g
            .general_definitions
            .light_sources
            .as_ref()
            .expect("light_sources")
            .fixed_light_source[0];
        assert_eq!(
            fls.spectrum_reference
                .as_ref()
                .map(|r| r.spectrum_id.as_str()),
            Some(spectrum.id.as_str()),
            "FixedLightSource.SpectrumReference not wired to the Spectrum"
        );

        // Emitter binds the LS and the Photometry.
        let em = &g
            .general_definitions
            .emitters
            .as_ref()
            .expect("emitters")
            .emitter[0];
        let fle = &em.fixed_light_emitter[0];
        assert_eq!(
            fle.light_source_reference.fixed_light_source_id.as_deref(),
            Some(fls.id.as_str()),
            "LS↔emitter binding broken"
        );
        assert_eq!(
            fle.photometry_reference.photometry_id, photometry.id,
            "Photometry↔emitter binding broken"
        );

        // EXACTLY one embedded byte buffer — no duplicate of the
        // TM-33 bytes across .ldt + .spd entries.
        assert_eq!(
            bundle.files.len(),
            1,
            "expected 1 embedded BufFile (the TM-33), got {}",
            bundle.files.len()
        );
        assert_eq!(
            bundle.files[0].content.as_deref(),
            Some(data.as_slice()),
            "embedded bytes don't match the source TM-33 verbatim — \
             lossy round-trip"
        );
    }
}
