//! Portable wizard logic shared between the Leptos forms (WASM
//! frontend) and the ratatui modal (TUI frontend). The wizard's
//! per-step UI lives in each frontend; this module owns the
//! frontend-agnostic [`WizardDraft`] data shape and the [`apply_wizard`]
//! reducer that dispatches actions in XSD-conformant order.

use crate::action::{GldfAction, LightEmitterKind, ModelEmitterBinding};
use crate::emitter_draft::EmitterDraft;
use crate::state::GldfState;
use gldf_rs::gldf::general_definitions::lightsources::{
    ChangeableLightSource, FixedLightSource, MultiChannelLightSource,
};
use gldf_rs::gldf::header::{Locale, LocaleFoo};

/// Which LightSource kind the user picked in the LightSource step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum WizardLightSourceKind {
    #[default]
    Fixed,
    Changeable,
    MultiChannel,
}

/// Plain-data wizard draft. Both the Leptos wizard (whose RwSignal-backed
/// form fields populate this on Finish) and the TUI wizard (whose
/// textarea buffers populate this) build the same struct and feed it to
/// [`apply_wizard`].
///
/// Serializes via `serde` so CLI / Python consumers can ship a wizard run
/// as a JSON file (`gldf-cli wizard --draft draft.json -o out.gldf`).
/// Every field is `#[serde(default)]` so a draft (or a batch template,
/// see [`crate::batch`]) only needs to spell out the fields it overrides.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WizardDraft {
    pub header_author: String,
    pub header_manufacturer: String,
    pub header_created_with: String,
    pub header_default_language: String,
    pub photometry_bytes: Option<Vec<u8>>,
    pub photometry_filename: String,
    pub ls_kind: WizardLightSourceKind,
    pub ls_name: String,
    pub ls_manufacturer: String,
    pub ls_rated_power: String,
    pub emitter_draft: Option<EmitterDraft>,
    /// Display name for the first product Variant. The XSD requires at
    /// least one Variant per GLDF, so the wizard always creates one
    /// (empty → "Standard"). Empty string means "use the default".
    pub variant_name: String,
    /// L3D bytes for the Geometry step. `None` → wizard binds the
    /// emitter to the variant via a bare `<EmitterReference>` (no 3D
    /// model). `Some(bytes)` → wizard registers an L3D `<File>` +
    /// `<ModelGeometry>` and writes a `ModelGeometryReference` into
    /// the variant.
    pub geometry_bytes: Option<Vec<u8>>,
    pub geometry_filename: String,
    /// Driver step (optional). When `driver_enabled` is false the
    /// other driver_* fields are ignored and no ControlGear / Equipment /
    /// ControlGearReference is written. When true, the reducer creates
    /// one of each in XSD-conformant order and binds them to the
    /// wizard's emitter + light source.
    pub driver_enabled: bool,
    pub driver_name: String,
    pub driver_dimmable: bool,
    /// Standby power in watts as a string (matches the WASM widget's raw
    /// text input). Empty / unparseable → no value written.
    pub driver_standby_power: String,
    /// Mounting step (optional). When `mounting_type` is empty no
    /// `<Mountings>` block is written. Otherwise the reducer creates a
    /// `Variant.Mountings` record with the matching branch:
    ///   - "ceiling/recessed" → `<Ceiling><Recessed recessedDepth=…/></Ceiling>`
    ///   - "ceiling/surface"  → `<Ceiling><SurfaceMounted/></Ceiling>`
    ///   - "ceiling/pendant"  → `<Ceiling><Pendant pendantLength=…/></Ceiling>`
    ///   - "wall"             → `<Wall mountingHeight=…/>` (+ optional Recessed)
    ///   - "ground/pole_top"  → `<Ground><PoleTop poleHeight=…/></Ground>`
    ///   - "ground/free_standing" → `<Ground><FreeStanding/></Ground>`
    ///   - "working_plane"    → `<WorkingPlane><FreeStanding/></WorkingPlane>`
    ///
    /// Drives the Bevy 3D viewer's `MountingConfig` placement so a
    /// freshly-built downlight visually sinks into the ceiling cavity
    /// instead of floating below it.
    pub mounting_type: String,
    /// Recessed depth in mm (used for ceiling/recessed and wall+recessed).
    /// Stored as a string to mirror the WASM widget's raw input. Empty /
    /// unparseable → no Recessed sub-block is written.
    pub mounting_recessed_depth_mm: String,
    /// Pendant length in mm (ceiling/pendant). Same parsing as above.
    pub mounting_pendant_length_mm: String,
    /// Mounting height in mm (wall). Same parsing as above. Empty for
    /// other branches.
    pub mounting_height_mm: String,
    /// Pole height in mm (ground/pole_top / ground/pole_integrated).
    pub mounting_pole_height_mm: String,
    /// Sensor step (optional). When `sensor_bytes` is `Some`, the wizard
    /// dispatches `AddSensorFile` which embeds the bytes as a
    /// `sensor/sensxml` or `sensor/sensldt` `<File>` AND mints a fresh
    /// `<Sensor>` entry pointing at it. The Files page's drop sniffer
    /// can't tell a sensor sheet from a photometric LDT just from the
    /// extension, so plumbing this through the wizard lets the user
    /// declare intent up front rather than retype-after-drop.
    pub sensor_bytes: Option<Vec<u8>>,
    pub sensor_filename: String,
    /// Emergency step (optional). When `emergency_bytes` is `Some`, the
    /// wizard dispatches `AddPhotometryFile` for the emergency LDT/IES so
    /// the GLDF carries TWO `<Photometry>` entries: the normal-mode beam
    /// (from the Photometry step) AND a dimmer emergency-mode beam keyed
    /// to a battery-pack driver. Vendors typically declare only the
    /// normal photometry plus an `EmergencyBehaviour` tag and leave
    /// consumers guessing the emergency beam shape — this step makes the
    /// emergency Photometry a first-class entry so the LightSource card
    /// can render both thumbnails side by side. The wizard does NOT bind
    /// this to an emitter — the demo / user creates an `EmergencyOnly`
    /// emitter that references this photometry id later.
    pub emergency_bytes: Option<Vec<u8>>,
    pub emergency_filename: String,
}

