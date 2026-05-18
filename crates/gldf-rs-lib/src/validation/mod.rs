//! Validation surface for GLDF files.
//!
//! Two complementary walkers, both producing structured findings against
//! a parsed [`crate::gldf::GldfProduct`]:
//!
//! - **[`engine`]** — schema-rule validation (required fields, reference
//!   integrity, data consistency). Reports [`ValidationError`] with a
//!   [`ValidationLevel`] severity. Predates the rc.4 work.
//! - **[`canonical`]** — closed-XSD-enum value validation. Walks every
//!   field bound to an `<xs:enumeration>` and reports any value outside
//!   the canonical set as a [`Violation`]. Powers the viewer's
//!   ⚠ banner and is callable from any consumer (CLI, FFI, server).
//! - **[`xsd_enums`]** — the canonical-values tables themselves
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

// Engine surface — original schema-rule validator. Re-exported flat so
// existing `gldf_rs::validation::ValidationError` paths keep working
// after the file → folder split.
pub use engine::{ValidationError, ValidationLevel, ValidationResult};

// Canonical-enum walker surface.
pub use canonical::{validate, Violation};
