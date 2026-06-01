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
    /// Swap the `LightSourceReference` on the n-th FixedLightEmitter
    /// child. Pass `None` to clear (schema invalid, but the user might
    /// want to fix the binding in two clicks).
    SetFixedLightEmitterLightSource {
        emitter_id: String,
        index: usize,
        light_source_id: Option<String>,
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
    /// Rename a `<File>` id, cascading into every referencing slot
    /// (Photometry / Spectrum / LightSource images / Geometry /
    /// Sensor / ProductMetaData pictures / Variant pictures+symbol).
    /// Also moves the bytes in `embedded_files` so the bundle stays
    /// internally consistent. Delegates to `GldfProduct::rename_file`.
    RenameFile { old_id: String, new_id: String },
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
