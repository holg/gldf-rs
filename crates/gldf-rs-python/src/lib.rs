//! Python bindings for GLDF (General Lighting Data Format) parser
//!
//! This module provides Python bindings for parsing, converting, and manipulating GLDF files.

#![allow(clippy::useless_conversion)]

use gldf_rs::convert::ldt_to_gldf;
use gldf_rs::gldf::GldfProduct;
use gldf_rs::EditableGldf;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Convert a GLDF file to XML string
#[pyfunction]
fn gldf_to_xml(path: &str) -> PyResult<String> {
    let loaded = GldfProduct::load_gldf(path)
        .map_err(|e| PyValueError::new_err(format!("Failed to load GLDF: {}", e)))?;
    loaded
        .to_xml()
        .map_err(|e| PyValueError::new_err(format!("Failed to convert to XML: {}", e)))
}

/// Convert a GLDF file to JSON string
#[pyfunction]
fn gldf_to_json(path: &str) -> PyResult<String> {
    let loaded = GldfProduct::load_gldf(path)
        .map_err(|e| PyValueError::new_err(format!("Failed to load GLDF: {}", e)))?;
    loaded
        .to_json()
        .map_err(|e| PyValueError::new_err(format!("Failed to convert to JSON: {}", e)))
}

/// Parse XML string to JSON
#[pyfunction]
fn json_from_xml_str(xml_str: &str) -> PyResult<String> {
    let loaded = GldfProduct::from_xml(xml_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to parse XML: {}", e)))?;
    loaded
        .to_json()
        .map_err(|e| PyValueError::new_err(format!("Failed to convert to JSON: {}", e)))
}

/// Parse JSON string to XML
#[pyfunction]
fn xml_from_json(json_str: &str) -> PyResult<String> {
    let loaded = GldfProduct::from_json(json_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {}", e)))?;
    loaded
        .to_xml()
        .map_err(|e| PyValueError::new_err(format!("Failed to convert to XML: {}", e)))
}

/// Load GLDF from bytes and return JSON representation
#[pyfunction]
fn gldf_from_bytes(data: &[u8]) -> PyResult<String> {
    let loaded = GldfProduct::load_gldf_from_buf(data.to_vec())
        .map_err(|e| PyValueError::new_err(format!("Failed to load GLDF from bytes: {}", e)))?;
    loaded
        .to_json()
        .map_err(|e| PyValueError::new_err(format!("Failed to convert to JSON: {}", e)))
}

/// Convert LDT/IES photometry file to GLDF and return JSON representation
///
/// Args:
///     data: The LDT or IES file content as bytes
///     filename: The original filename (used to determine format and product name)
///
/// Returns:
///     JSON string representation of the GLDF product
#[pyfunction]
fn ldt_to_gldf_json(data: &[u8], filename: &str) -> PyResult<String> {
    let gldf = ldt_to_gldf(data, filename)
        .map_err(|e| PyValueError::new_err(format!("Failed to convert LDT/IES to GLDF: {}", e)))?;

    gldf.gldf
        .to_json()
        .map_err(|e| PyValueError::new_err(format!("Failed to convert to JSON: {}", e)))
}

/// Convert LDT/IES photometry file to GLDF bytes (ZIP archive)
///
/// Args:
///     data: The LDT or IES file content as bytes
///     filename: The original filename (used to determine format and product name)
///
/// Returns:
///     GLDF file content as bytes
#[pyfunction]
fn ldt_to_gldf_bytes<'py>(
    py: Python<'py>,
    data: &[u8],
    filename: &str,
) -> PyResult<Bound<'py, pyo3::types::PyBytes>> {
    let file_buf_gldf = ldt_to_gldf(data, filename)
        .map_err(|e| PyValueError::new_err(format!("Failed to convert LDT/IES to GLDF: {}", e)))?;

    // Create EditableGldf from the result to use save_to_buf
    let mut editable = EditableGldf::from_product(file_buf_gldf.gldf);

    // Add embedded files
    for buf_file in file_buf_gldf.files {
        if let (Some(file_id), Some(content)) = (buf_file.file_id, buf_file.content) {
            editable.embedded_files.insert(file_id, content);
        }
    }

    let gldf_bytes = editable
        .save_to_buf()
        .map_err(|e| PyValueError::new_err(format!("Failed to create GLDF bytes: {}", e)))?;

    Ok(pyo3::types::PyBytes::new(py, &gldf_bytes))
}

/// Export GLDF from JSON to bytes (ZIP archive)
///
/// Args:
///     json_str: JSON string representation of the GLDF product
///
/// Returns:
///     GLDF file content as bytes (ZIP archive)
#[pyfunction]
fn gldf_json_to_bytes<'py>(
    py: Python<'py>,
    json_str: &str,
) -> PyResult<Bound<'py, pyo3::types::PyBytes>> {
    let gldf = GldfProduct::from_json(json_str)
        .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {}", e)))?;

    let editable = EditableGldf::from_product(gldf);

    let gldf_bytes = editable
        .save_to_buf()
        .map_err(|e| PyValueError::new_err(format!("Failed to create GLDF bytes: {}", e)))?;

    Ok(pyo3::types::PyBytes::new(py, &gldf_bytes))
}