/// Steps the wizard walks through. Used to drive both the Leptos step
/// switcher and the TUI step indicator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WizardStep {
    Header,
    Photometry,
    Sensor,
    Emergency,
    LightSource,
    Emitter,
    Geometry,
    Mounting,
    Driver,
    Variant,
    Review,
}

impl WizardStep {
    pub const ALL: &'static [Self] = &[
        Self::Header,
        Self::Photometry,
        Self::Sensor,
        Self::Emergency,
        Self::LightSource,
        Self::Emitter,
        Self::Geometry,
        Self::Mounting,
        Self::Driver,
        Self::Variant,
        Self::Review,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Header => "Header",
            Self::Photometry => "Photometry",
            Self::Sensor => "Sensor",
            Self::Emergency => "Emergency",
            Self::LightSource => "Light source",
            Self::Emitter => "Emitter",
            Self::Geometry => "Geometry",
            Self::Mounting => "Mounting",
            Self::Driver => "Driver",
            Self::Variant => "Variant",
            Self::Review => "Review & Finish",
        }
    }

    /// One-based step number for UI display (`"1"`, `"2"`, …). Together
    /// with [`Self::label`] this is what every frontend uses as the step
    /// heading. Lives in the lib so WASM, TUI, and CLI render the
    /// same numbering.
    pub fn number(&self) -> u8 {
        match self {
            Self::Header => 1,
            Self::Photometry => 2,
            Self::Sensor => 3,
            Self::Emergency => 4,
            Self::LightSource => 5,
            Self::Emitter => 6,
            Self::Geometry => 7,
            Self::Mounting => 8,
            Self::Driver => 9,
            Self::Variant => 10,
            Self::Review => 11,
        }
    }

    /// Headline shown above the step body: e.g. `"1 · Header"`. Owned
    /// `String` so WASM (no lifetime story across reactive closures)
    /// and TUI (`Line<'static>`) both consume cleanly.
    pub fn headline(&self) -> String {
        format!("{} · {}", self.number(), self.label())
    }

    /// One-sentence description shown under the headline. The single
    /// source of truth for the wizard's helper copy — frontends must
    /// not duplicate this in their view layer.
    pub fn blurb(&self) -> &'static str {
        match self {
            Self::Header => {
                "Document-level metadata. Author and manufacturer show up on the Overview card."
            }
            Self::Photometry => {
                "Drop a .ldt (EULUMDAT) or .ies (IESNA LM-63) file. The wizard will create the matching <File> and <Photometry> entries when you click Create at the end."
            }
            Self::Sensor => {
                "Optional. Drop a sensor data sheet (.sensxml or .sensldt) if the luminaire ships with a motion / presence / daylight detector. The wizard registers it as a <File contentType=\"sensor/sensldt\"> AND mints a <Sensor> entry pointing at it — no retype-after-drop needed. Skip if the luminaire has no sensor."
            }
            Self::Emergency => {
                "Optional. Drop the EMERGENCY-mode photometry (.ldt / .ies) if the luminaire has battery-pack EmergencyOnly / Combined behaviour. The wizard adds a second <Photometry> entry so the LightSource card shows both normal and emergency beam thumbnails. Vendors usually ship only the normal beam — declaring the emergency beam as a first-class photometry is what the gldf.icu demo shows off here. Skip if the luminaire is normal-only."
            }
            Self::LightSource => {
                "The lamp itself. Fixed = baked into the fixture. Changeable = user-replaceable bulb. MultiChannel = tunable-white / RGB."
            }
            Self::Emitter => {
                "An Emitter is a slot on the luminaire that binds the photometry (beam shape) to the light source (lamp). The form below uses the same XSD-conformant type picker as the editor."
            }
            Self::Geometry => {
                "The 3D shell of the variant. Drop an .l3d file to bind the emitter into a 3D model, or skip — the wizard then writes a bare EmitterReference. Either way the GLDF will have a valid Variant.Geometry on Finish."
            }
            Self::Mounting => {
                "How the luminaire installs. Pick a primitive (Ceiling recessed/surface/pendant, Wall, Ground, Working plane) and the wizard writes a <Variant.Mountings> block. Drives the Bevy 3D viewer's placement (a recessed downlight visibly sinks into the ceiling cavity). Skip — no Mountings block is written and the viewer falls back to its default scene placement."
            }
            Self::Driver => {
                "Optional driver / ballast for the luminaire. Tick \"include\" to declare a ControlGear with dimmable + standby-power metadata, an Equipment that bundles it with the LightSource, and a ControlGearReference on the emitter's first slot. Skip and the sidebar Control Gear / Equipment pages still let you add one later."
            }
            Self::Variant => {
                "A Variant is a sellable configuration of the luminaire. GLDF requires at least one — the wizard will create a single 'Standard' variant if you leave this blank."
            }
            Self::Review => {
                "Confirm what the wizard will create. Clicking Create GLDF dispatches all changes in order and drops you on the Overview page."
            }
        }
    }
}

