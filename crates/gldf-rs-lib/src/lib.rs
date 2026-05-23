//! # gldf_rs
//! GLDF (Global Lighting Data Format) Library
//!
//! **GitHub:** <https://github.com/holg/gldf-rs>
//!
//! The GLDF crate provides a set of structures and tools for working with the Global Lighting Data Format (GLDF),
//! a standardized format for describing lighting products, their characteristics, and technical details.
//!
//! GLDF is used in the lighting industry to exchange product information between manufacturers, designers,
//! and other stakeholders, ensuring consistent representation and interoperability across various software tools.
//!
//! This crate offers utilities for serializing and deserializing GLDF data, enabling you to read and write GLDF files
//! while adhering to the ISO 7127 standard. It also provides helper macros for working with GLDF-specific attributes.
//!
//! For more information about GLDF and its specifications, <https::gldf.io> and refer to the ISO 7127 standard.
//!
//! # Features
//!
//! - Serialize and deserialize GLDF files in compliance with ISO 7127 standard.
//! - From XML into JSON and vice versa.
//! - Define GLDF-specific attributes using custom procedural macros.
//! - Easily work with GLDF data structures and their components.
//!
//! For more usage examples and detailed documentation, please refer to the documentation of individual modules and structs.
//! Most functions are implemented as methods on the struct GldfProduct, which shall represent the Root of the XML structure.
//! **For more information see : gldf_rs::gldf::GldfProduct**
//!
//! [`GldfProduct`]
//! # Example
//! ```rust,no_run
//! use gldf_rs::gldf::GldfProduct;
//! let loaded: GldfProduct = GldfProduct::load_gldf("tests/data/test.gldf").unwrap();
//! println!("{:?}", loaded);
//! // Display pretty printed XML
//! let x_serialized = loaded.to_xml().unwrap();
//! println!("{}", x_serialized);
//! let json_str = loaded.to_json().unwrap();
//! println!("{}", json_str);
//! let j_loaded: GldfProduct = GldfProduct::from_json(&json_str).unwrap();
//! let x_reserialized =  j_loaded.to_xml().unwrap();
//! println!("{}", x_reserialized);
//! assert_eq!(x_serialized, x_reserialized);
//! ```
//!
//!
//! For more information about GLDF and its specifications, refer to the ISO 7127 standard.
//!
//! # License
//!
//! This project is licensed under the terms of the MIT license.

/// the gldf module (src/gldf/mod.rs)
pub mod gldf;
pub use gldf::*;

/// Editable GLDF wrapper for editing and saving GLDF files
pub mod editable;
pub use editable::{EditableGldf, EditableGldfStats};

/// Fix-up utilities for legacy / non-conforming GLDF archives.
pub mod fix;
pub use fix::fix_legacy_content_types;

/// Validation engine for GLDF files
pub mod validation;
pub use validation::{ValidationError, ValidationLevel, ValidationResult};

/// CRUD operations for GldfProduct
pub mod operations;

/// L3D to LDT mapping utilities
pub mod mapping;
pub use mapping::{
    get_first_l3d_with_ldt, get_l3d_files_with_ldt, get_l3d_ldt_mappings, get_variant_emitter_data,
    EmitterRenderData, L3dLdtMapping, L3dWithLdt, MountingInfo, MountingType, VariantEmitterData,
};

/// Conversion utilities for creating GLDF from other formats (LDT/IES)
#[cfg(feature = "eulumdat")]
pub mod convert;
#[cfg(feature = "eulumdat")]
pub use convert::{ldt_metadata_to_gldf, ldt_to_gldf, LdtMetadata};

/// Photometric file export and variant-aware LDT patching
#[cfg(feature = "eulumdat")]
pub mod photometry;
#[cfg(feature = "eulumdat")]
pub use photometry::{
    export_photometry, patch_ldt_for_variant, resolve_variant_photometries,
    PhotometryExportFormat, VariantPhotometryResolution,
};

