//! Editor actions — the public, serde-friendly surface that every
//! frontend (WASM, TUI, CLI, Python, FFI) dispatches into the reducer.
//!
//! Every variant is plain data and serializes via
//! `#[serde(tag = "action", content = "data")]`. Binary fields use
//! `serde_bytes` so JSON encodings are compact byte arrays instead of
//! UTF-8-lossy strings.
//!
//! Variants and their semantics are lifted verbatim from the original
//! WASM reducer (`gldf-rs-wasm/src/state/gldf_state.rs`); the reducer
//! body lives in `state.rs`.

// Only types that actually appear in `GldfAction` variant payloads are
// imported here. The reducer (`state.rs`) imports the rest.
use gldf_rs::gldf::{GldfProduct, LocaleFoo};
use std::collections::HashMap;

/// Edit scope for fields that exist at both ProductMetaData and per-Variant
/// levels in the XSD (Electrical / Mechanical / Marketing → Applications).
///
/// `Product` writes to `ProductMetaData/DescriptiveAttributes/...` (the
/// default that every variant inherits unless it overrides). `Variant(idx)`
/// writes to `Variants/Variant[idx]/DescriptiveAttributes/...` (per-SKU
/// override — this is what makes "indoor sibling has IP20, outdoor has IP65"
/// possible without two product files).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Scope {
    Product,
    Variant(usize),
}

/// Which of the four LightEmitter child kinds an action targets inside
/// an `<Emitter>`. Mirrors the `xs:choice` in the schema (XSD L2915-2918).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LightEmitterKind {
    Fixed,
    Changeable,
    MultiChannel,
    Sensor,
}

/// One row of a [`SetVariantGeometryModel`] action — binds an Emitter
/// to a named object inside the 3D model. The optional model-type
/// marker (`l3d` / `m3d` / `r3d`) disambiguates when a variant ships
/// multiple 3D model formats; most files leave it empty.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModelEmitterBinding {
    pub emitter_id: String,
    pub external_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_model_type: Option<String>,
}

