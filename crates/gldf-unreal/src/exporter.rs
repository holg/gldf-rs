//! Top-level orchestration: load a GLDF, walk variants, write bundles.
//!
//! Phase 1 keeps the body skeletal — `export()` returns an empty report so
//! the public API surface is locked in and the workspace builds. Phases 2+
//! fill in the actual mesh + photometry + Datasmith XML pipeline.

use std::path::{Path, PathBuf};

use gldf_rs::GldfProduct;

use crate::error::{ExportError, Result};
use crate::options::ExportOptions;

/// An exporter bound to one parsed GLDF document. Build once, then call
/// [`Self::export`] any number of times (e.g. per option preset).
pub struct Exporter {
    gldf: GldfProduct,
    source_path: Option<PathBuf>,
}

impl Exporter {
    /// Load a `.gldf` file from disk.
    pub fn from_path(path: &Path) -> Result<Self> {
        let path_str = path.to_string_lossy().into_owned();
        let gldf = GldfProduct::load_gldf(&path_str).map_err(|e| ExportError::GldfLoad(e.to_string()))?;
        Ok(Self {
            gldf,
            source_path: Some(path.to_path_buf()),
        })
    }

    /// Load a `.gldf` from an in-memory byte buffer.
    pub fn from_bytes(buf: Vec<u8>) -> Result<Self> {
        let gldf = GldfProduct::load_gldf_from_buf(buf)
            .map_err(|e| ExportError::GldfLoad(e.to_string()))?;
        Ok(Self {
            gldf,
            source_path: None,
        })
    }

    /// Read-only access to the parsed GLDF.
    pub fn gldf(&self) -> &GldfProduct {
        &self.gldf
    }

    /// The file path the exporter was loaded from (if any).
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// Run the export.
    ///
    /// Phase 1: this is a no-op that returns an empty [`ExportReport`].
    /// Phase 2 introduces mesh rewriting; phase 3 adds photometry; phase 4
    /// adds CLI + FFI plumbing on top of the same function.
    pub fn export(&self, _opts: &ExportOptions) -> Result<ExportReport> {
        Ok(ExportReport { bundles: vec![] })
    }
}

/// Summary of what `export()` actually wrote, per variant.
#[derive(Debug, Clone, Default)]
pub struct ExportReport {
    pub bundles: Vec<BundleArtifact>,
}

#[derive(Debug, Clone)]
pub struct BundleArtifact {
    /// GLDF variant id this bundle was emitted for.
    pub variant_id: String,
    /// Path to the `.udatasmith` root file.
    pub udatasmith_path: PathBuf,
    /// All asset paths inside `Assets/` (OBJ, MTL, IES, textures, manifest).
    pub asset_paths: Vec<PathBuf>,
}

/// Convenience free function: load a `.gldf` from `gldf_path` and export
/// using `opts`. Equivalent to `Exporter::from_path(...)?.export(opts)`.
pub fn export_gldf_to_datasmith(gldf_path: &Path, opts: &ExportOptions) -> Result<ExportReport> {
    Exporter::from_path(gldf_path)?.export(opts)
}
