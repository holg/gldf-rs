//! Export helpers — format-specific writers that operate on a parsed
//! [`crate::GldfProduct`] plus its embedded file bytes.
//!
//! Currently scoped to per-LightSource exports (LDT / IES / TM-33 XML /
//! IFC fragment). Whole-product exports live on
//! [`crate::GldfProduct`] itself (`to_buf`, `to_xml`) and on the
//! product-level [`crate::ifc::GldfToIfc`].

/// Per-LightSource exports (one LightSource → one LDT / IES / TM-33 / IFC).
#[cfg(feature = "eulumdat")]
pub mod lightsource;

/// Whole-product transforms that rewrite a `GldfProduct` + its
/// embedded asset bytes ahead of zipping. Driven by
/// [`crate::editable::ExportOptions`]. Each transform is opt-in;
/// real-GLDF export is untouched.
pub mod inline_spectrum;

/// TM-33 merge transform: collapse paired (photometry, spectrum)
/// into one `ldc/iesxml`. Gated by `feature = "eulumdat"` because
/// the TM-33 writer lives there.
#[cfg(feature = "eulumdat")]
pub mod tm33_merge;
