// gldf-rs-ffi/src/lib.rs
//! FFI bindings for GLDF library
//! Provides iOS/macOS/Android support via UniFFI

use serde::Deserialize;
use std::io::{Cursor, Read};
use std::sync::{Arc, RwLock};
use zip::ZipArchive;

// Setup UniFFI scaffolding
uniffi::setup_scaffolding!();

// -----------------------------------------------------------------------------
// Error Handling
// -----------------------------------------------------------------------------

#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum GldfError {
    #[error("Failed to parse GLDF: {msg}")]
    ParseError { msg: String },

    #[error("Failed to serialize: {msg}")]
    SerializeError { msg: String },

    #[error("File not found: {msg}")]
    FileNotFound { msg: String },

    #[error("Invalid data: {msg}")]
    InvalidData { msg: String },
}

// -----------------------------------------------------------------------------
// DTOs (Data Transfer Objects)
// -----------------------------------------------------------------------------

/// Header information from GLDF file
#[derive(uniffi::Record, Debug, Clone)]
pub struct GldfHeader {
    pub manufacturer: String,
    pub author: String,
    pub format_version: String,
    pub created_with_application: String,
    pub creation_time_code: String,
}

/// File definition from GLDF
#[derive(uniffi::Record, Debug, Clone)]
pub struct GldfFile {
    pub id: String,
    pub file_name: String,
    pub content_type: String,
    pub file_type: String, // "localFileName" or "url"
}

/// Light source information (simplified - covers both fixed and changeable)
#[derive(uniffi::Record, Debug, Clone)]
pub struct GldfLightSource {
    pub id: String,
    pub name: String,
    pub light_source_type: String, // "fixed" or "changeable"
}

/// Product variant information
#[derive(uniffi::Record, Debug, Clone)]
pub struct GldfVariant {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Geometry ID reference (if variant has 3D model)
    pub geometry_id: Option<String>,
    /// List of emitter references with their external names
    pub emitter_refs: Vec<GldfEmitterRef>,
}

/// Emitter reference in a variant
#[derive(uniffi::Record, Debug, Clone)]
pub struct GldfEmitterRef {
    pub emitter_id: String,
    pub external_name: Option<String>,
}

/// Emitter data for 3D rendering
#[derive(uniffi::Record, Debug, Clone)]
pub struct GldfEmitterData {
    pub emitter_id: String,
    /// Photometry file ID (for IES/LDT lookup)
    pub photometry_file_id: Option<String>,
    /// Light source type
    pub light_source_type: String,
    /// Rated luminous flux in lumens
    pub rated_luminous_flux: Option<i32>,
}

/// Statistics about loaded GLDF
#[derive(uniffi::Record, Debug, Clone)]
pub struct GldfStats {
    pub files_count: u64,
    pub fixed_light_sources_count: u64,
    pub changeable_light_sources_count: u64,
    pub variants_count: u64,
    pub photometries_count: u64,
    pub simple_geometries_count: u64,
    pub model_geometries_count: u64,
}

// -----------------------------------------------------------------------------
// Main Engine
// -----------------------------------------------------------------------------

/// Extracted file content from GLDF archive
#[derive(uniffi::Record, Debug, Clone)]
pub struct GldfFileContent {
    pub file_id: String,
    pub file_name: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

/// Electrical attributes from GLDF
#[derive(uniffi::Record, Debug, Clone, Default)]
pub struct GldfElectrical {
    pub safety_class: Option<String>,
    pub ip_code: Option<String>,
    pub power_factor: Option<f64>,
    pub constant_light_output: Option<bool>,
    pub light_distribution: Option<String>,
    pub switching_capacity: Option<String>,
}

/// GLDF Engine for parsing and manipulating GLDF files
#[derive(uniffi::Object)]
pub struct GldfEngine {
    product: RwLock<gldf_rs::GldfProduct>,
    raw_data: RwLock<Option<Vec<u8>>>,
    is_modified: RwLock<bool>,
}

#[uniffi::export]
impl GldfEngine {
    // =========================================================================
    // Constructors
    // =========================================================================

    /// Create a new GLDF engine from raw GLDF file bytes (ZIP archive)
    #[uniffi::constructor]
    pub fn from_bytes(data: Vec<u8>) -> Result<Arc<Self>, GldfError> {
        let product = gldf_rs::GldfProduct::load_gldf_from_buf(data.clone())
            .map_err(|e| GldfError::ParseError { msg: e.to_string() })?;

        Ok(Arc::new(Self {
            product: RwLock::new(product),
            raw_data: RwLock::new(Some(data)),
            is_modified: RwLock::new(false),
        }))
    }

    /// Create a new GLDF engine from JSON string
    #[uniffi::constructor]
    pub fn from_json(json: String) -> Result<Arc<Self>, GldfError> {
        let product = gldf_rs::GldfProduct::from_json(&json)
            .map_err(|e| GldfError::ParseError { msg: e.to_string() })?;

        Ok(Arc::new(Self {
            product: RwLock::new(product),
            raw_data: RwLock::new(None),
            is_modified: RwLock::new(false),
        }))
    }

    /// Create a new empty GLDF engine
    #[uniffi::constructor]
    pub fn new_empty() -> Arc<Self> {
        Arc::new(Self {
            product: RwLock::new(gldf_rs::GldfProduct::default()),
            raw_data: RwLock::new(None),
            is_modified: RwLock::new(false),
        })
    }

    // =========================================================================
    // Read Methods
    // =========================================================================

    /// Check if the product has been modified
    pub fn is_modified(&self) -> bool {
        *self.is_modified.read().unwrap()
    }

    /// Get header information
    pub fn get_header(&self) -> GldfHeader {
        let product = self.product.read().unwrap();
        GldfHeader {
            manufacturer: product.header.manufacturer.clone(),
            author: product.header.author.clone(),
            format_version: product.header.format_version.to_version_string(),
            created_with_application: product.header.created_with_application.clone(),
            creation_time_code: product.header.creation_time_code.clone(),
        }
    }

    /// Get all file definitions
    pub fn get_files(&self) -> Vec<GldfFile> {
        let product = self.product.read().unwrap();
        product
            .general_definitions
            .files
            .file
            .iter()
            .map(|f| GldfFile {
                id: f.id.clone(),
                file_name: f.file_name.clone(),
                content_type: f.content_type.clone(),
                file_type: f.type_attr.clone(),
            })
            .collect()
    }