/// IFC (Industry Foundation Classes) integration for BIM interoperability
pub mod ifc;
pub use ifc::{
    // IFC Import
    ifc_to_gldf,
    import_ifc,
    GldfGenerator,
    GldfToIfc,
    IfcImporter,
    ImportedLightSource,
    ImportedLuminaire,
    ImportedProperties,
    ImportedVariant,
    LightEmissionSourceEnum,
    LightFixtureTypeEnum,
    StepWriter,
};

/// Plugin system for embedded WASM viewer plugins
pub mod plugin;
pub use plugin::{Plugin, PluginManager, PluginManifest};

#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use regex::Regex;
use serde_json::from_str as serde_from_str;
use std::fs::File as StdFile;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use zip::ZipArchive;

/// Target GLDF schema revision when serializing a `GldfProduct`.
///
/// rc.4 introduced backwards-compatible spelling fixes (the corrected
/// `RatedChromaticityCoordinateValues` element name and `Fluorescent`
/// enum values), keeping the old forms as deprecated aliases. The
/// internal model always stores the rc.4-correct form; this enum picks
/// which spelling lands in the output XML so producers can keep shipping
/// rc.3-validating files until consumers catch up.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GldfSchemaVersion {
    /// Output the rc.3 spellings (`RatedChromacityCoordinateValues`,
    /// `Flourescent Triphosphor`, `Flourescent Halophosphate`). Use this
    /// for compatibility with consumers pinned to the pre-rc.4 schema.
    Rc3,
    /// Output the rc.4-corrected spellings. This is the default and
    /// validates against both rc.4 and rc.3 (rc.4 keeps the old forms
    /// as deprecated aliases, but the corrected forms are also accepted
    /// by older consumers since the deprecation runs the other way).
    #[default]
    Rc4,
}

/// Rewrite rc.4-canonical spellings back to their rc.3 typo'd forms.
///
/// The rc.4 schema kept both spellings valid via `<xs:choice>` wrappers
/// and deprecated-form enum aliases, so this rewrite is needed only for
/// consumers that haven't bumped to rc.4 yet. Pure string replacement —
/// the substrings we replace don't appear anywhere except as element
/// names (`<RatedChromaticityCoordinateValues>` open/close tags) and as
/// the literal enum values inside `<Cie97LampType>...</Cie97LampType>`
/// element bodies, so global replace is safe.
fn downconvert_rc4_to_rc3(xml: &str) -> String {
    xml
        // Element name (rc.4 only added this; rc.3 only knew the typo'd form)
        .replace(
            "RatedChromaticityCoordinateValues",
            "RatedChromacityCoordinateValues",
        )
        // Cie97LampType enum values
        .replace("Fluorescent Triphosphor", "Flourescent Triphosphor")
        .replace("Fluorescent Halophosphate", "Flourescent Halophosphate")
}

