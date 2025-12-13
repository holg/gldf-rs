//! Validation engine for GLDF files.
//!
//! Provides validation of GLDF products against the schema rules,
//! checking for required fields, reference integrity, and data consistency.

use crate::gldf::GldfProduct;
use std::collections::{HashMap, HashSet};

/// Severity level of a validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationLevel {
    /// Critical error - the GLDF is invalid and cannot be used
    Error,
    /// Warning - the GLDF may work but has potential issues
    Warning,
    /// Informational - suggestion for improvement
    Info,
}

/// A validation error or warning.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// The path to the problematic field (e.g., "variants[0].geometry")
    pub path: String,
    /// The severity level
    pub level: ValidationLevel,
    /// Human-readable description of the issue
    pub message: String,
    /// Error code for programmatic handling
    pub code: &'static str,
}

impl ValidationError {
    /// Creates a new validation error.
    pub fn error(path: impl Into<String>, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            level: ValidationLevel::Error,
            message: message.into(),
            code,
        }
    }

    /// Creates a new validation warning.
    pub fn warning(path: impl Into<String>, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            level: ValidationLevel::Warning,
            message: message.into(),
            code,
        }
    }

    /// Creates a new informational validation message.
    pub fn info(path: impl Into<String>, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            level: ValidationLevel::Info,
            message: message.into(),
            code,
        }
    }
}

/// Result of validating a GLDF product.
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    /// All validation issues found
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    /// Creates an empty validation result.
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Adds an error to the result.
    pub fn add(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Returns true if there are any errors (not warnings or info).
    pub fn has_errors(&self) -> bool {
        self.errors.iter().any(|e| e.level == ValidationLevel::Error)
    }

    /// Returns true if there are any warnings.
    pub fn has_warnings(&self) -> bool {
        self.errors.iter().any(|e| e.level == ValidationLevel::Warning)
    }

    /// Returns true if the result is completely clean (no issues at all).
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns only errors (excludes warnings and info).
    pub fn errors_only(&self) -> Vec<&ValidationError> {
        self.errors.iter().filter(|e| e.level == ValidationLevel::Error).collect()
    }

    /// Returns only warnings.
    pub fn warnings_only(&self) -> Vec<&ValidationError> {
        self.errors.iter().filter(|e| e.level == ValidationLevel::Warning).collect()
    }

    /// Returns the count of issues by level.
    pub fn count_by_level(&self) -> (usize, usize, usize) {
        let errors = self.errors.iter().filter(|e| e.level == ValidationLevel::Error).count();
        let warnings = self.errors.iter().filter(|e| e.level == ValidationLevel::Warning).count();
        let info = self.errors.iter().filter(|e| e.level == ValidationLevel::Info).count();
        (errors, warnings, info)
    }
}

/// Validates a GLDF product.
///
/// # Arguments
/// * `product` - The GLDF product to validate
/// * `embedded_files` - Map of file IDs to their binary content (for checking file references)
///
/// # Returns
/// A `ValidationResult` containing all validation issues found.
pub fn validate_gldf(
    product: &GldfProduct,
    embedded_files: &HashMap<String, Vec<u8>>,
) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Collect all defined file IDs for reference checking
    let file_ids: HashSet<&str> = product
        .general_definitions
        .files
        .file
        .iter()
        .map(|f| f.id.as_str())
        .collect();

    // Validate header
    validate_header(product, &mut result);

    // Validate file definitions
    validate_files(product, embedded_files, &mut result);

    // Validate photometries
    validate_photometries(product, &file_ids, &mut result);

    // Validate geometries
    validate_geometries(product, &file_ids, &mut result);

    // Validate light sources
    validate_light_sources(product, &mut result);

    // Validate emitters
    validate_emitters(product, &mut result);

    // Validate variants
    validate_variants(product, &mut result);

    // Check ID uniqueness
    validate_id_uniqueness(product, &mut result);

    result
}

fn validate_header(product: &GldfProduct, result: &mut ValidationResult) {
    let header = &product.header;

    // Author is required
    if header.author.is_empty() {
        result.add(ValidationError::error(
            "header.author",
            "HEADER_001",
            "Author is required",
        ));
    }

    // Manufacturer is required
    if header.manufacturer.is_empty() {
        result.add(ValidationError::error(
            "header.manufacturer",
            "HEADER_002",
            "Manufacturer is required",
        ));
    }

    // Format version check
    let version = &header.format_version;
    if version.major == 0 && version.minor == 0 {
        result.add(ValidationError::warning(
            "header.formatVersion",
            "HEADER_003",
            "Format version 0.0 may not be valid",
        ));
    }

    // Creation time code recommended
    if header.creation_time_code.is_empty() {
        result.add(ValidationError::info(
            "header.creationTimeCode",
            "HEADER_004",
            "Consider adding a creation time code",
        ));
    }
}

