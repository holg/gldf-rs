//! Editor state + reducer. Lifted from
//! `gldf-rs-wasm/src/state/gldf_state.rs`. The leptos imports are gone;
//! the `SetUiLanguage` arm calls `self.lang_store.store(&lang)` via the
//! `UiLanguageStore` adapter instead of touching `localStorage` directly.

use crate::action::{GldfAction, LightEmitterKind, Scope};
use crate::ui_language::{LangStore, NullLangStore};
use gldf_rs::gldf::{
    general_definitions::photometries::{
        DescriptivePhotometry, Photometries, Photometry, UGR4H8H705020LQ,
    },
    product_definitions::{
        Applications, DescriptiveAttributes, DurationTimeAndFlux, Electrical, Emergency, Flux,
        Labels, Marketing, Mechanical, ProductMetaData, ProductSize,
    },
    FormatVersion, GldfProduct,
};
use gldf_rs::EditableGldf;
use std::collections::HashMap;
use std::sync::Arc;


/// State of the GLDF editor.
///
/// `product` + `embedded_files` together mirror `gldf_rs::EditableGldf`. We keep
/// them as two public fields (instead of a wrapped `EditableGldf` value) only so
/// every existing editor component reading `gldf.product.X` keeps compiling; the
/// builder actions internally hand both fields to an `EditableGldf` and call the
/// core-lib API. That keeps the drop-and-link logic in the lib where every
/// cross-platform binding can reuse it.
#[derive(Clone)]
pub struct GldfState {
    /// The current GLDF product being edited
    pub product: GldfProduct,
    /// Embedded file bytes, keyed by `<File id="...">`. Mirrors
    /// `EditableGldf::embedded_files`; populated by `GldfAction::LoadWithFiles`
    /// and grown by drop-and-link actions (`AddPhotometryFile`, `AddSpectrumFile`).
    pub embedded_files: HashMap<String, Vec<u8>>,
    /// Whether the product has been modified
    pub is_modified: bool,
    /// Last builder-error message, if any (e.g. "unknown light source id").
    /// Cleared by the next successful builder action or by `Reset`.
    /// UI components can surface this to the user.
    pub last_error: Option<String>,
    /// Currently active language for LocaleFoo viewing/editing — selected
    /// from the document's own locale set (the LanguageBanner dropdown).
    /// Not part of GldfProduct — viewer-only.
    pub active_language: String,
    /// UI translation language — independent of the document. Drives the
    /// viewer's translation tables. Seeded by the frontend's
    /// [`LangStore`] adapter (`BrowserLangStore` in WASM,
    /// `FsLangStore` in the TUI, `NullLangStore` in tests).
    pub ui_language: String,
    /// Persistence adapter for the UI language preference. Each
    /// frontend supplies its own; the reducer's `SetUiLanguage` arm
    /// calls `self.lang_store.store(&lang)` when the user picks.
    pub lang_store: LangStore,
}

impl Default for GldfState {
    fn default() -> Self {
        let lang_store: LangStore = Arc::new(NullLangStore);
        let ui_language = crate::ui_language::initial_ui_language(&*lang_store);
        Self {
            product: GldfProduct::default(),
            embedded_files: HashMap::new(),
            is_modified: false,
            last_error: None,
            active_language: "en".to_string(),
            ui_language,
            lang_store,
        }
    }
}

impl GldfState {
    /// Build a state with a specific UI-language store. Each frontend
    /// (WASM, TUI, CLI, Python) calls this with its own adapter.
    pub fn with_lang_store(lang_store: LangStore) -> Self {
        let ui_language = crate::ui_language::initial_ui_language(&*lang_store);
        Self {
            product: GldfProduct::default(),
            embedded_files: HashMap::new(),
            is_modified: false,
            last_error: None,
            active_language: "en".to_string(),
            ui_language,
            lang_store,
        }
    }
}

impl GldfState {
    /// Build a transient `EditableGldf` for one operation (builder method or
    /// `save_to_buf`). After the operation, copy the (possibly mutated) parts
    /// back into the state with [`Self::take_from_editable`].
    fn to_editable(&self) -> EditableGldf {
        let mut e = EditableGldf::from_product(self.product.clone());
        e.embedded_files = self.embedded_files.clone();
        e
    }

    /// Absorb the result of a builder operation back into the state.
    fn take_from_editable(&mut self, editable: EditableGldf) {
        self.product = editable.product;
        self.embedded_files = editable.embedded_files;
    }

    /// Serialize the current state to GLDF bytes via the core-lib builder.
    /// This is the cross-platform export path — `save_to_buf` writes
    /// `product.xml` (with the canonical online XSD) and zips every embedded
    /// file under its content-type-derived folder.
    pub fn save_to_buf(&self) -> anyhow::Result<Vec<u8>> {
        self.to_editable().save_to_buf()
    }
}