/// Field-level helper text — labels, placeholders, status snippets.
/// Every frontend reads these strings instead of hard-coding their
/// own copy. Keep this surface small and stable; if a frontend needs
/// a new label, add it here and let every consumer pick it up.
pub mod text {
    // --- Header step ---
    pub const HEADER_AUTHOR_LABEL: &str = "Author";
    pub const HEADER_AUTHOR_PLACEHOLDER: &str = "Your name or organization";
    pub const HEADER_MANUFACTURER_LABEL: &str = "Manufacturer";
    pub const HEADER_MANUFACTURER_PLACEHOLDER: &str = "The luminaire brand";
    pub const HEADER_CREATED_WITH_LABEL: &str = "Created with";
    pub const HEADER_DEFAULT_LANGUAGE_LABEL: &str = "Default language";
    pub const HEADER_DEFAULT_LANGUAGE_PLACEHOLDER: &str = "en / de / fr / …";

    // --- Photometry step ---
    pub const PHOTOMETRY_DROP_LABEL: &str = "Drop or pick a photometry file";
    pub const PHOTOMETRY_SKIP_HINT: &str =
        "Optional — Skip with Next if you'll add photometry later.";
    /// Format the "loaded" line shown once the user picked a file.
    /// Frontends should call this rather than building the string
    /// themselves so wording stays in sync.
    pub fn photometry_loaded(filename: &str, byte_count: usize) -> String {
        format!("✓ {} ({} bytes) staged", filename, byte_count)
    }

    // --- LightSource step ---
    pub const LS_KIND_LABEL: &str = "Kind";
    pub const LS_KIND_FIXED: &str = "Fixed (baked-in lamp)";
    pub const LS_KIND_CHANGEABLE: &str = "Changeable (user-replaceable)";
    pub const LS_KIND_MULTICHANNEL: &str = "MultiChannel (tunable / RGB)";
    pub const LS_NAME_LABEL: &str = "Display name";
    pub const LS_NAME_PLACEHOLDER: &str = "e.g. LED 3000K chip";
    pub const LS_MANUFACTURER_LABEL: &str = "Manufacturer";
    pub const LS_MANUFACTURER_PLACEHOLDER: &str = "Lamp brand (optional)";
    pub const LS_POWER_LABEL: &str = "Rated power (W)";
    pub const LS_POWER_PLACEHOLDER: &str = "e.g. 24";

