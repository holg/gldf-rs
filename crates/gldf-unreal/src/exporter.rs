//! Top-level orchestration: load a GLDF, walk variants, write bundles.
//!
//! For each requested variant we:
//!
//! 1. Resolve the L3D bytes from the GLDF (via [`get_first_l3d_with_ldt`]
//!    with a fallback scan of `buf.files`).
//! 2. Convert L3D → ASCII FBX 7.4 using `l3d_rs::fbx::export_fbx_ascii`
//!    (gated by the `fbx-export` feature on `l3d_rs`). The FBX writer
//!    handles the L3D-meters → UE-centimetres unit conversion itself
//!    (via FBX `GlobalSettings.UnitScaleFactor=100`), so we don't apply
//!    any extra coord transform here.
//! 3. Resolve per-emitter photometry via
//!    [`build_ies_outputs_for_variant`]; write the IES files.
//! 4. Emit a minimal `.udatasmith` referencing the FBX as a
//!    `<StaticMesh>` and each IES as an `<ActorLight type="Spot">`.
//!
//! Earlier revisions wrote OBJ instead of FBX. UE 5.7's Datasmith
//! Importer hangs on external OBJ references (it parses them as
//! `.udsmesh` binary), so we use FBX which Datasmith handles via its
//! first-party FBX translator.

use std::path::{Path, PathBuf};

use gldf_rs::mapping::get_first_l3d_with_ldt;
use gldf_rs::{FileBufGldf, GldfProduct};

use crate::datasmith::bundle::BundleWriter;
use crate::datasmith::elements::{
    Actor, ActorChild, ActorLight, ActorMesh, DatasmithDocument, LightType, StaticMesh, Transform,
};
use crate::error::{ExportError, Result};
use crate::options::{ExportOptions, VariantSelector};
use crate::photometry::{build_ies_outputs_for_variant, EmitterIes};

/// Datasmith schema version targeted by the writer. Update in lockstep
/// with `<SDKVersion>` when bumping UE compatibility.
const DATASMITH_VERSION: &str = "0.24";
const DATASMITH_SDK_VERSION: &str = "5.4";

/// An exporter bound to one parsed GLDF document. Build once, then call
/// [`Self::export`] any number of times (e.g. per option preset).
pub struct Exporter {
    buf: FileBufGldf,
    source_path: Option<PathBuf>,
}

impl Exporter {
    /// Load a `.gldf` file from disk.
    pub fn from_path(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).map_err(|e| ExportError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Self::from_bytes(bytes).map(|mut e| {
            e.source_path = Some(path.to_path_buf());
            e
        })
    }

    /// Load a `.gldf` from an in-memory byte buffer.
    pub fn from_bytes(buf: Vec<u8>) -> Result<Self> {
        let file_buf = GldfProduct::load_gldf_from_buf_all(buf)
            .map_err(|e| ExportError::GldfLoad(e.to_string()))?;
        Ok(Self {
            buf: file_buf,
            source_path: None,
        })
    }

    /// Read-only access to the parsed GLDF product tree.
    pub fn gldf(&self) -> &GldfProduct {
        &self.buf.gldf
    }

    /// Read-only access to the underlying file buffer (parsed product +
    /// raw file bytes for L3D/LDT lookup).
    pub fn file_buf(&self) -> &FileBufGldf {
        &self.buf
    }

    /// The file path the exporter was loaded from (if any).
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// Run the export. Emits one bundle per requested variant.
    pub fn export(&self, opts: &ExportOptions) -> Result<ExportReport> {
        let variant_ids = select_variant_ids(&self.buf.gldf, &opts.variants)?;
        let mut bundles = Vec::with_capacity(variant_ids.len());
        for vid in variant_ids {
            let artifact = self.export_one_variant(opts, &vid)?;
            bundles.push(artifact);
        }
        Ok(ExportReport { bundles })
    }

    fn export_one_variant(
        &self,
        opts: &ExportOptions,
        variant_id: &str,
    ) -> Result<BundleArtifact> {
        // Resolve the L3D bytes. Prefer the strict mapping (variant →
        // geometry → L3D file) because that's what the photometry pipeline
        // also uses. If the GLDF doesn't wire L3D via
        // `<ModelGeometryReference>` (some authoring tools differ), fall
        // back to scanning the file buffer for any `.l3d` entry.
        let l3d_bytes: Vec<u8> = get_first_l3d_with_ldt(&self.buf)
            .and_then(|m| m.l3d_content)
            .or_else(|| first_l3d_in_buffer(&self.buf))
            .ok_or(ExportError::NoGeometry)?;

        // Convert L3D → ASCII FBX 7.4 via l3d_rs's fbx-export feature.
        // The FBX writer reads structure.xml, walks the part/joint
        // hierarchy, embeds the OBJ mesh data inline as FBX Geometry
        // nodes, emits LEOs as FBX Light nodes + synthetic emitter
        // meshes, and applies the L3D→UE unit conversion (meters →
        // centimetres via GlobalSettings.UnitScaleFactor=100). That
        // means we do NOT also apply `crate::coords` / `crate::mesh`
        // transforms here — the FBX writer owns the coord conversion
        // end-to-end and we'd double-flip if we did.
        let l3d = l3d_rs::from_buffer(&l3d_bytes);
        let fbx_text = l3d_rs::fbx::export_fbx_ascii(&l3d)
            .map_err(|e| ExportError::Internal(format!("FBX export failed: {e}")))?;

        // Choose a stable mesh name + filename. The L3D's
        // `<GeometryFileDefinition>` typically has a `partName` we could
        // use, but for v0.0.1 we just use the bundle name — one luminaire
        // per bundle, one mesh per luminaire.
        let mesh_name = sanitize_asset_name(&opts.bundle_name);
        let fbx_filename = format!("{mesh_name}.fbx");

        // Write the bundle.
        let mut w = BundleWriter::new(&opts.out_dir, &opts.bundle_name, variant_id, opts.overwrite);
        w.prepare()?;
        let _fbx_path = w.write_geometry(&fbx_filename, fbx_text.as_bytes())?;

        // Resolve per-emitter photometry for this variant and emit IES files.
        let ies_outputs = build_ies_outputs_for_variant(&self.buf, variant_id)?;
        let mut light_children: Vec<ActorChild> = Vec::with_capacity(ies_outputs.len());
        for emitter in ies_outputs {
            let actor_light = self.write_emitter_and_build_light(&mut w, &emitter)?;
            light_children.push(ActorChild::Light(actor_light));
        }

        // Datasmith document: one StaticMesh referencing the FBX, one
        // Actor with the mesh + N ActorLights.
        let mesh_ref_path = format!("Assets/Geometry/{fbx_filename}");
        let mut children = Vec::with_capacity(1 + light_children.len());
        children.push(ActorChild::Mesh(ActorMesh {
            name: "Body".into(),
            mesh_ref: mesh_name.clone(),
            transform: Transform::identity(),
        }));
        children.extend(light_children);

        let doc = DatasmithDocument {
            version: DATASMITH_VERSION.into(),
            sdk_version: DATASMITH_SDK_VERSION.into(),
            host: "gldf-to-datasmith".into(),
            resource_path: "Assets".into(),
            static_meshes: vec![StaticMesh {
                name: mesh_name.clone(),
                label: mesh_name,
                file_path: mesh_ref_path,
            }],
            actors: vec![Actor {
                name: format!("{}__{}", opts.bundle_name, variant_id),
                layer: "GLDF".into(),
                transform: Transform::identity(),
                children,
            }],
        };

        w.finish(&doc)
    }

