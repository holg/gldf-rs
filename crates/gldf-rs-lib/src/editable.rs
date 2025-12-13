//! Editable GLDF wrapper for editing and saving GLDF files.
//!
//! This module provides `EditableGldf`, a wrapper around `GldfProduct` that adds:
//! - Binary file management (embedded files like LDT, IES, images, L3D)
//! - Undo/redo history
//! - Modification tracking
//! - Save/export capabilities

use crate::gldf::GldfProduct;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

/// Maximum number of undo/redo snapshots to keep
const MAX_HISTORY_SIZE: usize = 50;

/// A snapshot of the GLDF product state for undo/redo
#[derive(Clone)]
struct GldfSnapshot {
    product_json: String,
}

/// An editable GLDF document with binary file management and undo/redo support.
///
/// `EditableGldf` wraps a `GldfProduct` and provides:
/// - Storage for embedded binary files (photometry, images, 3D models)
/// - Undo/redo history
/// - Modification tracking
/// - Save to GLDF file (ZIP archive)
///
/// # Example
/// ```rust,no_run
/// use gldf_rs::EditableGldf;
///
/// // Load an existing GLDF file
/// let mut editable = EditableGldf::from_gldf("product.gldf").unwrap();
///
/// // Make changes
/// editable.product.header.author = "New Author".to_string();
/// editable.mark_modified();
///
/// // Save changes
/// editable.save_to_file("product_modified.gldf").unwrap();
/// ```
#[derive(Clone)]
pub struct EditableGldf {
    /// The GLDF product data
    pub product: GldfProduct,

    /// Embedded binary files, keyed by file ID
    /// Maps file_id -> binary content
    pub embedded_files: HashMap<String, Vec<u8>>,

    /// Undo history (past states)
    history: Vec<GldfSnapshot>,

    /// Current position in history (for redo support)
    history_index: usize,

    /// Whether the document has been modified since last save
    is_modified: bool,

    /// Original file path (if loaded from file)
    original_path: Option<String>,
}

impl Default for EditableGldf {
    fn default() -> Self {
        Self::new()
    }
}

impl EditableGldf {
    /// Creates a new empty `EditableGldf` with default values.
    ///
    /// Use this for creating a new GLDF from scratch or from a template.
    pub fn new() -> Self {
        Self {
            product: GldfProduct::default(),
            embedded_files: HashMap::new(),
            history: Vec::new(),
            history_index: 0,
            is_modified: false,
            original_path: None,
        }
    }

    /// Creates an `EditableGldf` from an existing `GldfProduct`.
    ///
    /// Note: This does not load embedded files. Use `from_gldf` or `from_buf`
    /// to load a complete GLDF with embedded files.
    pub fn from_product(product: GldfProduct) -> Self {
        Self {
            product,
            embedded_files: HashMap::new(),
            history: Vec::new(),
            history_index: 0,
            is_modified: false,
            original_path: None,
        }
    }

    /// Loads an `EditableGldf` from a GLDF file path.
    ///
    /// This loads both the product.xml and all embedded binary files.
    pub fn from_gldf(path: &str) -> Result<Self> {
        let file_buf = std::fs::read(path).context("Failed to read GLDF file")?;
        let mut editable = Self::from_buf(file_buf)?;
        editable.original_path = Some(path.to_string());
        editable.product.path = path.to_string();
        Ok(editable)
    }

