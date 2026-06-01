//! `EmitterDraft` — plain data describing one LightEmitter the user
//! wants to add to an Emitter slot. Lifted out of
//! `gldf-rs-wasm/src/components/emitter_type_form.rs` so both the
//! Leptos wizard and the ratatui wizard can produce one.

use crate::action::LightEmitterKind;

/// Captured form values from the type-first picker. The caller dispatches
/// the right `GldfAction` based on `kind`; the other fields are populated
/// only for the relevant kind.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EmitterDraft {
    /// Display name for the LightEmitter child (`<Name>` locale, not the
    /// parent `<Emitter>` id which is auto-generated).
    pub child_name: String,
    /// Which LightEmitter type the user picked.
    pub kind: LightEmitterKind,
    /// PhotometryReference id (Fixed + Changeable).
    pub photometry_id: String,
    /// FixedLightSourceReference id (Fixed).
    pub fixed_light_source_id: String,
    /// MultiChannelLightSourceReference id (MultiChannel).
    pub multi_channel_light_source_id: String,
    /// SensorReference id (Sensor).
    pub sensor_id: String,
}

impl Default for EmitterDraft {
    fn default() -> Self {
        Self {
            child_name: String::new(),
            kind: LightEmitterKind::Fixed,
            photometry_id: String::new(),
            fixed_light_source_id: String::new(),
            multi_channel_light_source_id: String::new(),
            sensor_id: String::new(),
        }
    }
}