    /// Get photometric files (LDT, IES)
    pub fn get_photometric_files(&self) -> Vec<GldfFile> {
        let product = self.product.read().unwrap();
        product
            .general_definitions
            .files
            .file
            .iter()
            .filter(|f| f.content_type.starts_with("ldc"))
            .map(|f| GldfFile {
                id: f.id.clone(),
                file_name: f.file_name.clone(),
                content_type: f.content_type.clone(),
                file_type: f.type_attr.clone(),
            })
            .collect()
    }

    /// Get image files
    pub fn get_image_files(&self) -> Vec<GldfFile> {
        let product = self.product.read().unwrap();
        product
            .general_definitions
            .files
            .file
            .iter()
            .filter(|f| f.content_type.starts_with("image"))
            .map(|f| GldfFile {
                id: f.id.clone(),
                file_name: f.file_name.clone(),
                content_type: f.content_type.clone(),
                file_type: f.type_attr.clone(),
            })
            .collect()
    }

    /// Get geometry (L3D) files
    pub fn get_geometry_files(&self) -> Vec<GldfFile> {
        let product = self.product.read().unwrap();
        product
            .general_definitions
            .files
            .file
            .iter()
            .filter(|f| f.content_type == "geo/l3d")
            .map(|f| GldfFile {
                id: f.id.clone(),
                file_name: f.file_name.clone(),
                content_type: f.content_type.clone(),
                file_type: f.type_attr.clone(),
            })
            .collect()
    }

    /// Get light sources (both fixed and changeable)
    pub fn get_light_sources(&self) -> Vec<GldfLightSource> {
        let product = self.product.read().unwrap();
        let mut result = Vec::new();

        if let Some(ref ls) = product.general_definitions.light_sources {
            for fixed in &ls.fixed_light_source {
                result.push(GldfLightSource {
                    id: fixed.id.clone(),
                    name: fixed
                        .name
                        .locale
                        .first()
                        .map(|n| n.value.clone())
                        .unwrap_or_default(),
                    light_source_type: "fixed".to_string(),
                });
            }

            for changeable in &ls.changeable_light_source {
                result.push(GldfLightSource {
                    id: changeable.id.clone(),
                    name: changeable.name.value.clone(),
                    light_source_type: "changeable".to_string(),
                });
            }
        }

        result
    }