impl GldfProduct {
    pub fn detach(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufFile {
    pub name: Option<String>,
    pub content: Option<Vec<u8>>,
    pub file_id: Option<String>,
    pub path: Option<String>,
}

pub struct FileBufGldf {
    pub files: Vec<BufFile>,
    pub gldf: GldfProduct,
}

/// Implementations for the per se informational GldfProduct struct
impl GldfProduct {
    /// Loads a GLDF file from a given path as String and return the XML String of the product.xml file
    pub fn load_gldf_file_str(&self, path: String) -> anyhow::Result<String> {
        let zipfile = StdFile::open(Path::new(&self.path))?;
        let mut zip = ZipArchive::new(zipfile)?;
        let mut some_str = String::new();
        let mut some_file = zip.by_name(&path)?;
        some_file.read_to_string(&mut some_str)?;
        Ok(some_str)
    }

    /// Loads a GLDF file from a given path as `Vec<u8>` and return the `Vec<u8>` of the product.xml file
    /// last but not least for WASM usage.
    pub fn load_gldf_file(&self, path: String) -> anyhow::Result<Vec<u8>> {
        let zipfile = StdFile::open(Path::new(&self.path))?;
        let mut zip = ZipArchive::new(zipfile)?;
        let mut file_buf = Vec::new();
        let mut some_file = zip.by_name(&path)?;
        some_file.read_to_end(&mut file_buf)?;
        Ok(file_buf)
    }

    /// a helper function to used by the load_gldf function
    /// takes a PathBuf and returns the XML String of the product.xml file
    pub fn get_xml_str_from_gldf(path: PathBuf) -> anyhow::Result<String> {
        let zipfile = StdFile::open(path)?;
        let mut zip = ZipArchive::new(zipfile)?;
        let mut xmlfile = zip.by_name("product.xml")?;
        let mut xml_str = String::new();
        xmlfile.read_to_string(&mut xml_str)?;
        Ok(xml_str)
    }

    /// a helper function to remove the UTF8 Bom, if present from a given String
    /// takes a String and returns a String
    /// needed for some GLDF files, which have BOM in the XML file
    pub fn remove_bom(s: &str) -> String {
        if let Some(stripped) = s.strip_prefix("\u{FEFF}") {
            stripped.to_string()
        } else {
            s.to_string()
        }
    }

    /// a helper function to sanitize the XML String
    /// takes a String and returns a String
    /// GldfProduct does not really care about the XSD version, so we remove it
    /// and add our own later
    pub fn sanitize_xml_str(xml_str: &str) -> String {
        let cleaned_str = Self::remove_bom(xml_str);
        let re = Regex::new(r"<Root .*?>").unwrap();
        // well we are lazy for now and simple replace the root element with a generic one
        let cleaned = re.replace_all(&cleaned_str, "<Root>").to_string();
        // Upconvert rc.3 typo'd forms to the rc.4 corrected spellings
        // before the serde-XML pass sees them. The element name is
        // already covered by `serde(alias = ...)` on the field, so this
        // step only matters for the enum string values inside
        // `<Cie97LampType>...</Cie97LampType>` element bodies — those
        // arrive as plain `String`, no alias machinery applies.
        cleaned
            .replace("Flourescent Triphosphor", "Fluorescent Triphosphor")
            .replace("Flourescent Halophosphate", "Fluorescent Halophosphate")
    }

    /// a helper function to load a XML String and return the GldfProduct struct
    pub fn from_xml(xml_str: &str) -> anyhow::Result<GldfProduct> {
        let my_xml_str = Self::sanitize_xml_str(xml_str);
        let result: GldfProduct = quick_xml::de::from_str(&my_xml_str)
            .map_err(|e| anyhow::anyhow!("XML parsing error: {}", e))?;
        Ok(result)
    }

    /// Argument the &str path to the GLDF file and return the GldfProduct struct
    pub fn load_gldf(path: &str) -> anyhow::Result<GldfProduct> {
        let path_buf = Path::new(path).to_path_buf();
        let xml_str = GldfProduct::get_xml_str_from_gldf(path_buf)
            .map_err(anyhow::Error::msg)
            .context("Failed to parse XML string")?;
        let mut loaded: GldfProduct = GldfProduct::from_xml(&xml_str)?;
        loaded.path = path.to_string();
        Ok(loaded)
    }

    /// A helper for the WASM, which has the GLDF file as `Vec<u8>` and returns all the files as `Vec<BufFile>`
    /// which can be later rendered into HTML, e.g. for some GLDF Viewer
    pub fn load_gldf_from_buf_all(gldf_buf: Vec<u8>) -> anyhow::Result<FileBufGldf> {
        let zip_buf = std::io::Cursor::new(gldf_buf);
        let mut zip = ZipArchive::new(zip_buf).context("Failed to open GLDF as ZIP archive")?;
        let mut file_bufs: Vec<BufFile> = Vec::new();
        let mut xmlfile = zip
            .by_name("product.xml")
            .context("product.xml not found in GLDF archive")?;
        let mut xml_str = String::new();
        xmlfile
            .read_to_string(&mut xml_str)
            .context("Failed to read product.xml")?;
        let loaded: GldfProduct =
            GldfProduct::from_xml(&xml_str).context("Failed to parse product.xml")?;
        drop(xmlfile);

        for i in 0..zip.len() {
            if let Ok(mut file) = zip.by_index(i) {
                if file.is_file() {
                    let mut buf: Vec<u8> = Vec::new();
                    if file.read_to_end(&mut buf).is_ok() {
                        let buf_file = BufFile {
                            name: Some(file.name().to_string()),
                            content: Some(buf),
                            file_id: None,
                            path: Some(file.name().to_string()),
                        };
                        file_bufs.push(buf_file);
                    }
                }
            }
        }
        let file_buf = FileBufGldf {
            files: file_bufs,
            gldf: loaded,
        };

        Ok(file_buf)
    }

    /// A helper function. Argument is the `Vec<u8>` of the GLDF file and returns the GldfProduct struct
    /// WASM usage e.g.
    pub fn load_gldf_from_buf(file_buf: Vec<u8>) -> anyhow::Result<GldfProduct> {
        let zip_buf = std::io::Cursor::new(file_buf);
        let mut zip = ZipArchive::new(zip_buf).context("Failed to open GLDF as ZIP archive")?;
        let mut xmlfile = zip
            .by_name("product.xml")
            .context("product.xml not found in GLDF archive")?;
        let mut xml_str = String::new();
        xmlfile
            .read_to_string(&mut xml_str)
            .context("Failed to read product.xml")?;
        let loaded: GldfProduct =
            GldfProduct::from_xml(&xml_str).context("Failed to parse product.xml")?;
        Ok(loaded)
    }

    /// represent the GldfProduct as JSON String
    pub fn to_json(&self) -> anyhow::Result<String> {
        let json_str = serde_json::to_string(&self)?;
        Ok(json_str)
    }

    /// represent the GldfProduct as pretty pretty JSON String
    pub fn to_pretty_json(&self) -> anyhow::Result<String> {
        let json_str =
            serde_json::to_string_pretty(&self).context("Failed to serialize to JSON")?;
        Ok(json_str)
    }

    /// loads a given JSON &str and returns the GldfProduct struct
    pub fn from_json(json_str: &str) -> anyhow::Result<GldfProduct> {
        let j_loaded: GldfProduct = serde_from_str(json_str)?;
        Ok(j_loaded)
    }

    /// loads a given JSON file from a PathBuf and returns the GldfProduct struct
    /// last but not least for WASM usage.
    pub fn from_json_file(path: PathBuf) -> anyhow::Result<GldfProduct> {
        let mut json_file = StdFile::open(&path)
            .with_context(|| format!("Failed to open JSON file: {:?}", path))?;
        let mut json_str = String::new();
        json_file
            .read_to_string(&mut json_str)
            .context("Failed to read JSON file")?;
        GldfProduct::from_json(&json_str).context("Failed to parse JSON content")
    }

    /// Represent the GldfProduct as an XML string targeting the rc.4
    /// schema (the default). Internal model always stores the rc.4
    /// canonical form (`RatedChromaticityCoordinateValues`,
    /// `Fluorescent Triphosphor`, etc.), so the rc.4 path is a straight
    /// serialize. For rc.3 output use [`Self::to_xml_with_schema`].
    pub fn to_xml(&self) -> anyhow::Result<String> {
        self.to_xml_with_schema(GldfSchemaVersion::Rc4)
    }

    /// Serialize as XML targeting a specific GLDF schema revision.
    ///
    /// **rc.4** (default) writes the corrected element names and enum
    /// values (`RatedChromaticityCoordinateValues`,
    /// `Fluorescent Triphosphor`, `Fluorescent Halophosphate`).
    ///
    /// **rc.3** rewrites those back to the historical typo'd forms
    /// (`RatedChromacityCoordinateValues`, `Flourescent Triphosphor`,
    /// `Flourescent Halophosphate`) so files validate against the
    /// pre-rc.4 XSD that some downstream tools still use. The conversion
    /// is a pure string-replace pass on the serialized XML — no
    /// duplicate types or feature flags involved.
    pub fn to_xml_with_schema(&self, version: GldfSchemaVersion) -> anyhow::Result<String> {
        let xml_str = quick_xml::se::to_string(&self)?;
        let xml_str = match version {
            GldfSchemaVersion::Rc4 => xml_str,
            GldfSchemaVersion::Rc3 => downconvert_rc4_to_rc3(&xml_str),
        };
        Ok(format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}",
            xml_str
        ))
    }

