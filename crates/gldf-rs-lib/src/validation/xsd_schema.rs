//! XSD validation against the GLDF rc-3 schema, via the pure-Rust
//! `uppsala` crate.
//!
//! The rc-3 XSD is embedded into the binary (`include_str!`) so consumers
//! don't have to ship it alongside. uppsala compiles the schema once;
//! subsequent calls reuse the compiled validator via a OnceLock cache.
//!
//! Surface (gated on the `xsd` feature so WASM builds that don't want
//! the extra ~150 KB of schema text can opt out):
//!
//! - [`validate_product_xml`] — validate a `product.xml` string.
//! - [`validate_gldf_archive`] — read `product.xml` from a `.gldf` ZIP
//!   byte slice and validate.
//! - [`XsdFinding`] — flat per-error record (message + line + column).

use std::sync::OnceLock;

/// Embedded XSD text. The rc-3 schema is the de-facto target for our
/// exporter; rc-4 is internal canonical form that re-serializes to rc-3
/// via `GldfProduct::to_xml`.
const RC3_XSD: &str = include_str!("../../../../tests/data/gldf-1.0.0-rc-3.xsd");

/// One XSD violation as reported by the validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XsdFinding {
    /// Human-readable description.
    pub message: String,
    /// 1-based source line of the offending element/attribute, if known.
    pub line: Option<usize>,
    /// 1-based column, if known.
    pub column: Option<usize>,
}

impl std::fmt::Display for XsdFinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, self.column) {
            (Some(l), Some(c)) => write!(f, "[line {l}, col {c}] {}", self.message),
            (Some(l), None) => write!(f, "[line {l}] {}", self.message),
            _ => f.write_str(&self.message),
        }
    }
}

/// Top-level error from XSD validation.
#[derive(Debug)]
pub enum XsdError {
    /// XSD itself can't be parsed/compiled (should never happen — fixture
    /// is shipped in-tree and tested).
    Schema(String),
    /// product.xml is not parseable XML.
    BadXml(String),
    /// The .gldf ZIP could not be read or has no product.xml.
    Archive(String),
}

impl std::fmt::Display for XsdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XsdError::Schema(s) => write!(f, "XSD schema error: {s}"),
            XsdError::BadXml(s) => write!(f, "Bad product.xml: {s}"),
            XsdError::Archive(s) => write!(f, "GLDF archive error: {s}"),
        }
    }
}

impl std::error::Error for XsdError {}

/// Result type. Ok(Vec) — possibly empty, meaning fully clean. Err means
/// validation couldn't even run (bad input).
pub type XsdResult<T> = Result<T, XsdError>;

fn validator() -> XsdResult<&'static uppsala::XsdValidator> {
    static VALIDATOR: OnceLock<Result<uppsala::XsdValidator, String>> = OnceLock::new();
    let cached = VALIDATOR.get_or_init(|| {
        let doc = uppsala::parse(RC3_XSD).map_err(|e| format!("parse xsd: {e:?}"))?;
        uppsala::XsdValidator::from_schema(&doc)
            .map_err(|e| format!("compile xsd: {e:?}"))
    });
    cached
        .as_ref()
        .map_err(|e| XsdError::Schema(e.clone()))
}

/// Validate a `product.xml` source against the rc-3 XSD.
///
/// An empty Vec means clean. A non-empty Vec is the list of violations.
pub fn validate_product_xml(xml: &str) -> XsdResult<Vec<XsdFinding>> {
    let v = validator()?;
    let doc = uppsala::parse(xml).map_err(|e| XsdError::BadXml(format!("{e:?}")))?;
    Ok(v.validate(&doc).into_iter().map(into_finding).collect())
}

/// Validate the `product.xml` inside a `.gldf` archive byte slice.
pub fn validate_gldf_archive(bytes: &[u8]) -> XsdResult<Vec<XsdFinding>> {
    use std::io::Read;
    let reader = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(reader)
        .map_err(|e| XsdError::Archive(format!("open zip: {e}")))?;
    let mut entry = zip
        .by_name("product.xml")
        .map_err(|e| XsdError::Archive(format!("no product.xml: {e}")))?;
    let mut xml = String::new();
    entry
        .read_to_string(&mut xml)
        .map_err(|e| XsdError::Archive(format!("read product.xml: {e}")))?;
    validate_product_xml(&xml)
}

fn into_finding(e: uppsala::ValidationError) -> XsdFinding {
    XsdFinding {
        message: e.message,
        line: e.line,
        column: e.column,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_compiles() {
        assert!(validator().is_ok(), "rc-3 XSD should compile cleanly");
    }

    #[test]
    fn empty_xml_is_an_error() {
        let r = validate_product_xml("");
        assert!(matches!(r, Err(XsdError::BadXml(_))));
    }
}
