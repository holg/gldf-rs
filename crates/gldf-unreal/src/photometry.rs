//! Per-emitter IES emission for a variant.
//!
//! Thin wrapper that composes `gldf_rs::photometry` calls in the order an
//! Unreal export needs:
//!
//! 1. [`gldf_rs::photometry::resolve_variant_photometries`] gives us one
//!    [`VariantPhotometryResolution`](gldf_rs::photometry::VariantPhotometryResolution)
//!    per emitter on the variant, with the resolved `lumens` / `watts`.
//! 2. For each resolution, look up the raw LDT bytes by `photometry_id`.
//! 3. Patch them with `patch_ldt_for_variant(bytes, lumens, watts)`.
//! 4. Parse the patched bytes as `eulumdat::Eulumdat` and run
//!    `export_photometry(.., PhotometryExportFormat::Ies)` → IES bytes.
//!
//! Phase 3 wires this into the bundle writer; the module is declared here
//! so the crate's structure is locked in from Phase 1.

#![allow(dead_code)]