    /// returns the photometric files as Vec<&File>
    pub fn get_phot_files(&self) -> anyhow::Result<Vec<&File>> {
        let mut result: Vec<&File> = Vec::new();
        for f in self.general_definitions.files.file.iter() {
            let content_type = &f.content_type;
            if content_type.starts_with("ldc") {
                result.push(f)
            }
        }
        Ok(result.to_owned())
    }

    /// returns the image files as Vec<&File>
    pub fn get_image_def_files(&self) -> Result<Vec<&File>> {
        let mut result: Vec<&File> = Vec::new();
        for f in self.general_definitions.files.file.iter() {
            let content_type = &f.content_type;
            if content_type.starts_with("image") {
                result.push(f)
            }
        }
        Ok(result.to_owned())
    }

    /// returns the image files as Vec<&File>
    pub fn get_image_zip_files(&self) -> anyhow::Result<Vec<&File>> {
        let mut result: Vec<&File> = Vec::new();
        for f in self.general_definitions.files.file.iter() {
            let content_type = &f.content_type;
            if content_type.starts_with("image") {
                result.push(f)
            }
        }
        Ok(result.to_owned())
    }

    /// from the given file_id of the ldc file reference, return the ldc file as String
    /// it could as well be the type_attr "url", which will be fetched from the web first
    /// overridden for the WASM portage, so not used for WASM portage
    #[cfg(feature = "http")]
    pub async fn get_ldc_by_id(&self, file_id: String) -> Result<String, anyhow::Error> {
        let mut result: String = "".to_owned();
        for f in self.general_definitions.files.file.iter() {
            if f.id == file_id {
                let mut ldc_path = "ldc/".to_owned();
                let file_name = f.file_name.to_owned();
                if f.type_attr == "url" {
                    let tmp = fetch_text_from_url(&file_name).await?;
                    result.push_str(tmp.as_str());
                } else {
                    ldc_path.push_str(&file_name);
                    result.push_str(&self.load_gldf_file_str(ldc_path)?);
                }
            }
        }
        Ok(result)
    }