/// `EditorState` — Python wrapper around `gldf_rs_editor::GldfState`.
///
/// Exposes the same `apply` reducer that the WASM, TUI, and CLI
/// frontends drive, via JSON in / JSON out. A notebook can build a
/// GLDF from zero without ever touching FFI byte layers:
///
/// ```python
/// import gldf_rs_python as gldf
/// s = gldf.EditorState()
/// s.apply_json('{"action": "SetAuthor", "data": "me"}')
/// s.apply_json('{"action": "SetManufacturer", "data": "Acme"}')
/// s.save("out.gldf")
/// ```
#[pyclass]
struct EditorState {
    inner: gldf_rs_editor::GldfState,
}

#[pymethods]
impl EditorState {
    /// Construct an empty editor state.
    #[new]
    fn new() -> Self {
        Self {
            inner: gldf_rs_editor::GldfState::default(),
        }
    }

    /// Load a GLDF file from disk into this state. Replaces any prior
    /// content; modifies `is_modified = false` on success.
    fn load(&mut self, path: &str) -> PyResult<()> {
        let editable = EditableGldf::from_gldf(path)
            .map_err(|e| PyValueError::new_err(format!("load {}: {}", path, e)))?;
        self.inner.apply(gldf_rs_editor::GldfAction::LoadWithFiles {
            product: editable.product,
            embedded_files: editable.embedded_files,
        });
        Ok(())
    }

    /// Apply a single action expressed as JSON. Use the same shape the
    /// `gldf-cli apply` subcommand reads — `{"action": "<variant>",
    /// "data": ...}`. On error, returns `ValueError` with the parse or
    /// reducer error string; on success, returns `None`.
    fn apply_json(&mut self, action_json: &str) -> PyResult<()> {
        let action: gldf_rs_editor::GldfAction = serde_json::from_str(action_json)
            .map_err(|e| PyValueError::new_err(format!("parse action: {}", e)))?;
        self.inner.apply(action);
        if let Some(err) = self.inner.last_error.as_deref() {
            return Err(PyValueError::new_err(err.to_string()));
        }
        Ok(())
    }

    /// Apply a sequence of actions. Returns on the first failure.
    fn apply_many(&mut self, actions_json: &str) -> PyResult<()> {
        let actions: Vec<gldf_rs_editor::GldfAction> = serde_json::from_str(actions_json)
            .map_err(|e| PyValueError::new_err(format!("parse actions: {}", e)))?;
        for action in actions {
            self.inner.apply(action);
            if let Some(err) = self.inner.last_error.as_deref() {
                return Err(PyValueError::new_err(err.to_string()));
            }
        }
        Ok(())
    }

    /// Apply a `WizardDraft` non-interactively. Mirrors `gldf-cli wizard`.
    fn apply_wizard_json(&mut self, draft_json: &str) -> PyResult<()> {
        let draft: gldf_rs_editor::WizardDraft = serde_json::from_str(draft_json)
            .map_err(|e| PyValueError::new_err(format!("parse draft: {}", e)))?;
        gldf_rs_editor::apply_wizard(&mut self.inner, &draft);
        Ok(())
    }

    /// Save the current state to a `.gldf` file on disk.
    fn save(&self, path: &str) -> PyResult<()> {
        let bytes = self
            .inner
            .save_to_buf()
            .map_err(|e| PyValueError::new_err(format!("save: {}", e)))?;
        std::fs::write(path, bytes)
            .map_err(|e| PyValueError::new_err(format!("write {}: {}", path, e)))
    }

    /// Serialize the current product (no embedded files) as a pretty
    /// JSON string. Use for inspection / diffing in scripts.
    fn dump_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner.product)
            .map_err(|e| PyValueError::new_err(format!("serialize: {}", e)))
    }

    /// `True` if the document has unsaved changes.
    #[getter]
    fn is_modified(&self) -> bool {
        self.inner.is_modified
    }

    /// Last reducer error, if any. Cleared by the next successful action.
    #[getter]
    fn last_error(&self) -> Option<String> {
        self.inner.last_error.clone()
    }
}

/// A Python module for GLDF file handling implemented in Rust.
///
/// Functions:
///     gldf_to_xml(path): Load GLDF file and convert to XML string
///     gldf_to_json(path): Load GLDF file and convert to JSON string
///     json_from_xml_str(xml_str): Parse XML string to JSON
///     xml_from_json(json_str): Parse JSON string to XML
///     gldf_from_bytes(data): Load GLDF from bytes and return JSON
///     ldt_to_gldf_json(data, filename): Convert LDT/IES to GLDF JSON
///     ldt_to_gldf_bytes(data, filename): Convert LDT/IES to GLDF bytes
///     gldf_json_to_bytes(json_str): Export GLDF JSON to bytes
///
/// Classes:
///     EditorState: stateful editor driven by the same JSON action
///         surface as the CLI / TUI / WASM frontends.
#[pymodule]
fn gldf_rs_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(gldf_to_xml, m)?)?;
    m.add_function(wrap_pyfunction!(gldf_to_json, m)?)?;
    m.add_function(wrap_pyfunction!(xml_from_json, m)?)?;
    m.add_function(wrap_pyfunction!(json_from_xml_str, m)?)?;
    m.add_function(wrap_pyfunction!(gldf_from_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(ldt_to_gldf_json, m)?)?;
    m.add_function(wrap_pyfunction!(ldt_to_gldf_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(gldf_json_to_bytes, m)?)?;
    m.add_class::<EditorState>()?;
    Ok(())
}
