//! GLDF state management for the WASM editor (Leptos port).
//!
//! The editor state is backed by `gldf_rs::EditableGldf` — a `GldfProduct` plus
//! a `HashMap<String, Vec<u8>>` of embedded file bytes. That makes drop-and-link
//! workflows (drop a `.spd` / `.ies` and link it to a light source) and the
//! round-trip `save_to_buf` reuse a single cross-platform builder API in the
//! core lib, the same one any FFI / Unreal binding can drive.
//!
//! ## Leptos translation
//!
//! The Yew side was a `Reducible` consumed via `use_reducer`. Here we hold the
//! state in an `RwSignal<GldfState>` and dispatch by calling
//! `signal.update(|s| s.apply(action))`. The reducer arms are unchanged in
//! shape — they just mutate `&mut self` instead of a cloned `new_state`.
//!
//! `is_modified` defaults to true at the top of `apply` and is reverted by
//! arms whose work doesn't actually change the document (e.g. language picker,
//! UI translation language, error reporting). To keep the revert pattern
//! working in an in-place mutation, we capture the prior value at entry as
//! `prev_modified` and restore it in those arms.

use gldf_rs::gldf::{
    general_definitions::photometries::{
        DescriptivePhotometry, Photometries, Photometry, UGR4H8H705020LQ,
    },
    product_definitions::{
        Applications, DescriptiveAttributes, DurationTimeAndFlux, Electrical, Emergency, Flux,
        Labels, Marketing, Mechanical, ProductMetaData, ProductSize,
    },
    FormatVersion, GldfProduct, LocaleFoo,
};
use gldf_rs::EditableGldf;
use leptos::prelude::*;
use std::collections::HashMap;


/// Edit scope for fields that exist at both ProductMetaData and per-Variant
/// levels in the XSD (Electrical / Mechanical / Marketing → Applications).
///
/// `Product` writes to `ProductMetaData/DescriptiveAttributes/...` (the
/// default that every variant inherits unless it overrides). `Variant(idx)`
/// writes to `Variants/Variant[idx]/DescriptiveAttributes/...` (per-SKU
/// override — this is what makes "indoor sibling has IP20, outdoor has IP65"
/// possible without two product files).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    Product,
    Variant(usize),
}