    // --- Emitter step ---
    pub const EMITTER_STAGED: &str = "✓ Emitter staged";
    pub const EMITTER_INCLUDE_LABEL: &str = "Include Emitter";
    pub const EMITTER_NAME_LABEL: &str = "Emitter name";
    pub const EMITTER_SUBMIT_LABEL: &str = "Stage Emitter";

    // --- Variant step ---
    pub const VARIANT_NAME_LABEL: &str = "Variant name";
    pub const VARIANT_NAME_PLACEHOLDER: &str = "Standard";
    pub const VARIANT_HINT: &str =
        "Leave blank for a single 'Standard' variant — add more later in the editor.";

    // --- Review step ---
    pub const REVIEW_FOOTNOTE: &str =
        "After Create, use the toolbar's Save button to download the .gldf file.";
    pub const REVIEW_PHOTOMETRY_SKIPPED: &str = "(skipped; add later via the editor)";
    pub const REVIEW_EMITTER_SKIPPED: &str = "(skipped)";
}

impl WizardLightSourceKind {
    /// Long, descriptive label used by the LightSource step's kind picker.
    pub fn long_label(&self) -> &'static str {
        match self {
            Self::Fixed => text::LS_KIND_FIXED,
            Self::Changeable => text::LS_KIND_CHANGEABLE,
            Self::MultiChannel => text::LS_KIND_MULTICHANNEL,
        }
    }

    /// Short label used in compact contexts (TUI status line, CLI
    /// review). Mirrors the enum variant name.
    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Fixed => "Fixed",
            Self::Changeable => "Changeable",
            Self::MultiChannel => "MultiChannel",
        }
    }
}

