//! GLDF editor state.
//!
//! Single submodule for now (`gldf_state`); re-exported at the module root so
//! callers can write `use crate::state::{GldfState, GldfAction, use_gldf};`
//! without spelling out the submodule.

pub mod gldf_state;

pub use gldf_state::{
    collect_languages, normalise_ui_language, provide_gldf_state, ui_language_native_name,
    use_gldf, GldfAction, GldfContext, GldfState, Scope, SUPPORTED_UI_LANGUAGES,
};