    /// Loads an `EditableGldf` from a buffer (useful for WASM).
    ///
    /// This loads both the product.xml and all embedded binary files.
    pub fn from_buf(gldf_buf: Vec<u8>) -> Result<Self> {
        let zip_buf = Cursor::new(gldf_buf);
        let mut zip = ZipArchive::new(zip_buf).context("Failed to open GLDF as ZIP archive")?;

        // Load product.xml
        let mut xmlfile = zip
            .by_name("product.xml")
            .context("product.xml not found in GLDF archive")?;
        let mut xml_str = String::new();
        xmlfile
            .read_to_string(&mut xml_str)
            .context("Failed to read product.xml")?;
        let product =
            GldfProduct::from_xml(&xml_str).context("Failed to parse product.xml")?;
        drop(xmlfile);

        // Load all embedded files
        let mut embedded_files = HashMap::new();

        // Build a map of file_name -> file_id from the product definitions
        let mut filename_to_id: HashMap<String, String> = HashMap::new();
        for file_def in &product.general_definitions.files.file {
            if file_def.type_attr != "url" {
                // Build the expected path in the ZIP
                let zip_path = Self::get_zip_path_for_file(&file_def.content_type, &file_def.file_name);
                filename_to_id.insert(zip_path, file_def.id.clone());
            }
        }

        // Extract all files from the ZIP
        for i in 0..zip.len() {
            if let Ok(mut file) = zip.by_index(i) {
                if file.is_file() {
                    let file_name = file.name().to_string();
                    // Skip product.xml
                    if file_name == "product.xml" {
                        continue;
                    }

                    let mut buf = Vec::new();
                    if file.read_to_end(&mut buf).is_ok() {
                        // Try to find the file ID
                        if let Some(file_id) = filename_to_id.get(&file_name) {
                            embedded_files.insert(file_id.clone(), buf);
                        } else {
                            // Store with path as key if no ID mapping found
                            embedded_files.insert(file_name, buf);
                        }
                    }
                }
            }
        }

        Ok(Self {
            product,
            embedded_files,
            history: Vec::new(),
            history_index: 0,
            is_modified: false,
            original_path: None,
        })
    }

    /// Loads an `EditableGldf` from a JSON string.
    ///
    /// Note: This only loads the product data, not embedded files.
    pub fn from_json(json_str: &str) -> Result<Self> {
        let product = GldfProduct::from_json(json_str)?;
        Ok(Self::from_product(product))
    }

    /// Gets the ZIP path for a file based on its content type and filename.
    fn get_zip_path_for_file(content_type: &str, file_name: &str) -> String {
        let folder = match content_type {
            ct if ct.starts_with("ldc") => "ldc",
            ct if ct.starts_with("geo") => "geo",
            ct if ct.starts_with("image") => "image",
            ct if ct.starts_with("document") => "doc",
            ct if ct.starts_with("spectrum") => "spectrum",
            ct if ct.starts_with("sensor") => "sensor",
            ct if ct.starts_with("symbol") => "symbol",
            _ => "other",
        };
        format!("{}/{}", folder, file_name)
    }

    // ==================== Binary File Management ====================

    /// Adds or replaces an embedded binary file.
    ///
    /// # Arguments
    /// * `id` - The file ID (must match a file definition in the product)
    /// * `data` - The binary content
    pub fn add_embedded_file(&mut self, id: &str, data: Vec<u8>) {
        self.embedded_files.insert(id.to_string(), data);
        self.is_modified = true;
    }

    /// Removes an embedded binary file.
    pub fn remove_embedded_file(&mut self, id: &str) -> Option<Vec<u8>> {
        self.is_modified = true;
        self.embedded_files.remove(id)
    }

    /// Gets an embedded binary file by ID.
    pub fn get_embedded_file(&self, id: &str) -> Option<&[u8]> {
        self.embedded_files.get(id).map(|v| v.as_slice())
    }

    /// Checks if an embedded file exists.
    pub fn has_embedded_file(&self, id: &str) -> bool {
        self.embedded_files.contains_key(id)
    }

    /// Gets all embedded file IDs.
    pub fn embedded_file_ids(&self) -> Vec<&str> {
        self.embedded_files.keys().map(|s| s.as_str()).collect()
    }

    // ==================== Undo/Redo ====================