/// Dispatch all actions in a [`WizardDraft`] against `state` in
/// XSD-conformant order: Header → Photometry → LightSource → Emitter
/// (the order matters because the Emitter's child binds the
/// auto-generated photometry/lightsource ids the prior steps just
/// created).
pub fn apply_wizard(state: &mut GldfState, draft: &WizardDraft) {
    // Stamp the XSD-required Header fields (UniqueGldfId,
    // GldfCreationTimeCode) and ProductMetaData (UniqueProductId)
    // once at wizard time. This is the right place because (a) a
    // wizard-created GLDF has never been validated by anyone, and
    // (b) generating the UUID here keeps the lib's serializer
    // round-trip stable — once stamped, the value is part of the model.
    state.product.header.ensure_xsd_required();
    {
        use gldf_rs::gldf::product_definitions::ProductMetaData;
        let pmd = state
            .product
            .product_definitions
            .product_meta_data
            .get_or_insert_with(ProductMetaData::default);
        pmd.ensure_xsd_required();
    }

    // Header
    let author = draft.header_author.trim();
    if !author.is_empty() {
        state.apply(GldfAction::SetAuthor(author.to_string()));
    }
    let manufacturer = draft.header_manufacturer.trim();
    if !manufacturer.is_empty() {
        state.apply(GldfAction::SetManufacturer(manufacturer.to_string()));
    }
    let created_with = draft.header_created_with.trim();
    if !created_with.is_empty() {
        state.apply(GldfAction::SetCreatedWithApplication(
            created_with.to_string(),
        ));
    }
    let default_lang = draft.header_default_language.trim();
    if !default_lang.is_empty() {
        state.apply(GldfAction::SetDefaultLanguage(Some(
            default_lang.to_string(),
        )));
    }

    // Photometry
    if let Some(bytes) = &draft.photometry_bytes {
        state.apply(GldfAction::AddPhotometryFile {
            bytes: bytes.clone(),
            filename: draft.photometry_filename.clone(),
        });
    }
    // Capture the main photometry's auto-generated id NOW — the
    // Emergency step below adds a second <Photometry>, so "last" is only
    // the main beam at this point. The Emitter step falls back to this
    // id when the draft's photometry_id is blank (the common case: the
    // UI pickers can't know an id that doesn't exist until Finish).
    let main_photometry_id = if draft.photometry_bytes.is_some() {
        state
            .product
            .general_definitions
            .photometries
            .as_ref()
            .and_then(|p| p.photometry.last())
            .map(|p| p.id.clone())
    } else {
        None
    };

    // Sensor — optional. When the user dropped a sensor data sheet,
    // dispatch AddSensorFile which embeds the bytes as a sensor/sens*
    // <File> AND mints a fresh <Sensor> entry pointing at it. This
    // means the demo (and any consumer driving the wizard) doesn't
    // need a follow-up retype dispatch on the Files page — the
    // content_type is correct from the start.
    if let Some(bytes) = &draft.sensor_bytes {
        state.apply(GldfAction::AddSensorFile {
            bytes: bytes.clone(),
            filename: draft.sensor_filename.clone(),
        });
    }

    // Emergency photometry — optional. Same dispatch as the main
    // Photometry step (`AddPhotometryFile`) but tagged with a hint
    // filename so the downstream rename pass can map the auto id
    // (e.g. `photometryFile_2`) to `ph_emergency`. The wizard does NOT
    // bind it to any emitter — declaring the emergency beam as a
    // first-class Photometry is the point of this step; consumers /
    // demos wire an EmergencyOnly emitter to the resulting id
    // separately.
    if let Some(bytes) = &draft.emergency_bytes {
        state.apply(GldfAction::AddPhotometryFile {
            bytes: bytes.clone(),
            filename: draft.emergency_filename.clone(),
        });
    }

    // Light source — build the name locale once, then dispatch the typed Add.
    let name = draft.ls_name.trim();
    let name_locale = if name.is_empty() {
        LocaleFoo::default()
    } else {
        LocaleFoo {
            locale: vec![Locale {
                language: String::new(),
                value: name.to_string(),
            }],
        }
    };
    let manufacturer = draft.ls_manufacturer.trim();
    let manufacturer_opt = if manufacturer.is_empty() {
        None
    } else {
        Some(manufacturer.to_string())
    };
    let power = draft.ls_rated_power.trim().parse::<f64>().ok();

    match draft.ls_kind {
        WizardLightSourceKind::Fixed => {
            let ls = FixedLightSource {
                name: name_locale,
                manufacturer: manufacturer_opt,
                rated_input_power: power,
                ..Default::default()
            };
            state.apply(GldfAction::AddFixedLightSource {
                payload: Box::new(ls),
            });
        }
        WizardLightSourceKind::Changeable => {
            let ls = ChangeableLightSource {
                name: name_locale,
                manufacturer: manufacturer_opt,
                ..Default::default()
            };
            state.apply(GldfAction::AddChangeableLightSource {
                payload: Box::new(ls),
            });
        }
        WizardLightSourceKind::MultiChannel => {
            let ls = MultiChannelLightSource {
                name: name_locale,
                manufacturer: manufacturer_opt,
                rated_input_power: power.unwrap_or(0.0),
                ..Default::default()
            };
            state.apply(GldfAction::AddMultiChannelLightSource {
                payload: Box::new(ls),
            });
        }
    }

    // Emitter (atomic with first LightEmitter child). Blank reference
    // ids in the draft resolve to the ids the prior steps just
    // auto-generated — that's the whole point of dispatching in
    // XSD-conformant order (see the fn doc). Leaving them blank used to
    // write empty IDREFs, which the rc-3 XSD rejects
    // (KeyRef 'FixedEmitter-Photometry' / '-FixedLightSource').
    if let Some(emitter) = &draft.emitter_draft {
        let child_name_opt = if emitter.child_name.trim().is_empty() {
            None
        } else {
            Some(emitter.child_name.clone())
        };
        let last_ls_id = |kind: WizardLightSourceKind| -> Option<String> {
            let ls = state.product.general_definitions.light_sources.as_ref()?;
            match kind {
                WizardLightSourceKind::Fixed => ls.fixed_light_source.last().map(|l| l.id.clone()),
                WizardLightSourceKind::Changeable => {
                    ls.changeable_light_source.last().map(|l| l.id.clone())
                }
                WizardLightSourceKind::MultiChannel => {
                    ls.multi_channel_light_source.last().map(|l| l.id.clone())
                }
            }
        };
        let mut photometry_id = emitter.photometry_id.clone();
        if photometry_id.trim().is_empty() {
            if let Some(id) = &main_photometry_id {
                photometry_id = id.clone();
            }
        }
        let mut fixed_light_source_id = emitter.fixed_light_source_id.clone();
        if fixed_light_source_id.trim().is_empty()
            && matches!(draft.ls_kind, WizardLightSourceKind::Fixed)
        {
            if let Some(id) = last_ls_id(WizardLightSourceKind::Fixed) {
                fixed_light_source_id = id;
            }
        }
        let mut multi_channel_light_source_id = emitter.multi_channel_light_source_id.clone();
        if multi_channel_light_source_id.trim().is_empty()
            && matches!(draft.ls_kind, WizardLightSourceKind::MultiChannel)
        {
            if let Some(id) = last_ls_id(WizardLightSourceKind::MultiChannel) {
                multi_channel_light_source_id = id;
            }
        }
        state.apply(GldfAction::AddEmitterWithChild {
            child_name: child_name_opt,
            kind: emitter.kind,
            photometry_id,
            fixed_light_source_id,
            multi_channel_light_source_id,
            sensor_id: emitter.sensor_id.clone(),
        });
    }

    // Variant — always create at least one, since the XSD requires it.
    let name = draft.variant_name.trim();
    let name = if name.is_empty() { "Standard" } else { name };
    state.apply(GldfAction::AddVariantSimple {
        name: name.to_string(),
    });

    // Geometry — bind the freshly-created emitter into the freshly-
    // created variant. The XSD requires Variant.Geometry to reference
    // at least one Emitter, otherwise the file is incomplete.
    // - With L3D bytes → register as a File + ModelGeometry, then
    //   write a ModelGeometryReference with the emitter bound to
    //   "Primary" (a sane default external_name; the editor lets the
    //   user rename).
    // - Without bytes → write a bare EmitterReference.
    // Either way: skip if no emitter was created OR if no variant
    // ended up in the document (e.g. AddVariantSimple errored).
    let emitter_id = state
        .product
        .general_definitions
        .emitters
        .as_ref()
        .and_then(|es| es.emitter.last())
        .map(|e| e.id.clone());
    let variant_idx = state
        .product
        .product_definitions
        .variants
        .as_ref()
        .map(|vs| vs.variant.len().saturating_sub(1));
    // Step 1: register the L3D file unconditionally. Even if the user
    // skipped the Emitter step (so `emitter_id` is None below), the L3D
    // bytes must still land in `embedded_files` so the Bevy 3D viewer
    // (and the Files page / Geometries page) can find them. Gating the
    // file-embed on having an emitter to bind silently dropped the L3D
    // on Finish — the regression `wizard_with_geometry_but_no_emitter_…`
    // guards.
    let geometry_id = if let Some(bytes) = draft.geometry_bytes.clone() {
        let filename = if draft.geometry_filename.trim().is_empty() {
            "luminaire.l3d".to_string()
        } else {
            draft.geometry_filename.clone()
        };
        state.apply(GldfAction::ImportL3dFile { bytes, filename });
        state
            .product
            .general_definitions
            .geometries
            .as_ref()
            .and_then(|gs| gs.model_geometry.last())
            .map(|g| g.id.clone())
    } else {
        None
    };

    // Step 2: wire the variant's Geometry. Four cases:
    //   geometry + emitter → ModelGeometryReference with one binding
    //   geometry, no emitter → ModelGeometryReference with empty bindings
    //                          (the 3D viewer still resolves the L3D from
    //                          the geometry_id; user can wire emitters
    //                          later from the Variants page)
    //   no geometry, emitter → bare EmitterReference
    //   nothing → leave Variant.Geometry as None
    if let Some(variant_idx) = variant_idx {
        match (geometry_id, emitter_id.clone()) {
            (Some(geometry_id), Some(emitter_id)) => {
                let external_name = draft
                    .emitter_draft
                    .as_ref()
                    .map(|e| e.child_name.clone())
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| "Primary".to_string());
                state.apply(GldfAction::SetVariantGeometryModel {
                    variant_idx,
                    geometry_id,
                    bindings: vec![ModelEmitterBinding {
                        emitter_id,
                        external_name,
                        target_model_type: Some("l3d".to_string()),
                    }],
                });
            }
            (Some(geometry_id), None) => {
                state.apply(GldfAction::SetVariantGeometryModel {
                    variant_idx,
                    geometry_id,
                    bindings: Vec::new(),
                });
            }
            (None, Some(emitter_id)) => {
                state.apply(GldfAction::SetVariantGeometryEmitterRefs {
                    variant_idx,
                    emitter_ids: vec![emitter_id],
                });
            }
            (None, None) => {}
        }
    }

    // Mountings — optional. When the user picked a primitive, mint a
    // `Variant.Mountings` block using the same struct shape the variant
    // editor's `VariantMountingsEditor` builds. The choices are encoded
    // as `mounting_type` slash-strings (see WizardDraft docs) to keep
    // the parameter surface flat.
    if let Some(variant_idx) = variant_idx {
        if let Some(payload) = build_mountings_from_draft(draft) {
            state.apply(GldfAction::SetVariantMountings {
                index: variant_idx,
                payload: Some(Box::new(payload)),
            });
        }
    }

    // Driver — optional. When enabled, the wizard mints a ControlGear
    // (auto-id), customises it with the user's choices, then mints an
    // Equipment bundling the ControlGear with the wizard's LightSource,
    // and finally binds a ControlGearReference on the freshly-created
    // emitter's first FixedLightEmitter slot (if any).
    //
    // The whole block is gated on `driver_enabled` — skipping it is the
    // default. All three side-effects (ControlGear / Equipment /
    // ControlGearReference) are idempotent via id; the editor's sidebar
    // pages cover the same surface for users who want to add a driver
    // after the wizard finishes.
    if draft.driver_enabled {
        // Step 1: mint a ControlGear with a sensible default id. The
        // AddControlGear action auto-generates an id when `prefix` is
        // None; we then read it back off the last entry of the freshly-
        // populated list.
        state.apply(GldfAction::AddControlGear { prefix: None });
        let control_gear_id = state
            .product
            .general_definitions
            .control_gears
            .as_ref()
            .and_then(|cg| cg.control_gear.last())
            .map(|c| c.id.clone());

        // Customise the ControlGear with the user's choices. Standby
        // power passes through verbatim; an unparseable string lands as
        // `None` (the action models it as optional, the editor's UI
        // mirrors that).
        if let Some(id) = control_gear_id.as_ref() {
            let standby = draft.driver_standby_power.trim().parse::<f64>().ok();
            let name = draft.driver_name.trim();
            state.apply(GldfAction::UpdateControlGear {
                id: id.clone(),
                name: if name.is_empty() {
                    String::new()
                } else {
                    name.to_string()
                },
                nominal_voltage_value: None,
                standby_power: standby,
                dimmable: Some(draft.driver_dimmable),
                color_controllable: None,
                constant_light_output_start_power: None,
                constant_light_output_end_power: None,
            });
        }

        // Step 2: figure out which LightSource id the wizard just
        // created. We added a Fixed/Changeable/MultiChannel in the
        // LightSource step above; pull the matching id off the last
        // entry of the corresponding sub-array.
        let (light_source_id, light_source_kind) = match draft.ls_kind {
            WizardLightSourceKind::Fixed => (
                state
                    .product
                    .general_definitions
                    .light_sources
                    .as_ref()
                    .and_then(|ls| ls.fixed_light_source.last())
                    .map(|l| l.id.clone()),
                LightEmitterKind::Fixed,
            ),
            WizardLightSourceKind::Changeable => (
                state
                    .product
                    .general_definitions
                    .light_sources
                    .as_ref()
                    .and_then(|ls| ls.changeable_light_source.last())
                    .map(|l| l.id.clone()),
                LightEmitterKind::Changeable,
            ),
            WizardLightSourceKind::MultiChannel => (
                state
                    .product
                    .general_definitions
                    .light_sources
                    .as_ref()
                    .and_then(|ls| ls.multi_channel_light_source.last())
                    .map(|l| l.id.clone()),
                LightEmitterKind::MultiChannel,
            ),
        };

        // Step 3: mint an Equipment that bundles the LightSource +
        // ControlGear. Rated input power is left at the LS's own value
        // (the wizard's lib LS step set it); the Equipment widget
        // captures the assembly's idle rating, which equals the LS
        // rating for the wizard's single-light-source flow.
        let rated = draft.ls_rated_power.trim().parse::<f64>().unwrap_or(0.0);
        if light_source_id.is_some() && control_gear_id.is_some() {
            state.apply(GldfAction::AddEquipment { prefix: None });
            let equipment_id = state
                .product
                .general_definitions
                .equipments
                .as_ref()
                .and_then(|eq| eq.equipment.last())
                .map(|e| e.id.clone());
            if let Some(eq_id) = equipment_id {
                state.apply(GldfAction::UpdateEquipment {
                    id: eq_id,
                    light_source_id: light_source_id.clone(),
                    light_source_kind: Some(light_source_kind),
                    control_gear_id: control_gear_id.clone(),
                    rated_input_power: rated,
                    emergency_ballast_lumen_factor: None,
                    emergency_rated_luminous_flux: None,
                });
            }
        }

        // Step 4: bind a ControlGearReference on the wizard's emitter
        // (first FixedLightEmitter / ChangeableLightEmitter / etc.,
        // matching the LightSource kind). Skip if there's no emitter
        // (the user skipped that step) or no ControlGear (mint above
        // failed). Index 0 is the wizard's only emitter-child.
        if let (Some(eid), Some(cg_id)) = (emitter_id.as_ref(), control_gear_id.as_ref()) {
            state.apply(GldfAction::SetEmitterControlGearReference {
                emitter_id: eid.clone(),
                kind: light_source_kind,
                light_emitter_index: 0,
                control_gear_id: Some(cg_id.clone()),
            });
        }
    }
}