fn validate_files(
    product: &GldfProduct,
    embedded_files: &HashMap<String, Vec<u8>>,
    result: &mut ValidationResult,
) {
    for (i, file) in product.general_definitions.files.file.iter().enumerate() {
        let path = format!("generalDefinitions.files[{}]", i);

        // File ID is required
        if file.id.is_empty() {
            result.add(ValidationError::error(
                format!("{}.id", path),
                "FILE_001",
                "File ID is required",
            ));
        }

        // Content type is required
        if file.content_type.is_empty() {
            result.add(ValidationError::error(
                format!("{}.contentType", path),
                "FILE_002",
                "Content type is required",
            ));
        }

        // File name is required
        if file.file_name.is_empty() {
            result.add(ValidationError::error(
                format!("{}.fileName", path),
                "FILE_003",
                "File name is required",
            ));
        }

        // For local files, check if embedded content exists
        if file.type_attr != "url" && !file.id.is_empty() {
            if !embedded_files.contains_key(&file.id) {
                result.add(ValidationError::warning(
                    format!("{}", path),
                    "FILE_004",
                    format!("Embedded file '{}' not found for file definition '{}'", file.file_name, file.id),
                ));
            }
        }

        // Validate content type format
        let valid_content_types = [
            "ldc/eulumdat", "ldc/ies",
            "geo/l3d", "geo/m3d", "geo/r3d",
            "image/png", "image/jpg", "image/jpeg", "image/svg",
            "document/pdf",
            "spectrum/txt",
            "sensor/sens-ldt",
            "symbol/dxf", "symbol/svg",
            "other",
        ];

        if !file.content_type.is_empty() && !valid_content_types.iter().any(|ct| file.content_type.starts_with(ct.split('/').next().unwrap_or(""))) {
            result.add(ValidationError::warning(
                format!("{}.contentType", path),
                "FILE_005",
                format!("Unusual content type: '{}'", file.content_type),
            ));
        }
    }
}

fn validate_photometries(
    product: &GldfProduct,
    file_ids: &HashSet<&str>,
    result: &mut ValidationResult,
) {
    if let Some(ref photometries) = product.general_definitions.photometries {
        for (i, photometry) in photometries.photometry.iter().enumerate() {
            let path = format!("generalDefinitions.photometries[{}]", i);

            // Check photometry ID
            if photometry.id.is_empty() {
                result.add(ValidationError::error(
                    format!("{}.id", path),
                    "PHOT_001",
                    "Photometry ID is required",
                ));
            }

            // Check file reference if present
            if let Some(ref file_ref) = photometry.photometry_file_reference {
                if !file_ids.contains(file_ref.file_id.as_str()) {
                    result.add(ValidationError::error(
                        format!("{}.photometryFileReference.fileId", path),
                        "PHOT_002",
                        format!("Referenced file '{}' not found in file definitions", file_ref.file_id),
                    ));
                }
            }
        }
    }
}

fn validate_geometries(
    product: &GldfProduct,
    file_ids: &HashSet<&str>,
    result: &mut ValidationResult,
) {
    if let Some(ref geometries) = product.general_definitions.geometries {
        // Validate simple geometries
        for (i, geom) in geometries.simple_geometry.iter().enumerate() {
            let path = format!("generalDefinitions.geometries.simpleGeometry[{}]", i);

            if geom.id.is_empty() {
                result.add(ValidationError::error(
                    format!("{}.id", path),
                    "GEOM_001",
                    "Geometry ID is required",
                ));
            }
        }

        // Validate model geometries
        for (i, geom) in geometries.model_geometry.iter().enumerate() {
            let path = format!("generalDefinitions.geometries.modelGeometry[{}]", i);

            if geom.id.is_empty() {
                result.add(ValidationError::error(
                    format!("{}.id", path),
                    "GEOM_002",
                    "Model geometry ID is required",
                ));
            }

            // Check geometry file references
            for (j, file_ref) in geom.geometry_file_reference.iter().enumerate() {
                if !file_ids.contains(file_ref.file_id.as_str()) {
                    result.add(ValidationError::error(
                        format!("{}.geometryFileReference[{}].fileId", path, j),
                        "GEOM_003",
                        format!("Referenced geometry file '{}' not found", file_ref.file_id),
                    ));
                }
            }
        }
    }
}

fn validate_light_sources(product: &GldfProduct, result: &mut ValidationResult) {
    if let Some(ref light_sources) = product.general_definitions.light_sources {
        // Validate fixed light sources
        for (i, source) in light_sources.fixed_light_source.iter().enumerate() {
            let path = format!("generalDefinitions.lightSources.fixedLightSource[{}]", i);

            if source.id.is_empty() {
                result.add(ValidationError::error(
                    format!("{}.id", path),
                    "LS_001",
                    "Fixed light source ID is required",
                ));
            }
        }

        // Validate changeable light sources
        for (i, source) in light_sources.changeable_light_source.iter().enumerate() {
            let path = format!("generalDefinitions.lightSources.changeableLightSource[{}]", i);

            if source.id.is_empty() {
                result.add(ValidationError::error(
                    format!("{}.id", path),
                    "LS_002",
                    "Changeable light source ID is required",
                ));
            }
        }
    }
}