/// Actions that can be performed on the GLDF state
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant, dead_code)]
pub enum GldfAction {
    /// Load a new GLDF product (without embedded file bytes).
    Load(GldfProduct),
    /// Load a GLDF product together with its embedded files. Use this when the
    /// .gldf was just unzipped and you have the binary contents alongside the XML.
    LoadWithFiles {
        product: GldfProduct,
        embedded_files: HashMap<String, Vec<u8>>,
    },
    /// Drop-and-link: embed a photometry file (`.ldt` / `.ies`) and create the
    /// matching `<File>` + `<Photometry>` entries. Delegates to
    /// `EditableGldf::add_photometry_file` so every binding shares one
    /// implementation. On error, `last_error` is set and `is_modified` stays as
    /// it was.
    AddPhotometryFile { bytes: Vec<u8>, filename: String },
    /// Drop-and-link: embed a spectrum file (`.spd`) linked to a light source.
    /// Delegates to `EditableGldf::add_spectrum_file`. Errors (e.g. unknown
    /// light-source id) surface via `last_error`.
    AddSpectrumFile {
        bytes: Vec<u8>,
        filename: String,
        light_source_id: String,
    },
    /// Add a product variant by name (English locale). Delegates to
    /// `EditableGldf::add_variant_simple`.
    AddVariantSimple { name: String },
    /// Replace the file behind an existing `<Photometry>` (id stable so
    /// emitter references survive). Delegates to
    /// `EditableGldf::replace_photometry_file`.
    ReplacePhotometryFile {
        photometry_id: String,
        bytes: Vec<u8>,
        filename: String,
    },
    /// Rename a FixedLightSource id (e.g. the auto-created "lightsource_1"
    /// from the start-page IES drop) and update every emitter + equipment
    /// reference. Delegates to `GldfProduct::rename_fixed_light_source`.
    RenameFixedLightSource { old_id: String, new_id: String },
    /// Update the header author
    SetAuthor(String),
    /// Update the header manufacturer
    SetManufacturer(String),
    /// Update the creation time code
    SetCreationTimeCode(String),
    /// Update the created with application
    SetCreatedWithApplication(String),
    /// Update the default language
    SetDefaultLanguage(Option<String>),
    /// Update format version (e.g., "1.0.0-rc.3")
    SetFormatVersion(String),
    /// Add a file to the files collection
    AddFile {
        id: String,
        content_type: String,
        type_attr: String,
        file_name: String,
        language: Option<String>,
    },
    /// Remove a file by id
    RemoveFile(String),
    /// Update a file's properties
    UpdateFile {
        id: String,
        content_type: String,
        type_attr: String,
        file_name: String,
    },
    // --- Electrical attributes (scope-aware: product default or per-variant override) ---
    SetElectricalSafetyClass(Scope, Option<String>),
    SetIngressProtectionIPCode(Scope, Option<String>),
    SetPowerFactor(Scope, Option<f64>),
    SetConstantLightOutput(Scope, Option<bool>),
    SetLightDistribution(Scope, Option<String>),
    SetSwitchingCapacity(Scope, Option<String>),
    // --- Applications (scope-aware) ---
    SetApplications(Scope, Vec<String>),
    AddApplication(Scope, String),
    RemoveApplication(Scope, usize),
    // --- Labels (Marketing/Labels/Label, scope-aware) ---
    AddLabel(Scope, String),
    RemoveLabel(Scope, usize),
    // --- Photometry (DescriptivePhotometry) ---
    /// Update photometry by index
    SetPhotometryCieFluxCode {
        index: usize,
        value: Option<String>,
    },
    SetPhotometryLightOutputRatio {
        index: usize,
        value: Option<f64>,
    },
    SetPhotometryLuminousEfficacy {
        index: usize,
        value: Option<f64>,
    },
    SetPhotometryDownwardFluxFraction {
        index: usize,
        value: Option<f64>,
    },
    SetPhotometryDownwardLOR {
        index: usize,
        value: Option<f64>,
    },
    SetPhotometryUpwardLOR {
        index: usize,
        value: Option<f64>,
    },
    SetPhotometryCutOffAngle {
        index: usize,
        value: Option<f64>,
    },
    SetPhotometryLuminaireLuminance {
        index: usize,
        value: Option<i32>,
    },
    SetPhotometryUgrX {
        index: usize,
        value: Option<f64>,
    },
    SetPhotometryUgrY {
        index: usize,
        value: Option<f64>,
    },
    SetPhotometryPhotometricCode {
        index: usize,
        value: Option<String>,
    },
    SetPhotometryBugRating {
        index: usize,
        value: Option<String>,
    },
    /// Switch the active display/edit language for LocaleFoo fields.
    /// This is viewer-only state, not persisted into the GLDF product.
    SetActiveLanguage(String),
    /// Switch the UI translation language. Independent of the document's
    /// own languages — drives the viewer's translation tables (XSD enum
    /// labels, eventually the UI shell).
    SetUiLanguage(String),
    // --- ProductMetaData LocaleFoo fields (whole-field replacement) ---
    SetProductMetaName(LocaleFoo),
    SetProductMetaDescription(LocaleFoo),
    SetProductMetaProductNumber(LocaleFoo),
    SetProductMetaTenderText(LocaleFoo),
    // --- Variant LocaleFoo fields (whole-field replacement, indexed by variant position) ---
    SetVariantName(usize, LocaleFoo),
    SetVariantDescription(usize, LocaleFoo),
    SetVariantProductNumber(usize, LocaleFoo),
    SetVariantTenderText(usize, LocaleFoo),
    // --- FixedLightSource LocaleFoo fields (indexed by position in fixed_light_source vec) ---
    SetFixedLightSourceName(usize, LocaleFoo),
    SetFixedLightSourceDescription(usize, LocaleFoo),
    // --- ChangeableLightSource LocaleFoo fields ---
    SetChangeableLightSourceName(usize, LocaleFoo),
    SetChangeableLightSourceDescription(usize, LocaleFoo),
    // --- Mechanical attributes (scope-aware) ---
    SetMechanicalIKRating(Scope, Option<String>),
    SetMechanicalProductForm(Scope, Option<String>),
    SetMechanicalWeight(Scope, Option<f64>),
    SetMechanicalLength(Scope, Option<i32>),
    SetMechanicalWidth(Scope, Option<i32>),
    SetMechanicalHeight(Scope, Option<i32>),
    SetMechanicalSealingMaterial(Scope, LocaleFoo),
    // --- Pictures (ProductMetaData.Pictures) ---
    AddProductMetaPicture {
        file_id: String,
        image_type: String,
    },
    UpdateProductMetaPicture {
        index: usize,
        file_id: String,
        image_type: String,
    },
    RemoveProductMetaPicture(usize),
    // --- Hyperlinks (ProductMetaData.DescriptiveAttributes.Marketing.Hyperlinks) ---
    AddHyperlink,
    UpdateHyperlink {
        index: usize,
        href: String,
        language: Option<String>,
        region: Option<String>,
        country_code: Option<String>,
        value: String,
    },
    RemoveHyperlink(usize),
    // --- Emergency (ProductMetaData.DescriptiveAttributes.Emergency) ---
    SetDedicatedEmergencyLightingType(Option<String>),
    AddEmergencyFlux,
    UpdateEmergencyFlux {
        index: usize,
        hours: i32,
        flux: i32,
    },
    RemoveEmergencyFlux(usize),
    /// Reset state to default
    #[allow(dead_code)]
    Reset,
}