/// Translate the wizard's flat mounting fields into the typed `Mountings`
/// payload that `SetVariantMountings` expects. Returns `None` when the
/// user skipped the step (empty `mounting_type`) so the reducer can
/// no-op instead of writing an empty block.
///
/// The encoding mirrors what the WASM editor's `VariantMountingsEditor`
/// produces, so a value chosen in the wizard would land in the editor's
/// same widgets on subsequent open. The "ceiling/surface" /
/// "ceiling/recessed" / "ceiling/pendant" tri-state is encoded as a
/// slash-string in `mounting_type`; ground gets pole_top / free_standing
/// the same way. This keeps the wizard's draft parameter count low (one
/// string + four ints), which matters for the TUI's keyboard-driven
/// form and the CLI's JSON draft.
fn build_mountings_from_draft(
    draft: &WizardDraft,
) -> Option<gldf_rs::gldf::product_definitions::Mountings> {
    use gldf_rs::gldf::product_definitions::{
        Ceiling, FreeStanding, Ground, Mountings, Pendant, PoleTop, Recessed, SurfaceMounted, Wall,
        WorkingPlane,
    };

    let mt = draft.mounting_type.trim();
    if mt.is_empty() {
        return None;
    }

    let depth = draft.mounting_recessed_depth_mm.trim().parse::<i32>().ok();
    let pendant_len = draft.mounting_pendant_length_mm.trim().parse::<i32>().ok();
    let mount_h = draft.mounting_height_mm.trim().parse::<i32>().ok();
    let pole_h = draft.mounting_pole_height_mm.trim().parse::<i32>().ok();

    let mut m = Mountings::default();
    match mt {
        "ceiling/recessed" => {
            m.ceiling = Some(Ceiling {
                recessed: Some(Recessed {
                    recessed_depth: depth.unwrap_or(0),
                    ..Default::default()
                }),
                surface_mounted: None,
                pendant: None,
            });
        }
        "ceiling/surface" => {
            m.ceiling = Some(Ceiling {
                recessed: None,
                surface_mounted: Some(SurfaceMounted {}),
                pendant: None,
            });
        }
        "ceiling/pendant" => {
            m.ceiling = Some(Ceiling {
                recessed: None,
                surface_mounted: None,
                pendant: Some(Pendant {
                    pendant_length: pendant_len.unwrap_or(0),
                }),
            });
        }
        "wall" => {
            let mut w = Wall {
                mounting_height: mount_h.unwrap_or(2000),
                ..Default::default()
            };
            if let Some(d) = depth {
                w.recessed = Some(Recessed {
                    recessed_depth: d,
                    ..Default::default()
                });
            }
            m.wall = Some(w);
        }
        "ground/pole_top" => {
            let pt = PoleTop {
                pole_height: Some(pole_h.unwrap_or(4000)),
                pole_height_element: None,
            };
            m.ground = Some(Ground {
                pole_top: Some(pt),
                pole_integrated: None,
                free_standing: None,
                surface_mounted: None,
                recessed: None,
            });
        }
        "ground/free_standing" => {
            m.ground = Some(Ground {
                pole_top: None,
                pole_integrated: None,
                free_standing: Some(FreeStanding {}),
                surface_mounted: None,
                recessed: None,
            });
        }
        "working_plane" => {
            m.working_plane = Some(WorkingPlane {
                free_standing: Some(FreeStanding {}),
            });
        }
        _ => return None,
    }
    Some(m)
}
