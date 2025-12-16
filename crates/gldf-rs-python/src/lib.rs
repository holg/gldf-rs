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
    Ok(())
}