    /// Creates a checkpoint for undo/redo.
    ///
    /// Call this before making changes that should be undoable.
    pub fn checkpoint(&mut self) {
        // Serialize current state
        if let Ok(json) = self.product.to_json() {
            let snapshot = GldfSnapshot { product_json: json };

            // Truncate any redo history
            if self.history_index < self.history.len() {
                self.history.truncate(self.history_index);
            }

            // Add new snapshot
            self.history.push(snapshot);
            self.history_index = self.history.len();

            // Limit history size
            if self.history.len() > MAX_HISTORY_SIZE {
                self.history.remove(0);
                self.history_index = self.history.len();
            }
        }
    }

    /// Undoes the last change.
    ///
    /// Returns `true` if undo was successful, `false` if nothing to undo.
    pub fn undo(&mut self) -> bool {
        if self.history_index > 0 {
            // Save current state for redo if we're at the end
            if self.history_index == self.history.len() {
                if let Ok(json) = self.product.to_json() {
                    self.history.push(GldfSnapshot { product_json: json });
                }
            }

            self.history_index -= 1;
            if let Some(snapshot) = self.history.get(self.history_index) {
                if let Ok(product) = GldfProduct::from_json(&snapshot.product_json) {
                    self.product = product;
                    self.is_modified = true;
                    return true;
                }
            }
        }
        false
    }

    /// Redoes the last undone change.
    ///
    /// Returns `true` if redo was successful, `false` if nothing to redo.
    pub fn redo(&mut self) -> bool {
        if self.history_index < self.history.len().saturating_sub(1) {
            self.history_index += 1;
            if let Some(snapshot) = self.history.get(self.history_index) {
                if let Ok(product) = GldfProduct::from_json(&snapshot.product_json) {
                    self.product = product;
                    self.is_modified = true;
                    return true;
                }
            }
        }
        false
    }

    /// Returns `true` if there are changes that can be undone.
    pub fn can_undo(&self) -> bool {
        self.history_index > 0
    }

    /// Returns `true` if there are changes that can be redone.
    pub fn can_redo(&self) -> bool {
        self.history_index < self.history.len().saturating_sub(1)
    }

    /// Clears the undo/redo history.
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.history_index = 0;
    }

    // ==================== Modification Tracking ====================

    /// Returns `true` if the document has been modified since last save.
    pub fn is_modified(&self) -> bool {
        self.is_modified
    }

    /// Marks the document as modified.
    pub fn mark_modified(&mut self) {
        self.is_modified = true;
    }

    /// Marks the document as unmodified (e.g., after saving).
    pub fn mark_saved(&mut self) {
        self.is_modified = false;
    }

    /// Gets the original file path (if loaded from a file).
    pub fn original_path(&self) -> Option<&str> {
        self.original_path.as_deref()
    }

    // ==================== Save/Export ====================

    /// Saves the GLDF to a file.
    ///
    /// Creates a ZIP archive with product.xml and all embedded files.
    pub fn save_to_file(&mut self, path: &str) -> Result<()> {
        let buf = self.save_to_buf()?;
        std::fs::write(path, buf).context("Failed to write GLDF file")?;
        self.original_path = Some(path.to_string());
        self.is_modified = false;
        Ok(())
    }

    /// Saves the GLDF to a buffer (useful for WASM downloads).
    ///
    /// Returns the ZIP archive as a byte vector.
    pub fn save_to_buf(&self) -> Result<Vec<u8>> {
        let cursor = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(cursor);

        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        // Write product.xml
        let xml = self.product.to_xml().context("Failed to serialize product to XML")?;
        zip.start_file("product.xml", options)
            .context("Failed to start product.xml in ZIP")?;
        zip.write_all(xml.as_bytes())
            .context("Failed to write product.xml")?;

        // Write embedded files
        for file_def in &self.product.general_definitions.files.file {
            // Skip URL files - they're not embedded
            if file_def.type_attr == "url" {
                continue;
            }

            // Get the embedded content
            if let Some(content) = self.embedded_files.get(&file_def.id) {
                let zip_path = Self::get_zip_path_for_file(&file_def.content_type, &file_def.file_name);
                zip.start_file(&zip_path, options)
                    .with_context(|| format!("Failed to start {} in ZIP", zip_path))?;
                zip.write_all(content)
                    .with_context(|| format!("Failed to write {} to ZIP", zip_path))?;
            }
        }

        let cursor = zip.finish().context("Failed to finalize ZIP archive")?;
        Ok(cursor.into_inner())
    }

    /// Exports the product data as JSON string.
    pub fn to_json(&self) -> Result<String> {
        self.product.to_json()
    }

    /// Exports the product data as pretty-printed JSON string.
    pub fn to_pretty_json(&self) -> Result<String> {
        self.product.to_pretty_json()
    }

    /// Exports the product data as XML string.
    pub fn to_xml(&self) -> Result<String> {
        self.product.to_xml()
    }

    // ==================== Utilities ====================

    /// Gets statistics about the GLDF.
    pub fn stats(&self) -> EditableGldfStats {
        let file_count = self.product.general_definitions.files.file.len();
        let embedded_count = self.embedded_files.len();
        let total_embedded_size: usize = self.embedded_files.values().map(|v| v.len()).sum();

        let variant_count = self
            .product
            .product_definitions
            .variants
            .as_ref()
            .map(|v| v.variant.len())
            .unwrap_or(0);

        let light_source_count = self
            .product
            .general_definitions
            .light_sources
            .as_ref()
            .map(|ls| {
                ls.fixed_light_source.len() + ls.changeable_light_source.len()
            })
            .unwrap_or(0);

        EditableGldfStats {
            file_definition_count: file_count,
            embedded_file_count: embedded_count,
            total_embedded_size,
            variant_count,
            light_source_count,
            is_modified: self.is_modified,
            history_depth: self.history.len(),
        }
    }
}

