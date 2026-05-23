//! Error type for the exporter.

use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ExportError>;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to load GLDF: {0}")]
    GldfLoad(String),

    #[error("variant {0:?} not found in GLDF")]
    VariantNotFound(String),

    #[error("no L3D geometry in GLDF (geometry-less variants are not yet supported)")]
    NoGeometry,

    #[error("photometry processing failed: {0}")]
    Photometry(String),

    #[error("output path {0} already exists (pass --overwrite to replace)")]
    OutputExists(PathBuf),

    #[error("XML serialization failed: {0}")]
    Xml(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl ExportError {
    /// Stable error code for the FFI surface. The discriminant numbers are
    /// part of the C ABI contract — never reorder, only append.
    pub fn code(&self) -> i32 {
        match self {
            ExportError::Io { .. } => 1,
            ExportError::GldfLoad(_) => 2,
            ExportError::VariantNotFound(_) => 3,
            ExportError::NoGeometry => 4,
            ExportError::Photometry(_) => 5,
            ExportError::OutputExists(_) => 6,
            ExportError::Xml(_) => 7,
            ExportError::Internal(_) => 99,
        }
    }
}
