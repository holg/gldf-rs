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
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
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
}

/// Steps the wizard walks through. Used to drive both the Leptos step
/// switcher and the TUI step indicator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WizardStep {
    Header,
    Photometry,
    LightSource,
    Emitter,
    Geometry,
    Driver,
    Variant,
    Review,
}

impl WizardStep {
    pub const ALL: &'static [Self] = &[
        Self::Header,
        Self::Photometry,
        Self::LightSource,
        Self::Emitter,
        Self::Geometry,
        Self::Driver,
        Self::Variant,
        Self::Review,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Header => "Header",
            Self::Photometry => "Photometry",
            Self::LightSource => "Light source",
            Self::Emitter => "Emitter",
            Self::Geometry => "Geometry",
            Self::Driver => "Driver",
            Self::Variant => "Variant",
            Self::Review => "Review & Finish",
        }
    }

    /// One-based step number for UI display (`"1"`, `"2"`, …). Together
    /// with [`label`] this is what every frontend uses as the step
    /// heading. Lives in the lib so WASM, TUI, and CLI render the
    /// same numbering.
    pub fn number(&self) -> u8 {
        match self {
            Self::Header => 1,
            Self::Photometry => 2,
            Self::LightSource => 3,
            Self::Emitter => 4,
            Self::Geometry => 5,
            Self::Driver => 6,
            Self::Variant => 7,
            Self::Review => 8,
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
            Self::LightSource => {
                "The lamp itself. Fixed = baked into the fixture. Changeable = user-replaceable bulb. MultiChannel = tunable-white / RGB."
            }
            Self::Emitter => {
                "An Emitter is a slot on the luminaire that binds the photometry (beam shape) to the light source (lamp). The form below uses the same XSD-conformant type picker as the editor."
            }
            Self::Geometry => {
                "The 3D shell of the variant. Drop an .l3d file to bind the emitter into a 3D model, or skip — the wizard then writes a bare EmitterReference. Either way the GLDF will have a valid Variant.Geometry on Finish."
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
        state.apply(GldfAction::SetCreatedWithApplication(created_with.to_string()));
    }
    let default_lang = draft.header_default_language.trim();
    if !default_lang.is_empty() {
        state.apply(GldfAction::SetDefaultLanguage(Some(default_lang.to_string())));
    }

    // Photometry
    if let Some(bytes) = &draft.photometry_bytes {
        state.apply(GldfAction::AddPhotometryFile {
            bytes: bytes.clone(),
            filename: draft.photometry_filename.clone(),
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
            let mut ls = FixedLightSource::default();
            ls.name = name_locale;
            ls.manufacturer = manufacturer_opt;
            ls.rated_input_power = power;
            state.apply(GldfAction::AddFixedLightSource {
                payload: Box::new(ls),
            });
        }
        WizardLightSourceKind::Changeable => {
            let mut ls = ChangeableLightSource::default();
            ls.name = name_locale;
            ls.manufacturer = manufacturer_opt;
            state.apply(GldfAction::AddChangeableLightSource {
                payload: Box::new(ls),
            });
        }
        WizardLightSourceKind::MultiChannel => {
            let mut ls = MultiChannelLightSource::default();
            ls.name = name_locale;
            ls.manufacturer = manufacturer_opt;
            ls.rated_input_power = power.unwrap_or(0.0);
            state.apply(GldfAction::AddMultiChannelLightSource {
                payload: Box::new(ls),
            });
        }
    }

    // Emitter (atomic with first LightEmitter child)
    if let Some(emitter) = &draft.emitter_draft {
        let child_name_opt = if emitter.child_name.trim().is_empty() {
            None
        } else {
            Some(emitter.child_name.clone())
        };
        state.apply(GldfAction::AddEmitterWithChild {
            child_name: child_name_opt,
            kind: emitter.kind,
            photometry_id: emitter.photometry_id.clone(),
            fixed_light_source_id: emitter.fixed_light_source_id.clone(),
            multi_channel_light_source_id: emitter.multi_channel_light_source_id.clone(),
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
        if let (Some(eid), Some(cg_id)) =
            (emitter_id.as_ref(), control_gear_id.as_ref())
        {
            state.apply(GldfAction::SetEmitterControlGearReference {
                emitter_id: eid.clone(),
                kind: light_source_kind,
                light_emitter_index: 0,
                control_gear_id: Some(cg_id.clone()),
            });
        }
    }
}