/// Actions that can be performed on the GLDF state.
///
/// Serialized as `{"action": "<variant>", "data": <variant payload>}`
/// thanks to `#[serde(tag, content)]`. Binary fields (`bytes: Vec<u8>`)
/// use `serde_bytes` so JSON encodes them as compact byte arrays.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "action", content = "data")]
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
    /// Drop-and-link: embed a sensor data sheet (`.sensxml` / `.sensldt`)
    /// and mint a fresh `<Sensor>` linked to it. Mirrors AddPhotometryFile
    /// but writes to `GeneralDefinitions/Sensors` instead of `Photometries`
    /// and uses the correct `sensor/*` content type so the Files page
    /// doesn't need a follow-up retype dispatch. Delegates to
    /// `EditableGldf::add_sensor_file`.
    AddSensorFile { bytes: Vec<u8>, filename: String },
    /// Drop-and-link: embed a spectrum file (`.spd`) linked to a light source.
    /// Delegates to `EditableGldf::add_spectrum_file`. Errors (e.g. unknown
    /// light-source id) surface via `last_error`.
    AddSpectrumFile {
        bytes: Vec<u8>,
        filename: String,
        light_source_id: String,
    },
    /// Bind an already-declared `<Spectrum>` to a light source by setting
    /// its `SpectrumReference`. Pass `None` to clear the binding. Targets
    /// the FixedLightSource / ChangeableLightSource with that id.
    SetLightSourceSpectrum {
        light_source_id: String,
        spectrum_id: Option<String>,
    },
    /// Bind an already-declared `<Photometry>` to a `ChangeableLightSource`
    /// by setting its `PhotometryReference`. Pass `None` to clear.
    /// FixedLightSource has no `PhotometryReference` field per the XSD
    /// (photometry on a Fixed light source is reached via the Emitter's
    /// FixedLightEmitter), so this action errors if `light_source_id`
    /// names a FixedLightSource instead of a Changeable one.
    SetChangeableLightSourcePhotometry {
        light_source_id: String,
        photometry_id: Option<String>,
    },
    /// Add a product variant by name (English locale). Delegates to
    /// `EditableGldf::add_variant_simple`.
    AddVariantSimple { name: String },
    /// Rename a Variant id. Variant refs are auto-generated; only the
    /// id changes, no cascade needed.
    RenameVariant { old_id: String, new_id: String },
    /// Remove a Variant by id. Delegates to `GldfProduct::remove_variant`.
    RemoveVariant { id: String },
    /// Deep-clone an existing Variant. The new variant gets a fresh
    /// auto-generated id and "(copy)" appended to its Name; everything
    /// else (GTIN, mountings, geometry, DescriptiveAttributes, etc.) is
    /// duplicated verbatim. The most common multi-SKU workflow is
    /// "clone, then change CCT/length/whatever".
    DuplicateVariant { src_idx: usize },
    /// Set / clear the GTIN attribute on the n-th Variant.
    SetVariantGTIN { index: usize, gtin: Option<String> },
    /// Set / clear the `@sortOrder` attribute on the n-th Variant.
    /// XSD: lower numbers are shown first; missing falls back to
    /// document order.
    SetVariantSortOrder { index: usize, sort_order: Option<i32> },
    /// Move the n-th Variant up or down in the document order. `delta`
    /// is -1 / +1 in normal use. No-op if the move would go out of
    /// bounds. Note: document order is the canonical fallback when
    /// `sortOrder` is missing; sortOrder is a hint, not a guarantee.
    MoveVariant { index: usize, delta: i32 },
    /// Set / clear the variant `<Symbol fileId="…"/>` — a 2D CAD graphic
    /// (SVG/DXF). The referenced File must already exist under
    /// GeneralDefinitions/Files; the reducer does not validate (the
    /// Linkage view flags dangling ids).
    SetVariantSymbol {
        index: usize,
        file_id: Option<String>,
    },
    /// Replace Variant[idx].Geometry with a list of bare EmitterReferences
    /// (no 3D geometry attached). The variant's Geometry's other
    /// `xs:choice` branches (simple, model) are cleared. The XSD
    /// requires *at least one* binding for a valid GLDF; an empty list
    /// clears the Geometry entirely.
    SetVariantGeometryEmitterRefs {
        variant_idx: usize,
        emitter_ids: Vec<String>,
    },
    /// Replace Variant[idx].Geometry with a SimpleGeometryReference.
    /// Clears the other choice branches. Both ids must reference
    /// existing entries (geometry under `GeneralDefinitions/Geometries`,
    /// emitter under `GeneralDefinitions/Emitters`); the reducer does
    /// not validate.
    SetVariantGeometrySimple {
        variant_idx: usize,
        geometry_id: String,
        emitter_id: String,
    },
    /// Replace Variant[idx].Geometry with a ModelGeometryReference.
    /// `bindings` lists each emitter slot inside the 3D model, with
    /// the named object inside the model and an optional model-type
    /// disambiguator (l3d / m3d / r3d).
    SetVariantGeometryModel {
        variant_idx: usize,
        geometry_id: String,
        bindings: Vec<ModelEmitterBinding>,
    },
    /// Clear Variant[idx].Geometry to None.
    ClearVariantGeometry { variant_idx: usize },
    /// One-shot L3D import: stores the bytes as an embedded `<File>`
    /// with `contentType="application/x-gldf-l3d"`, auto-registers a
    /// matching `<ModelGeometry>` entry, and the new geometry id can
    /// be read off the resulting GldfProduct. The wizard's Geometry
    /// step and the Files page (`.l3d` drop) both use this.
    ImportL3dFile {
        bytes: Vec<u8>,
        filename: String,
    },
    /// Register a new `<SimpleGeometry>` with a single Cuboid child.
    /// Dimensions are in millimetres (matches XSD). Auto-generates an
    /// id; the new id ends up as the last entry in
    /// `general_definitions.geometries.simple_geometry`.
    AddSimpleGeometryCuboid {
        width_mm: i32,
        length_mm: i32,
        height_mm: i32,
    },
    /// Register a new `<SimpleGeometry>` with a single Cylinder child.
    /// `plane` is optional (defaults to "XY" per XSD).
    AddSimpleGeometryCylinder {
        plane: Option<String>,
        diameter_mm: i32,
        height_mm: i32,
    },
    /// Remove a SimpleGeometry by id from
    /// `general_definitions.geometries.simple_geometry`. Any variant
    /// `SimpleGeometryReference` pointing at this id will dangle;
    /// surfaced by the Linkage page.
    RemoveSimpleGeometry { id: String },
    /// Drop an `.spd` file: embed bytes as a `<File>` and auto-register
    /// a matching `<Spectrum>` entry. Unlike `AddSpectrumFile` this
    /// does NOT bind to a light source — the user picks the binding
    /// afterwards in the LightSources page. Same pattern as
    /// `ImportL3dFile` for L3D drops.
    ImportSpectrumFile {
        bytes: Vec<u8>,
        filename: String,
    },
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
    /// Rename a ChangeableLightSource id (cascades into Equipment refs).
    /// Delegates to `GldfProduct::rename_changeable_light_source`.
    RenameChangeableLightSource { old_id: String, new_id: String },
    /// Rename an Emitter id, cascading into every
    /// `Variant/Geometry/EmitterReference` (bare and nested forms).
    /// Delegates to `GldfProduct::rename_emitter`.
    RenameEmitter { old_id: String, new_id: String },
    /// Rename a MultiChannelLightSource id (cascades into every
    /// `Emitter/MultiChannelLightEmitter/LightSourceReference`).
    RenameMultiChannelLightSource { old_id: String, new_id: String },
    /// Append a fully-formed MultiChannelLightSource to the catalog. If
    /// `payload.id` is empty, an auto id is generated.
    AddMultiChannelLightSource {
        payload: Box<gldf_rs::gldf::general_definitions::lightsources::MultiChannelLightSource>,
    },
    /// Replace a MultiChannelLightSource in place. The payload's `id` is
    /// overwritten with the slot id so existing MultiChannelLightEmitter
    /// references keep resolving.
    UpdateMultiChannelLightSource {
        id: String,
        payload: Box<gldf_rs::gldf::general_definitions::lightsources::MultiChannelLightSource>,
    },
    /// Append a new Channel onto the named MultiChannelLightSource. The
    /// channel carries its own DisplayName / SpectrumReference /
    /// PhotometryReference / RatedLuminousFlux per the XSD.
    AddMultiChannelChannel {
        ls_id: String,
        channel: Box<gldf_rs::gldf::general_definitions::lightsources::MultiChannelChannel>,
    },
    /// Remove the n-th Channel from the named MultiChannelLightSource.
    RemoveMultiChannelChannel { ls_id: String, index: usize },
    /// Replace the n-th Channel in place.
    UpdateMultiChannelChannel {
        ls_id: String,
        index: usize,
        channel: Box<gldf_rs::gldf::general_definitions::lightsources::MultiChannelChannel>,
    },
    /// Append a fully-formed `FixedLightSource` to the catalog. If `payload.id`
    /// is empty a unique id is generated via `generate_unique_id("lightsource")`.
    /// Delegates to `GldfProduct::add_fixed_light_source`.
    AddFixedLightSource { payload: Box<gldf_rs::gldf::general_definitions::lightsources::FixedLightSource> },
    /// Append a fully-formed `ChangeableLightSource` to the catalog. If
    /// `payload.id` is empty a unique id is generated. Delegates to
    /// `GldfProduct::add_changeable_light_source`.
    AddChangeableLightSource { payload: Box<gldf_rs::gldf::general_definitions::lightsources::ChangeableLightSource> },
    /// Replace a FixedLightSource in place. `id` selects the slot; the
    /// payload's own `id` field is overwritten with `id` so the slot
    /// identity stays stable and existing references keep resolving.
    UpdateFixedLightSource {
        id: String,
        payload: Box<gldf_rs::gldf::general_definitions::lightsources::FixedLightSource>,
    },
    /// Replace a ChangeableLightSource in place. Same contract as
    /// `UpdateFixedLightSource`.
    UpdateChangeableLightSource {
        id: String,
        payload: Box<gldf_rs::gldf::general_definitions::lightsources::ChangeableLightSource>,
    },
    /// Remove a light source by id — fixed or changeable, the action looks
    /// in both lists. Surfaces the `last_error` if the id is unknown.
    /// Does NOT scrub dangling references; the Linkage map will surface
    /// those as orphans for the user to resolve.
    RemoveLightSource { id: String },
    /// Append a new empty `<Emitter>` to GeneralDefinitions/Emitters with
    /// an auto-generated id. The emitter has no LightEmitter children yet
    /// — the schema actually requires at least one, but we let the user
    /// add them via `AddFixedLightEmitter` / `AddChangeableLightEmitter`
    /// in the next click. The Linkage map will flag the empty emitter
    /// until it has a child.
    AddEmitter,
    /// Atomic "create a new `<Emitter>` with one initial LightEmitter
    /// child" — the XSD-conformant path because an Emitter without a
    /// child is invalid. The id is auto-generated; the caller is the new
    /// `<EmitterTypeForm>` which has already validated that the picked
    /// refs exist in the catalog.
    ///
    /// `child_name` populates the LightEmitter child's `<Name>` locale
    /// (the schema's "Translatable display name of the LightEmitter").
    /// The Emitter element itself has no Name in the schema — only an id.
    AddEmitterWithChild {
        /// Optional display name for the first LightEmitter child.
        child_name: Option<String>,
        /// Which LightEmitter kind to create as the first child.
        kind: LightEmitterKind,
        /// PhotometryReference for Fixed/Changeable. Empty for MC/Sensor.
        photometry_id: String,
        /// LightSourceReference for Fixed (FixedLightSourceId).
        fixed_light_source_id: String,
        /// LightSourceReference for MultiChannel (MultiChannelLightSourceId).
        multi_channel_light_source_id: String,
        /// SensorReference for Sensor.
        sensor_id: String,
    },
    /// Remove an Emitter by id. Variants that bind this emitter will
    /// dangle — surface via Linkage. Delegates to `remove_emitter`.
    RemoveEmitter { id: String },
    /// Create an `<Emitter>` with an EXPLICIT id whose single
    /// FixedLightEmitter binds the photometry + light source and carries
    /// the given RatedLuminousFlux — all atomically. Unlike
    /// `AddEmitterWithChild` (which auto-generates the id and sets flux
    /// to 0), this lets scripted / generated callers reference the
    /// emitter by a stable name and set its lumens in one step, so there
    /// are no predicted-id off-by-one bugs and no "0 lm" gaps. If
    /// `emitter_id` is empty an id is auto-generated.
    AddFixedEmitter {
        emitter_id: String,
        child_name: Option<String>,
        photometry_id: String,
        light_source_id: String,
        rated_luminous_flux: i32,
    },
    /// Add a new `<FixedLightEmitter>` child to the named Emitter,
    /// referencing the given photometry + fixed light source. Both ids
    /// should exist in the catalog; the action does not validate them
    /// (Linkage flags unknown ids).
    AddFixedLightEmitter {
        emitter_id: String,
        photometry_id: String,
        light_source_id: String,
    },
    /// Add a new `<ChangeableLightEmitter>` child to the named Emitter,
    /// referencing the given photometry.
    AddChangeableLightEmitter {
        emitter_id: String,
        photometry_id: String,
    },
    /// Add a new `<MultiChannelLightEmitter>` child to the named Emitter,
    /// pointing at a declared MultiChannelLightSource. Per the XSD the
    /// photometry+spectrum live PER-CHANNEL inside the LightSource, so
    /// only the lamp id is bound at the emitter level.
    AddMultiChannelLightEmitter {
        emitter_id: String,
        multi_channel_light_source_id: String,
    },
    /// Add a new `<SensorEmitter>` child to the named Emitter, pointing
    /// at a declared `<Sensor>` (GeneralDefinitions/Sensors).
    AddSensorEmitter {
        emitter_id: String,
        sensor_id: String,
    },
    /// Remove the n-th LightEmitter child of the given kind from the
    /// named Emitter. Errors if the index is out of range.
    RemoveLightEmitter {
        emitter_id: String,
        kind: LightEmitterKind,
        index: usize,
    },
    /// Swap the `PhotometryReference` on the n-th FixedLightEmitter child
    /// of the named Emitter.
    SetFixedLightEmitterPhotometry {
        emitter_id: String,
        index: usize,
        photometry_id: String,
    },
    /// Set `RatedLuminousFlux` on the n-th FixedLightEmitter child of
    /// the named Emitter (lumens, integer per rc.3 XSD). Used by the
    /// Trahe ACME e2e to give wizard-built emitters realistic light
    /// output so the Bevy 3D viewer doesn't render a dark scene.
    SetFixedLightEmitterFlux {
        emitter_id: String,
        index: usize,
        rated_luminous_flux: i32,
    },
    /// Swap the `LightSourceReference` on the n-th FixedLightEmitter
    /// child. Pass `None` to clear (schema invalid, but the user might
    /// want to fix the binding in two clicks).
    SetFixedLightEmitterLightSource {
        emitter_id: String,
        index: usize,
        light_source_id: Option<String>,
    },
    /// Set / clear `EmergencyBehaviour` on the n-th FixedLightEmitter
    /// child. Empty string → `None`; otherwise stores the XSD enum
    /// verbatim (e.g. `"EmergencyOnly"`, `"Combined"`, `"NormalOnly"`).
    /// gldf-bevy reads this off the per-emitter config and applies its
    /// 0.1× dim multiplier when the value is `"EmergencyOnly"` so the
    /// 3D viewer shows a realistic emergency-mode brightness.
    SetFixedLightEmitterEmergencyBehaviour {
        emitter_id: String,
        index: usize,
        behaviour: Option<String>,
    },
    /// Swap the `PhotometryReference` on the n-th ChangeableLightEmitter
    /// child.
    SetChangeableLightEmitterPhotometry {
        emitter_id: String,
        index: usize,
        photometry_id: String,
    },
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
    /// Embed arbitrary bytes as a new `<File>` in one shot: assigns a
    /// unique id, infers the content_type from the filename extension
    /// (`.ldt` → `ldc/eulumdat`, `.spd` → `spectrum/text`, etc.), pushes
    /// the `<File>` entry, and stores the bytes in `embedded_files`. This
    /// is the generic "drop a file into the bundle" affordance — the
    /// Photometry / Spectrum drop flows still go through their typed
    /// actions because they also create the wrapper entry (`<Photometry>`
    /// / `<Spectrum>`) and (for spectrum) the link to a light source.
    AddFileWithBytes { bytes: Vec<u8>, filename: String },
    /// Drop-and-link a product picture: embed the image bytes as a
    /// `<File>` (content_type inferred from the extension) AND add a
    /// matching `ProductMetaData/Pictures/Image` reference with the
    /// given `image_type` (e.g. "Product Picture"), atomically. Mirrors
    /// `AddPhotometryFile` so scripted callers don't have to predict the
    /// auto file id. Scope is always Product.
    AddProductPictureFile {
        bytes: Vec<u8>,
        filename: String,
        image_type: String,
    },
    /// Rename a `<File>` id, cascading into every referencing slot
    /// (Photometry / Spectrum / LightSource images / Geometry /
    /// Sensor / ProductMetaData pictures / Variant pictures+symbol).
    /// Also moves the bytes in `embedded_files` so the bundle stays
    /// internally consistent. Delegates to `GldfProduct::rename_file`.
    RenameFile { old_id: String, new_id: String },
    /// Rename a Photometry by id; cascades to every PhotometryReference.
    RenamePhotometry { old_id: String, new_id: String },
    /// Remove a `<Photometry>` entry by id. Does NOT touch the underlying
    /// `<File>` it points to — drop or repurpose that separately on the
    /// Files page. Use when an auto-imported Photometry block is no longer
    /// appropriate (e.g. the file was reclassified as a sensor payload).
    RemovePhotometry { id: String },
    // --- Sensors (GeneralDefinitions/Sensors/Sensor) ---
    AddSensor,
    RemoveSensor { index: usize },
    RenameSensor { old_id: String, new_id: String },
    SetSensorFileId { index: usize, file_id: Option<String> },
    /// Replace the full DetectorCharacteristic list (XSD enum, max 3).
    SetSensorDetectorCharacteristics { index: usize, values: Vec<String> },
    /// Replace the full DetectionMethod list (XSD enum, max 6).
    SetSensorDetectionMethods { index: usize, values: Vec<String> },
    /// Replace the full DetectorType list (XSD enum, max 4).
    SetSensorDetectorTypes { index: usize, values: Vec<String> },
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
    // --- Pictures (ProductMetaData.Pictures · Variant.Pictures) ---
    // Scope-aware: Scope::Product writes into ProductMetaData.Pictures;
    // Scope::Variant(idx) writes into Variants/Variant[idx].Pictures.
    AddProductMetaPicture {
        scope: Scope,
        file_id: String,
        image_type: String,
    },
    UpdateProductMetaPicture {
        scope: Scope,
        index: usize,
        file_id: String,
        image_type: String,
    },
    RemoveProductMetaPicture {
        scope: Scope,
        index: usize,
    },
    // --- Hyperlinks (ProductMetaData.DescriptiveAttributes.Marketing.Hyperlinks ·
    //                 Variant.DescriptiveAttributes.Marketing.Hyperlinks) ---
    AddHyperlink(Scope),
    UpdateHyperlink {
        scope: Scope,
        index: usize,
        href: String,
        language: Option<String>,
        region: Option<String>,
        country_code: Option<String>,
        value: String,
    },
    RemoveHyperlink {
        scope: Scope,
        index: usize,
    },
    // --- Marketing.HousingColors (RAL + locale name, scope-aware) ---
    AddHousingColor(Scope),
    UpdateHousingColor {
        scope: Scope,
        index: usize,
        ral: Option<i32>,
        language: String,
        value: String,
    },
    RemoveHousingColor {
        scope: Scope,
        index: usize,
    },
    // --- Marketing.Markets.Region (Locale entries, scope-aware) ---
    AddMarketRegion(Scope),
    UpdateMarketRegion {
        scope: Scope,
        index: usize,
        language: String,
        value: String,
    },
    RemoveMarketRegion {
        scope: Scope,
        index: usize,
    },
    // --- Marketing.ApprovalMarks.ApprovalMark (string list, scope-aware) ---
    AddApprovalMark(Scope),
    UpdateApprovalMark {
        scope: Scope,
        index: usize,
        value: String,
    },
    RemoveApprovalMark {
        scope: Scope,
        index: usize,
    },
    // --- DescriptiveAttributes.CustomProperties.Property (scope-aware).
    // XSD: id (string), Name (Locale), PropertySource (string, e.g. ISO/CIE/DIN),
    // Value (string) XOR FileReference (file id). We expose Value via this
    // action set; FileReference editing isn't covered yet (rare, and would
    // need a file picker UI). ---
    AddCustomProperty(Scope),
    UpdateCustomPropertyId {
        scope: Scope,
        index: usize,
        value: String,
    },
    UpdateCustomPropertyName {
        scope: Scope,
        index: usize,
        value: gldf_rs::gldf::LocaleFoo,
    },
    UpdateCustomPropertySource {
        scope: Scope,
        index: usize,
        value: String,
    },
    UpdateCustomPropertyValue {
        scope: Scope,
        index: usize,
        value: String,
    },
    RemoveCustomProperty {
        scope: Scope,
        index: usize,
    },
    // --- Marketing.DesignAwards.DesignAward (string list, scope-aware) ---
    AddDesignAward(Scope),
    UpdateDesignAward {
        scope: Scope,
        index: usize,
        value: String,
    },
    RemoveDesignAward {
        scope: Scope,
        index: usize,
    },
    // --- ControlGear (GeneralDefinitions/ControlGears/ControlGear) ---
    AddControlGear {
        prefix: Option<String>,
    },
    UpdateControlGear {
        id: String,
        name: String,
        nominal_voltage_value: Option<f64>,
        standby_power: Option<f64>,
        dimmable: Option<bool>,
        color_controllable: Option<bool>,
        constant_light_output_start_power: Option<f64>,
        constant_light_output_end_power: Option<f64>,
    },
    RemoveControlGear {
        id: String,
    },
    RenameControlGear {
        old_id: String,
        new_id: String,
    },
    // --- Equipment (GeneralDefinitions/Equipments/Equipment) ---
    AddEquipment {
        prefix: Option<String>,
    },
    UpdateEquipment {
        id: String,
        light_source_id: Option<String>,
        light_source_kind: Option<LightEmitterKind>,
        control_gear_id: Option<String>,
        rated_input_power: f64,
        emergency_ballast_lumen_factor: Option<f64>,
        emergency_rated_luminous_flux: Option<i32>,
    },
    RemoveEquipment {
        id: String,
    },
    RenameEquipment {
        old_id: String,
        new_id: String,
    },
    /// Bind/clear ControlGearReference on a Fixed/Changeable/MultiChannel
    /// LightEmitter inside a given Emitter. The emitter is found by id,
    /// then the right sub-array is updated by `kind` + `index`.
    SetEmitterControlGearReference {
        emitter_id: String,
        kind: LightEmitterKind,
        light_emitter_index: usize,
        control_gear_id: Option<String>,
    },
    // --- LightSource.ColorInformation (per fixed/changeable light source) ---
    SetLightSourceCri {
        light_source_id: String,
        value: Option<i32>,
    },
    SetLightSourceCct {
        light_source_id: String,
        value: Option<i32>,
    },
    SetLightSourceCctRange {
        light_source_id: String,
        lower: Option<i32>,
        upper: Option<i32>,
    },
    SetLightSourceInitialColorTolerance {
        light_source_id: String,
        value: Option<String>,
    },
    SetLightSourceMaintainedColorTolerance {
        light_source_id: String,
        value: Option<String>,
    },
    SetLightSourceTlci {
        light_source_id: String,
        value: Option<i32>,
    },
    SetLightSourceMelanopicFactor {
        light_source_id: String,
        value: Option<f64>,
    },
    // --- Emergency (ProductMetaData.DescriptiveAttributes.Emergency ·
    //                Variant.DescriptiveAttributes.Emergency) ---
    SetDedicatedEmergencyLightingType(Scope, Option<String>),
    AddEmergencyFlux(Scope),
    UpdateEmergencyFlux {
        scope: Scope,
        index: usize,
        hours: i32,
        flux: i32,
    },
    RemoveEmergencyFlux {
        scope: Scope,
        index: usize,
    },
    /// Force a fresh sync of the n-th Variant's L3D MTL materials from
    /// its first HousingColor's RAL value. Same machinery that auto-fires
    /// from `UpdateHousingColor` at variant scope, but reachable as a
    /// standalone trigger — useful when the user drops a new L3D file
    /// AFTER setting the RAL (the file-drop path doesn't see the RAL
    /// change, so auto-fire doesn't catch it).
    ///
    /// Outcome (success or hard error) lands in `last_error` for the
    /// caller to surface. Non-actionable cases (no RAL, unknown RAL,
    /// no L3D) are silent no-ops on purpose — they're routine mid-edit
    /// states, not user-visible problems.
    EnsureL3dMtlForVariant { variant_idx: usize },
    /// Replace the n-th Variant's `<Mountings>` block. Passing `None`
    /// clears it. The XSD allows any combination of the four top-level
    /// branches (Ceiling / Wall / Ground / WorkingPlane), so the action
    /// takes the full `Mountings` payload — the WASM editor builds it
    /// up from its sub-pickers.
    SetVariantMountings {
        index: usize,
        payload: Option<Box<gldf_rs::gldf::product_definitions::Mountings>>,
    },
    /// Replace the n-th Variant's `<ProductSeries>` block. Each entry
    /// carries an `@id` plus an optional `<Name>` Locale (other XSD
    /// slots — Description, Pictures, Hyperlinks — exist on the
    /// `ProductSerie` struct but stay defaulted by the basic editor;
    /// future cuts can expand). Passing an empty Vec clears the field
    /// entirely (XSD makes it optional).
    SetVariantProductSeries {
        index: usize,
        payload: Vec<gldf_rs::gldf::general_definitions::ProductSerie>,
    },
    /// Reset state to default
    #[allow(dead_code)]
    Reset,
}