/// State of the GLDF editor.
///
/// `product` + `embedded_files` together mirror `gldf_rs::EditableGldf`. We keep
/// them as two public fields (instead of a wrapped `EditableGldf` value) only so
/// every existing editor component reading `gldf.product.X` keeps compiling; the
/// builder actions internally hand both fields to an `EditableGldf` and call the
/// core-lib API. That keeps the drop-and-link logic in the lib where every
/// cross-platform binding can reuse it.
#[derive(Clone, Debug, PartialEq)]
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
    /// viewer's translation tables (XSD enum labels and eventually the
    /// chrome). Defaults from `localStorage["gldf.ui_language"]`, then the
    /// browser locale, then `"en"`.
    pub ui_language: String,
}

impl Default for GldfState {
    fn default() -> Self {
        Self {
            product: GldfProduct::default(),
            embedded_files: HashMap::new(),
            is_modified: false,
            last_error: None,
            active_language: "en".to_string(),
            ui_language: detect_initial_ui_language(),
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
                match editable.add_photometry_file(bytes, &filename) {
                    Ok(_id) => {
                        self.take_from_editable(editable);
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
            GldfAction::SetActiveLanguage(lang) => {
                self.active_language = lang;
                // Language switch alone does not modify the product.
                self.is_modified = prev_modified;
            }
            GldfAction::SetUiLanguage(lang) => {
                store_ui_language(&lang);
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
            GldfAction::RemoveFile(id) => {
                self
                    .product
                    .general_definitions
                    .files
                    .file
                    .retain(|f| f.id != id);
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
                file_id,
                image_type,
            } => {
                ensure_product_meta_pictures(self).image.push(
                    gldf_rs::gldf::general_definitions::Image {
                        file_id,
                        image_type,
                    },
                );
            }
            GldfAction::UpdateProductMetaPicture {
                index,
                file_id,
                image_type,
            } => {
                if let Some(img) = ensure_product_meta_pictures(self)
                    .image
                    .get_mut(index)
                {
                    img.file_id = file_id;
                    img.image_type = image_type;
                }
            }
            GldfAction::RemoveProductMetaPicture(index) => {
                if let Some(pics) = self
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_mut()
                    .and_then(|m| m.pictures.as_mut())
                {
                    if index < pics.image.len() {
                        pics.image.remove(index);
                    }
                }
            }
            GldfAction::AddHyperlink => {
                if let Some(hs) = ensure_hyperlinks(self, Scope::Product) {
                    hs.hyperlink.push(gldf_rs::gldf::Hyperlink::default());
                }
            }
            GldfAction::UpdateHyperlink {
                index,
                href,
                language,
                region,
                country_code,
                value,
            } => {
                if let Some(h) = ensure_hyperlinks(self, Scope::Product)
                    .and_then(|hs| hs.hyperlink.get_mut(index))
                {
                    h.href = href;
                    h.language = language;
                    h.region = region;
                    h.country_code = country_code;
                    h.value = value;
                }
            }
            GldfAction::RemoveHyperlink(index) => {
                if let Some(hs) = self
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_mut()
                    .and_then(|m| m.descriptive_attributes.as_mut())
                    .and_then(|a| a.marketing.as_mut())
                    .and_then(|m| m.hyperlinks.as_mut())
                {
                    if index < hs.hyperlink.len() {
                        hs.hyperlink.remove(index);
                    }
                }
            }
            GldfAction::SetDedicatedEmergencyLightingType(value) => {
                if let Some(e) = ensure_emergency(self, Scope::Product) {
                    e.dedicated_emergency_lighting_type = value;
                }
            }
            GldfAction::AddEmergencyFlux => {
                if let Some(d) = ensure_emergency_duration(self, Scope::Product) {
                    d.flux.push(Flux::default());
                }
            }
            GldfAction::UpdateEmergencyFlux { index, hours, flux } => {
                if let Some(entry) = ensure_emergency_duration(self, Scope::Product)
                    .and_then(|d| d.flux.get_mut(index))
                {
                    entry.hours = hours;
                    entry.value = flux;
                }
            }
            GldfAction::RemoveEmergencyFlux(index) => {
                if let Some(d) = self
                    .product
                    .product_definitions
                    .product_meta_data
                    .as_mut()
                    .and_then(|m| m.descriptive_attributes.as_mut())
                    .and_then(|a| a.emergency.as_mut())
                    .and_then(|e| e.duration_time_and_flux.as_mut())
                {
                    if index < d.flux.len() {
                        d.flux.remove(index);
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
/// read with `.get()` / `.with(...)` and dispatch with `.update(|s| s.apply(...))`.
pub type GldfContext = RwSignal<GldfState>;

/// Install a fresh `GldfState` signal as Leptos context. Call once near the
/// App root. Children retrieve the handle via [`use_gldf`].
pub fn provide_gldf_state() -> GldfContext {
    let signal = RwSignal::new(GldfState::default());
    provide_context(signal);
    signal
}

/// Retrieve the editor state context. Panics if no `GldfContext` is in scope —
/// install one with [`provide_gldf_state`] at the App root.
pub fn use_gldf() -> GldfContext {
    expect_context::<GldfContext>()
}


/// Helper to ensure ProductMetaData exists and return mutable reference
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
fn ensure_product_meta_pictures(
    state: &mut GldfState,
) -> &mut gldf_rs::gldf::general_definitions::Images {
    let meta = ensure_product_meta_data(state);
    if meta.pictures.is_none() {
        meta.pictures = Some(gldf_rs::gldf::general_definitions::Images::default());
    }
    meta.pictures.as_mut().unwrap()
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

/// Collect every distinct language code that appears in any LocaleFoo in the
/// product, sorted alphabetically. Used by the language picker UI.
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

/// `localStorage` key for the UI translation language. Distinct from
/// `gldf.display_language` (which the LanguageBanner uses for the
/// document's own locales).
const UI_LANGUAGE_STORAGE_KEY: &str = "gldf.ui_language";

/// Languages the viewer ships translation tables for. The UI language
/// dropdown only shows these; anything else falls back to "en".
///
/// Codes match `eulumdat-i18n` (the family-shared translation source)
/// so the picker, the bundled JSON locale files, and the rest of the
/// eulumdat-rs ecosystem agree on what `"zh"` etc. mean.
pub const SUPPORTED_UI_LANGUAGES: &[&str] = &["de", "en", "es", "fr", "it", "ru", "zh"];

/// Native-language label for each supported UI code (used by the picker
/// dropdown so the user reads "Deutsch" instead of "de"). Returns `None`
/// for unsupported codes; callers can fall back to the raw code.
pub fn ui_language_native_name(code: &str) -> Option<&'static str> {
    match code {
        "de" => Some("Deutsch"),
        "en" => Some("English"),
        "es" => Some("Español"),
        "fr" => Some("Français"),
        "it" => Some("Italiano"),
        "ru" => Some("Русский"),
        "zh" => Some("中文"),
        _ => None,
    }
}

/// Pick the UI language at startup. Uses the user's last choice from
/// `localStorage` if it's one of the supported languages, otherwise
/// defaults to `"en"`. Browser-locale auto-detection would need an
/// extra `web-sys` feature for `Navigator`; the explicit picker is
/// always one click away in the toolbar.
fn detect_initial_ui_language() -> String {
    if let Some(stored) = load_ui_language() {
        if SUPPORTED_UI_LANGUAGES.iter().any(|l| **l == stored) {
            return stored;
        }
    }
    "en".to_string()
}

/// Read the user's stored UI language preference, if any.
fn load_ui_language() -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()
        .flatten()?
        .get_item(UI_LANGUAGE_STORAGE_KEY)
        .ok()
        .flatten()
}

/// Persist the chosen UI language. No-op outside the browser.
fn store_ui_language(lang: &str) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(UI_LANGUAGE_STORAGE_KEY, lang);
        }
    }
}

/// Map common locale variants to the canonical SUPPORTED_UI_LANGUAGES code.
/// Examples: `"en-US"` → `"en"`, `"de-AT"` → `"de"`, `"zh"` / `"cn"` /
/// `"zh-Hans"` / `"zh-CN"` / `"zh-TW"` → `"zh-CN"`.
pub fn normalise_ui_language(input: &str) -> String {
    let lower = input.to_lowercase();
    if lower == "cn" || lower.starts_with("zh") {
        return "zh-CN".to_string();
    }
    // Strip region tag: "de-AT" → "de", keep canonical code.
    let primary = lower.split(['-', '_']).next().unwrap_or("en");
    primary.to_string()
}