    /// from the given file_id of the ldc file reference, return the ldc file as String
    /// (non-HTTP version - does not support URL type files)
    #[cfg(not(feature = "http"))]
    pub fn get_ldc_by_id(&self, file_id: String) -> Result<String, anyhow::Error> {
        let mut result: String = "".to_owned();
        for f in self.general_definitions.files.file.iter() {
            if f.id == file_id {
                if f.type_attr == "url" {
                    return Err(anyhow::anyhow!(
                        "URL file type not supported without http feature"
                    ));
                }
                let mut ldc_path = "ldc/".to_owned();
                ldc_path.push_str(&f.file_name);
                result.push_str(&self.load_gldf_file_str(ldc_path)?);
            }
        }
        Ok(result)
    }

    /// Gets all the file definitions as `Vec<File>`
    pub fn get_all_file_definitions(&self) -> anyhow::Result<Vec<File>> {
        let mut result: Vec<File> = Vec::new();
        for f in self.general_definitions.files.file.iter() {
            result.push(f.to_owned());
        }
        Ok(result)
    }

    /// Gets all the file definitions which are of content_type url as `Vec<File>`
    pub fn get_url_file_definitions(&self) -> anyhow::Result<Vec<File>> {
        let mut result: Vec<File> = Vec::new();
        for f in self.general_definitions.files.file.iter() {
            if f.content_type == "url" {
                result.push(f.to_owned());
            }
        }
        Ok(result)
    }
}

/// helper function to get the content of the url as File from the given url
#[cfg(feature = "http")]
pub async fn fetch_text_from_url(url: &str) -> Result<String, reqwest::Error> {
    let response = reqwest::get(url).await?;
    let text = response.text().await?;
    Ok(text)
}

#[cfg(feature = "http")]
pub async fn fetch_content_from_url(url: &str) -> Result<Vec<u8>, reqwest::Error> {
    let response = reqwest::get(url).await?;
    let content = response.bytes().await?;
    Ok(content.to_vec())
}
