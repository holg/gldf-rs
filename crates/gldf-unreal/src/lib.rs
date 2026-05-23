//! GLDF → Unreal Engine Datasmith exporter.
//!
//! Reads a GLDF file (luminaire 3D model + IES/LDT photometry + per-variant
//! lumens/watts overrides), and emits one `.udatasmith` bundle per variant
//! that Unreal Engine 5.4+ imports natively via its Datasmith Importer.
//!
//! See the crate README for the bundle layout, the C ABI for the UE plugin,
//! and the licence note.

pub mod error;
pub mod options;
pub mod exporter;

// Phase 2+ modules — declared here so the public API surface is stable from
// the start, even when bodies are stubs.
pub mod coords;
pub mod mesh;
pub mod photometry;
pub mod datasmith;

// Phase 4 — FFI surface for the Unreal plugin.
pub mod ffi;

pub use error::{ExportError, Result};
pub use exporter::{export_gldf_to_datasmith, BundleArtifact, ExportReport, Exporter};
pub use options::{ExportOptions, UnitSystem, VariantSelector};
