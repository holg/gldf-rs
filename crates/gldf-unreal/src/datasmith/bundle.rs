//! Filesystem layout for a Datasmith bundle.
//!
//! ```text
//! <out_dir>/<bundle_name>__<variant_id>/
//!     <bundle_name>__<variant_id>.udatasmith
//!     Assets/
//!         Geometry/   luminaire_body.obj  +  .mtl  +  textures/
//!         Ies/        emitter_<photometry_id>.ies
//!         Manifest.json
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{ExportError, Result};
use crate::exporter::BundleArtifact;

use super::elements::DatasmithDocument;
use super::xml::write_document;

pub struct BundleWriter {
    pub root: PathBuf,
    pub bundle_name: String,
    pub variant_id: String,
    pub overwrite: bool,
    asset_paths: Vec<PathBuf>,
}

impl BundleWriter {
    pub fn new(out_dir: &Path, bundle_name: &str, variant_id: &str, overwrite: bool) -> Self {
        let root = out_dir.join(format!("{bundle_name}__{variant_id}"));
        Self {
            root,
            bundle_name: bundle_name.to_string(),
            variant_id: variant_id.to_string(),
            overwrite,
            asset_paths: vec![],
        }
    }

    /// Create the directory layout (idempotent if `overwrite = true`).
    pub fn prepare(&self) -> Result<()> {
        if self.root.exists() {
            if !self.overwrite {
                return Err(ExportError::OutputExists(self.root.clone()));
            }
            fs::remove_dir_all(&self.root).map_err(|e| ExportError::Io {
                path: self.root.clone(),
                source: e,
            })?;
        }
        for sub in ["Assets/Geometry", "Assets/Ies"] {
            let p = self.root.join(sub);
            fs::create_dir_all(&p).map_err(|e| ExportError::Io {
                path: p.clone(),
                source: e,
            })?;
        }
        Ok(())
    }

    /// Write the OBJ (and optional MTL) into `Assets/Geometry/`. The
    /// `obj_filename` may contain forward-slash subpaths (e.g.
    /// `geom_1/luminaire.obj`); parent directories are created as needed
    /// so the L3D's internal layout is preserved.
    pub fn write_geometry(&mut self, obj_filename: &str, obj_bytes: &[u8]) -> Result<PathBuf> {
        let path = self.root.join("Assets/Geometry").join(obj_filename);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ExportError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        fs::write(&path, obj_bytes).map_err(|e| ExportError::Io {
            path: path.clone(),
            source: e,
        })?;
        self.asset_paths.push(path.clone());
        Ok(path)
    }

    /// Write an IES into `Assets/Ies/`.
    pub fn write_ies(&mut self, ies_filename: &str, ies_bytes: &[u8]) -> Result<PathBuf> {
        let path = self.root.join("Assets/Ies").join(ies_filename);
        fs::write(&path, ies_bytes).map_err(|e| ExportError::Io {
            path: path.clone(),
            source: e,
        })?;
        self.asset_paths.push(path.clone());
        Ok(path)
    }

    /// Write a debug manifest. Optional — pass `None` to skip.
    pub fn write_manifest_json(&mut self, manifest: &str) -> Result<PathBuf> {
        let path = self.root.join("Assets/Manifest.json");
        fs::write(&path, manifest).map_err(|e| ExportError::Io {
            path: path.clone(),
            source: e,
        })?;
        self.asset_paths.push(path.clone());
        Ok(path)
    }

    /// Serialize the Datasmith document and write the `.udatasmith` file.
    /// Consumes the writer and returns the artifact summary.
    pub fn finish(self, doc: &DatasmithDocument) -> Result<BundleArtifact> {
        let xml = write_document(doc);
        let udatasmith_path = self
            .root
            .join(format!("{}__{}.udatasmith", self.bundle_name, self.variant_id));
        fs::write(&udatasmith_path, xml).map_err(|e| ExportError::Io {
            path: udatasmith_path.clone(),
            source: e,
        })?;
        Ok(BundleArtifact {
            variant_id: self.variant_id,
            udatasmith_path,
            asset_paths: self.asset_paths,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasmith::elements::*;
    use tempfile::tempdir;

    #[test]
    fn prepare_creates_expected_layout() {
        let tmp = tempdir().unwrap();
        let mut w = BundleWriter::new(tmp.path(), "lamp", "v1", true);
        w.prepare().unwrap();
        assert!(w.root.join("Assets/Geometry").is_dir());
        assert!(w.root.join("Assets/Ies").is_dir());

        w.write_geometry("body.obj", b"v 0 0 0\n").unwrap();
        w.write_ies("e1.ies", b"IESNA:LM-63-2002\n").unwrap();

        let doc = DatasmithDocument {
            version: "0.24".into(),
            sdk_version: "5.4".into(),
            host: "test".into(),
            resource_path: "Assets".into(),
            static_meshes: vec![StaticMesh {
                name: "body".into(),
                label: "body".into(),
                file_path: "Assets/Geometry/body.obj".into(),
            }],
            actors: vec![],
        };
        let artifact = w.finish(&doc).unwrap();
        assert!(artifact.udatasmith_path.is_file());
        assert!(artifact
            .udatasmith_path
            .to_string_lossy()
            .ends_with("lamp__v1.udatasmith"));
        assert_eq!(artifact.asset_paths.len(), 2);
    }

    #[test]
    fn prepare_fails_when_exists_and_overwrite_false() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("lamp__v1");
        fs::create_dir_all(&root).unwrap();

        let w = BundleWriter::new(tmp.path(), "lamp", "v1", false);
        let err = w.prepare().unwrap_err();
        assert!(matches!(err, ExportError::OutputExists(_)));
    }

    #[test]
    fn prepare_overwrites_when_flag_set() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("lamp__v1");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("stale.txt"), "old").unwrap();

        let w = BundleWriter::new(tmp.path(), "lamp", "v1", true);
        w.prepare().unwrap();
        assert!(!root.join("stale.txt").exists());
        assert!(root.join("Assets/Geometry").is_dir());
    }
}
