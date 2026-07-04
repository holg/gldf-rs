//! GLDF Editor — cross-platform (Windows, macOS, Linux) egui application.
//!
//! Binaries:
//! - `gldf-editor-egui` — the editor (shared `GldfAction` reducer), incl.
//!   the batch-convert page. Builds without `viewer3d` for cross-compiles.
//! - `gldf-l3d-viewer` — standalone three-d L3D viewer (`viewer3d`).
//!
//! The former read-only `gldf-viewer-egui` app was retired in favour of
//! the editor; its file-preview renderers live on in [`preview`].

pub mod batch_app;
pub mod editor;
#[cfg(feature = "viewer3d")]
pub mod l3d_render;
pub(crate) mod preview;
pub mod template_editor;

pub use batch_app::BatchPanel;
