
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use gldf_rs::gldf::{GldfProduct};
use std::path::Path;

#[pyfunction]
fn gldf_to_xml(path: &str) -> PyResult<String> {
    let loaded: GldfProduct = GldfProduct::load_gldf(path).unwrap();
    Ok(loaded.to_xml().unwrap())
}

#[pyfunction]
fn gldf_to_json(path: &str) -> PyResult<String> {
    let loaded: GldfProduct = GldfProduct::load_gldf(path).unwrap();
    Ok(loaded.to_json().unwrap())
}

#[pyfunction]
fn json_from_xml_str(xml_str: &str) -> PyResult<String> {
    let loaded: GldfProduct = GldfProduct::from_xml(&xml_str.to_string()).unwrap();
    Ok(loaded.to_json().unwrap())
}

#[pyfunction]
fn xml_from_json(json_str: &str) -> PyResult<String> {
    let loaded: GldfProduct = GldfProduct::from_json(json_str).unwrap();
    Ok(loaded.to_xml().unwrap())
}

// #[pyfunction]
// fn gldf_product(path: &str) -> PyResult<GldfProduct> {
//     let loaded: GldfProduct = GldfProduct::from_json(json_str).unwrap();
//     Ok(loaded.to_xml().unwrap())
// }

/// A Python module implemented in Rust.
#[pymodule]
fn gldf_rs_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(gldf_to_xml, m)?)?;
    m.add_function(wrap_pyfunction!(gldf_to_json, m)?)?;
    m.add_function(wrap_pyfunction!(xml_from_json, m)?)?;
    m.add_function(wrap_pyfunction!(json_from_xml_str, m)?)?;
    Ok(())
}