fn validate_emitters(product: &GldfProduct, result: &mut ValidationResult) {
    if let Some(ref emitters) = product.general_definitions.emitters {
        for (i, emitter) in emitters.emitter.iter().enumerate() {
            let path = format!("generalDefinitions.emitters[{}]", i);

            if emitter.id.is_empty() {
                result.add(ValidationError::error(
                    format!("{}.id", path),
                    "EMIT_001",
                    "Emitter ID is required",
                ));
            }
        }
    }
}

fn validate_variants(product: &GldfProduct, result: &mut ValidationResult) {
    if let Some(ref variants) = product.product_definitions.variants {
        if variants.variant.is_empty() {
            result.add(ValidationError::warning(
                "productDefinitions.variants",
                "VAR_001",
                "No variants defined - consider adding at least one variant",
            ));
        }

        for (i, variant) in variants.variant.iter().enumerate() {
            let path = format!("productDefinitions.variants[{}]", i);

            if variant.id.is_empty() {
                result.add(ValidationError::error(
                    format!("{}.id", path),
                    "VAR_002",
                    "Variant ID is required",
                ));
            }
        }
    } else {
        result.add(ValidationError::warning(
            "productDefinitions.variants",
            "VAR_003",
            "No variants section - consider adding variants",
        ));
    }
}

fn validate_id_uniqueness(product: &GldfProduct, result: &mut ValidationResult) {
    // Check file ID uniqueness
    let mut file_ids = HashSet::new();
    for file in &product.general_definitions.files.file {
        if !file.id.is_empty() && !file_ids.insert(&file.id) {
            result.add(ValidationError::error(
                format!("generalDefinitions.files.{}", file.id),
                "UNIQUE_001",
                format!("Duplicate file ID: '{}'", file.id),
            ));
        }
    }

    // Check variant ID uniqueness
    if let Some(ref variants) = product.product_definitions.variants {
        let mut variant_ids = HashSet::new();
        for variant in &variants.variant {
            if !variant.id.is_empty() && !variant_ids.insert(&variant.id) {
                result.add(ValidationError::error(
                    format!("productDefinitions.variants.{}", variant.id),
                    "UNIQUE_002",
                    format!("Duplicate variant ID: '{}'", variant.id),
                ));
            }
        }
    }

    // Check photometry ID uniqueness
    if let Some(ref photometries) = product.general_definitions.photometries {
        let mut phot_ids = HashSet::new();
        for photometry in &photometries.photometry {
            if !photometry.id.is_empty() && !phot_ids.insert(&photometry.id) {
                result.add(ValidationError::error(
                    format!("generalDefinitions.photometries.{}", photometry.id),
                    "UNIQUE_003",
                    format!("Duplicate photometry ID: '{}'", photometry.id),
                ));
            }
        }
    }

    // Check emitter ID uniqueness
    if let Some(ref emitters) = product.general_definitions.emitters {
        let mut emitter_ids = HashSet::new();
        for emitter in &emitters.emitter {
            if !emitter.id.is_empty() && !emitter_ids.insert(&emitter.id) {
                result.add(ValidationError::error(
                    format!("generalDefinitions.emitters.{}", emitter.id),
                    "UNIQUE_004",
                    format!("Duplicate emitter ID: '{}'", emitter.id),
                ));
            }
        }
    }
}

impl GldfProduct {
    /// Validates this GLDF product.
    ///
    /// # Arguments
    /// * `embedded_files` - Map of file IDs to their binary content
    ///
    /// # Returns
    /// A `ValidationResult` containing all validation issues.
    pub fn validate(&self, embedded_files: &HashMap<String, Vec<u8>>) -> ValidationResult {
        validate_gldf(self, embedded_files)
    }

    /// Validates this GLDF product without checking embedded files.
    ///
    /// This is useful when you only want to validate the structure.
    pub fn validate_structure(&self) -> ValidationResult {
        validate_gldf(self, &HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_creation() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid());
        assert!(!result.has_errors());

        result.add(ValidationError::error("test", "TEST_001", "Test error"));
        assert!(!result.is_valid());
        assert!(result.has_errors());
    }

    #[test]
    fn test_validation_levels() {
        let mut result = ValidationResult::new();
        result.add(ValidationError::error("a", "E001", "Error"));
        result.add(ValidationError::warning("b", "W001", "Warning"));
        result.add(ValidationError::info("c", "I001", "Info"));

        let (errors, warnings, info) = result.count_by_level();
        assert_eq!(errors, 1);
        assert_eq!(warnings, 1);
        assert_eq!(info, 1);
    }

    #[test]
    fn test_default_product_validation() {
        let product = GldfProduct::default();
        let result = product.validate_structure();

        // Default product should have some errors (missing required fields)
        assert!(result.has_errors());
    }
}
