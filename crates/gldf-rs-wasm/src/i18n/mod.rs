//! Viewer-side internationalisation tables for XSD enum values.
//!
//! The values themselves are fixed by the GLDF schema (e.g. `"Direct"`,
//! `"Asymmetrical"`); these tables translate the *display* of those values
//! for the chosen UI language. Lookup is keyed by the canonical XSD string
//! and falls back silently to the canonical (English) form when the active
//! UI language has no entry.

pub mod applications;
pub mod light_distribution;
pub mod ui;

// `xsd_enums` moved to `gldf_rs::validation::xsd_enums` so non-UI
// consumers (FFI, Python, server-side linters) can validate without
// pulling the viewer's wasm/yew/web-sys deps. Re-export here so editor
// modules that reach for `crate::i18n::xsd_enums::*` keep working.
pub use gldf_rs::validation::xsd_enums;
