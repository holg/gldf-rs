//! Editable GLDF wrapper for editing and saving GLDF files.
//!
//! This module provides `EditableGldf`, a wrapper around `GldfProduct` that adds:
//! - Binary file management (embedded files like LDT, IES, images, L3D)
//! - Undo/redo history
//! - Modification tracking
//! - Save/export capabilities

use crate::gldf::general_definitions::files::File;
#[cfg(feature = "eulumdat")]
use crate::gldf::general_definitions::photometries::DescriptivePhotometry;
use crate::gldf::general_definitions::photometries::{
    Photometry, PhotometryFileReference, Spectrum, SpectrumFileReference,
};
use crate::gldf::header::{Locale, LocaleFoo};
use crate::gldf::product_definitions::Variant;
use crate::gldf::GldfProduct;
use anyhow::{anyhow, Context, Result};
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
/// Options for [`EditableGldf::save_to_buf_with_options`].
///
/// Default = canonical GLDF, no consumer-specific quirks. Each flag
/// opts into one independent transform that's applied to a clone of
/// the in-memory state; `self` is never mutated.
///
/// The transforms compose in the order listed in this struct: TM-33
/// merge runs first, then inline-spectrum, then DIALux sanitisation.
/// Conflicts are resolved by the rules in the field docs (e.g.
/// `merge_into_tm33 = true` implicitly drops the SPD before
/// `inline_spectrum` even gets a chance to inline it).
///
/// Naming convention: anything DIALux-specific lives behind
/// [`crate::dialux_compat`] so a future "make the output compatible
/// with consumer X" toggle can sit alongside it without contaminating
/// real GLDF validation.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct ExportOptions {
    /// Apply DIALux-import compatibility tweaks. Currently strips
    /// `#` comment lines from every `spectrum/text` payload (DIALux
    /// silently rejects the whole archive otherwise — see
    /// [`crate::dialux_compat`]). Real-GLDF readers tolerate the
    /// comments, so this is off by default.
    pub dialux_compat: bool,

    /// Inline every external `spectrum/text` SPD into
    /// `<Spectrum><Intensity wavelength=…>…</Intensity></Spectrum>`
    /// inside `product.xml`, dropping the external `.spd` file
    /// entries. When all spectra are already inline this is a no-op
    /// (the caller is expected to clear the flag and surface a note
    /// to the user). Implemented in
    /// [`crate::export::inline_spectrum::apply_inline_spectrum`].
    pub inline_spectrum: bool,

    /// Merge each photometry that has a linked spectrum into a
    /// single TM-33 (`ldc/iesxml`) file, replacing the `.ldt`/`.ies`
    /// photometry and `.spd` spectrum entries with one XML payload.
    /// Photometries without a linked spectrum are left alone. Only
    /// available when the lib is compiled with `feature = "eulumdat"`
    /// (the TM-33 writer lives behind that flag); off by default and
    /// silently skipped in eulumdat-less builds.
    /// Implemented in
    /// [`crate::export::tm33_merge::apply_tm33_merge`].
    pub merge_into_tm33: bool,
}

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
        let product = GldfProduct::from_xml(&xml_str).context("Failed to parse product.xml")?;
        drop(xmlfile);

        // Load all embedded files
        let mut embedded_files = HashMap::new();

        // Build a map of file_name -> file_id from the product definitions
        let mut filename_to_id: HashMap<String, String> = HashMap::new();
        for file_def in &product.general_definitions.files.file {
            if file_def.type_attr != "url" {
                // Build the expected path in the ZIP
                let zip_path =
                    Self::get_zip_path_for_file(&file_def.content_type, &file_def.file_name);
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
    ///
    /// `pub(crate)` so other lib entry points (e.g. `GldfProduct::load_gldf_from_buf_all`)
    /// can resolve a zip-path back to a `<File>` id during load.
    pub(crate) fn get_zip_path_for_file(content_type: &str, file_name: &str) -> String {
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

    // ==================== High-level "drop & link" builders ====================
    //
    // These bundle the two halves a referenced file needs: the `<File>` entry
    // (+ definition) in product.xml AND the embedded bytes. Each generates a
    // unique id, embeds the bytes, and wires the model references — so a caller
    // (e.g. a GLDF-builder UI) just drops a file and gets a usable, standard
    // GLDF back.

    /// Embed a photometry file (`.ies` / `.ldt`) and create its `<File>` +
    /// `<Photometry>`. Returns the new photometry id (reference it from an
    /// emitter via `PhotometryReference`).
    ///
    /// `content_type` is `ldc/ies` for `.ies`, else `ldc/eulumdat`.
    pub fn add_photometry_file(&mut self, bytes: Vec<u8>, filename: &str) -> Result<String> {
        let is_ies = filename.to_lowercase().ends_with(".ies");
        let content_type = if is_ies { "ldc/ies" } else { "ldc/eulumdat" };

        let file_id = self.product.generate_unique_id("photometryFile");
        self.product.add_file(File {
            id: file_id.clone(),
            content_type: content_type.to_string(),
            type_attr: "localFileName".to_string(),
            language: String::new(),
            file_name: filename.to_string(),
        })?;

        let photometry_id = self.product.generate_unique_id("photometry");
        self.product.add_photometry(Photometry {
            id: photometry_id.clone(),
            photometry_file_reference: Some(PhotometryFileReference {
                file_id: file_id.clone(),
            }),
            descriptive_photometry: None,
        })?;

        self.add_embedded_file(&file_id, bytes);
        Ok(photometry_id)
    }

    /// Parse photometric bytes (.ldt or .ies) and populate the named
    /// Photometry entry's `<DescriptivePhotometry>` block with the
    /// fields derivable from the file. Only writes fields that were
    /// previously `None`; existing user overrides are preserved.
    ///
    /// Returns `Ok(true)` if any fields were populated, `Ok(false)` if
    /// the file parsed but yielded no usable summary (very rare —
    /// truly empty LDT), and `Err` only when parsing itself failed.
    /// Callers that want robust file-embed-even-on-summary-failure
    /// flows should match on the error and continue.
    #[cfg(feature = "eulumdat")]
    pub fn fill_descriptive_photometry(
        &mut self,
        photometry_id: &str,
        bytes: &[u8],
        filename: &str,
    ) -> Result<bool> {
        use eulumdat::{Eulumdat, IesParser, PhotometricSummary};

        // Decode bytes — both parsers accept the raw text. We prefer
        // UTF-8 then fall back to Latin-1 since EULUMDAT/IES historically
        // shipped in ISO-8859-1.
        let text = match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => bytes.iter().map(|&b| b as char).collect::<String>(),
        };

        let is_ies = filename.to_lowercase().ends_with(".ies");
        let ldt = if is_ies {
            IesParser::parse(&text).context("parse IES")?
        } else {
            Eulumdat::parse(&text).context("parse EULUMDAT")?
        };
        let summary = PhotometricSummary::from_eulumdat(&ldt);

        // Find the photometry entry to mutate. If it's missing the
        // caller dispatched a stale id; treat as a no-op rather than an
        // error so the file-embed path stays robust.
        let photometries = match self.product.general_definitions.photometries.as_mut() {
            Some(p) => p,
            None => return Ok(false),
        };
        let entry = match photometries
            .photometry
            .iter_mut()
            .find(|p| p.id == photometry_id)
        {
            Some(e) => e,
            None => return Ok(false),
        };

        let dp = entry
            .descriptive_photometry
            .get_or_insert_with(DescriptivePhotometry::default);

        let mut wrote = false;

        // Helper: round to N decimals using banker's rounding; keeps
        // serialised numbers tidy without producing 0.299999…-style
        // float drift.
        let round_to = |v: f64, decimals: u32| -> f64 {
            let factor = 10f64.powi(decimals as i32);
            (v * factor).round_ties_even() / factor
        };

        // Ratio fields are 0..1 per rc-3 XSD (xs:double with
        // maxInclusive=1). EULUMDAT supplies them as percentages, so
        // divide on the way in.
        let pct_to_ratio = |v: f64| (v / 100.0).clamp(0.0, 1.0);

        // light_output_ratio (LOR) — luminaire flux / lamp flux.
        if dp.light_output_ratio.is_none() {
            dp.light_output_ratio = Some(round_to(pct_to_ratio(summary.lor), 4));
            wrote = true;
        }
        // downward_light_output_ratio (DLOR).
        if dp.downward_light_output_ratio.is_none() {
            dp.downward_light_output_ratio = Some(round_to(pct_to_ratio(summary.dlor), 4));
            wrote = true;
        }
        // upward_light_output_ratio (ULOR).
        if dp.upward_light_output_ratio.is_none() {
            dp.upward_light_output_ratio = Some(round_to(pct_to_ratio(summary.ulor), 4));
            wrote = true;
        }
        // luminous_efficacy (lm/W) — NOT a ratio, no rescale.
        if dp.luminous_efficacy.is_none() && summary.luminaire_efficacy.is_finite() {
            dp.luminous_efficacy = Some(round_to(summary.luminaire_efficacy, 2));
            wrote = true;
        }
        // downward_flux_fraction — sourced directly from the parsed LDT
        // (line 22 in EULUMDAT). LDT stores it as a percentage, GLDF
        // expects 0..1.
        if dp.downward_flux_fraction.is_none() && ldt.downward_flux_fraction > 0.0 {
            dp.downward_flux_fraction =
                Some(round_to(pct_to_ratio(ldt.downward_flux_fraction), 4));
            wrote = true;
        }
        // cie_flux_code — Display impl rounds to bankers' integers.
        if dp.cie_flux_code.is_none() {
            dp.cie_flux_code = Some(summary.cie_flux_codes.to_string());
            wrote = true;
        }

        if wrote {
            self.is_modified = true;
        }
        Ok(wrote)
    }

    /// Replace the file backing an existing `<Photometry>` entry with new
    /// bytes — same photometry id, so every `<PhotometryReference>` on an
    /// emitter stays valid. Removes the old `<File>` entry and its embedded
    /// bytes; embeds the new file under a fresh file id.
    ///
    /// Use this when a placeholder photometry (e.g. one auto-created on the
    /// start drop with weak IES metadata) needs to be swapped for a curated
    /// upload without disturbing the rest of the product.
    ///
    /// # Errors
    /// - `photometry_id` doesn't exist.
    /// - The existing photometry has no `<PhotometryFileReference>` (i.e.
    ///   it's an inline / descriptive-only photometry — nothing to replace).
    pub fn replace_photometry_file(
        &mut self,
        photometry_id: &str,
        bytes: Vec<u8>,
        filename: &str,
    ) -> Result<()> {
        let is_ies = filename.to_lowercase().ends_with(".ies");
        let content_type = if is_ies { "ldc/ies" } else { "ldc/eulumdat" };

        // 1) Find the existing photometry + its old file id.
        let photometries = self
            .product
            .general_definitions
            .photometries
            .as_mut()
            .ok_or_else(|| anyhow!("No photometries to replace"))?;
        let photometry = photometries
            .photometry
            .iter_mut()
            .find(|p| p.id == photometry_id)
            .ok_or_else(|| anyhow!("Photometry with id '{}' not found", photometry_id))?;
        let old_file_id = photometry
            .photometry_file_reference
            .as_ref()
            .ok_or_else(|| anyhow!("Photometry '{}' has no file reference to replace", photometry_id))?
            .file_id
            .clone();

        // 2) Mint a new file id and `<File>` entry.
        let new_file_id = self.product.generate_unique_id("photometryFile");
        self.product.add_file(File {
            id: new_file_id.clone(),
            content_type: content_type.to_string(),
            type_attr: "localFileName".to_string(),
            language: String::new(),
            file_name: filename.to_string(),
        })?;

        // 3) Point the photometry at the new file id (id unchanged).
        if let Some(p) = self
            .product
            .general_definitions
            .photometries
            .as_mut()
            .and_then(|ps| ps.photometry.iter_mut().find(|p| p.id == photometry_id))
        {
            p.photometry_file_reference = Some(PhotometryFileReference {
                file_id: new_file_id.clone(),
            });
        }

        // 4) Remove the old `<File>` entry + its embedded bytes.
        //    `remove_file` errors if the id is unknown; tolerate that (the
        //    old file may already have been pruned by hand).
        let _ = self.product.remove_file(&old_file_id);
        self.embedded_files.remove(&old_file_id);

        // 5) Embed the new bytes under the new file id.
        self.add_embedded_file(&new_file_id, bytes);
        Ok(())
    }

    /// Embed a spectral power distribution file (`.spd`, contentType
    /// `spectrum/text`), create its `<File>` + `<Spectrum>` (with
    /// `SpectrumFileReference`), and link it to the named light source via
    /// `SpectrumReference`. Returns the new spectrum id.
    ///
    /// This is the standard, schema-conformant way to attach spectral data —
    /// no inline `<Intensity wavelength=…>` and no forked/bundled schema.
    ///
    /// # Errors
    /// `light_source_id` must name an existing Fixed/Changeable light source.
    pub fn add_spectrum_file(
        &mut self,
        bytes: Vec<u8>,
        filename: &str,
        light_source_id: &str,
    ) -> Result<String> {
        // Validate the target light source up front so we don't half-build.
        let known = self
            .product
            .general_definitions
            .light_sources
            .as_ref()
            .map(|ls| {
                ls.fixed_light_source.iter().any(|s| s.id == light_source_id)
                    || ls
                        .changeable_light_source
                        .iter()
                        .any(|s| s.id == light_source_id)
            })
            .unwrap_or(false);
        if !known {
            return Err(anyhow!(
                "Cannot link spectrum: no light source with ID '{}'",
                light_source_id
            ));
        }

        let file_id = self.product.generate_unique_id("spectrumFile");
        self.product.add_file(File {
            id: file_id.clone(),
            content_type: "spectrum/text".to_string(),
            type_attr: "localFileName".to_string(),
            language: String::new(),
            file_name: filename.to_string(),
        })?;

        let spectrum_id = self.product.generate_unique_id("spectrum");
        self.product.add_spectrum(Spectrum {
            id: spectrum_id.clone(),
            spectrum_file_reference: Some(SpectrumFileReference {
                file_id: file_id.clone(),
            }),
            intensity: Vec::new(),
        })?;

        self.product
            .add_spectrum_reference_to_light_source(light_source_id, &spectrum_id)?;

        self.add_embedded_file(&file_id, bytes);
        Ok(spectrum_id)
    }

    /// Embed a sensor data sheet (.sensxml or .sensldt) and link it via
    /// a fresh `<Sensor>` entry under `GeneralDefinitions/Sensors`.
    ///
    /// Mirrors the photometry / spectrum file-drop helpers: registers a
    /// `<File>` with the right content type (`sensor/sensxml` for `.xml`
    /// or `.sensxml`; `sensor/sensldt` for everything else including
    /// `.ldt` and `.sensldt`), mints a `<Sensor id="…">` with a
    /// `SensorFileReference` pointing at it, and embeds the bytes.
    ///
    /// Returns the sensor id so callers (wizard, Files-page drop
    /// handler) can rename it semantically. Detector characteristic /
    /// detection method / detector type enum fields default to empty —
    /// the editor's Sensors page lets the user tick them later.
    pub fn add_sensor_file(&mut self, bytes: Vec<u8>, filename: &str) -> Result<String> {
        use crate::gldf::general_definitions::sensors::{Sensor, SensorFileReference};
        let lower = filename.to_lowercase();
        let is_xml = lower.ends_with(".xml") || lower.ends_with(".sensxml");
        let content_type = if is_xml {
            "sensor/sensxml"
        } else {
            "sensor/sensldt"
        };

        let file_id = self.product.generate_unique_id("sensorFile");
        self.product.add_file(File {
            id: file_id.clone(),
            content_type: content_type.to_string(),
            type_attr: "localFileName".to_string(),
            language: String::new(),
            file_name: filename.to_string(),
        })?;

        let sensor_id = self.product.generate_unique_id("sensor");
        let mut sensor = Sensor::default();
        sensor.id = sensor_id.clone();
        sensor.sensor_file_reference = Some(SensorFileReference {
            file_id: file_id.clone(),
        });
        self.product.add_sensor(sensor)?;

        self.add_embedded_file(&file_id, bytes);
        Ok(sensor_id)
    }

    /// Add a fully-specified `Variant`. If its `id` is empty, a unique one is
    /// generated. Returns the variant id.
    pub fn add_variant(&mut self, mut variant: Variant) -> Result<String> {
        if variant.id.is_empty() {
            variant.id = self.product.generate_unique_id("variant");
        }
        let id = variant.id.clone();
        self.product.add_variant(variant)?;
        self.is_modified = true;
        Ok(id)
    }

    /// Convenience: add a variant with just a display name (English locale).
    /// Returns the new variant id.
    pub fn add_variant_simple(&mut self, name: &str) -> Result<String> {
        let id = self.product.generate_unique_id("variant");
        self.product.add_variant(Variant {
            id: id.clone(),
            name: Some(LocaleFoo {
                locale: vec![Locale {
                    language: "en".to_string(),
                    value: name.to_string(),
                }],
            }),
            ..Default::default()
        })?;
        self.is_modified = true;
        Ok(id)
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
    /// Returns the ZIP archive as a byte vector. Uses default
    /// [`ExportOptions`] — i.e. canonical GLDF, no DIALux quirks.
    /// Call [`Self::save_to_buf_with_options`] to opt into the
    /// DIALux-compat / inline-spectrum / TM-33 transforms.
    pub fn save_to_buf(&self) -> Result<Vec<u8>> {
        self.save_to_buf_with_options(&ExportOptions::default())
    }

    /// Saves the GLDF to a buffer with explicit [`ExportOptions`].
    ///
    /// Flags compose in the order: TM-33 merge → inline-spectrum →
    /// DIALux-compat sanitisation. Each transform clones what it
    /// needs; `self` is never mutated. The default (all flags off)
    /// behaves identically to [`Self::save_to_buf`].
    pub fn save_to_buf_with_options(&self, opts: &ExportOptions) -> Result<Vec<u8>> {
        // Inline-spectrum + TM-33 transforms mutate the product
        // model itself, so they need owned working copies. DIALux
        // compat only mutates byte payloads, so it can stay on a
        // cow. We always emit from `product_ref` + `embedded`.
        let mut product_owned: Option<GldfProduct> = None;
        let mut embedded: std::borrow::Cow<'_, HashMap<String, Vec<u8>>> =
            std::borrow::Cow::Borrowed(&self.embedded_files);

        // TM-33 merge runs FIRST: it consumes the spectrum's SPD file
        // entirely. If both `merge_into_tm33` and `inline_spectrum`
        // are set, merge wins (the spectrum no longer exists as a
        // standalone entry by the time inline-spectrum walks).
        #[cfg(feature = "eulumdat")]
        if opts.merge_into_tm33 {
            let mut prod = product_owned.take().unwrap_or_else(|| self.product.clone());
            let mut owned_em = embedded.into_owned();
            let _outcome = crate::export::tm33_merge::apply_tm33_merge(
                &mut prod,
                &mut owned_em,
            )
            .context("apply TM-33 merge")?;
            product_owned = Some(prod);
            embedded = std::borrow::Cow::Owned(owned_em);
        }

        if opts.inline_spectrum {
            // Rewrite SpectrumFileReference → inline Intensity rows.
            // Drops the corresponding File entries + bytes.
            let mut prod = product_owned.take().unwrap_or_else(|| self.product.clone());
            let mut owned_em = embedded.into_owned();
            let _outcome = crate::export::inline_spectrum::apply_inline_spectrum(
                &mut prod,
                &mut owned_em,
            );
            product_owned = Some(prod);
            embedded = std::borrow::Cow::Owned(owned_em);
        }

        // `product_ref` and `embedded` are the bytes/model the rest
        // of this fn writes from. Avoid moving `product_owned` into
        // closures so subsequent transforms can keep reading.
        let product_ref: &GldfProduct = product_owned
            .as_ref()
            .unwrap_or(&self.product);

        if opts.dialux_compat {
            // Strip `#` comment lines from every spectrum/text payload.
            // DIALux silently aborts otherwise; see `dialux_compat`.
            let mut owned = embedded.into_owned();
            for file_def in &product_ref.general_definitions.files.file {
                if file_def.content_type != "spectrum/text" {
                    continue;
                }
                if let Some(bytes) = owned.get(&file_def.id) {
                    let text = String::from_utf8_lossy(bytes);
                    let sanitised = crate::dialux_compat::sanitize_spd(&text);
                    owned.insert(file_def.id.clone(), sanitised.into_bytes());
                }
            }
            embedded = std::borrow::Cow::Owned(owned);
        }

        let cursor = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(cursor);

        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        // Write product.xml
        let xml = product_ref
            .to_xml()
            .context("Failed to serialize product to XML")?;
        zip.start_file("product.xml", options)
            .context("Failed to start product.xml in ZIP")?;
        zip.write_all(xml.as_bytes())
            .context("Failed to write product.xml")?;

        // Write embedded files
        for file_def in &product_ref.general_definitions.files.file {
            // Skip URL files - they're not embedded
            if file_def.type_attr == "url" {
                continue;
            }

            // Get the embedded content
            if let Some(content) = embedded.get(&file_def.id) {
                let zip_path =
                    Self::get_zip_path_for_file(&file_def.content_type, &file_def.file_name);
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
            .map(|ls| ls.fixed_light_source.len() + ls.changeable_light_source.len())
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

    /// End-to-end: `inline_spectrum=true` rewrites the product.xml
    /// to embed `<Intensity>` rows and drops the `.spd` file from
    /// the zip. Locks in the slice-2 transform contract.
    #[test]
    fn inline_spectrum_export_drops_spd_and_inlines_intensities() {
        use crate::gldf::general_definitions::files::File as GldfFile;
        use crate::gldf::general_definitions::photometries::{
            Spectrum, SpectrumFileReference, Spectrums,
        };
        use std::io::Read;

        let mut e = EditableGldf::new();
        e.product.general_definitions.files.file.push(GldfFile {
            id: "spd1".into(),
            content_type: "spectrum/text".into(),
            type_attr: "localFileName".into(),
            language: String::new(),
            file_name: "spec.spd".into(),
        });
        e.product.general_definitions.spectrums = Some(Spectrums {
            spectrum: vec![Spectrum {
                id: "s1".into(),
                spectrum_file_reference: Some(SpectrumFileReference {
                    file_id: "spd1".into(),
                }),
                intensity: vec![],
            }],
        });
        e.embedded_files
            .insert("spd1".into(), b"380\t0.5\n385\t0.6\n".to_vec());

        let out = e
            .save_to_buf_with_options(&ExportOptions {
                inline_spectrum: true,
                ..ExportOptions::default()
            })
            .expect("inline export");

        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(out)).expect("zip");
        // SPD file gone
        assert!(zip.by_name("spectrum/spec.spd").is_err());
        // product.xml carries the inlined Intensity rows + no File entry
        let mut xml = String::new();
        zip.by_name("product.xml")
            .expect("product.xml")
            .read_to_string(&mut xml)
            .unwrap();
        assert!(
            xml.contains(r#"<Intensity wavelength="380">0.5</Intensity>"#),
            "expected inlined Intensity row, got: {xml}"
        );
        assert!(
            !xml.contains(r#"contentType="spectrum/text""#),
            "File entry should be gone: {xml}"
        );
    }

    /// `save_to_buf` (default options) leaves SPD bytes untouched —
    /// real GLDF is comment-tolerant. `save_to_buf_with_options` with
    /// `dialux_compat = true` strips `#` lines from every
    /// `spectrum/text` payload as it lands in the zip. Anchors both
    /// the "we don't mutate by default" promise and the DIALux fix.
    #[test]
    fn dialux_compat_export_strips_spd_comments_only_when_flagged() {
        use crate::gldf::general_definitions::files::File as GldfFile;
        use std::io::Read;

        let mut e = EditableGldf::new();
        e.product.general_definitions.files.file.push(GldfFile {
            id: "spd1".to_string(),
            content_type: "spectrum/text".to_string(),
            type_attr: "localFileName".to_string(),
            language: String::new(),
            file_name: "spec.spd".to_string(),
        });
        let dirty = b"# header\n380\t0.5\n385\t0.6\n";
        e.embedded_files
            .insert("spd1".to_string(), dirty.to_vec());

        let read_back = |buf: &[u8]| -> Vec<u8> {
            let mut zip = zip::ZipArchive::new(std::io::Cursor::new(buf.to_vec()))
                .expect("open zip");
            let mut f = zip.by_name("spectrum/spec.spd").expect("entry");
            let mut out = Vec::new();
            f.read_to_end(&mut out).unwrap();
            out
        };

        // Default = no transform, comments survive
        let default_out = e.save_to_buf().expect("default export");
        assert_eq!(read_back(&default_out), dirty);

        // dialux_compat = on → comments stripped
        let dialux_out = e
            .save_to_buf_with_options(&ExportOptions {
                dialux_compat: true,
                ..ExportOptions::default()
            })
            .expect("dialux export");
        assert_eq!(read_back(&dialux_out), b"380\t0.5\n385\t0.6\n");
    }

    #[test]
    fn test_embedded_file_management() {
        let mut editable = EditableGldf::new();

        // Add a file
        editable.add_embedded_file("test_file", vec![1, 2, 3, 4]);
        assert!(editable.has_embedded_file("test_file"));
        assert_eq!(
            editable.get_embedded_file("test_file"),
            Some(&[1, 2, 3, 4][..])
        );
        assert!(editable.is_modified());

        // Remove the file
        let removed = editable.remove_embedded_file("test_file");
        assert_eq!(removed, Some(vec![1, 2, 3, 4]));
        assert!(!editable.has_embedded_file("test_file"));
    }

    /// End-to-end: build a GLDF from an .ies, attach a .spd spectrum linked to
    /// the light source, add a second photometry + a variant, save, reload,
    /// and verify the standard-conformant structure (no forked schema).
    #[cfg(feature = "eulumdat")]
    #[test]
    fn test_add_spectrum_photometry_variant_roundtrip() {
        let ies = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/data/test.ies"),
        )
        .expect("test.ies fixture");

        // Base GLDF from one .ies (emits light source id "lightsource_1").
        let base = crate::ldt_to_gldf(&ies, "test.ies").expect("ldt_to_gldf");
        let mut editable = EditableGldf::from_product(base.gldf);
        for bf in base.files {
            if let (Some(id), Some(c)) = (bf.file_id, bf.content) {
                editable.add_embedded_file(&id, c);
            }
        }

        // Attach a spectral .spd linked to the light source.
        let spd = b"# minimal SPD\n380 0.0\n780 0.0\n".to_vec();
        let spectrum_id = editable
            .add_spectrum_file(spd.clone(), "test.spd", "lightsource_1")
            .expect("add_spectrum_file");

        // Linking to an unknown source must error (no half-build).
        assert!(editable
            .add_spectrum_file(spd, "x.spd", "no_such_source")
            .is_err());

        // Add a second photometry + a variant.
        let _phot2 = editable
            .add_photometry_file(ies.clone(), "second.ies")
            .expect("add_photometry_file");
        let variant_id = editable.add_variant_simple("Wide beam").expect("add_variant_simple");

        // Round-trip through the zip.
        let bytes = editable.save_to_buf().expect("save_to_buf");
        let reloaded = EditableGldf::from_buf(bytes.clone()).expect("reload");
        let p = &reloaded.product;

        // 1. Spectrum exists + file is embedded in the spectrum/ folder.
        let spectrums = p
            .general_definitions
            .spectrums
            .as_ref()
            .expect("Spectrums present");
        let spectrum = spectrums
            .spectrum
            .iter()
            .find(|s| s.id == spectrum_id)
            .expect("our spectrum present");
        let spd_file_id = &spectrum
            .spectrum_file_reference
            .as_ref()
            .expect("spectrum has a file reference")
            .file_id;
        let spd_file = p
            .general_definitions
            .files
            .file
            .iter()
            .find(|f| &f.id == spd_file_id)
            .expect("spectrum <File> present");
        assert_eq!(spd_file.content_type, "spectrum/text");
        // the bytes round-tripped into the zip under spectrum/
        {
            let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
            assert!(
                (0..zip.len()).any(|i| zip.by_index(i).unwrap().name().starts_with("spectrum/")),
                "spectrum/ folder present in the zip"
            );
        }

        // 2. The light source carries the SpectrumReference to our spectrum.
        let ls = p.general_definitions.light_sources.as_ref().unwrap();
        let fixed = ls
            .fixed_light_source
            .iter()
            .find(|s| s.id == "lightsource_1")
            .expect("light source");
        assert_eq!(
            fixed.spectrum_reference.as_ref().map(|r| r.spectrum_id.as_str()),
            Some(spectrum_id.as_str()),
            "light source links our spectrum"
        );

        // 3. Two photometries + the new variant present.
        assert_eq!(
            p.general_definitions.photometries.as_ref().unwrap().photometry.len(),
            2
        );
        assert!(p
            .product_definitions
            .variants
            .as_ref()
            .unwrap()
            .variant
            .iter()
            .any(|v| v.id == variant_id));

        // 4. Canonical schema URL on output (NOT a bundled local GldfSchema.xsd).
        let xml = p.to_xml().expect("to_xml");
        assert!(
            xml.contains("https://gldf.io/xsd/gldf/1.0.0-rc.3/gldf.xsd"),
            "canonical noNamespaceSchemaLocation"
        );
        assert!(!xml.contains("GldfSchema.xsd"), "no bundled local schema ref");
    }

    /// `rename_fixed_light_source` updates the source id and every reference
    /// across emitters + equipments. `replace_photometry_file` swaps the file
    /// behind a photometry without changing the photometry id (so emitter
    /// PhotometryReference stays valid).
    #[cfg(feature = "eulumdat")]
    #[test]
    fn test_rename_and_replace() {
        let ies = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/data/test.ies"),
        )
        .expect("test.ies fixture");
        let buf_gldf = crate::ldt_to_gldf(&ies, "test.ies").expect("ldt_to_gldf");
        let mut editable = EditableGldf::from_product(buf_gldf.gldf);
        for f in buf_gldf.files {
            if let (Some(id), Some(c)) = (f.file_id, f.content) {
                editable.embedded_files.insert(id, c);
            }
        }

        // The conversion creates one fixed light source "lightsource_1" and
        // one photometry "photometry" with one emitter referencing both.
        editable
            .product
            .rename_fixed_light_source("lightsource_1", "MyCustomSource")
            .expect("rename");

        // Source itself renamed.
        assert!(editable
            .product
            .general_definitions
            .light_sources
            .as_ref()
            .unwrap()
            .fixed_light_source
            .iter()
            .any(|s| s.id == "MyCustomSource"));
        assert!(!editable
            .product
            .general_definitions
            .light_sources
            .as_ref()
            .unwrap()
            .fixed_light_source
            .iter()
            .any(|s| s.id == "lightsource_1"));

        // Emitter reference rewritten.
        let any_emitter_points_at_new = editable
            .product
            .general_definitions
            .emitters
            .as_ref()
            .unwrap()
            .emitter
            .iter()
            .flat_map(|e| e.fixed_light_emitter.iter())
            .any(|fle| {
                fle.light_source_reference.fixed_light_source_id.as_deref()
                    == Some("MyCustomSource")
            });
        assert!(any_emitter_points_at_new, "emitter still points at old id");

        // Conflict detection: renaming to an in-use id errors and leaves state intact.
        assert!(editable
            .product
            .rename_fixed_light_source("MyCustomSource", "MyCustomSource")
            .is_ok());
        editable
            .product
            .add_fixed_light_source(crate::gldf::general_definitions::lightsources::FixedLightSource {
                id: "second".to_string(),
                ..Default::default()
            })
            .expect("add second");
        assert!(editable
            .product
            .rename_fixed_light_source("MyCustomSource", "second")
            .is_err());

        // -- replace_photometry_file: swap the file, keep the id --
        let photometry_id = editable
            .product
            .general_definitions
            .photometries
            .as_ref()
            .unwrap()
            .photometry
            .first()
            .map(|p| p.id.clone())
            .expect("at least one photometry");

        let old_file_id = editable
            .product
            .general_definitions
            .photometries
            .as_ref()
            .unwrap()
            .photometry
            .iter()
            .find(|p| p.id == photometry_id)
            .and_then(|p| p.photometry_file_reference.as_ref().map(|r| r.file_id.clone()))
            .expect("photometry has a file ref");

        let fake_ies_bytes = b"IESNA:LM-63-1995\nfake replacement\n".to_vec();
        editable
            .replace_photometry_file(&photometry_id, fake_ies_bytes.clone(), "better.ies")
            .expect("replace");

        // Same photometry id, NEW file id, old file gone.
        let p_after = editable
            .product
            .general_definitions
            .photometries
            .as_ref()
            .unwrap()
            .photometry
            .iter()
            .find(|p| p.id == photometry_id)
            .unwrap();
        let new_file_id = p_after
            .photometry_file_reference
            .as_ref()
            .unwrap()
            .file_id
            .clone();
        assert_ne!(old_file_id, new_file_id, "file id should change");
        assert!(!editable
            .product
            .general_definitions
            .files
            .file
            .iter()
            .any(|f| f.id == old_file_id), "old File entry removed");
        assert!(editable.embedded_files.get(&old_file_id).is_none());
        assert_eq!(editable.embedded_files.get(&new_file_id), Some(&fake_ies_bytes));
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