/// Statistics about an `EditableGldf`.
#[derive(Debug, Clone)]
pub struct EditableGldfStats {
    /// Number of file definitions in the product
    pub file_definition_count: usize,
    /// Number of actually embedded binary files
    pub embedded_file_count: usize,
    /// Total size of embedded files in bytes
    pub total_embedded_size: usize,
    /// Number of product variants
    pub variant_count: usize,
    /// Number of light sources (fixed + changeable)
    pub light_source_count: usize,
    /// Whether the document has unsaved changes
    pub is_modified: bool,
    /// Number of undo steps available
    pub history_depth: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_editable() {
        let editable = EditableGldf::new();
        assert!(!editable.is_modified());
        assert!(editable.embedded_files.is_empty());
        assert!(!editable.can_undo());
        assert!(!editable.can_redo());
    }

    #[test]
    fn test_embedded_file_management() {
        let mut editable = EditableGldf::new();

        // Add a file
        editable.add_embedded_file("test_file", vec![1, 2, 3, 4]);
        assert!(editable.has_embedded_file("test_file"));
        assert_eq!(editable.get_embedded_file("test_file"), Some(&[1, 2, 3, 4][..]));
        assert!(editable.is_modified());

        // Remove the file
        let removed = editable.remove_embedded_file("test_file");
        assert_eq!(removed, Some(vec![1, 2, 3, 4]));
        assert!(!editable.has_embedded_file("test_file"));
    }

    #[test]
    fn test_checkpoint_and_undo() {
        let mut editable = EditableGldf::new();
        editable.product.header.author = "Original".to_string();

        // Create checkpoint
        editable.checkpoint();

        // Make change
        editable.product.header.author = "Modified".to_string();

        // Undo
        assert!(editable.undo());
        assert_eq!(editable.product.header.author, "Original");

        // Redo
        assert!(editable.redo());
        assert_eq!(editable.product.header.author, "Modified");
    }

    #[test]
    fn test_stats() {
        let editable = EditableGldf::new();
        let stats = editable.stats();
        assert_eq!(stats.file_definition_count, 0);
        assert_eq!(stats.embedded_file_count, 0);
        assert_eq!(stats.variant_count, 0);
    }
}
