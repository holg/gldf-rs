//! XSD validation against the GLDF schema, via the pure-Rust `uppsala`
//! crate.
//!
//! Two schema revisions are embedded (`include_str!`) so consumers don't
//! ship them alongside; each compiles once and is cached in its own
//! `OnceLock`:
//!
//! - **rc-3** — the latest *published* GLDF schema
//!   (`gldf.io/xsd/gldf/1.0.0-rc.3/gldf.xsd`). Default validation target.
//! - **rc-4** — the proposed schema from globallightingdata/gldf#30
//!   (spelling cleanup, six control protocols, per-channel power, decimal
//!   SDCM). Public on the PR branch at the flat `xsd/gldf.xsd` path but
//!   not yet at a versioned `gldf.io` URL. Selectable now so producers can
//!   author and validate rc-4 features ahead of the (slow) merge.
//!
//! Which revision a call validates against is chosen with
//! [`GldfSchemaVersion`]; the version-less entry points default to
//! [`DEFAULT_VALIDATION_SCHEMA`] (= the latest *published* rc, so rc-3
//! today — flip that one constant when rc-4 publishes).
//!
//! Surface (gated on the `xsd` feature so WASM builds that don't want the
//! extra schema text can opt out):
//!
//! - [`validate_product_xml`] / [`validate_product_xml_with_schema`]
//! - [`validate_gldf_archive`] / [`validate_gldf_archive_with_schema`]
//! - [`XsdFinding`] — flat per-error record (message + line + column).

use std::sync::OnceLock;

pub use crate::GldfSchemaVersion;

/// Default schema revision for the version-less validation entry points:
/// the latest *published* GLDF rc. rc-4 is still an open PR (#30) with no
/// versioned `gldf.io` URL yet, so producers who validate without opting
/// in get the published rc-3 behaviour. Flip this to `Rc4` the day rc-4
/// publishes — every version-less caller then upgrades in one edit.
pub const DEFAULT_VALIDATION_SCHEMA: GldfSchemaVersion = GldfSchemaVersion::Rc3;

/// Embedded rc-3 XSD — the latest published schema (default target).
const RC3_XSD: &str = include_str!("../../schemas/gldf-1.0.0-rc-3.xsd");

/// Embedded rc-4 XSD — the proposed schema (globallightingdata/gldf#30),
/// bundled from the PR branch so rc-4 features can be validated pre-merge.
const RC4_XSD: &str = include_str!("../../schemas/gldf-1.0.0-rc-4.xsd");

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

/// Compiled validator for one schema revision, cached for the process
/// lifetime. Each revision compiles at most once (first use).
fn validator(version: GldfSchemaVersion) -> XsdResult<&'static uppsala::XsdValidator> {
    static RC3: OnceLock<Result<uppsala::XsdValidator, String>> = OnceLock::new();
    static RC4: OnceLock<Result<uppsala::XsdValidator, String>> = OnceLock::new();

    let compile = |xsd: &str| -> Result<uppsala::XsdValidator, String> {
        let doc = uppsala::parse(xsd).map_err(|e| format!("parse xsd: {e:?}"))?;
        uppsala::XsdValidator::from_schema(&doc).map_err(|e| format!("compile xsd: {e:?}"))
    };

    let cached = match version {
        GldfSchemaVersion::Rc3 => RC3.get_or_init(|| compile(RC3_XSD)),
        GldfSchemaVersion::Rc4 => RC4.get_or_init(|| compile(RC4_XSD)),
    };
    cached.as_ref().map_err(|e| XsdError::Schema(e.clone()))
}

/// Validate a `product.xml` source against the default published schema
/// ([`DEFAULT_VALIDATION_SCHEMA`] — rc-3 today).
///
/// An empty Vec means clean. A non-empty Vec is the list of violations.
pub fn validate_product_xml(xml: &str) -> XsdResult<Vec<XsdFinding>> {
    validate_product_xml_with_schema(xml, DEFAULT_VALIDATION_SCHEMA)
}

/// Validate a `product.xml` source against an explicitly chosen schema
/// revision. Use [`GldfSchemaVersion::Rc4`] to check the proposed rc-4
/// features (control protocols, decimal SDCM, …) ahead of the merge.
pub fn validate_product_xml_with_schema(
    xml: &str,
    version: GldfSchemaVersion,
) -> XsdResult<Vec<XsdFinding>> {
    let v = validator(version)?;
    let doc = uppsala::parse(xml).map_err(|e| XsdError::BadXml(format!("{e:?}")))?;
    Ok(v.validate(&doc).into_iter().map(into_finding).collect())
}

/// Validate the `product.xml` inside a `.gldf` archive byte slice against
/// the default published schema ([`DEFAULT_VALIDATION_SCHEMA`]).
pub fn validate_gldf_archive(bytes: &[u8]) -> XsdResult<Vec<XsdFinding>> {
    validate_gldf_archive_with_schema(bytes, DEFAULT_VALIDATION_SCHEMA)
}

/// Validate the `product.xml` inside a `.gldf` archive against an
/// explicitly chosen schema revision.
pub fn validate_gldf_archive_with_schema(
    bytes: &[u8],
    version: GldfSchemaVersion,
) -> XsdResult<Vec<XsdFinding>> {
    use std::io::Read;
    let reader = std::io::Cursor::new(bytes);
    let mut zip =
        zip::ZipArchive::new(reader).map_err(|e| XsdError::Archive(format!("open zip: {e}")))?;
    let mut entry = zip
        .by_name("product.xml")
        .map_err(|e| XsdError::Archive(format!("no product.xml: {e}")))?;
    let mut xml = String::new();
    entry
        .read_to_string(&mut xml)
        .map_err(|e| XsdError::Archive(format!("read product.xml: {e}")))?;
    validate_product_xml_with_schema(&xml, version)
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
    fn both_schemas_compile() {
        assert!(
            validator(GldfSchemaVersion::Rc3).is_ok(),
            "rc-3 XSD should compile cleanly"
        );
        assert!(
            validator(GldfSchemaVersion::Rc4).is_ok(),
            "rc-4 XSD should compile cleanly"
        );
    }

    #[test]
    fn default_is_latest_published_rc() {
        assert_eq!(
            DEFAULT_VALIDATION_SCHEMA,
            GldfSchemaVersion::Rc3,
            "version-less validation defaults to the latest PUBLISHED rc"
        );
    }

    #[test]
    fn empty_xml_is_an_error() {
        let r = validate_product_xml("");
        assert!(matches!(r, Err(XsdError::BadXml(_))));
    }
}

#[cfg(test)]
mod schema_copies {
    /// `schemas/` holds the crate's own copies of the XSDs (a published
    /// crate can only embed its own files). In the repository they must
    /// equal the files the other tests read from `tests/data`.
    #[test]
    fn embedded_schemas_match_tests_data() {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data");
        if !data.is_dir() {
            return;
        }
        for (name, embedded) in [
            ("gldf-1.0.0-rc-3.xsd", super::RC3_XSD),
            ("gldf-1.0.0-rc-4.xsd", super::RC4_XSD),
        ] {
            let original = std::fs::read_to_string(data.join(name)).unwrap();
            assert!(
                original == embedded,
                "schemas/{name} differs from tests/data/{name}"
            );
        }
    }
}