    /// Get product variants with geometry and emitter references
    pub fn get_variants(&self) -> Vec<GldfVariant> {
        let product = self.product.read().unwrap();
        product
            .product_definitions
            .variants
            .as_ref()
            .map(|variants| {
                variants
                    .variant
                    .iter()
                    .map(|v| {
                        // Extract geometry ID and emitter refs from ModelGeometryReference
                        let (geometry_id, emitter_refs) = v
                            .geometry
                            .as_ref()
                            .and_then(|g| g.model_geometry_reference.as_ref())
                            .map(|mgr| {
                                let geom_id = mgr.geometry_id.clone();
                                let refs: Vec<GldfEmitterRef> = mgr
                                    .emitter_reference
                                    .iter()
                                    .map(|er| GldfEmitterRef {
                                        emitter_id: er.emitter_id.clone(),
                                        external_name: Some(
                                            er.emitter_object_external_name.clone(),
                                        ),
                                    })
                                    .collect();
                                (Some(geom_id), refs)
                            })
                            .unwrap_or((None, vec![]));

                        GldfVariant {
                            id: v.id.clone(),
                            name: v
                                .name
                                .as_ref()
                                .and_then(|n| n.locale.first())
                                .map(|l| l.value.clone())
                                .unwrap_or_default(),
                            description: v
                                .description
                                .as_ref()
                                .and_then(|d| d.locale.first())
                                .map(|l| l.value.clone())
                                .unwrap_or_default(),
                            geometry_id,
                            emitter_refs,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get emitter data for 3D rendering (photometry info, luminous flux)
    pub fn get_emitter_data(&self, emitter_id: String) -> Option<GldfEmitterData> {
        let product = self.product.read().unwrap();
        let emitters = product.general_definitions.emitters.as_ref()?;

        for emitter in &emitters.emitter {
            if emitter.id == emitter_id {
                // Check fixed light emitters first
                if let Some(fle) = emitter.fixed_light_emitter.first() {
                    let photometry_id = &fle.photometry_reference.photometry_id;
                    let photometry_file_id = product
                        .general_definitions
                        .photometries
                        .as_ref()
                        .and_then(|phots| {
                            phots
                                .photometry
                                .iter()
                                .find(|p| &p.id == photometry_id)
                                .and_then(|p| {
                                    p.photometry_file_reference
                                        .as_ref()
                                        .map(|pfr| pfr.file_id.clone())
                                })
                        });

                    return Some(GldfEmitterData {
                        emitter_id: emitter_id.clone(),
                        photometry_file_id,
                        light_source_type: "fixed".to_string(),
                        rated_luminous_flux: fle.rated_luminous_flux,
                    });
                }

                // Check changeable light emitters
                if let Some(cle) = emitter.changeable_light_emitter.first() {
                    let photometry_id = &cle.photometry_reference.photometry_id;
                    let photometry_file_id = product
                        .general_definitions
                        .photometries
                        .as_ref()
                        .and_then(|phots| {
                            phots
                                .photometry
                                .iter()
                                .find(|p| &p.id == photometry_id)
                                .and_then(|p| {
                                    p.photometry_file_reference
                                        .as_ref()
                                        .map(|pfr| pfr.file_id.clone())
                                })
                        });

                    return Some(GldfEmitterData {
                        emitter_id: emitter_id.clone(),
                        photometry_file_id,
                        light_source_type: "changeable".to_string(),
                        rated_luminous_flux: None,
                    });
                }

                // Check sensor emitters
                if !emitter.sensor_emitter.is_empty() {
                    return Some(GldfEmitterData {
                        emitter_id: emitter_id.clone(),
                        photometry_file_id: None,
                        light_source_type: "sensor".to_string(),
                        rated_luminous_flux: None,
                    });
                }
            }
        }

        None
    }

    /// Get statistics about the loaded GLDF
    pub fn get_stats(&self) -> GldfStats {
        let product = self.product.read().unwrap();
        let ls = product.general_definitions.light_sources.as_ref();

        GldfStats {
            files_count: product.general_definitions.files.file.len() as u64,
            fixed_light_sources_count: ls.map(|l| l.fixed_light_source.len()).unwrap_or(0) as u64,
            changeable_light_sources_count: ls.map(|l| l.changeable_light_source.len()).unwrap_or(0)
                as u64,
            variants_count: product
                .product_definitions
                .variants
                .as_ref()
                .map(|v| v.variant.len())
                .unwrap_or(0) as u64,
            photometries_count: product
                .general_definitions
                .photometries
                .as_ref()
                .map(|p| p.photometry.len())
                .unwrap_or(0) as u64,
            simple_geometries_count: product
                .general_definitions
                .geometries
                .as_ref()
                .map(|g| g.simple_geometry.len())
                .unwrap_or(0) as u64,
            model_geometries_count: product
                .general_definitions
                .geometries
                .as_ref()
                .map(|g| g.model_geometry.len())
                .unwrap_or(0) as u64,
        }
    }

    /// Get geometry file content by geometry ID (from variant)
    /// This resolves: geometry_id -> ModelGeometry -> GeometryFileReference -> File content
    pub fn get_geometry_content(&self, geometry_id: String) -> Result<GldfFileContent, GldfError> {
        // Resolve geometry_id to file_id
        let file_id = self
            .get_geometry_file_id(geometry_id.clone())
            .ok_or_else(|| GldfError::FileNotFound {
                msg: format!(
                    "Geometry '{}' not found or has no file reference",
                    geometry_id
                ),
            })?;

        // Use existing method to get file content
        self.get_file_content(file_id)
    }

    /// Get the file ID for a geometry ID (resolves ModelGeometry reference)
    pub fn get_geometry_file_id(&self, geometry_id: String) -> Option<String> {
        let product = self.product.read().unwrap();

        product
            .general_definitions
            .geometries
            .as_ref()
            .and_then(|geometries| {
                geometries
                    .model_geometry
                    .iter()
                    .find(|mg| mg.id == geometry_id)
            })
            .and_then(|mg| mg.geometry_file_reference.first())
            .map(|fr| fr.file_id.clone())
    }

    // =========================================================================
    // File Extraction Methods
    // =========================================================================

    /// Check if raw archive data is available for file extraction
    pub fn has_archive_data(&self) -> bool {
        self.raw_data.read().unwrap().is_some()
    }

    /// Extract file content by file ID
    /// Returns the binary content of the file from the GLDF archive
    pub fn get_file_content(&self, file_id: String) -> Result<GldfFileContent, GldfError> {
        let raw_data = self.raw_data.read().unwrap();
        let data = raw_data.as_ref().ok_or_else(|| GldfError::InvalidData {
            msg: "No archive data available (loaded from JSON)".to_string(),
        })?;

        // Find the file definition
        let product = self.product.read().unwrap();
        let file_def = product
            .general_definitions
            .files
            .file
            .iter()
            .find(|f| f.id == file_id)
            .ok_or_else(|| GldfError::FileNotFound {
                msg: format!("File with ID '{}' not found", file_id),
            })?;

        let file_name = file_def.file_name.clone();
        let content_type = file_def.content_type.clone();

        // Determine the path in the archive based on content type
        let archive_path = get_archive_path(&content_type, &file_name);
        drop(product);

        // Extract from ZIP
        let cursor = Cursor::new(data.clone());
        let mut zip = ZipArchive::new(cursor)
            .map_err(|e: zip::result::ZipError| GldfError::ParseError { msg: e.to_string() })?;

        let mut zip_file = zip
            .by_name(&archive_path)
            .map_err(|_| GldfError::FileNotFound {
                msg: format!("File '{}' not found in archive", archive_path),
            })?;

        let mut content = Vec::new();
        zip_file
            .read_to_end(&mut content)
            .map_err(|e: std::io::Error| GldfError::ParseError { msg: e.to_string() })?;

        Ok(GldfFileContent {
            file_id,
            file_name,
            content_type,
            data: content,
        })
    }

    /// Extract file content as string (for text-based files like LDT, IES)
    pub fn get_file_content_as_string(&self, file_id: String) -> Result<String, GldfError> {
        let content = self.get_file_content(file_id)?;
        String::from_utf8(content.data).map_err(|e| GldfError::InvalidData { msg: e.to_string() })
    }

    /// List all files in the archive with their paths
    pub fn list_archive_files(&self) -> Result<Vec<String>, GldfError> {
        let raw_data = self.raw_data.read().unwrap();
        let data = raw_data.as_ref().ok_or_else(|| GldfError::InvalidData {
            msg: "No archive data available".to_string(),
        })?;

        let cursor = Cursor::new(data.clone());
        let mut zip = ZipArchive::new(cursor)
            .map_err(|e: zip::result::ZipError| GldfError::ParseError { msg: e.to_string() })?;

        let mut result = Vec::new();
        for i in 0..zip.len() {
            if let Ok(file) = zip.by_index(i) {
                result.push(file.name().to_string());
            }
        }
        Ok(result)
    }

    /// Extract raw file from archive by path
    pub fn get_archive_file(&self, path: String) -> Result<Vec<u8>, GldfError> {
        let raw_data = self.raw_data.read().unwrap();
        let data = raw_data.as_ref().ok_or_else(|| GldfError::InvalidData {
            msg: "No archive data available".to_string(),
        })?;

        let cursor = Cursor::new(data.clone());
        let mut zip = ZipArchive::new(cursor)
            .map_err(|e: zip::result::ZipError| GldfError::ParseError { msg: e.to_string() })?;

        let mut zip_file = zip.by_name(&path).map_err(|_| GldfError::FileNotFound {
            msg: format!("File '{}' not found in archive", path),
        })?;

        let mut content = Vec::new();
        zip_file
            .read_to_end(&mut content)
            .map_err(|e: std::io::Error| GldfError::ParseError { msg: e.to_string() })?;

        Ok(content)
    }

    // =========================================================================
    // Edit Methods - Header
    // =========================================================================

    /// Set the author
    pub fn set_author(&self, author: String) {
        let mut product = self.product.write().unwrap();
        product.header.author = author;
        *self.is_modified.write().unwrap() = true;
    }

    /// Set the manufacturer
    pub fn set_manufacturer(&self, manufacturer: String) {
        let mut product = self.product.write().unwrap();
        product.header.manufacturer = manufacturer;
        *self.is_modified.write().unwrap() = true;
    }

    /// Set the creation time code
    pub fn set_creation_time_code(&self, time_code: String) {
        let mut product = self.product.write().unwrap();
        product.header.creation_time_code = time_code;
        *self.is_modified.write().unwrap() = true;
    }

    /// Set the created with application
    pub fn set_created_with_application(&self, app: String) {
        let mut product = self.product.write().unwrap();
        product.header.created_with_application = app;
        *self.is_modified.write().unwrap() = true;
    }

    /// Set the default language
    pub fn set_default_language(&self, language: Option<String>) {
        let mut product = self.product.write().unwrap();
        product.header.default_language = language;
        *self.is_modified.write().unwrap() = true;
    }

    /// Set the format version (e.g., "1.0.0-rc.3")
    pub fn set_format_version(&self, version: String) {
        use gldf_rs::gldf::FormatVersion;
        let mut product = self.product.write().unwrap();
        product.header.format_version = FormatVersion::from_string(&version);
        *self.is_modified.write().unwrap() = true;
    }

    // =========================================================================
    // Edit Methods - Files
    // =========================================================================

    /// Add a file definition
    pub fn add_file(&self, id: String, file_name: String, content_type: String, file_type: String) {
        use gldf_rs::gldf::general_definitions::files::File;
        let mut product = self.product.write().unwrap();
        product.general_definitions.files.file.push(File {
            id,
            file_name,
            content_type,
            type_attr: file_type,
            language: String::new(),
        });
        *self.is_modified.write().unwrap() = true;
    }

    /// Remove a file by ID
    pub fn remove_file(&self, id: String) {
        let mut product = self.product.write().unwrap();
        product
            .general_definitions
            .files
            .file
            .retain(|f| f.id != id);
        *self.is_modified.write().unwrap() = true;
    }

    /// Update a file definition
    pub fn update_file(
        &self,
        id: String,
        file_name: String,
        content_type: String,
        file_type: String,
    ) {
        let mut product = self.product.write().unwrap();
        if let Some(file) = product
            .general_definitions
            .files
            .file
            .iter_mut()
            .find(|f| f.id == id)
        {
            file.file_name = file_name;
            file.content_type = content_type;
            file.type_attr = file_type;
            *self.is_modified.write().unwrap() = true;
        }
    }

    // =========================================================================
    // Electrical Attributes Editing
    // =========================================================================

    /// Set electrical safety class (I, II, III or None)
    pub fn set_electrical_safety_class(&self, value: Option<String>) {
        let mut product = self.product.write().unwrap();
        ensure_electrical(&mut product).electrical_safety_class = value;
        *self.is_modified.write().unwrap() = true;
    }

    /// Set IP code (ingress protection)
    pub fn set_ip_code(&self, value: Option<String>) {
        let mut product = self.product.write().unwrap();
        ensure_electrical(&mut product).ingress_protection_ip_code = value;
        *self.is_modified.write().unwrap() = true;
    }

    /// Set power factor (0.0 - 1.0)
    pub fn set_power_factor(&self, value: Option<f64>) {
        let mut product = self.product.write().unwrap();
        ensure_electrical(&mut product).power_factor = value;
        *self.is_modified.write().unwrap() = true;
    }

    /// Set constant light output (CLO)
    pub fn set_constant_light_output(&self, value: Option<bool>) {
        let mut product = self.product.write().unwrap();
        ensure_electrical(&mut product).constant_light_output = value;
        *self.is_modified.write().unwrap() = true;
    }

    /// Set light distribution type
    pub fn set_light_distribution(&self, value: Option<String>) {
        let mut product = self.product.write().unwrap();
        ensure_electrical(&mut product).light_distribution = value;
        *self.is_modified.write().unwrap() = true;
    }

    /// Set switching capacity
    pub fn set_switching_capacity(&self, value: Option<String>) {
        let mut product = self.product.write().unwrap();
        ensure_electrical(&mut product).switching_capacity = value;
        *self.is_modified.write().unwrap() = true;
    }

    /// Get electrical attributes
    pub fn get_electrical(&self) -> GldfElectrical {
        let product = self.product.read().unwrap();
        let electrical = product
            .product_definitions
            .product_meta_data
            .as_ref()
            .and_then(|m| m.descriptive_attributes.as_ref())
            .and_then(|d| d.electrical.as_ref());

        GldfElectrical {
            safety_class: electrical.and_then(|e| e.electrical_safety_class.clone()),
            ip_code: electrical.and_then(|e| e.ingress_protection_ip_code.clone()),
            power_factor: electrical.and_then(|e| e.power_factor),
            constant_light_output: electrical.and_then(|e| e.constant_light_output),
            light_distribution: electrical.and_then(|e| e.light_distribution.clone()),
            switching_capacity: electrical.and_then(|e| e.switching_capacity.clone()),
        }
    }

    // =========================================================================
    // Applications Editing
    // =========================================================================

    /// Get current applications list
    pub fn get_applications(&self) -> Vec<String> {
        let product = self.product.read().unwrap();
        product
            .product_definitions
            .product_meta_data
            .as_ref()
            .and_then(|m| m.descriptive_attributes.as_ref())
            .and_then(|d| d.marketing.as_ref())
            .and_then(|m| m.applications.as_ref())
            .map(|a| a.application.clone())
            .unwrap_or_default()
    }

    /// Add an application
    pub fn add_application(&self, application: String) {
        let mut product = self.product.write().unwrap();
        ensure_applications(&mut product)
            .application
            .push(application);
        *self.is_modified.write().unwrap() = true;
    }

    /// Remove an application by index
    pub fn remove_application(&self, index: u32) {
        let mut product = self.product.write().unwrap();
        let apps = &mut ensure_applications(&mut product).application;
        let idx = index as usize;
        if idx < apps.len() {
            apps.remove(idx);
            *self.is_modified.write().unwrap() = true;
        }
    }

    /// Set all applications
    pub fn set_applications(&self, applications: Vec<String>) {
        let mut product = self.product.write().unwrap();
        ensure_applications(&mut product).application = applications;
        *self.is_modified.write().unwrap() = true;
    }

    // =========================================================================
    // Export Methods
    // =========================================================================

    /// Export to JSON string
    pub fn to_json(&self) -> Result<String, GldfError> {
        let product = self.product.read().unwrap();
        product
            .to_json()
            .map_err(|e| GldfError::SerializeError { msg: e.to_string() })
    }

    /// Export to pretty JSON string
    pub fn to_pretty_json(&self) -> Result<String, GldfError> {
        let product = self.product.read().unwrap();
        product
            .to_pretty_json()
            .map_err(|e| GldfError::SerializeError { msg: e.to_string() })
    }

    /// Export to XML string
    pub fn to_xml(&self) -> Result<String, GldfError> {
        let product = self.product.read().unwrap();
        product
            .to_xml()
            .map_err(|e| GldfError::SerializeError { msg: e.to_string() })
    }

    /// Mark as saved (clears modified flag)
    pub fn mark_saved(&self) {
        *self.is_modified.write().unwrap() = false;
    }
}

// -----------------------------------------------------------------------------
// Utility Functions
// -----------------------------------------------------------------------------

/// Parse GLDF from bytes and return JSON string
#[uniffi::export]
pub fn gldf_to_json(data: Vec<u8>) -> Result<String, GldfError> {
    let engine = GldfEngine::from_bytes(data)?;
    engine.to_json()
}

/// Get GLDF library version string
#[uniffi::export]
pub fn gldf_library_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// -----------------------------------------------------------------------------
// Internal Helper Functions
// -----------------------------------------------------------------------------

/// Helper to determine archive path from content type
fn get_archive_path(content_type: &str, file_name: &str) -> String {
    let folder = if content_type.starts_with("ldc") {
        "ldc"
    } else if content_type.starts_with("image") {
        "image"
    } else if content_type == "geo/l3d" {
        "geo"
    } else if content_type.starts_with("document") {
        "document"
    } else if content_type.starts_with("sensor") {
        "sensor"
    } else if content_type.starts_with("symbol") {
        "symbol"
    } else if content_type.starts_with("spectrum") {
        "spectrum"
    } else {
        "other"
    };
    format!("{}/{}", folder, file_name)
}

// =============================================================================
// EULUMDAT (LDT) Parser
// =============================================================================

/// Parsed EULUMDAT photometric data
#[derive(uniffi::Record, Debug, Clone, Default)]
pub struct EulumdatData {
    /// Manufacturer/Header string
    pub manufacturer: String,
    /// Luminaire name
    pub luminaire_name: String,
    /// Luminaire number
    pub luminaire_number: String,
    /// Lamp type description
    pub lamp_type: String,
    /// Total luminous flux in lumens
    pub total_lumens: f64,
    /// Light Output Ratio Luminaire (%)
    pub lorl: f64,
    /// Number of C planes
    pub c_plane_count: i32,
    /// Number of gamma angles
    pub gamma_count: i32,
    /// Symmetry indicator (0-4)
    pub symmetry: i32,
    /// C plane angles in degrees
    pub c_angles: Vec<f64>,
    /// Gamma angles in degrees
    pub gamma_angles: Vec<f64>,
    /// Intensity values organized by C plane, then by gamma
    /// Access as: intensities[c_plane_index * gamma_count + gamma_index]
    pub intensities: Vec<f64>,
    /// Maximum intensity value (for normalization)
    pub max_intensity: f64,
    /// Unit conversion factor
    pub conversion_factor: f64,
    /// Luminaire dimensions in mm [length, width, height]
    pub luminaire_dimensions: Vec<f64>,
    /// Luminous area dimensions in mm [length, width]
    pub luminous_area_dimensions: Vec<f64>,
    /// Downward flux fraction (%)
    pub dff: f64,
    /// Color temperature (K)
    pub color_temperature: String,
    /// Wattage (W)
    pub wattage: f64,
}

/// Parse EULUMDAT (LDT) file from string content
#[uniffi::export]
pub fn parse_eulumdat(content: String) -> EulumdatData {
    let lines: Vec<&str> = content.lines().map(|l| l.trim()).collect();

    let mut data = EulumdatData::default();

    if lines.len() < 27 {
        return data;
    }

    // Parse header (lines are 1-indexed in spec, 0-indexed here)
    data.manufacturer = lines.first().unwrap_or(&"").to_string();
    data.symmetry = lines.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    data.c_plane_count = lines.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    let _dc = lines
        .get(4)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    data.gamma_count = lines.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
    let _dg = lines
        .get(6)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    data.luminaire_name = lines.get(8).unwrap_or(&"").to_string();
    data.luminaire_number = lines.get(9).unwrap_or(&"").to_string();

    // Luminaire dimensions
    let l_length = lines.get(12).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let l_width = lines.get(13).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let l_height = lines.get(14).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    data.luminaire_dimensions = vec![l_length, l_width, l_height];

    // Luminous area
    let la_length = lines.get(15).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let la_width = lines.get(16).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    data.luminous_area_dimensions = vec![la_length, la_width];

    // DFF and LORL
    data.dff = lines.get(21).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    data.lorl = lines.get(22).and_then(|s| s.parse().ok()).unwrap_or(100.0);
    data.conversion_factor = lines.get(23).and_then(|s| s.parse().ok()).unwrap_or(1.0);

    // Number of lamp sets
    let n_lamp_sets: usize = lines.get(25).and_then(|s| s.parse().ok()).unwrap_or(1);

    // Lamp section starts at line 27 (index 26)
    let lamp_section_start = 26;
    let n_lamp_params = 6;

    // Get lamp info from first lamp set
    if lamp_section_start + 1 < lines.len() {
        data.lamp_type = lines[lamp_section_start + 1].to_string();
    }
    if lamp_section_start + 2 < lines.len() {
        data.total_lumens = lines[lamp_section_start + 2].parse().unwrap_or(0.0);
    }
    if lamp_section_start + 3 < lines.len() {
        data.color_temperature = lines[lamp_section_start + 3].to_string();
    }
    if lamp_section_start + 5 < lines.len() {
        data.wattage = lines[lamp_section_start + 5].parse().unwrap_or(0.0);
    }

    // Calculate offsets
    let direct_ratios_start = lamp_section_start + n_lamp_params * n_lamp_sets;
    let c_angles_start = direct_ratios_start + 10;
    let g_angles_start = c_angles_start + data.c_plane_count as usize;
    let intensities_start = g_angles_start + data.gamma_count as usize;

    // Parse C angles
    for i in 0..data.c_plane_count as usize {
        let idx = c_angles_start + i;
        if idx < lines.len() {
            if let Ok(angle) = lines[idx].parse() {
                data.c_angles.push(angle);
            }
        }
    }

    // Parse gamma angles
    for i in 0..data.gamma_count as usize {
        let idx = g_angles_start + i;
        if idx < lines.len() {
            if let Ok(angle) = lines[idx].parse() {
                data.gamma_angles.push(angle);
            }
        }
    }

    // Calculate actual number of C planes with data based on symmetry
    let (mc1, mc2) = calculate_mc_range(data.symmetry, data.c_plane_count);
    let actual_c_planes = (mc2 - mc1 + 1) as usize;

    // Parse intensities
    let mut max_val: f64 = 0.0;
    let mut line_idx = intensities_start;

    for _ in 0..actual_c_planes {
        for _ in 0..data.gamma_count as usize {
            if line_idx < lines.len() {
                if let Ok(intensity) = lines[line_idx].parse::<f64>() {
                    data.intensities.push(intensity);
                    if intensity > max_val {
                        max_val = intensity;
                    }
                }
            }
            line_idx += 1;
        }
    }

    data.max_intensity = if max_val > 0.0 { max_val } else { 1000.0 };

    data
}

/// Calculate mc1 and mc2 range based on symmetry
fn calculate_mc_range(symmetry: i32, n_c_planes: i32) -> (i32, i32) {
    match symmetry {
        0 => (1, n_c_planes),         // No symmetry
        1 => (1, 1),                  // Symmetry about vertical axis
        2 => (1, n_c_planes / 2 + 1), // C0-C180 plane symmetry
        3 => {
            // C90-C270 plane symmetry
            let mc1 = 3 * (n_c_planes / 4) + 1;
            (mc1, mc1 + n_c_planes / 2)
        }
        4 => (1, n_c_planes / 4 + 1), // C0-C180 and C90-C270 symmetry
        _ => (1, n_c_planes.max(1)),
    }
}

/// Parse EULUMDAT from raw bytes
#[uniffi::export]
pub fn parse_eulumdat_bytes(data: Vec<u8>) -> EulumdatData {
    let content = String::from_utf8_lossy(&data).to_string();
    parse_eulumdat(content)
}

// =============================================================================
// L3D (3D Geometry) Parser
// =============================================================================

/// 3D vector
#[derive(uniffi::Record, Debug, Clone, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// 4x4 transformation matrix (column-major for OpenGL/Metal/SceneKit)
#[derive(uniffi::Record, Debug, Clone)]
pub struct Matrix4 {
    /// Matrix values in column-major order (m00, m10, m20, m30, m01, m11, ...)
    pub values: Vec<f64>,
}

impl Default for Matrix4 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Matrix4 {
    fn identity() -> Self {
        Self {
            values: vec![
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    fn from_translation(x: f64, y: f64, z: f64) -> Self {
        Self {
            values: vec![
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, x, y, z, 1.0,
            ],
        }
    }

    fn from_scale(s: f64) -> Self {
        Self {
            values: vec![
                s, 0.0, 0.0, 0.0, 0.0, s, 0.0, 0.0, 0.0, 0.0, s, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    fn from_rotation_xyz(rx: f64, ry: f64, rz: f64) -> Self {
        let rx = rx.to_radians();
        let ry = ry.to_radians();
        let rz = rz.to_radians();

        let (sx, cx) = (rx.sin(), rx.cos());
        let (sy, cy) = (ry.sin(), ry.cos());
        let (sz, cz) = (rz.sin(), rz.cos());

        // Combined rotation matrix: Rz * Ry * Rx
        Self {
            values: vec![
                cy * cz,
                cx * sz + sx * sy * cz,
                sx * sz - cx * sy * cz,
                0.0,
                -cy * sz,
                cx * cz - sx * sy * sz,
                sx * cz + cx * sy * sz,
                0.0,
                sy,
                -sx * cy,
                cx * cy,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
            ],
        }
    }

    fn multiply(&self, other: &Matrix4) -> Matrix4 {
        let mut result = vec![0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.values[k * 4 + row] * other.values[col * 4 + k];
                }
                result[col * 4 + row] = sum;
            }
        }
        Matrix4 { values: result }
    }
}

/// Geometry file definition in L3D
#[derive(uniffi::Record, Debug, Clone, Default)]
pub struct L3dGeometryDef {
    /// Unique ID for this geometry
    pub id: String,
    /// Filename of the OBJ file
    pub filename: String,
    /// Units: "m", "mm", "in"
    pub units: String,
}

/// A joint axis definition (for articulated luminaires)
#[derive(uniffi::Record, Debug, Clone, Default)]
pub struct L3dJointAxis {
    /// Axis type: "x", "y", or "z"
    pub axis: String,
    /// Minimum angle in degrees
    pub min: f64,
    /// Maximum angle in degrees
    pub max: f64,
    /// Step size in degrees
    pub step: f64,
}

/// Light emitting object (LEO) in L3D
#[derive(uniffi::Record, Debug, Clone, Default)]
pub struct L3dLightEmittingObject {
    /// Part name
    pub part_name: String,
    /// Position relative to parent
    pub position: Vec3,
    /// Rotation in degrees
    pub rotation: Vec3,
    /// Shape type: "circle" or "rectangle"
    pub shape_type: String,
    /// Diameter for circle, or [width, height] for rectangle
    pub shape_dimensions: Vec<f64>,
}

/// Face assignment for light emitting surfaces
#[derive(uniffi::Record, Debug, Clone, Default)]
pub struct L3dFaceAssignment {
    /// Light emitting object part name
    pub leo_part_name: String,
    /// Starting face index
    pub face_index_begin: i32,
    /// Ending face index
    pub face_index_end: i32,
}

/// A geometry part in the L3D scene hierarchy
#[derive(uniffi::Record, Debug, Clone, Default)]
pub struct L3dScenePart {
    /// Part name
    pub part_name: String,
    /// Geometry definition ID
    pub geometry_id: String,
    /// Local position
    pub position: Vec3,
    /// Local rotation (degrees)
    pub rotation: Vec3,
    /// Pre-computed world transform matrix (column-major)
    pub world_transform: Matrix4,
    /// Scale factor based on units
    pub scale: f64,
    /// Light emitting objects attached to this part
    pub light_emitting_objects: Vec<L3dLightEmittingObject>,
    /// Face assignments for LEOs
    pub face_assignments: Vec<L3dFaceAssignment>,
    /// Child joint names (for reference)
    pub joint_names: Vec<String>,
}

/// Joint definition for articulated parts
#[derive(uniffi::Record, Debug, Clone, Default)]
pub struct L3dJoint {
    /// Joint part name
    pub part_name: String,
    /// Position relative to parent
    pub position: Vec3,
    /// Rotation in degrees
    pub rotation: Vec3,
    /// Axis constraints
    pub axis: Option<L3dJointAxis>,
    /// Default rotation value (if specified)
    pub default_rotation: Option<Vec3>,
}

/// Complete L3D scene information
#[derive(uniffi::Record, Debug, Clone, Default)]
pub struct L3dScene {
    /// Application that created this file
    pub created_with_application: String,
    /// Creation timestamp
    pub creation_time_code: String,
    /// All geometry file definitions
    pub geometry_definitions: Vec<L3dGeometryDef>,
    /// Flattened list of all scene parts with world transforms
    pub parts: Vec<L3dScenePart>,
    /// Joint definitions (for articulated luminaires)
    pub joints: Vec<L3dJoint>,
    /// Raw structure.xml content (for debugging)
    pub raw_structure_xml: String,
}

/// Asset file extracted from L3D archive
#[derive(uniffi::Record, Debug, Clone, Default)]
pub struct L3dAsset {
    /// File name/path in archive
    pub name: String,
    /// File content (OBJ, MTL, textures)
    pub data: Vec<u8>,
}

/// Complete L3D file with scene and assets
#[derive(uniffi::Record, Debug, Clone, Default)]
pub struct L3dFile {
    /// Parsed scene information
    pub scene: L3dScene,
    /// All assets (OBJ files, MTL files, textures)
    pub assets: Vec<L3dAsset>,
}

// Internal XML parsing structures
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XmlLuminaire {
    header: XmlHeader,
    geometry_definitions: XmlGeometryDefinitions,
    structure: XmlStructure,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XmlHeader {
    name: Option<String>,
    description: Option<String>,
    created_with_application: Option<String>,
    creation_time_code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XmlGeometryDefinitions {
    geometry_file_definition: Vec<XmlGeometryFileDefinition>,
}

#[derive(Debug, Deserialize)]
struct XmlGeometryFileDefinition {
    id: String,
    filename: String,
    units: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XmlStructure {
    geometry: XmlGeometry,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XmlGeometry {
    #[serde(rename = "partName")]
    part_name: String,
    position: XmlVec3,
    rotation: XmlVec3,
    geometry_reference: XmlGeometryReference,
    joints: Option<XmlJoints>,
    light_emitting_objects: Option<XmlLightEmittingObjects>,
    light_emitting_face_assignments: Option<XmlLightEmittingFaceAssignments>,
}

#[derive(Debug, Deserialize)]
struct XmlVec3 {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct XmlGeometryReference {
    geometry_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XmlJoints {
    joint: Vec<XmlJoint>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XmlJoint {
    #[serde(rename = "partName")]
    part_name: String,
    position: XmlVec3,
    rotation: XmlVec3,
    #[serde(rename = "XAxis")]
    x_axis: Option<XmlAxis>,
    #[serde(rename = "YAxis")]
    y_axis: Option<XmlAxis>,
    #[serde(rename = "ZAxis")]
    z_axis: Option<XmlAxis>,
    default_rotation: Option<XmlVec3>,
    geometries: XmlGeometries,
}

#[derive(Debug, Deserialize)]
struct XmlAxis {
    min: f64,
    max: f64,
    step: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XmlGeometries {
    geometry: Vec<XmlGeometry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XmlLightEmittingObjects {
    light_emitting_object: Vec<XmlLightEmittingObject>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XmlLightEmittingObject {
    #[serde(rename = "partName")]
    part_name: String,
    position: XmlVec3,
    rotation: XmlVec3,
    circle: Option<XmlCircle>,
    rectangle: Option<XmlRectangle>,
}

#[derive(Debug, Deserialize)]
struct XmlCircle {
    diameter: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct XmlRectangle {
    size_x: f64,
    size_y: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XmlLightEmittingFaceAssignments {
    range_assignment: Option<Vec<XmlRangeAssignment>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct XmlRangeAssignment {
    light_emitting_part_name: String,
    face_index_begin: i32,
    face_index_end: i32,
}

/// Parse L3D file from raw bytes (ZIP archive)
#[uniffi::export]
pub fn parse_l3d(data: Vec<u8>) -> Result<L3dFile, GldfError> {
    let cursor = Cursor::new(&data);
    let mut zip = ZipArchive::new(cursor).map_err(|e| GldfError::ParseError {
        msg: format!("Invalid L3D archive: {}", e),
    })?;

    let mut structure_xml = String::new();
    let mut assets = Vec::new();

    // Extract all files
    for i in 0..zip.len() {
        let mut file = zip
            .by_index(i)
            .map_err(|e| GldfError::ParseError { msg: e.to_string() })?;

        if file.is_file() {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|e| GldfError::ParseError { msg: e.to_string() })?;

            if file.name() == "structure.xml" {
                structure_xml = String::from_utf8_lossy(&buf).to_string();
            } else {
                assets.push(L3dAsset {
                    name: file.name().to_string(),
                    data: buf,
                });
            }
        }
    }

    if structure_xml.is_empty() {
        return Err(GldfError::ParseError {
            msg: "structure.xml not found in L3D archive".to_string(),
        });
    }

    // Parse structure.xml
    let scene = parse_l3d_structure(structure_xml.clone())?;

    Ok(L3dFile {
        scene: L3dScene {
            raw_structure_xml: structure_xml,
            ..scene
        },
        assets,
    })
}

/// Parse L3D structure.xml content
#[uniffi::export]
pub fn parse_l3d_structure(xml_content: String) -> Result<L3dScene, GldfError> {
    let luminaire: XmlLuminaire =
        serde_xml_rs::from_str(&xml_content).map_err(|e| GldfError::ParseError {
            msg: format!("Invalid structure.xml: {}", e),
        })?;

    let mut scene = L3dScene {
        created_with_application: luminaire
            .header
            .created_with_application
            .unwrap_or_default(),
        creation_time_code: luminaire.header.creation_time_code.unwrap_or_default(),
        geometry_definitions: luminaire
            .geometry_definitions
            .geometry_file_definition
            .iter()
            .map(|g| L3dGeometryDef {
                id: g.id.clone(),
                filename: g.filename.clone(),
                units: g.units.clone(),
            })
            .collect(),
        parts: Vec::new(),
        joints: Vec::new(),
        raw_structure_xml: String::new(),
    };

    // Build a map of geometry IDs to units for scale calculation
    let units_map: std::collections::HashMap<String, String> = luminaire
        .geometry_definitions
        .geometry_file_definition
        .iter()
        .map(|g| (g.id.clone(), g.units.clone()))
        .collect();

    // Recursively parse geometry hierarchy
    let root_transform = Matrix4::identity();
    parse_geometry_recursive(
        &luminaire.structure.geometry,
        &root_transform,
        &units_map,
        &mut scene.parts,
        &mut scene.joints,
    );

    Ok(scene)
}

fn parse_geometry_recursive(
    geo: &XmlGeometry,
    parent_transform: &Matrix4,
    units_map: &std::collections::HashMap<String, String>,
    parts: &mut Vec<L3dScenePart>,
    joints: &mut Vec<L3dJoint>,
) {
    // Calculate local transform
    let translation = Matrix4::from_translation(geo.position.x, geo.position.y, geo.position.z);
    let rotation = Matrix4::from_rotation_xyz(geo.rotation.x, geo.rotation.y, geo.rotation.z);
    let local_transform = translation.multiply(&rotation);
    let world_transform = parent_transform.multiply(&local_transform);

    // Get scale from units
    let scale = units_map
        .get(&geo.geometry_reference.geometry_id)
        .map(|u| get_unit_scale(u))
        .unwrap_or(1.0);

    // Create scale transform for final world transform
    let scale_transform = Matrix4::from_scale(scale);
    let final_transform = world_transform.multiply(&scale_transform);

    // Parse light emitting objects
    let leos: Vec<L3dLightEmittingObject> = geo
        .light_emitting_objects
        .as_ref()
        .map(|leos| {
            leos.light_emitting_object
                .iter()
                .map(|leo| {
                    let (shape_type, shape_dimensions) = if let Some(circle) = &leo.circle {
                        ("circle".to_string(), vec![circle.diameter])
                    } else if let Some(rect) = &leo.rectangle {
                        ("rectangle".to_string(), vec![rect.size_x, rect.size_y])
                    } else {
                        ("unknown".to_string(), vec![])
                    };

                    L3dLightEmittingObject {
                        part_name: leo.part_name.clone(),
                        position: Vec3 {
                            x: leo.position.x,
                            y: leo.position.y,
                            z: leo.position.z,
                        },
                        rotation: Vec3 {
                            x: leo.rotation.x,
                            y: leo.rotation.y,
                            z: leo.rotation.z,
                        },
                        shape_type,
                        shape_dimensions,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // Parse face assignments
    let face_assignments: Vec<L3dFaceAssignment> = geo
        .light_emitting_face_assignments
        .as_ref()
        .and_then(|fa| fa.range_assignment.as_ref())
        .map(|assignments| {
            assignments
                .iter()
                .map(|ra| L3dFaceAssignment {
                    leo_part_name: ra.light_emitting_part_name.clone(),
                    face_index_begin: ra.face_index_begin,
                    face_index_end: ra.face_index_end,
                })
                .collect()
        })
        .unwrap_or_default();

    // Collect joint names
    let joint_names: Vec<String> = geo
        .joints
        .as_ref()
        .map(|j| j.joint.iter().map(|jt| jt.part_name.clone()).collect())
        .unwrap_or_default();

    // Add this part
    parts.push(L3dScenePart {
        part_name: geo.part_name.clone(),
        geometry_id: geo.geometry_reference.geometry_id.clone(),
        position: Vec3 {
            x: geo.position.x,
            y: geo.position.y,
            z: geo.position.z,
        },
        rotation: Vec3 {
            x: geo.rotation.x,
            y: geo.rotation.y,
            z: geo.rotation.z,
        },
        world_transform: final_transform.clone(),
        scale,
        light_emitting_objects: leos,
        face_assignments,
        joint_names,
    });

    // Process joints and child geometries
    if let Some(ref joint_list) = geo.joints {
        for joint in &joint_list.joint {
            // Calculate joint transform
            let joint_translation =
                Matrix4::from_translation(joint.position.x, joint.position.y, joint.position.z);
            let joint_rotation =
                Matrix4::from_rotation_xyz(joint.rotation.x, joint.rotation.y, joint.rotation.z);
            let joint_transform = world_transform
                .multiply(&joint_translation)
                .multiply(&joint_rotation);

            // Parse axis
            #[allow(clippy::manual_map)]
            let axis = if let Some(ref x) = joint.x_axis {
                Some(L3dJointAxis {
                    axis: "x".to_string(),
                    min: x.min,
                    max: x.max,
                    step: x.step,
                })
            } else if let Some(ref y) = joint.y_axis {
                Some(L3dJointAxis {
                    axis: "y".to_string(),
                    min: y.min,
                    max: y.max,
                    step: y.step,
                })
            } else if let Some(ref z) = joint.z_axis {
                Some(L3dJointAxis {
                    axis: "z".to_string(),
                    min: z.min,
                    max: z.max,
                    step: z.step,
                })
            } else {
                None
            };

            // Add joint info
            joints.push(L3dJoint {
                part_name: joint.part_name.clone(),
                position: Vec3 {
                    x: joint.position.x,
                    y: joint.position.y,
                    z: joint.position.z,
                },
                rotation: Vec3 {
                    x: joint.rotation.x,
                    y: joint.rotation.y,
                    z: joint.rotation.z,
                },
                axis,
                default_rotation: joint.default_rotation.as_ref().map(|r| Vec3 {
                    x: r.x,
                    y: r.y,
                    z: r.z,
                }),
            });

            // Recurse into child geometries
            for child_geo in &joint.geometries.geometry {
                parse_geometry_recursive(child_geo, &joint_transform, units_map, parts, joints);
            }
        }
    }
}

fn get_unit_scale(unit: &str) -> f64 {
    match unit {
        "mm" => 0.001,
        "in" => 0.0254,
        _ => 1.0, // "m" or default
    }
}

// -----------------------------------------------------------------------------
// Editing Helper Functions
// -----------------------------------------------------------------------------

use gldf_rs::gldf::product_definitions::{
    Applications, DescriptiveAttributes, Electrical, Marketing, ProductMetaData,
};

/// Helper to ensure ProductMetaData exists
fn ensure_product_meta_data(product: &mut gldf_rs::GldfProduct) -> &mut ProductMetaData {
    if product.product_definitions.product_meta_data.is_none() {
        product.product_definitions.product_meta_data = Some(ProductMetaData::default());
    }
    product
        .product_definitions
        .product_meta_data
        .as_mut()
        .unwrap()
}

/// Helper to ensure DescriptiveAttributes exists
fn ensure_descriptive_attributes(product: &mut gldf_rs::GldfProduct) -> &mut DescriptiveAttributes {
    let meta = ensure_product_meta_data(product);
    if meta.descriptive_attributes.is_none() {
        meta.descriptive_attributes = Some(DescriptiveAttributes::default());
    }
    meta.descriptive_attributes.as_mut().unwrap()
}

/// Helper to ensure Electrical exists
fn ensure_electrical(product: &mut gldf_rs::GldfProduct) -> &mut Electrical {
    let attrs = ensure_descriptive_attributes(product);
    if attrs.electrical.is_none() {
        attrs.electrical = Some(Electrical::default());
    }
    attrs.electrical.as_mut().unwrap()
}

/// Helper to ensure Marketing exists
fn ensure_marketing(product: &mut gldf_rs::GldfProduct) -> &mut Marketing {
    let attrs = ensure_descriptive_attributes(product);
    if attrs.marketing.is_none() {
        attrs.marketing = Some(Marketing::default());
    }
    attrs.marketing.as_mut().unwrap()
}

/// Helper to ensure Applications exists
fn ensure_applications(product: &mut gldf_rs::GldfProduct) -> &mut Applications {
    let marketing = ensure_marketing(product);
    if marketing.applications.is_none() {
        marketing.applications = Some(Applications::default());
    }
    marketing.applications.as_mut().unwrap()
}

/// Get asset from L3D file by filename
#[uniffi::export]
pub fn get_l3d_asset(l3d_file: &L3dFile, filename: String) -> Option<Vec<u8>> {
    l3d_file
        .assets
        .iter()
        .find(|a| a.name == filename || a.name.ends_with(&format!("/{}", filename)))
        .map(|a| a.data.clone())
}