    /// Write the IES bytes for an emitter into the bundle and return the
    /// matching `<ActorLight>` shell. Colour-temperature lookup walks the
    /// GLDF's `<LightSources>` for `emitter.light_source_id` and pulls
    /// `rated_cct` if present.
    fn write_emitter_and_build_light(
        &self,
        w: &mut BundleWriter,
        emitter: &EmitterIes,
    ) -> Result<ActorLight> {
        // Sanitise the photometry id for the filename — replace anything
        // that's not URL-/filesystem-safe with `_`.
        let safe_id: String = emitter
            .photometry_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let ies_filename = format!("emitter_{safe_id}.ies");
        w.write_ies(&ies_filename, &emitter.ies_bytes)?;

        let cct = emitter
            .light_source_id
            .as_ref()
            .and_then(|id| lookup_color_temperature_k(&self.buf.gldf, id));

        Ok(ActorLight {
            name: format!("Emitter_{safe_id}"),
            light_type: LightType::Spot,
            transform: Transform::identity(),
            intensity_lumens: emitter.lumens.map(|n| n as f64).unwrap_or(0.0),
            color_temperature_k: cct,
            ies_file: Some(format!("Assets/Ies/{ies_filename}")),
        })
    }
}

/// Look up a `FixedLightSource`'s `ColorInformation.CorrelatedColorTemperature`
/// in Kelvin by id. Returns `None` when the source isn't found or has no
/// CCT declared.
fn lookup_color_temperature_k(gldf: &GldfProduct, light_source_id: &str) -> Option<i32> {
    let ls = gldf.general_definitions.light_sources.as_ref()?;
    let src = ls
        .fixed_light_source
        .iter()
        .find(|s| s.id == light_source_id)?;
    src.color_information
        .as_ref()
        .and_then(|ci| ci.correlated_color_temperature)
}

/// Resolve the variant ids to emit, honouring [`VariantSelector`].
fn select_variant_ids(gldf: &GldfProduct, selector: &VariantSelector) -> Result<Vec<String>> {
    let all: Vec<String> = gldf
        .product_definitions
        .variants
        .as_ref()
        .map(|v| v.variant.iter().map(|x| x.id.clone()).collect())
        .unwrap_or_default();

    match selector {
        VariantSelector::All => Ok(all),
        VariantSelector::First => Ok(all.into_iter().take(1).collect()),
        VariantSelector::Only(wanted) => {
            // Preserve requested order; error on the first unknown id so
            // typos are caught early.
            let mut out = Vec::with_capacity(wanted.len());
            for w in wanted {
                if !all.iter().any(|v| v == w) {
                    return Err(ExportError::VariantNotFound(w.clone()));
                }
                out.push(w.clone());
            }
            Ok(out)
        }
    }
}

/// Scan the GLDF's raw file buffer for the first entry that looks like
/// an L3D (by name suffix or path), returning its bytes. Used as a
/// fallback when `mapping.rs`'s variant-driven walk doesn't find one
/// (e.g. GLDFs that wire L3D outside `<ModelGeometryReference>`).
fn first_l3d_in_buffer(buf: &FileBufGldf) -> Option<Vec<u8>> {
    buf.files.iter().find_map(|f| {
        let looks_l3d = f
            .name
            .as_ref()
            .map(|n| n.to_lowercase().ends_with(".l3d"))
            .unwrap_or(false)
            || f.path
                .as_ref()
                .map(|p| p.to_lowercase().ends_with(".l3d"))
                .unwrap_or(false);
        if looks_l3d {
            f.content.clone()
        } else {
            None
        }
    })
}

/// Sanitise a string for use as an asset filename. Replaces anything
/// outside `[A-Za-z0-9_-]` with `_`. Datasmith and FBX importers tolerate
/// most characters, but spaces / dots / quotes cause friction in UE's
/// asset paths.
fn sanitize_asset_name(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
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
