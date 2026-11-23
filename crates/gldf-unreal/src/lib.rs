//! GLDF → Unreal Engine bridge (Rust core for the UE Interchange plugin).
//!
//! Reads a GLDF file (luminaire 3D model + IES/LDT photometry + per-variant
//! lumens/watts overrides) and exposes its contents over a C ABI
//! (`ffi.rs`) so the UE 5.7 `GldfImporter` Interchange plugin can build
//! native UE assets (UTextureLightProfile, UStaticMesh, and — Phase 3 — a
//! variant-aware Blueprint) inside the editor.
//!
//! The Datasmith modules (`datasmith`, `coords`, `mesh`) are the
//! abandoned first approach — kept compiling for now, slated for clean
//! removal.

pub mod error;
pub mod exporter;
pub mod options;

// Photometry/mesh extraction consumed by the Interchange FFI.
pub mod mesh_payload;
pub mod photometry;

// Datasmith-era modules — abandoned, kept compiling until clean removal.
pub mod coords;
pub mod datasmith;
pub mod mesh;

// C ABI surface for the Unreal Interchange plugin.
pub mod ffi;
pub mod ffi_l3d;
pub mod ffi_photometry;

pub use error::{ExportError, Result};
pub use exporter::{export_gldf_to_datasmith, BundleArtifact, ExportReport, Exporter};
pub use options::{ExportOptions, UnitSystem, VariantSelector};