/// In-place dispatcher. The Yew side cloned `*self` into `new_state`,
/// mutated, then re-wrapped in `Rc::new`; we mutate `&mut self` directly
/// because the Leptos `RwSignal::update(|s| s.apply(action))` closure already
/// gives us a mutable reference. The body is otherwise identical to the
/// old `Reducible::reduce` implementation.
impl GldfState {
    pub fn apply(&mut self, action: GldfAction) {
        let prev_modified = self.is_modified;
        self.is_modified = true;


        match action {
            GldfAction::Load(product) => {
                self.active_language = pick_initial_language(&product);
                self.product = product;
                self.embedded_files.clear();
                self.is_modified = false;
                self.last_error = None;
            }
            GldfAction::LoadWithFiles {
                product,
                embedded_files,
            } => {
                self.active_language = pick_initial_language(&product);
                self.product = product;
                self.embedded_files = embedded_files;
                self.is_modified = false;
                self.last_error = None;
            }
            GldfAction::AddPhotometryFile { bytes, filename } => {
                let mut editable = self.to_editable();
                // Keep a copy for the auto-fill pass below — the file
                // embed consumes the bytes vec.
                let bytes_for_summary = bytes.clone();
                match editable.add_photometry_file(bytes, &filename) {
                    Ok(photometry_id) => {
                        // Best-effort: parse the LDT/IES and populate
                        // DescriptivePhotometry. On parse failure we
                        // silently skip the summary — the file still
                        // embeds and the Photometry entry still exists;
                        // the user can fill the fields manually. We log
                        // the parse error so it shows up in browser /
                        // server logs, but don't surface it via
                        // `last_error` because that would make
                        // apply_script abort and confuse callers who
                        // dropped a valid file with weak metadata.
                        let fill_outcome = editable.fill_descriptive_photometry(
                            &photometry_id,
                            &bytes_for_summary,
                            &filename,
                        );
                        self.take_from_editable(editable);
                        if let Err(e) = fill_outcome {
                            log::warn!(
                                "Photometry summary unavailable for {}: {}",
                                filename, e
                            );
                        }
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Add photometry: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::AddSpectrumFile {
                bytes,
                filename,
                light_source_id,
            } => {
                let mut editable = self.to_editable();
                match editable.add_spectrum_file(bytes, &filename, &light_source_id) {
                    Ok(_id) => {
                        self.take_from_editable(editable);
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Add spectrum: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::SetLightSourceSpectrum {
                light_source_id,
                spectrum_id,
            } => {
                let ls = match self.product.general_definitions.light_sources.as_mut() {
                    Some(ls) => ls,
                    None => {
                        self.last_error = Some(format!(
                            "Set spectrum: no light sources in this GLDF"
                        ));
                        self.is_modified = prev_modified;
                        return;
                    }
                };
                let new_ref = spectrum_id.as_ref().map(|sid| {
                    gldf_rs::gldf::general_definitions::lightsources::SpectrumReference {
                        spectrum_id: sid.clone(),
                    }
                });
                let mut hit = false;
                for fls in ls.fixed_light_source.iter_mut() {
                    if fls.id == light_source_id {
                        fls.spectrum_reference = new_ref.clone();
                        hit = true;
                        break;
                    }
                }
                if !hit {
                    for cls in ls.changeable_light_source.iter_mut() {
                        if cls.id == light_source_id {
                            cls.spectrum_reference = new_ref.clone();
                            hit = true;
                            break;
                        }
                    }
                }
                if !hit {
                    self.last_error = Some(format!(
                        "Set spectrum: no light source with id '{}'",
                        light_source_id
                    ));
                    self.is_modified = prev_modified;
                } else {
                    self.last_error = None;
                }
            }
            GldfAction::SetChangeableLightSourcePhotometry {
                light_source_id,
                photometry_id,
            } => {
                let ls = match self.product.general_definitions.light_sources.as_mut() {
                    Some(ls) => ls,
                    None => {
                        self.last_error = Some(
                            "Set photometry: no light sources in this GLDF".to_string(),
                        );
                        self.is_modified = prev_modified;
                        return;
                    }
                };
                let new_ref = photometry_id.as_ref().map(|pid| {
                    gldf_rs::gldf::general_definitions::lightsources::PhotometryReference {
                        photometry_id: pid.clone(),
                    }
                });
                let mut hit = false;
                for cls in ls.changeable_light_source.iter_mut() {
                    if cls.id == light_source_id {
                        cls.photometry_reference = new_ref.clone();
                        hit = true;
                        break;
                    }
                }
                if !hit {
                    // Surface a distinct error when the id resolves to a
                    // FixedLightSource — that's a common newbie mistake
                    // since both kinds live on the same page.
                    let is_fixed = ls
                        .fixed_light_source
                        .iter()
                        .any(|f| f.id == light_source_id);
                    if is_fixed {
                        self.last_error = Some(format!(
                            "Set photometry: '{}' is a FixedLightSource — only ChangeableLightSource carries a PhotometryReference (FixedLightSource photometry comes from its Emitter's FixedLightEmitter)",
                            light_source_id
                        ));
                    } else {
                        self.last_error = Some(format!(
                            "Set photometry: no changeable light source with id '{}'",
                            light_source_id
                        ));
                    }
                    self.is_modified = prev_modified;
                } else {
                    self.last_error = None;
                }
            }
            GldfAction::AddVariantSimple { name } => {
                let mut editable = self.to_editable();
                match editable.add_variant_simple(&name) {
                    Ok(_id) => {
                        self.take_from_editable(editable);
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Add variant: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::RenameVariant { old_id, new_id } => {
                if let Some(vs) = self.product.product_definitions.variants.as_mut() {
                    if let Some(v) = vs.variant.iter_mut().find(|v| v.id == old_id) {
                        v.id = new_id;
                        self.last_error = None;
                    } else {
                        self.last_error = Some(format!("Rename variant: unknown id {}", old_id));
                        self.is_modified = prev_modified;
                    }
                } else {
                    self.last_error = Some("Rename variant: no variants block".to_string());
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::RemoveVariant { id } => {
                match self.product.remove_variant(&id) {
                    Ok(_) => {
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Remove variant: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::DuplicateVariant { src_idx } => {
                let new_id = self.product.generate_unique_id("variant");
                let dup: Option<gldf_rs::gldf::product_definitions::Variant> = self
                    .product
                    .product_definitions
                    .variants
                    .as_ref()
                    .and_then(|vs| vs.variant.get(src_idx))
                    .cloned();
                match dup {
                    Some(mut clone) => {
                        clone.id = new_id;
                        // Append " (copy)" to the English-locale name if
                        // there is one, otherwise leave the name alone.
                        if let Some(ref mut name) = clone.name {
                            for loc in &mut name.locale {
                                if !loc.value.is_empty() {
                                    loc.value.push_str(" (copy)");
                                }
                            }
                        }
                        if let Some(vs) = self.product.product_definitions.variants.as_mut() {
                            // Insert right after the source so the new
                            // entry sits next to the one it was cloned
                            // from in the document.
                            let insert_at = (src_idx + 1).min(vs.variant.len());
                            vs.variant.insert(insert_at, clone);
                        }
                        self.last_error = None;
                    }
                    None => {
                        self.last_error = Some(format!(
                            "Duplicate variant: index {} out of range",
                            src_idx
                        ));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::SetVariantGTIN { index, gtin } => {
                if let Some(v) = self
                    .product
                    .product_definitions
                    .variants
                    .as_mut()
                    .and_then(|vs| vs.variant.get_mut(index))
                {
                    v.gtin = gtin.filter(|s| !s.is_empty());
                }
            }
            GldfAction::SetVariantSortOrder { index, sort_order } => {
                if let Some(v) = self
                    .product
                    .product_definitions
                    .variants
                    .as_mut()
                    .and_then(|vs| vs.variant.get_mut(index))
                {
                    v.sort_order = sort_order;
                }
            }
            GldfAction::MoveVariant { index, delta } => {
                if let Some(vs) = self.product.product_definitions.variants.as_mut() {
                    let len = vs.variant.len() as i32;
                    let from = index as i32;
                    let to = from + delta;
                    if to >= 0 && to < len && from < len {
                        let item = vs.variant.remove(index);
                        vs.variant.insert(to as usize, item);
                    } else {
                        // Out-of-bounds → keep state but don't flip modified.
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::SetVariantSymbol { index, file_id } => {
                if let Some(v) = self
                    .product
                    .product_definitions
                    .variants
                    .as_mut()
                    .and_then(|vs| vs.variant.get_mut(index))
                {
                    v.symbol = file_id
                        .filter(|s| !s.is_empty())
                        .map(|fid| gldf_rs::gldf::product_definitions::Symbol { file_id: fid });
                }
            }
            GldfAction::SetVariantGeometryEmitterRefs {
                variant_idx,
                emitter_ids,
            } => {
                if let Some(v) = self
                    .product
                    .product_definitions
                    .variants
                    .as_mut()
                    .and_then(|vs| vs.variant.get_mut(variant_idx))
                {
                    if emitter_ids.is_empty() {
                        v.geometry = None;
                    } else {
                        use gldf_rs::gldf::product_definitions::{
                            EmitterReferenceBare, Geometry,
                        };
                        v.geometry = Some(Geometry {
                            emitter_reference: emitter_ids
                                .into_iter()
                                .map(|eid| EmitterReferenceBare { emitter_id: eid })
                                .collect(),
                            simple_geometry_reference: None,
                            model_geometry_reference: None,
                        });
                    }
                } else {
                    self.last_error = Some(format!(
                        "SetVariantGeometryEmitterRefs: variant {} out of range",
                        variant_idx
                    ));
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::SetVariantGeometrySimple {
                variant_idx,
                geometry_id,
                emitter_id,
            } => {
                if let Some(v) = self
                    .product
                    .product_definitions
                    .variants
                    .as_mut()
                    .and_then(|vs| vs.variant.get_mut(variant_idx))
                {
                    use gldf_rs::gldf::product_definitions::{Geometry, SimpleGeometryReference};
                    v.geometry = Some(Geometry {
                        emitter_reference: Vec::new(),
                        simple_geometry_reference: Some(SimpleGeometryReference {
                            geometry_id,
                            emitter_id,
                        }),
                        model_geometry_reference: None,
                    });
                } else {
                    self.last_error = Some(format!(
                        "SetVariantGeometrySimple: variant {} out of range",
                        variant_idx
                    ));
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::SetVariantGeometryModel {
                variant_idx,
                geometry_id,
                bindings,
            } => {
                if let Some(v) = self
                    .product
                    .product_definitions
                    .variants
                    .as_mut()
                    .and_then(|vs| vs.variant.get_mut(variant_idx))
                {
                    use gldf_rs::gldf::product_definitions::{
                        EmitterReference, Geometry, ModelGeometryReference,
                    };
                    v.geometry = Some(Geometry {
                        emitter_reference: Vec::new(),
                        simple_geometry_reference: None,
                        model_geometry_reference: Some(ModelGeometryReference {
                            geometry_id,
                            emitter_reference: bindings
                                .into_iter()
                                .map(|b| EmitterReference {
                                    emitter_id: b.emitter_id,
                                    target_model_type: b.target_model_type,
                                    emitter_object_external_name: b.external_name,
                                })
                                .collect(),
                        }),
                    });
                } else {
                    self.last_error = Some(format!(
                        "SetVariantGeometryModel: variant {} out of range",
                        variant_idx
                    ));
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::ClearVariantGeometry { variant_idx } => {
                if let Some(v) = self
                    .product
                    .product_definitions
                    .variants
                    .as_mut()
                    .and_then(|vs| vs.variant.get_mut(variant_idx))
                {
                    v.geometry = None;
                } else {
                    self.last_error = Some(format!(
                        "ClearVariantGeometry: variant {} out of range",
                        variant_idx
                    ));
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::ImportL3dFile { bytes, filename } => {
                use gldf_rs::gldf::general_definitions::files::File as GldfFile;
                use gldf_rs::gldf::general_definitions::geometries::{
                    GeometryFileReference, ModelGeometry,
                };
                // Auto-generate file id + geometry id; embed bytes;
                // register both entries. Mirrors the existing
                // AddFileWithBytes flow but for the L3D / ModelGeometry
                // pair the wizard's Geometry step needs.
                let file_id = self.product.generate_unique_id("file");
                let push_file = self.product.add_file(GldfFile {
                    id: file_id.clone(),
                    content_type: "geo/l3d".to_string(),
                    type_attr: "localFileName".to_string(),
                    language: String::new(),
                    file_name: filename.clone(),
                });
                if let Err(e) = push_file {
                    self.last_error = Some(format!("ImportL3dFile (add_file): {}", e));
                    self.is_modified = prev_modified;
                } else {
                    self.embedded_files.insert(file_id.clone(), bytes);
                    let geom_id = self.product.generate_unique_id("geometry");
                    let push_geom = self.product.add_model_geometry(ModelGeometry {
                        id: geom_id,
                        geometry_file_reference: vec![GeometryFileReference {
                            file_id: file_id.clone(),
                            level_of_detail: None,
                        }],
                    });
                    if let Err(e) = push_geom {
                        self.last_error =
                            Some(format!("ImportL3dFile (add_model_geometry): {}", e));
                        self.is_modified = prev_modified;
                    } else {
                        self.last_error = None;
                    }
                }
            }
            GldfAction::AddSimpleGeometryCuboid {
                width_mm,
                length_mm,
                height_mm,
            } => {
                use gldf_rs::gldf::general_definitions::geometries::{Cuboid, SimpleGeometry};
                let id = self.product.generate_unique_id("geometry");
                let result = self.product.add_simple_geometry(SimpleGeometry {
                    id,
                    cuboid: vec![Cuboid {
                        width: vec![width_mm],
                        length: vec![length_mm],
                        height: vec![height_mm],
                    }],
                    ..Default::default()
                });
                if let Err(e) = result {
                    self.last_error = Some(format!("Add simple geometry: {}", e));
                    self.is_modified = prev_modified;
                } else {
                    self.last_error = None;
                }
            }
            GldfAction::AddSimpleGeometryCylinder {
                plane,
                diameter_mm,
                height_mm,
            } => {
                use gldf_rs::gldf::general_definitions::geometries::{Cylinder, SimpleGeometry};
                let id = self.product.generate_unique_id("geometry");
                let result = self.product.add_simple_geometry(SimpleGeometry {
                    id,
                    cylinder: vec![Cylinder {
                        plane,
                        diameter: vec![diameter_mm],
                        height: vec![height_mm.to_string()],
                    }],
                    ..Default::default()
                });
                if let Err(e) = result {
                    self.last_error = Some(format!("Add simple geometry: {}", e));
                    self.is_modified = prev_modified;
                } else {
                    self.last_error = None;
                }
            }
            GldfAction::RemoveSimpleGeometry { id } => {
                if let Some(geoms) = self.product.general_definitions.geometries.as_mut() {
                    let before = geoms.simple_geometry.len();
                    geoms.simple_geometry.retain(|g| g.id != id);
                    if geoms.simple_geometry.len() == before {
                        self.last_error = Some(format!("SimpleGeometry '{}' not found", id));
                        self.is_modified = prev_modified;
                    } else {
                        self.last_error = None;
                    }
                } else {
                    self.last_error = Some("No geometries block present".to_string());
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::ImportSpectrumFile { bytes, filename } => {
                use gldf_rs::gldf::general_definitions::files::File as GldfFile;
                use gldf_rs::gldf::general_definitions::photometries::{
                    Spectrum, SpectrumFileReference,
                };
                // Embed the .spd bytes as a <File>, then auto-register
                // a matching <Spectrum>. Mirrors ImportL3dFile. Does
                // NOT bind to a light source — the LightSources page
                // picker handles binding once the spectrum exists.
                let file_id = self.product.generate_unique_id("spectrumFile");
                let push_file = self.product.add_file(GldfFile {
                    id: file_id.clone(),
                    content_type: "spectrum/text".to_string(),
                    type_attr: "localFileName".to_string(),
                    language: String::new(),
                    file_name: filename.clone(),
                });
                if let Err(e) = push_file {
                    self.last_error = Some(format!("ImportSpectrumFile (add_file): {}", e));
                    self.is_modified = prev_modified;
                } else {
                    self.embedded_files.insert(file_id.clone(), bytes);
                    let spectrum_id = self.product.generate_unique_id("spectrum");
                    let push_spectrum = self.product.add_spectrum(Spectrum {
                        id: spectrum_id,
                        spectrum_file_reference: Some(SpectrumFileReference { file_id }),
                        ..Default::default()
                    });
                    if let Err(e) = push_spectrum {
                        self.last_error =
                            Some(format!("ImportSpectrumFile (add_spectrum): {}", e));
                        self.is_modified = prev_modified;
                    } else {
                        self.last_error = None;
                    }
                }
            }
            GldfAction::ReplacePhotometryFile {
                photometry_id,
                bytes,
                filename,
            } => {
                let mut editable = self.to_editable();
                match editable.replace_photometry_file(&photometry_id, bytes, &filename) {
                    Ok(()) => {
                        self.take_from_editable(editable);
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Replace photometry: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::RenameFixedLightSource { old_id, new_id } => {
                match self.product.rename_fixed_light_source(&old_id, &new_id) {
                    Ok(()) => {
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Rename light source: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::RenameChangeableLightSource { old_id, new_id } => {
                match self.product.rename_changeable_light_source(&old_id, &new_id) {
                    Ok(()) => {
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Rename changeable light source: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::RenameEmitter { old_id, new_id } => {
                match self.product.rename_emitter(&old_id, &new_id) {
                    Ok(()) => self.last_error = None,
                    Err(e) => {
                        self.last_error = Some(format!("Rename emitter: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::RenameMultiChannelLightSource { old_id, new_id } => {
                match self.product.rename_multi_channel_light_source(&old_id, &new_id) {
                    Ok(()) => self.last_error = None,
                    Err(e) => {
                        self.last_error = Some(format!("Rename multi-channel light source: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::AddMultiChannelLightSource { payload } => {
                let mut ls = *payload;
                if ls.id.is_empty() {
                    ls.id = self.product.generate_unique_id("lightsource");
                }
                match self.product.add_multi_channel_light_source(ls) {
                    Ok(()) => self.last_error = None,
                    Err(e) => {
                        self.last_error = Some(format!("Add multi-channel light source: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::UpdateMultiChannelLightSource { id, payload } => {
                match self.product.update_multi_channel_light_source(&id, *payload) {
                    Ok(()) => self.last_error = None,
                    Err(e) => {
                        self.last_error = Some(format!("Update multi-channel light source: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::AddMultiChannelChannel { ls_id, channel } => {
                let Some(ls_collection) = self
                    .product
                    .general_definitions
                    .light_sources
                    .as_mut()
                else {
                    self.last_error = Some("Add channel: no light sources".to_string());
                    self.is_modified = prev_modified;
                    return;
                };
                let Some(mc) = ls_collection
                    .multi_channel_light_source
                    .iter_mut()
                    .find(|m| m.id == ls_id)
                else {
                    self.last_error = Some(format!(
                        "Add channel: no MultiChannelLightSource with id '{}'",
                        ls_id
                    ));
                    self.is_modified = prev_modified;
                    return;
                };
                mc.channels.channel.push(*channel);
                self.last_error = None;
            }
            GldfAction::RemoveMultiChannelChannel { ls_id, index } => {
                let Some(ls_collection) = self
                    .product
                    .general_definitions
                    .light_sources
                    .as_mut()
                else {
                    self.last_error = Some("Remove channel: no light sources".to_string());
                    self.is_modified = prev_modified;
                    return;
                };
                let Some(mc) = ls_collection
                    .multi_channel_light_source
                    .iter_mut()
                    .find(|m| m.id == ls_id)
                else {
                    self.last_error = Some(format!(
                        "Remove channel: no MultiChannelLightSource with id '{}'",
                        ls_id
                    ));
                    self.is_modified = prev_modified;
                    return;
                };
                if index < mc.channels.channel.len() {
                    mc.channels.channel.remove(index);
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!(
                        "Remove channel: index {} out of range on MultiChannel '{}'",
                        index, ls_id
                    ));
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::UpdateMultiChannelChannel { ls_id, index, channel } => {
                let Some(ls_collection) = self
                    .product
                    .general_definitions
                    .light_sources
                    .as_mut()
                else {
                    self.last_error = Some("Update channel: no light sources".to_string());
                    self.is_modified = prev_modified;
                    return;
                };
                let Some(mc) = ls_collection
                    .multi_channel_light_source
                    .iter_mut()
                    .find(|m| m.id == ls_id)
                else {
                    self.last_error = Some(format!(
                        "Update channel: no MultiChannelLightSource with id '{}'",
                        ls_id
                    ));
                    self.is_modified = prev_modified;
                    return;
                };
                if index < mc.channels.channel.len() {
                    mc.channels.channel[index] = *channel;
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!(
                        "Update channel: index {} out of range on MultiChannel '{}'",
                        index, ls_id
                    ));
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::AddFixedLightSource { payload } => {
                let mut ls = *payload;
                if ls.id.is_empty() {
                    ls.id = self.product.generate_unique_id("lightsource");
                }
                match self.product.add_fixed_light_source(ls) {
                    Ok(()) => self.last_error = None,
                    Err(e) => {
                        self.last_error = Some(format!("Add fixed light source: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::AddChangeableLightSource { payload } => {
                let mut ls = *payload;
                if ls.id.is_empty() {
                    ls.id = self.product.generate_unique_id("lightsource");
                }
                match self.product.add_changeable_light_source(ls) {
                    Ok(()) => self.last_error = None,
                    Err(e) => {
                        self.last_error = Some(format!("Add changeable light source: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::UpdateFixedLightSource { id, payload } => {
                match self.product.update_fixed_light_source(&id, *payload) {
                    Ok(()) => self.last_error = None,
                    Err(e) => {
                        self.last_error = Some(format!("Update fixed light source: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::UpdateChangeableLightSource { id, payload } => {
                match self.product.update_changeable_light_source(&id, *payload) {
                    Ok(()) => self.last_error = None,
                    Err(e) => {
                        self.last_error = Some(format!("Update changeable light source: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::RemoveLightSource { id } => {
                // Try Fixed → Changeable → MultiChannel; the three live in
                // the same id namespace so any one of them may own the id.
                if self.product.remove_fixed_light_source(&id).is_ok() {
                    self.last_error = None;
                } else if self.product.remove_changeable_light_source(&id).is_ok() {
                    self.last_error = None;
                } else if self.product.remove_multi_channel_light_source(&id).is_ok() {
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!(
                        "Remove light source: no light source with id '{}'",
                        id
                    ));
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::AddEmitter => {
                use gldf_rs::gldf::general_definitions::lightsources::Emitter;
                let id = self.product.generate_unique_id("emitter");
                let emitter = Emitter {
                    id,
                    ..Default::default()
                };
                match self.product.add_emitter(emitter) {
                    Ok(()) => self.last_error = None,
                    Err(e) => {
                        self.last_error = Some(format!("Add emitter: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::AddEmitterWithChild {
                child_name,
                kind,
                photometry_id,
                fixed_light_source_id,
                multi_channel_light_source_id,
                sensor_id,
            } => {
                use gldf_rs::gldf::general_definitions::lightsources::{
                    ChangeableLightEmitter, Emitter, FixedLightEmitter, LightSourceReference,
                    MultiChannelLightEmitter, MultiChannelLightSourceReference,
                    PhotometryReference, SensorEmitter, SensorReference,
                };
                use gldf_rs::gldf::header::{Locale, LocaleFoo};
                let id = self.product.generate_unique_id("emitter");
                let name_locale = child_name.as_ref().filter(|s| !s.trim().is_empty()).map(|n| {
                    LocaleFoo {
                        locale: vec![Locale {
                            language: String::new(),
                            value: n.clone(),
                        }],
                    }
                });
                let mut emitter = Emitter {
                    id,
                    ..Default::default()
                };
                match kind {
                    LightEmitterKind::Fixed => {
                        emitter.fixed_light_emitter.push(FixedLightEmitter {
                            name: name_locale,
                            photometry_reference: PhotometryReference { photometry_id },
                            light_source_reference: LightSourceReference {
                                fixed_light_source_id: Some(fixed_light_source_id),
                                changeable_light_source_id: None,
                                light_source_count: None,
                            },
                            // XSD requires RatedLuminousFlux on FixedLightEmitter.
                            // Default to 0 so the file stays schema-valid; the
                            // user can fix it via the future per-row editor.
                            rated_luminous_flux: Some(0),
                            ..Default::default()
                        });
                    }
                    LightEmitterKind::Changeable => {
                        emitter.changeable_light_emitter.push(ChangeableLightEmitter {
                            name: name_locale,
                            photometry_reference: PhotometryReference { photometry_id },
                            ..Default::default()
                        });
                    }
                    LightEmitterKind::MultiChannel => {
                        emitter.multi_channel_light_emitter.push(MultiChannelLightEmitter {
                            name: name_locale,
                            light_source_reference: MultiChannelLightSourceReference {
                                multi_channel_light_source_id,
                            },
                            ..Default::default()
                        });
                    }
                    LightEmitterKind::Sensor => {
                        // SensorEmitter has no <Name> in the schema; the
                        // child_name is dropped for this kind.
                        let _ = name_locale;
                        let _ = photometry_id;
                        emitter.sensor_emitter.push(SensorEmitter {
                            sensor_reference: SensorReference { sensor_id },
                        });
                    }
                }
                match self.product.add_emitter(emitter) {
                    Ok(()) => self.last_error = None,
                    Err(e) => {
                        self.last_error = Some(format!("Add emitter with child: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::RemoveEmitter { id } => {
                match self.product.remove_emitter(&id) {
                    Ok(_) => self.last_error = None,
                    Err(e) => {
                        self.last_error = Some(format!("Remove emitter: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::AddFixedLightEmitter {
                emitter_id,
                photometry_id,
                light_source_id,
            } => {
                use gldf_rs::gldf::general_definitions::lightsources::{
                    FixedLightEmitter, LightSourceReference, PhotometryReference,
                };
                let Some(emitter) = self.product.get_emitter_mut(&emitter_id) else {
                    self.last_error = Some(format!("Emitter '{}' not found", emitter_id));
                    self.is_modified = prev_modified;
                    return;
                };
                emitter.fixed_light_emitter.push(FixedLightEmitter {
                    photometry_reference: PhotometryReference { photometry_id },
                    light_source_reference: LightSourceReference {
                        fixed_light_source_id: Some(light_source_id),
                        changeable_light_source_id: None,
                        light_source_count: None,
                    },
                    // FixedLightEmitter requires RatedLuminousFlux per the
                    // rc.3 XSD (no minOccurs=0). Default to 0 so the file
                    // remains schema-valid; the user can fix the value via
                    // a later editor.
                    rated_luminous_flux: Some(0),
                    ..Default::default()
                });
                self.last_error = None;
            }
            GldfAction::AddChangeableLightEmitter {
                emitter_id,
                photometry_id,
            } => {
                use gldf_rs::gldf::general_definitions::lightsources::{
                    ChangeableLightEmitter, PhotometryReference,
                };
                let Some(emitter) = self.product.get_emitter_mut(&emitter_id) else {
                    self.last_error = Some(format!("Emitter '{}' not found", emitter_id));
                    self.is_modified = prev_modified;
                    return;
                };
                emitter.changeable_light_emitter.push(ChangeableLightEmitter {
                    photometry_reference: PhotometryReference { photometry_id },
                    ..Default::default()
                });
                self.last_error = None;
            }
            GldfAction::AddMultiChannelLightEmitter {
                emitter_id,
                multi_channel_light_source_id,
            } => {
                use gldf_rs::gldf::general_definitions::lightsources::{
                    MultiChannelLightEmitter, MultiChannelLightSourceReference,
                };
                let Some(emitter) = self.product.get_emitter_mut(&emitter_id) else {
                    self.last_error = Some(format!("Emitter '{}' not found", emitter_id));
                    self.is_modified = prev_modified;
                    return;
                };
                emitter.multi_channel_light_emitter.push(MultiChannelLightEmitter {
                    light_source_reference: MultiChannelLightSourceReference {
                        multi_channel_light_source_id,
                    },
                    ..Default::default()
                });
                self.last_error = None;
            }
            GldfAction::AddSensorEmitter {
                emitter_id,
                sensor_id,
            } => {
                use gldf_rs::gldf::general_definitions::lightsources::{
                    SensorEmitter, SensorReference,
                };
                let Some(emitter) = self.product.get_emitter_mut(&emitter_id) else {
                    self.last_error = Some(format!("Emitter '{}' not found", emitter_id));
                    self.is_modified = prev_modified;
                    return;
                };
                emitter.sensor_emitter.push(SensorEmitter {
                    sensor_reference: SensorReference { sensor_id },
                });
                self.last_error = None;
            }
            GldfAction::RemoveLightEmitter {
                emitter_id,
                kind,
                index,
            } => {
                let Some(emitter) = self.product.get_emitter_mut(&emitter_id) else {
                    self.last_error = Some(format!("Emitter '{}' not found", emitter_id));
                    self.is_modified = prev_modified;
                    return;
                };
                let ok = match kind {
                    LightEmitterKind::Fixed => {
                        if index < emitter.fixed_light_emitter.len() {
                            emitter.fixed_light_emitter.remove(index);
                            true
                        } else {
                            false
                        }
                    }
                    LightEmitterKind::Changeable => {
                        if index < emitter.changeable_light_emitter.len() {
                            emitter.changeable_light_emitter.remove(index);
                            true
                        } else {
                            false
                        }
                    }
                    LightEmitterKind::MultiChannel => {
                        if index < emitter.multi_channel_light_emitter.len() {
                            emitter.multi_channel_light_emitter.remove(index);
                            true
                        } else {
                            false
                        }
                    }
                    LightEmitterKind::Sensor => {
                        if index < emitter.sensor_emitter.len() {
                            emitter.sensor_emitter.remove(index);
                            true
                        } else {
                            false
                        }
                    }
                };
                if ok {
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!(
                        "Remove LightEmitter: index {} out of range on emitter '{}'",
                        index, emitter_id
                    ));
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::SetFixedLightEmitterPhotometry {
                emitter_id,
                index,
                photometry_id,
            } => {
                let Some(emitter) = self.product.get_emitter_mut(&emitter_id) else {
                    self.last_error = Some(format!("Emitter '{}' not found", emitter_id));
                    self.is_modified = prev_modified;
                    return;
                };
                let Some(fle) = emitter.fixed_light_emitter.get_mut(index) else {
                    self.last_error = Some(format!(
                        "Fixed light emitter index {} out of range on emitter '{}'",
                        index, emitter_id
                    ));
                    self.is_modified = prev_modified;
                    return;
                };
                fle.photometry_reference.photometry_id = photometry_id;
                self.last_error = None;
            }
            GldfAction::SetFixedLightEmitterFlux {
                emitter_id,
                index,
                rated_luminous_flux,
            } => {
                let Some(emitter) = self.product.get_emitter_mut(&emitter_id) else {
                    self.last_error = Some(format!("Emitter '{}' not found", emitter_id));
                    self.is_modified = prev_modified;
                    return;
                };
                let Some(fle) = emitter.fixed_light_emitter.get_mut(index) else {
                    self.last_error = Some(format!(
                        "Fixed light emitter index {} out of range on emitter '{}'",
                        index, emitter_id
                    ));
                    self.is_modified = prev_modified;
                    return;
                };
                fle.rated_luminous_flux = Some(rated_luminous_flux);
                self.last_error = None;
            }
            GldfAction::SetFixedLightEmitterLightSource {
                emitter_id,
                index,
                light_source_id,
            } => {
                let Some(emitter) = self.product.get_emitter_mut(&emitter_id) else {
                    self.last_error = Some(format!("Emitter '{}' not found", emitter_id));
                    self.is_modified = prev_modified;
                    return;
                };
                let Some(fle) = emitter.fixed_light_emitter.get_mut(index) else {
                    self.last_error = Some(format!(
                        "Fixed light emitter index {} out of range on emitter '{}'",
                        index, emitter_id
                    ));
                    self.is_modified = prev_modified;
                    return;
                };
                fle.light_source_reference.fixed_light_source_id = light_source_id;
                fle.light_source_reference.changeable_light_source_id = None;
                self.last_error = None;
            }
            GldfAction::SetFixedLightEmitterEmergencyBehaviour {
                emitter_id,
                index,
                behaviour,
            } => {
                let Some(emitter) = self.product.get_emitter_mut(&emitter_id) else {
                    self.last_error = Some(format!("Emitter '{}' not found", emitter_id));
                    self.is_modified = prev_modified;
                    return;
                };
                let Some(fle) = emitter.fixed_light_emitter.get_mut(index) else {
                    self.last_error = Some(format!(
                        "Fixed light emitter index {} out of range on emitter '{}'",
                        index, emitter_id
                    ));
                    self.is_modified = prev_modified;
                    return;
                };
                // Empty string → clear so we don't persist `<EmergencyBehaviour/>`
                // with empty content. Any other value rides through as-is;
                // XSD-enum policing happens elsewhere (canonical validator).
                fle.emergency_behaviour = behaviour.and_then(|s| {
                    let s = s.trim().to_string();
                    if s.is_empty() {
                        None
                    } else {
                        Some(s)
                    }
                });
                self.last_error = None;
            }
            GldfAction::SetChangeableLightEmitterPhotometry {
                emitter_id,
                index,
                photometry_id,
            } => {
                let Some(emitter) = self.product.get_emitter_mut(&emitter_id) else {
                    self.last_error = Some(format!("Emitter '{}' not found", emitter_id));
                    self.is_modified = prev_modified;
                    return;
                };
                let Some(cle) = emitter.changeable_light_emitter.get_mut(index) else {
                    self.last_error = Some(format!(
                        "Changeable light emitter index {} out of range on emitter '{}'",
                        index, emitter_id
                    ));
                    self.is_modified = prev_modified;
                    return;
                };
                cle.photometry_reference.photometry_id = photometry_id;
                self.last_error = None;
            }
            GldfAction::SetActiveLanguage(lang) => {
                self.active_language = lang;
                // Language switch alone does not modify the product.
                self.is_modified = prev_modified;
            }
            GldfAction::SetUiLanguage(lang) => {
                self.lang_store.store(&lang);
                self.ui_language = lang;
                self.is_modified = prev_modified;
            }
            GldfAction::SetAuthor(author) => {
                self.product.header.author = author;
            }
            GldfAction::SetManufacturer(manufacturer) => {
                self.product.header.manufacturer = manufacturer;
            }
            GldfAction::SetCreationTimeCode(time_code) => {
                self.product.header.creation_time_code = time_code;
            }
            GldfAction::SetCreatedWithApplication(app) => {
                self.product.header.created_with_application = app;
            }
            GldfAction::SetDefaultLanguage(lang) => {
                self.product.header.default_language = lang;
            }
            GldfAction::SetFormatVersion(version) => {
                self.product.header.format_version = FormatVersion::from_string(&version);
            }
            GldfAction::AddFile {
                id,
                content_type,
                type_attr,
                file_name,
                language,
            } => {
                use gldf_rs::gldf::general_definitions::files::File;
                self.product.general_definitions.files.file.push(File {
                    id,
                    content_type,
                    type_attr,
                    file_name,
                    language: language.unwrap_or_default(),
                });
            }
            GldfAction::RenameFile { old_id, new_id } => {
                match self.product.rename_file(&old_id, &new_id) {
                    Ok(()) => {
                        // Move the embedded bytes to the new key so the
                        // zip writer's filename and the XML `<File>` id
                        // stay in lock-step.
                        if let Some(bytes) = self.embedded_files.remove(&old_id) {
                            self.embedded_files.insert(new_id.clone(), bytes);
                        }
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Rename file: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::AddFileWithBytes { bytes, filename } => {
                use gldf_rs::gldf::general_definitions::files::File;
                let lower = filename.to_lowercase();
                // Conservative sniff — matches the typed drop helpers
                // (.ldt → ldc/eulumdat, .ies → ldc/ies, .spd → spectrum/text,
                // .l3d/.m3d/.r3d → geo/*, images → image/<ext>, .pdf →
                // document/pdf). Anything else falls through to "other"
                // — the user can edit the content_type inline afterwards.
                let content_type = if lower.ends_with(".ies") { "ldc/ies" }
                    else if lower.ends_with(".ldt") { "ldc/eulumdat" }
                    else if lower.ends_with(".spd") { "spectrum/text" }
                    else if lower.ends_with(".l3d") { "geo/l3d" }
                    else if lower.ends_with(".m3d") { "geo/m3d" }
                    else if lower.ends_with(".r3d") { "geo/r3d" }
                    else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") { "image/jpeg" }
                    else if lower.ends_with(".png") { "image/png" }
                    else if lower.ends_with(".pdf") { "document/pdf" }
                    else { "other" };
                let id = self.product.generate_unique_id("file");
                let push_result = self.product.add_file(File {
                    id: id.clone(),
                    content_type: content_type.to_string(),
                    type_attr: "localFileName".to_string(),
                    language: String::new(),
                    file_name: filename.clone(),
                });
                match push_result {
                    Ok(()) => {
                        self.embedded_files.insert(id, bytes);
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(format!("Add file: {}", e));
                        self.is_modified = prev_modified;
                    }
                }
            }
            GldfAction::RemoveFile(id) => {
                self
                    .product
                    .general_definitions
                    .files
                    .file
                    .retain(|f| f.id != id);
                // Also drop the embedded bytes; otherwise a re-add with the
                // same id would silently reuse stale content.
                self.embedded_files.remove(&id);
            }
            GldfAction::UpdateFile {
                id,
                content_type,
                type_attr,
                file_name,
            } => {
                if let Some(file) = self
                    .product
                    .general_definitions
                    .files
                    .file
                    .iter_mut()
                    .find(|f| f.id == id)
                {
                    file.content_type = content_type;
                    file.type_attr = type_attr;
                    file.file_name = file_name;
                }
            }
            // --- Electrical attributes ---
            GldfAction::SetElectricalSafetyClass(scope, value) => {
                if let Some(e) = ensure_electrical(self, scope) {
                    e.electrical_safety_class = value;
                }
            }
            GldfAction::SetIngressProtectionIPCode(scope, value) => {
                if let Some(e) = ensure_electrical(self, scope) {
                    e.ingress_protection_ip_code = value;
                }
            }
            GldfAction::SetPowerFactor(scope, value) => {
                if let Some(e) = ensure_electrical(self, scope) {
                    e.power_factor = value;
                }
            }
            GldfAction::SetConstantLightOutput(scope, value) => {
                if let Some(e) = ensure_electrical(self, scope) {
                    e.constant_light_output = value;
                }
            }
            GldfAction::SetLightDistribution(scope, value) => {
                if let Some(e) = ensure_electrical(self, scope) {
                    e.light_distribution = value;
                }
            }
            GldfAction::SetSwitchingCapacity(scope, value) => {
                if let Some(e) = ensure_electrical(self, scope) {
                    e.switching_capacity = value;
                }
            }
            // --- Applications ---
            GldfAction::SetApplications(scope, apps) => {
                if let Some(a) = ensure_applications(self, scope) {
                    a.application = apps;
                }
            }
            GldfAction::AddApplication(scope, app) => {
                if let Some(a) = ensure_applications(self, scope) {
                    a.application.push(app);
                }
            }
            GldfAction::RemoveApplication(scope, index) => {
                if let Some(a) = ensure_applications(self, scope) {
                    if index < a.application.len() {
                        a.application.remove(index);
                    }
                }
            }
            // --- Labels ---
            GldfAction::AddLabel(scope, label) => {
                if let Some(l) = ensure_labels(self, scope) {
                    if !l.label.iter().any(|x| x == &label) {
                        l.label.push(label);
                    }
                }
            }
            GldfAction::RemoveLabel(scope, index) => {
                if let Some(l) = ensure_labels(self, scope) {
                    if index < l.label.len() {
                        l.label.remove(index);
                    }
                }
            }
            // --- Photometry ---
            GldfAction::SetPhotometryCieFluxCode { index, value } => {
                ensure_descriptive_photometry(self, index).cie_flux_code = value;
            }
            GldfAction::SetPhotometryLightOutputRatio { index, value } => {
                ensure_descriptive_photometry(self, index).light_output_ratio = value;
            }
            GldfAction::SetPhotometryLuminousEfficacy { index, value } => {
                ensure_descriptive_photometry(self, index).luminous_efficacy = value;
            }
            GldfAction::SetPhotometryDownwardFluxFraction { index, value } => {
                ensure_descriptive_photometry(self, index).downward_flux_fraction = value;
            }
            GldfAction::SetPhotometryDownwardLOR { index, value } => {
                ensure_descriptive_photometry(self, index).downward_light_output_ratio =
                    value;
            }
            GldfAction::SetPhotometryUpwardLOR { index, value } => {
                ensure_descriptive_photometry(self, index).upward_light_output_ratio =
                    value;
            }
            GldfAction::SetPhotometryCutOffAngle { index, value } => {
                ensure_descriptive_photometry(self, index).cut_off_angle = value;
            }
            GldfAction::SetPhotometryLuminaireLuminance { index, value } => {
                ensure_descriptive_photometry(self, index).luminaire_luminance = value;
            }
            GldfAction::SetPhotometryUgrX { index, value } => {
                ensure_ugr(self, index).x = value;
            }
            GldfAction::SetPhotometryUgrY { index, value } => {
                ensure_ugr(self, index).y = value;
            }
            GldfAction::SetPhotometryPhotometricCode { index, value } => {
                ensure_descriptive_photometry(self, index).photometric_code = value;
            }
            GldfAction::SetPhotometryBugRating { index, value } => {
                ensure_descriptive_photometry(self, index)
                    .light_distribution_bug_rating = value;
            }
            GldfAction::SetProductMetaName(value) => {
                ensure_product_meta_data(self).name = Some(value);
            }
            GldfAction::SetProductMetaDescription(value) => {
                ensure_product_meta_data(self).description = Some(value);
            }
            GldfAction::SetProductMetaProductNumber(value) => {
                ensure_product_meta_data(self).product_number = Some(value);
            }
            GldfAction::SetProductMetaTenderText(value) => {
                ensure_product_meta_data(self).tender_text = Some(value);
            }
            GldfAction::SetVariantName(idx, value) => {
                if let Some(v) = variant_at_mut(self, idx) {
                    v.name = Some(value);
                }
            }
            GldfAction::SetVariantDescription(idx, value) => {
                if let Some(v) = variant_at_mut(self, idx) {
                    v.description = Some(value);
                }
            }
            GldfAction::SetVariantProductNumber(idx, value) => {
                if let Some(v) = variant_at_mut(self, idx) {
                    v.product_number = Some(value);
                }
            }
            GldfAction::SetVariantTenderText(idx, value) => {
                if let Some(v) = variant_at_mut(self, idx) {
                    v.tender_text = Some(value);
                }
            }
            GldfAction::SetFixedLightSourceName(idx, value) => {
                if let Some(s) = fixed_light_source_at_mut(self, idx) {
                    s.name = value;
                }
            }
            GldfAction::SetFixedLightSourceDescription(idx, value) => {
                if let Some(s) = fixed_light_source_at_mut(self, idx) {
                    s.description = Some(value);
                }
            }
            GldfAction::SetChangeableLightSourceName(idx, value) => {
                if let Some(s) = changeable_light_source_at_mut(self, idx) {
                    s.name = value;
                }
            }
            GldfAction::SetChangeableLightSourceDescription(idx, value) => {
                if let Some(s) = changeable_light_source_at_mut(self, idx) {
                    s.description = Some(value);
                }
            }
            GldfAction::SetMechanicalIKRating(scope, value) => {
                if let Some(m) = ensure_mechanical(self, scope) {
                    m.ik_rating = value;
                }
            }
            GldfAction::SetMechanicalProductForm(scope, value) => {
                if let Some(m) = ensure_mechanical(self, scope) {
                    m.product_form = value;
                }
            }
            GldfAction::SetMechanicalWeight(scope, value) => {
                if let Some(m) = ensure_mechanical(self, scope) {
                    m.weight = value;
                }
            }
            GldfAction::SetMechanicalLength(scope, value) => {
                if let Some(s) = ensure_mechanical_size(self, scope) {
                    s.length = value.unwrap_or(0);
                }
            }
            GldfAction::SetMechanicalWidth(scope, value) => {
                if let Some(s) = ensure_mechanical_size(self, scope) {
                    s.width = value.unwrap_or(0);
                }
            }
            GldfAction::SetMechanicalHeight(scope, value) => {
                if let Some(s) = ensure_mechanical_size(self, scope) {
                    s.height = value.unwrap_or(0);
                }
            }
            GldfAction::SetMechanicalSealingMaterial(scope, value) => {
                if let Some(m) = ensure_mechanical(self, scope) {
                    m.sealing_material = if value.locale.is_empty() {
                        None
                    } else {
                        Some(value)
                    };
                }
            }
            GldfAction::AddProductMetaPicture {
                scope,
                file_id,
                image_type,
            } => {
                if let Some(pics) = ensure_product_meta_pictures(self, scope) {
                    pics.image.push(gldf_rs::gldf::general_definitions::Image {
                        file_id,
                        image_type,
                    });
                }
            }
            GldfAction::UpdateProductMetaPicture {
                scope,
                index,
                file_id,
                image_type,
            } => {
                if let Some(img) = ensure_product_meta_pictures(self, scope)
                    .and_then(|p| p.image.get_mut(index))
                {
                    img.file_id = file_id;
                    img.image_type = image_type;
                }
            }
            GldfAction::RemoveProductMetaPicture { scope, index } => {
                let pics_opt: Option<&mut gldf_rs::gldf::general_definitions::Images> =
                    match scope {
                        Scope::Product => self
                            .product
                            .product_definitions
                            .product_meta_data
                            .as_mut()
                            .and_then(|m| m.pictures.as_mut()),
                        Scope::Variant(i) => self
                            .product
                            .product_definitions
                            .variants
                            .as_mut()
                            .and_then(|vs| vs.variant.get_mut(i))
                            .and_then(|v| v.pictures.as_mut()),
                    };
                if let Some(pics) = pics_opt {
                    if index < pics.image.len() {
                        pics.image.remove(index);
                    }
                }
            }
            GldfAction::AddHyperlink(scope) => {
                if let Some(hs) = ensure_hyperlinks(self, scope) {
                    hs.hyperlink.push(gldf_rs::gldf::Hyperlink::default());
                }
            }
            GldfAction::UpdateHyperlink {
                scope,
                index,
                href,
                language,
                region,
                country_code,
                value,
            } => {
                if let Some(h) = ensure_hyperlinks(self, scope)
                    .and_then(|hs| hs.hyperlink.get_mut(index))
                {
                    h.href = href;
                    h.language = language;
                    h.region = region;
                    h.country_code = country_code;
                    h.value = value;
                }
            }
            GldfAction::RemoveHyperlink { scope, index } => {
                let hs_opt: Option<&mut gldf_rs::gldf::Hyperlinks> = match scope {
                    Scope::Product => self
                        .product
                        .product_definitions
                        .product_meta_data
                        .as_mut()
                        .and_then(|m| m.descriptive_attributes.as_mut())
                        .and_then(|a| a.marketing.as_mut())
                        .and_then(|m| m.hyperlinks.as_mut()),
                    Scope::Variant(i) => self
                        .product
                        .product_definitions
                        .variants
                        .as_mut()
                        .and_then(|vs| vs.variant.get_mut(i))
                        .and_then(|v| v.descriptive_attributes.as_mut())
                        .and_then(|a| a.marketing.as_mut())
                        .and_then(|m| m.hyperlinks.as_mut()),
                };
                if let Some(hs) = hs_opt {
                    if index < hs.hyperlink.len() {
                        hs.hyperlink.remove(index);
                    }
                }
            }
            GldfAction::AddHousingColor(scope) => {
                use gldf_rs::gldf::header::Locale;
                use gldf_rs::gldf::product_definitions::{HousingColor, HousingColors};
                if let Some(m) = ensure_marketing(self, scope) {
                    if m.housing_colors.is_none() {
                        m.housing_colors = Some(HousingColors::default());
                    }
                    if let Some(hc) = m.housing_colors.as_mut() {
                        hc.housing_color.push(HousingColor {
                            ral: None,
                            locale: Locale::default(),
                        });
                    }
                }
            }
            GldfAction::UpdateHousingColor {
                scope,
                index,
                ral,
                language,
                value,
            } => {
                if let Some(m) = ensure_marketing(self, scope) {
                    if let Some(hc) = m.housing_colors.as_mut() {
                        if let Some(entry) = hc.housing_color.get_mut(index) {
                            entry.ral = ral;
                            entry.locale.language = language;
                            entry.locale.value = value;
                        }
                    }
                }
                // Auto-sync the variant's L3D housing material from the
                // new RAL value. Only fires for variant-scope updates (the
                // product-level HousingColor is a catalog default, not
                // tied to a 3D model). All non-actionable outcomes
                // (no RAL, unknown RAL, no L3D) are silent no-ops; only
                // hard failures surface to `last_error`.
                if let Scope::Variant(variant_idx) = scope {
                    let product = self.product.clone();
                    match gldf_rs::l3d_mtl_sync::sync_l3d_mtl_for_housing_color(
                        &mut self.embedded_files,
                        &product,
                        variant_idx,
                    ) {
                        Ok(outcome) => {
                            log::debug!("L3D MTL sync: {:?}", outcome);
                        }
                        Err(e) => {
                            self.last_error =
                                Some(format!("L3D MTL sync: {}", e));
                        }
                    }
                }
            }
            GldfAction::RemoveHousingColor { scope, index } => {
                if let Some(m) = ensure_marketing(self, scope) {
                    if let Some(hc) = m.housing_colors.as_mut() {
                        if index < hc.housing_color.len() {
                            hc.housing_color.remove(index);
                        }
                    }
                }
            }
            GldfAction::AddMarketRegion(scope) => {
                use gldf_rs::gldf::header::Locale;
                use gldf_rs::gldf::product_definitions::Markets;
                if let Some(m) = ensure_marketing(self, scope) {
                    if m.markets.is_none() {
                        m.markets = Some(Markets::default());
                    }
                    if let Some(mk) = m.markets.as_mut() {
                        mk.region.push(Locale::default());
                    }
                }
            }
            GldfAction::UpdateMarketRegion {
                scope,
                index,
                language,
                value,
            } => {
                if let Some(m) = ensure_marketing(self, scope) {
                    if let Some(mk) = m.markets.as_mut() {
                        if let Some(entry) = mk.region.get_mut(index) {
                            entry.language = language;
                            entry.value = value;
                        }
                    }
                }
            }
            GldfAction::RemoveMarketRegion { scope, index } => {
                if let Some(m) = ensure_marketing(self, scope) {
                    if let Some(mk) = m.markets.as_mut() {
                        if index < mk.region.len() {
                            mk.region.remove(index);
                        }
                    }
                }
            }
            GldfAction::AddApprovalMark(scope) => {
                use gldf_rs::gldf::product_definitions::ApprovalMarks;
                if let Some(m) = ensure_marketing(self, scope) {
                    if m.approval_marks.is_none() {
                        m.approval_marks = Some(ApprovalMarks::default());
                    }
                    if let Some(am) = m.approval_marks.as_mut() {
                        am.approval_mark.push(String::new());
                    }
                }
            }
            GldfAction::UpdateApprovalMark {
                scope,
                index,
                value,
            } => {
                if let Some(m) = ensure_marketing(self, scope) {
                    if let Some(am) = m.approval_marks.as_mut() {
                        if let Some(slot) = am.approval_mark.get_mut(index) {
                            *slot = value;
                        }
                    }
                }
            }
            GldfAction::RemoveApprovalMark { scope, index } => {
                if let Some(m) = ensure_marketing(self, scope) {
                    if let Some(am) = m.approval_marks.as_mut() {
                        if index < am.approval_mark.len() {
                            am.approval_mark.remove(index);
                        }
                    }
                }
            }
            GldfAction::AddDesignAward(scope) => {
                use gldf_rs::gldf::product_definitions::DesignAwards;
                if let Some(m) = ensure_marketing(self, scope) {
                    if m.design_awards.is_none() {
                        m.design_awards = Some(DesignAwards::default());
                    }
                    if let Some(da) = m.design_awards.as_mut() {
                        da.design_award.push(String::new());
                    }
                }
            }
            GldfAction::UpdateDesignAward {
                scope,
                index,
                value,
            } => {
                if let Some(m) = ensure_marketing(self, scope) {
                    if let Some(da) = m.design_awards.as_mut() {
                        if let Some(slot) = da.design_award.get_mut(index) {
                            *slot = value;
                        }
                    }
                }
            }
            GldfAction::RemoveDesignAward { scope, index } => {
                if let Some(m) = ensure_marketing(self, scope) {
                    if let Some(da) = m.design_awards.as_mut() {
                        if index < da.design_award.len() {
                            da.design_award.remove(index);
                        }
                    }
                }
            }
            GldfAction::AddControlGear { prefix } => {
                use gldf_rs::gldf::general_definitions::electrical::ControlGear;
                let pfx = prefix.unwrap_or_else(|| "controlGear".to_string());
                let id = self.product.generate_unique_id(&pfx);
                let gear = ControlGear {
                    id: id.clone(),
                    ..ControlGear::default()
                };
                if let Err(e) = self.product.add_control_gear(gear) {
                    self.last_error = Some(format!("AddControlGear: {}", e));
                    self.is_modified = prev_modified;
                } else {
                    self.last_error = None;
                }
            }
            GldfAction::UpdateControlGear {
                id,
                name,
                nominal_voltage_value,
                standby_power,
                dimmable,
                color_controllable,
                constant_light_output_start_power,
                constant_light_output_end_power,
            } => {
                use gldf_rs::gldf::general_definitions::electrical::{
                    CurrentType, Frequency, Voltage,
                };
                use gldf_rs::gldf::header::Locale;
                let cgs_opt = self.product.general_definitions.control_gears.as_mut();
                if let Some(cgs) = cgs_opt {
                    if let Some(g) = cgs.control_gear.iter_mut().find(|g| g.id == id) {
                        if g.name.locale.is_empty() {
                            g.name.locale.push(Locale::default());
                        }
                        g.name.locale[0].value = name;
                        g.nominal_voltage = nominal_voltage_value.map(|v| Voltage {
                            type_attr: CurrentType::AC,
                            fixed_voltage: Some(v),
                            voltage_range: None,
                            frequency: Frequency::Hz50,
                        });
                        g.standby_power = standby_power;
                        g.dimmable = dimmable;
                        g.color_controllable = color_controllable;
                        g.constant_light_output_start_power =
                            constant_light_output_start_power;
                        g.constant_light_output_end_power = constant_light_output_end_power;
                        self.last_error = None;
                    } else {
                        self.last_error = Some(format!(
                            "UpdateControlGear: no control gear with id '{}'",
                            id
                        ));
                        self.is_modified = prev_modified;
                    }
                } else {
                    self.last_error =
                        Some("UpdateControlGear: no control gears defined".to_string());
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::RemoveControlGear { id } => {
                if let Err(e) = self.product.remove_control_gear(&id) {
                    self.last_error = Some(format!("RemoveControlGear: {}", e));
                    self.is_modified = prev_modified;
                } else {
                    self.last_error = None;
                }
            }
            GldfAction::RenameControlGear { old_id, new_id } => {
                let cgs_opt = self.product.general_definitions.control_gears.as_mut();
                if let Some(cgs) = cgs_opt {
                    if cgs.control_gear.iter().any(|g| g.id == new_id) {
                        self.last_error = Some(format!(
                            "RenameControlGear: id '{}' already exists",
                            new_id
                        ));
                        self.is_modified = prev_modified;
                    } else if let Some(g) =
                        cgs.control_gear.iter_mut().find(|g| g.id == old_id)
                    {
                        g.id = new_id.clone();
                        if let Some(eqs) = self.product.general_definitions.equipments.as_mut()
                        {
                            for e in eqs.equipment.iter_mut() {
                                if let Some(cgr) = e.control_gear_reference.as_mut() {
                                    if cgr.control_gear_id == old_id {
                                        cgr.control_gear_id = new_id.clone();
                                    }
                                }
                            }
                        }
                        if let Some(emitters) =
                            self.product.general_definitions.emitters.as_mut()
                        {
                            for em in emitters.emitter.iter_mut() {
                                for fe in em.fixed_light_emitter.iter_mut() {
                                    if let Some(cgr) = fe.control_gear_reference.as_mut() {
                                        if cgr.control_gear_id == old_id {
                                            cgr.control_gear_id = new_id.clone();
                                        }
                                    }
                                }
                                for me in em.multi_channel_light_emitter.iter_mut() {
                                    if let Some(cgr) = me.control_gear_reference.as_mut() {
                                        if cgr.control_gear_id == old_id {
                                            cgr.control_gear_id = new_id.clone();
                                        }
                                    }
                                }
                            }
                        }
                        self.last_error = None;
                    } else {
                        self.last_error = Some(format!(
                            "RenameControlGear: no control gear with id '{}'",
                            old_id
                        ));
                        self.is_modified = prev_modified;
                    }
                } else {
                    self.last_error =
                        Some("RenameControlGear: no control gears defined".to_string());
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::AddEquipment { prefix } => {
                use gldf_rs::gldf::general_definitions::electrical::Equipment;
                let pfx = prefix.unwrap_or_else(|| "equipment".to_string());
                let id = self.product.generate_unique_id(&pfx);
                let eq = Equipment {
                    id: id.clone(),
                    ..Equipment::default()
                };
                if let Err(e) = self.product.add_equipment(eq) {
                    self.last_error = Some(format!("AddEquipment: {}", e));
                    self.is_modified = prev_modified;
                } else {
                    self.last_error = None;
                }
            }
            GldfAction::UpdateEquipment {
                id,
                light_source_id,
                light_source_kind,
                control_gear_id,
                rated_input_power,
                emergency_ballast_lumen_factor,
                emergency_rated_luminous_flux,
            } => {
                use gldf_rs::gldf::general_definitions::electrical::ControlGearReference;
                use gldf_rs::gldf::general_definitions::lightsources::LightSourceReference;
                let eqs_opt = self.product.general_definitions.equipments.as_mut();
                if let Some(eqs) = eqs_opt {
                    if let Some(e) = eqs.equipment.iter_mut().find(|e| e.id == id) {
                        e.light_source_reference = match (light_source_id, light_source_kind) {
                            (Some(lid), Some(LightEmitterKind::Fixed)) => {
                                Some(LightSourceReference {
                                    fixed_light_source_id: Some(lid),
                                    changeable_light_source_id: None,
                                    light_source_count: None,
                                })
                            }
                            (Some(lid), Some(LightEmitterKind::Changeable)) => {
                                Some(LightSourceReference {
                                    fixed_light_source_id: None,
                                    changeable_light_source_id: Some(lid),
                                    light_source_count: None,
                                })
                            }
                            _ => None,
                        };
                        e.control_gear_reference = control_gear_id.map(|cid| {
                            ControlGearReference {
                                control_gear_id: cid,
                                control_gear_count: None,
                            }
                        });
                        e.rated_input_power = rated_input_power;
                        e.emergency_ballast_lumen_factor = emergency_ballast_lumen_factor;
                        e.emergency_rated_luminous_flux = emergency_rated_luminous_flux;
                        self.last_error = None;
                    } else {
                        self.last_error = Some(format!(
                            "UpdateEquipment: no equipment with id '{}'",
                            id
                        ));
                        self.is_modified = prev_modified;
                    }
                } else {
                    self.last_error =
                        Some("UpdateEquipment: no equipments defined".to_string());
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::RemoveEquipment { id } => {
                if let Err(e) = self.product.remove_equipment(&id) {
                    self.last_error = Some(format!("RemoveEquipment: {}", e));
                    self.is_modified = prev_modified;
                } else {
                    self.last_error = None;
                }
            }
            GldfAction::RenameEquipment { old_id, new_id } => {
                let eqs_opt = self.product.general_definitions.equipments.as_mut();
                if let Some(eqs) = eqs_opt {
                    if eqs.equipment.iter().any(|e| e.id == new_id) {
                        self.last_error = Some(format!(
                            "RenameEquipment: id '{}' already exists",
                            new_id
                        ));
                        self.is_modified = prev_modified;
                    } else if let Some(e) =
                        eqs.equipment.iter_mut().find(|e| e.id == old_id)
                    {
                        e.id = new_id;
                        self.last_error = None;
                    } else {
                        self.last_error = Some(format!(
                            "RenameEquipment: no equipment with id '{}'",
                            old_id
                        ));
                        self.is_modified = prev_modified;
                    }
                } else {
                    self.last_error =
                        Some("RenameEquipment: no equipments defined".to_string());
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::SetEmitterControlGearReference {
                emitter_id,
                kind,
                light_emitter_index,
                control_gear_id,
            } => {
                use gldf_rs::gldf::general_definitions::electrical::ControlGearReference;
                let new_ref = control_gear_id.map(|cid| ControlGearReference {
                    control_gear_id: cid,
                    control_gear_count: None,
                });
                let emitters_opt = self.product.general_definitions.emitters.as_mut();
                if let Some(emitters) = emitters_opt {
                    if let Some(em) =
                        emitters.emitter.iter_mut().find(|e| e.id == emitter_id)
                    {
                        let hit = match kind {
                            LightEmitterKind::Fixed => em
                                .fixed_light_emitter
                                .get_mut(light_emitter_index)
                                .map(|fe| fe.control_gear_reference = new_ref.clone())
                                .is_some(),
                            LightEmitterKind::MultiChannel => em
                                .multi_channel_light_emitter
                                .get_mut(light_emitter_index)
                                .map(|me| me.control_gear_reference = new_ref.clone())
                                .is_some(),
                            // ChangeableLightEmitter has no ControlGearReference
                            // in the XSD — only Fixed/MultiChannel carry it.
                            LightEmitterKind::Changeable | LightEmitterKind::Sensor => false,
                        };
                        if hit {
                            self.last_error = None;
                        } else {
                            self.last_error = Some(format!(
                                "SetEmitterControlGearReference: kind/index not found in emitter '{}'",
                                emitter_id
                            ));
                            self.is_modified = prev_modified;
                        }
                    } else {
                        self.last_error = Some(format!(
                            "SetEmitterControlGearReference: no emitter '{}'",
                            emitter_id
                        ));
                        self.is_modified = prev_modified;
                    }
                } else {
                    self.last_error = Some(
                        "SetEmitterControlGearReference: no emitters defined".to_string(),
                    );
                    self.is_modified = prev_modified;
                }
            }
            GldfAction::SetLightSourceCri {
                light_source_id,
                value,
            } => {
                if let Some(ci) = ensure_color_information(self, &light_source_id) {
                    ci.color_rendering_index = value;
                }
            }
            GldfAction::SetLightSourceCct {
                light_source_id,
                value,
            } => {
                if let Some(ci) = ensure_color_information(self, &light_source_id) {
                    ci.correlated_color_temperature = value;
                }
            }
            GldfAction::SetLightSourceCctRange {
                light_source_id,
                lower,
                upper,
            } => {
                use gldf_rs::gldf::general_definitions::lightsources::ColorTemperatureAdjustingRange;
                if let Some(ci) = ensure_color_information(self, &light_source_id) {
                    match (lower, upper) {
                        (None, None) => ci.color_temperature_adjusting_range = None,
                        (l, u) => {
                            ci.color_temperature_adjusting_range =
                                Some(ColorTemperatureAdjustingRange { lower: l, upper: u });
                        }
                    }
                }
            }
            GldfAction::SetLightSourceInitialColorTolerance {
                light_source_id,
                value,
            } => {
                if let Some(ci) = ensure_color_information(self, &light_source_id) {
                    ci.initial_color_tolerance = value;
                }
            }
            GldfAction::SetLightSourceMaintainedColorTolerance {
                light_source_id,
                value,
            } => {
                if let Some(ci) = ensure_color_information(self, &light_source_id) {
                    ci.maintained_color_tolerance = value;
                }
            }
            GldfAction::SetLightSourceTlci {
                light_source_id,
                value,
            } => {
                if let Some(ci) = ensure_color_information(self, &light_source_id) {
                    ci.tlci = value;
                }
            }
            GldfAction::SetLightSourceMelanopicFactor {
                light_source_id,
                value,
            } => {
                if let Some(ci) = ensure_color_information(self, &light_source_id) {
                    ci.melanopic_factor = value;
                }
            }
            GldfAction::SetDedicatedEmergencyLightingType(scope, value) => {
                if let Some(e) = ensure_emergency(self, scope) {
                    e.dedicated_emergency_lighting_type = value;
                }
            }
            GldfAction::AddEmergencyFlux(scope) => {
                if let Some(d) = ensure_emergency_duration(self, scope) {
                    d.flux.push(Flux::default());
                }
            }
            GldfAction::UpdateEmergencyFlux {
                scope,
                index,
                hours,
                flux,
            } => {
                if let Some(entry) = ensure_emergency_duration(self, scope)
                    .and_then(|d| d.flux.get_mut(index))
                {
                    entry.hours = hours;
                    entry.value = flux;
                }
            }
            GldfAction::RemoveEmergencyFlux { scope, index } => {
                let d_opt: Option<&mut gldf_rs::gldf::DurationTimeAndFlux> = match scope {
                    Scope::Product => self
                        .product
                        .product_definitions
                        .product_meta_data
                        .as_mut()
                        .and_then(|m| m.descriptive_attributes.as_mut())
                        .and_then(|a| a.emergency.as_mut())
                        .and_then(|e| e.duration_time_and_flux.as_mut()),
                    Scope::Variant(i) => self
                        .product
                        .product_definitions
                        .variants
                        .as_mut()
                        .and_then(|vs| vs.variant.get_mut(i))
                        .and_then(|v| v.descriptive_attributes.as_mut())
                        .and_then(|a| a.emergency.as_mut())
                        .and_then(|e| e.duration_time_and_flux.as_mut()),
                };
                if let Some(d) = d_opt {
                    if index < d.flux.len() {
                        d.flux.remove(index);
                    }
                }
            }
            GldfAction::EnsureL3dMtlForVariant { variant_idx } => {
                // Re-run the same sync the `UpdateHousingColor` arm above
                // does — but on demand. Useful when the user drops a fresh
                // L3D AFTER setting the RAL: the file-drop path doesn't
                // know about the RAL change, so auto-fire doesn't fire,
                // and the L3D's MTL stays at whatever colour the
                // manufacturer shipped.
                let product = self.product.clone();
                match gldf_rs::l3d_mtl_sync::sync_l3d_mtl_for_housing_color(
                    &mut self.embedded_files,
                    &product,
                    variant_idx,
                ) {
                    Ok(outcome) => {
                        log::debug!("Sync .mtl button: {:?}", outcome);
                    }
                    Err(e) => {
                        self.last_error = Some(format!("L3D MTL sync: {}", e));
                    }
                }
            }
            GldfAction::SetVariantMountings { index, payload } => {
                // Replace the n-th Variant's Mountings. `None` clears it entirely
                // (XSD allows omission). We deref the Box → owned Mountings on the
                // write so the Variant doesn't carry an extra heap indirection.
                if let Some(v) = self
                    .product
                    .product_definitions
                    .variants
                    .as_mut()
                    .and_then(|vs| vs.variant.get_mut(index))
                {
                    v.mountings = payload.map(|b| *b);
                }
            }
            GldfAction::SetVariantProductSeries { index, payload } => {
                use gldf_rs::gldf::general_definitions::ProductSeries;
                if let Some(v) = self
                    .product
                    .product_definitions
                    .variants
                    .as_mut()
                    .and_then(|vs| vs.variant.get_mut(index))
                {
                    if payload.is_empty() {
                        v.product_series = None;
                    } else {
                        v.product_series = Some(ProductSeries {
                            product_serie: payload,
                        });
                    }
                }
            }
            GldfAction::Reset => {
                *self = GldfState::default();
            }
        }

    }
}

// ---- Leptos context plumbing ----

/// Editor state context. A signal so any component below the provider can

pub fn collect_languages(product: &GldfProduct) -> Vec<String> {
    use gldf_rs::gldf::LocaleFoo;
    use std::collections::BTreeSet;

    fn collect_from(lf: &LocaleFoo, out: &mut BTreeSet<String>) {
        for entry in &lf.locale {
            if !entry.language.is_empty() {
                out.insert(entry.language.clone());
            }
        }
    }
    fn collect_from_opt(lf: &Option<LocaleFoo>, out: &mut BTreeSet<String>) {
        if let Some(v) = lf {
            collect_from(v, out);
        }
    }

    let mut langs: BTreeSet<String> = BTreeSet::new();

    if let Some(meta) = product.product_definitions.product_meta_data.as_ref() {
        collect_from_opt(&meta.product_number, &mut langs);
        collect_from_opt(&meta.name, &mut langs);
        collect_from_opt(&meta.description, &mut langs);
        collect_from_opt(&meta.tender_text, &mut langs);
        if let Some(series) = meta.product_series.as_ref() {
            for s in &series.product_serie {
                collect_from_opt(&s.name, &mut langs);
                collect_from_opt(&s.description, &mut langs);
            }
        }
    }
    if let Some(variants) = product.product_definitions.variants.as_ref() {
        for variant in &variants.variant {
            collect_from_opt(&variant.product_number, &mut langs);
            collect_from_opt(&variant.name, &mut langs);
            collect_from_opt(&variant.description, &mut langs);
            collect_from_opt(&variant.tender_text, &mut langs);
            if let Some(series) = variant.product_series.as_ref() {
                for s in &series.product_serie {
                    collect_from_opt(&s.name, &mut langs);
                    collect_from_opt(&s.description, &mut langs);
                }
            }
        }
    }
    if let Some(ls) = product.general_definitions.light_sources.as_ref() {
        for fls in &ls.fixed_light_source {
            collect_from(&fls.name, &mut langs);
            collect_from_opt(&fls.description, &mut langs);
        }
        for cls in &ls.changeable_light_source {
            collect_from(&cls.name, &mut langs);
            collect_from_opt(&cls.description, &mut langs);
        }
    }

    langs.into_iter().collect()
}

fn ensure_product_meta_data(state: &mut GldfState) -> &mut ProductMetaData {
    if state
        .product
        .product_definitions
        .product_meta_data
        .is_none()
    {
        state.product.product_definitions.product_meta_data = Some(ProductMetaData::default());
    }
    state
        .product
        .product_definitions
        .product_meta_data
        .as_mut()
        .unwrap()
}

/// Scope-aware DescriptiveAttributes accessor. For `Scope::Product` the block
/// lives under `ProductMetaData`; for `Scope::Variant(idx)` it lives on the
/// variant at that index. Returns `None` only when the variant index is out
/// of range — both blocks are vivified on demand.
fn ensure_descriptive_attributes(
    state: &mut GldfState,
    scope: Scope,
) -> Option<&mut DescriptiveAttributes> {
    match scope {
        Scope::Product => {
            let meta = ensure_product_meta_data(state);
            if meta.descriptive_attributes.is_none() {
                meta.descriptive_attributes = Some(DescriptiveAttributes::default());
            }
            meta.descriptive_attributes.as_mut()
        }
        Scope::Variant(idx) => {
            let v = variant_at_mut(state, idx)?;
            if v.descriptive_attributes.is_none() {
                v.descriptive_attributes = Some(DescriptiveAttributes::default());
            }
            v.descriptive_attributes.as_mut()
        }
    }
}

fn ensure_electrical(state: &mut GldfState, scope: Scope) -> Option<&mut Electrical> {
    let attrs = ensure_descriptive_attributes(state, scope)?;
    if attrs.electrical.is_none() {
        attrs.electrical = Some(Electrical::default());
    }
    attrs.electrical.as_mut()
}

fn ensure_mechanical(state: &mut GldfState, scope: Scope) -> Option<&mut Mechanical> {
    let attrs = ensure_descriptive_attributes(state, scope)?;
    if attrs.mechanical.is_none() {
        attrs.mechanical = Some(Mechanical::default());
    }
    attrs.mechanical.as_mut()
}

fn ensure_mechanical_size(state: &mut GldfState, scope: Scope) -> Option<&mut ProductSize> {
    let mech = ensure_mechanical(state, scope)?;
    if mech.product_size.is_none() {
        mech.product_size = Some(ProductSize::default());
    }
    mech.product_size.as_mut()
}

/// Helper to ensure ProductMetaData.Pictures exists and return mutable reference
/// Scope-aware getter for the Pictures container. `Scope::Product` writes
/// into `ProductMetaData.Pictures`; `Scope::Variant(idx)` writes into
/// `Variants/Variant[idx].Pictures`. Returns `None` if the variant index
/// is out of range. Lazily creates the container.
fn ensure_product_meta_pictures(
    state: &mut GldfState,
    scope: Scope,
) -> Option<&mut gldf_rs::gldf::general_definitions::Images> {
    match scope {
        Scope::Product => {
            let meta = ensure_product_meta_data(state);
            if meta.pictures.is_none() {
                meta.pictures = Some(gldf_rs::gldf::general_definitions::Images::default());
            }
            Some(meta.pictures.as_mut().unwrap())
        }
        Scope::Variant(idx) => {
            let variants = state.product.product_definitions.variants.as_mut()?;
            let v = variants.variant.get_mut(idx)?;
            if v.pictures.is_none() {
                v.pictures = Some(gldf_rs::gldf::general_definitions::Images::default());
            }
            Some(v.pictures.as_mut().unwrap())
        }
    }
}

/// Hyperlinks live under Marketing — Marketing exists at both ProductMetaData
/// and per-Variant scope, so the helper is scope-aware. Existing call sites
/// pass `Scope::Product` (the only scope the Hyperlinks editor currently
/// supports).
fn ensure_hyperlinks(
    state: &mut GldfState,
    scope: Scope,
) -> Option<&mut gldf_rs::gldf::Hyperlinks> {
    let marketing = ensure_marketing(state, scope)?;
    if marketing.hyperlinks.is_none() {
        marketing.hyperlinks = Some(gldf_rs::gldf::Hyperlinks::default());
    }
    marketing.hyperlinks.as_mut()
}

fn ensure_emergency(state: &mut GldfState, scope: Scope) -> Option<&mut Emergency> {
    let attrs = ensure_descriptive_attributes(state, scope)?;
    if attrs.emergency.is_none() {
        attrs.emergency = Some(Emergency::default());
    }
    attrs.emergency.as_mut()
}

fn ensure_emergency_duration(
    state: &mut GldfState,
    scope: Scope,
) -> Option<&mut DurationTimeAndFlux> {
    let emergency = ensure_emergency(state, scope)?;
    if emergency.duration_time_and_flux.is_none() {
        emergency.duration_time_and_flux = Some(DurationTimeAndFlux::default());
    }
    emergency.duration_time_and_flux.as_mut()
}

fn ensure_marketing(state: &mut GldfState, scope: Scope) -> Option<&mut Marketing> {
    let attrs = ensure_descriptive_attributes(state, scope)?;
    if attrs.marketing.is_none() {
        attrs.marketing = Some(Marketing::default());
    }
    attrs.marketing.as_mut()
}

/// Locate a Fixed- or ChangeableLightSource by id and vivify its
/// `ColorInformation` block. The XSD declares `ColorInformation` on both
/// `FixedLightSource` and `ChangeableLightSource` with identical shape
/// (CRI, CCT, adjusting range, chromaticity, TLCI, …). Returns `None`
/// when no light source carries the requested id.
fn ensure_color_information<'a>(
    state: &'a mut GldfState,
    light_source_id: &str,
) -> Option<&'a mut gldf_rs::gldf::general_definitions::lightsources::ColorInformation> {
    use gldf_rs::gldf::general_definitions::lightsources::ColorInformation;
    let ls = state.product.general_definitions.light_sources.as_mut()?;
    for fls in ls.fixed_light_source.iter_mut() {
        if fls.id == light_source_id {
            if fls.color_information.is_none() {
                fls.color_information = Some(ColorInformation::default());
            }
            return fls.color_information.as_mut();
        }
    }
    for cls in ls.changeable_light_source.iter_mut() {
        if cls.id == light_source_id {
            if cls.color_information.is_none() {
                cls.color_information = Some(ColorInformation::default());
            }
            return cls.color_information.as_mut();
        }
    }
    None
}

fn ensure_applications(state: &mut GldfState, scope: Scope) -> Option<&mut Applications> {
    let marketing = ensure_marketing(state, scope)?;
    if marketing.applications.is_none() {
        marketing.applications = Some(Applications::default());
    }
    marketing.applications.as_mut()
}

/// Vivify `Marketing/Labels` for the chosen scope. Mirrors the
/// Applications helper — Labels lives at the same depth in the XSD tree.
fn ensure_labels(state: &mut GldfState, scope: Scope) -> Option<&mut Labels> {
    let marketing = ensure_marketing(state, scope)?;
    if marketing.labels.is_none() {
        marketing.labels = Some(Labels::default());
    }
    marketing.labels.as_mut()
}

/// Helper to ensure Photometries exists and return mutable reference
fn ensure_photometries(state: &mut GldfState) -> &mut Photometries {
    if state.product.general_definitions.photometries.is_none() {
        state.product.general_definitions.photometries = Some(Photometries::default());
    }
    state
        .product
        .general_definitions
        .photometries
        .as_mut()
        .unwrap()
}

/// Helper to ensure a specific Photometry exists at index
fn ensure_photometry(state: &mut GldfState, index: usize) -> &mut Photometry {
    let photometries = ensure_photometries(state);
    // Extend the vector if needed
    while photometries.photometry.len() <= index {
        photometries.photometry.push(Photometry {
            id: format!("photometry_{}", photometries.photometry.len()),
            ..Default::default()
        });
    }
    &mut photometries.photometry[index]
}

/// Helper to ensure DescriptivePhotometry exists for a specific Photometry
fn ensure_descriptive_photometry(
    state: &mut GldfState,
    index: usize,
) -> &mut DescriptivePhotometry {
    let photometry = ensure_photometry(state, index);
    if photometry.descriptive_photometry.is_none() {
        photometry.descriptive_photometry = Some(DescriptivePhotometry::default());
    }
    photometry.descriptive_photometry.as_mut().unwrap()
}

/// Mutable reference to a Variant at the given index, or `None` if out of range
/// (or if no Variants block exists at all).
fn variant_at_mut(
    state: &mut GldfState,
    index: usize,
) -> Option<&mut gldf_rs::gldf::product_definitions::Variant> {
    state
        .product
        .product_definitions
        .variants
        .as_mut()?
        .variant
        .get_mut(index)
}

/// Mutable reference to a FixedLightSource at the given index, or `None` if out
/// of range (or if no LightSources block exists at all).
fn fixed_light_source_at_mut(
    state: &mut GldfState,
    index: usize,
) -> Option<&mut gldf_rs::gldf::general_definitions::lightsources::FixedLightSource> {
    state
        .product
        .general_definitions
        .light_sources
        .as_mut()?
        .fixed_light_source
        .get_mut(index)
}

/// Mutable reference to a ChangeableLightSource at the given index, or `None`
/// if out of range (or if no LightSources block exists at all).
fn changeable_light_source_at_mut(
    state: &mut GldfState,
    index: usize,
) -> Option<&mut gldf_rs::gldf::general_definitions::lightsources::ChangeableLightSource> {
    state
        .product
        .general_definitions
        .light_sources
        .as_mut()?
        .changeable_light_source
        .get_mut(index)
}

/// Helper to ensure UGR exists for a specific Photometry
fn ensure_ugr(state: &mut GldfState, index: usize) -> &mut UGR4H8H705020LQ {
    let desc = ensure_descriptive_photometry(state, index);
    if desc.ugr4_h8_h705020_lq.is_none() {
        desc.ugr4_h8_h705020_lq = Some(UGR4H8H705020LQ::default());
    }
    desc.ugr4_h8_h705020_lq.as_mut().unwrap()
}

/// Pick a sensible initial active language for a freshly loaded product.
///
/// Order: Header.default_language → first language seen in any LocaleFoo
/// across the product → "en" as last resort.
fn pick_initial_language(product: &GldfProduct) -> String {
    if let Some(lang) = product.header.default_language.as_ref() {
        if !lang.is_empty() {
            return lang.clone();
        }
    }
    let langs = collect_languages(product);
    langs.into_iter().next().unwrap_or_else(|| "en".to_string())
}
