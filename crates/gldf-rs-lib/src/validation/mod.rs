//! Validation surface for GLDF files.
//!
//! Two complementary walkers, both producing structured findings against
//! a parsed [`crate::gldf::GldfProduct`]:
//!
//! - **[`engine`](crate::validation::engine)** — schema-rule validation (required fields, reference
//!   integrity, data consistency). Reports [`ValidationError`] with a
//!   [`ValidationLevel`] severity. Predates the rc.4 work.
//! - **[`canonical`](crate::validation::canonical)** — closed-XSD-enum value validation. Walks every
//!   field bound to an `<xs:enumeration>` and reports any value outside
//!   the canonical set as a [`Violation`](crate::validation::Violation). Powers the viewer's
//!   ⚠ banner and is callable from any consumer (CLI, FFI, server).
//! - **[`xsd_enums`](crate::validation::xsd_enums)** — the canonical-values tables themselves
//!   (IK ratings, IP codes, hazardous-area enums, etc.). Pure data,
//!   shared by `canonical` and the editor's dropdown widgets.
//!
//! Living under `gldf-rs-lib` (the parser side) rather than the WASM
//! viewer keeps it dep-free of `yew`/`web-sys`/`gloo`, so the FFI/Python
//! and any future CLI lints reuse the same enum tables and walker
//! without pulling a browser runtime.

pub mod canonical;
pub mod engine;
pub mod xsd_enums;

#[cfg(feature = "xsd")]
pub mod xsd_schema;

// Engine surface — original schema-rule validator. Re-exported flat so
// existing `gldf_rs::validation::ValidationError` paths keep working
// after the file → folder split.
pub use engine::{validate_gldf, ValidationError, ValidationLevel, ValidationResult};

// Canonical-enum walker surface.
pub use canonical::{validate, Violation};

// XSD-validator surface — selectable rc-3 / rc-4 schema check via uppsala.
// Version-less entry points default to `DEFAULT_VALIDATION_SCHEMA` (the
// latest published rc = rc-3 today); the `*_with_schema` variants take an
// explicit `GldfSchemaVersion` so rc-4 can be validated before it ships.
#[cfg(feature = "xsd")]
pub use xsd_schema::{
    validate_gldf_archive as xsd_validate_gldf_archive,
    validate_gldf_archive_with_schema as xsd_validate_gldf_archive_with_schema,
    validate_product_xml as xsd_validate_product_xml,
    validate_product_xml_with_schema as xsd_validate_product_xml_with_schema, GldfSchemaVersion,
    XsdError, XsdFinding, XsdResult, DEFAULT_VALIDATION_SCHEMA,
};
