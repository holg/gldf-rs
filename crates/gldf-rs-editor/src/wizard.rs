//! Portable wizard logic shared between the Leptos forms (WASM
//! frontend) and the ratatui modal (TUI frontend). The wizard's
//! per-step UI lives in each frontend; this module owns the
//! frontend-agnostic [`WizardDraft`] data shape and the [`apply_wizard`]
//! reducer that dispatches actions in XSD-conformant order.

use crate::action::GldfAction;
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
}

/// Steps the wizard walks through. Used to drive both the Leptos step
/// switcher and the TUI step indicator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WizardStep {
    Header,
    Photometry,
    LightSource,
    Emitter,
    Review,
}

impl WizardStep {
    pub const ALL: &'static [Self] = &[
        Self::Header,
        Self::Photometry,
        Self::LightSource,
        Self::Emitter,
        Self::Review,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Header => "Header",
            Self::Photometry => "Photometry",
            Self::LightSource => "Light source",
            Self::Emitter => "Emitter",
            Self::Review => "Review & Finish",
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
}
