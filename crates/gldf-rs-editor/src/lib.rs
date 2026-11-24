//! Frontend-agnostic state machine for the gldf-rs editor.
//!
//! This crate owns the [`GldfAction`] enum, the [`GldfState`] struct, and
//! the `apply` reducer. Both frontends (Leptos/WASM and ratatui/TUI)
//! drive the same reducer; the difference is only the view layer and a
//! couple of pluggable side-effects (UI language persistence, file I/O
//! delivery).
//!
//! Architecture:
//! - [`GldfAction`] — variants are pure data (`String`, `Vec<u8>`, etc.).
//! - [`GldfState`] — plain Rust struct holding the loaded product, its
//!   embedded files, edit metadata, and pluggable adapters.
//! - [`GldfState::apply`] — pure-function reducer over `&mut self`.
//! - [`UiLanguageStore`] — the only non-pure dependency, abstracted as a
//!   trait so the WASM frontend writes to `localStorage` and the TUI
//!   writes to `~/.config/gldf-edit/lang`.
//! - [`wizard::WizardDraft`] / [`wizard::apply_wizard`] — portable wizard
//!   logic shared between Leptos forms and ratatui modals.

pub mod action;
pub mod batch;
pub mod emitter_draft;
pub mod script;
pub mod state;
pub mod ui_language;
#[cfg(feature = "xsd")]
pub mod validation;
pub mod wizard;

pub use action::{GldfAction, LightEmitterKind, ModelEmitterBinding, Scope};
#[cfg(not(target_arch = "wasm32"))]
pub use batch::{collect_inputs, run_batch, BatchEvent, BatchJob, FileReport};
pub use batch::{process_one, BatchOptions, BatchOutput, BatchTemplate, TemplateDraft};
pub use batch::{register_container_converter, ContainerConverter, ConvertedContainer};
pub use emitter_draft::EmitterDraft;
#[cfg(not(target_arch = "wasm32"))]
pub use script::FsResolver;
pub use script::{apply_script, expand_file_refs, FileResolver, MemoryResolver};
pub use state::{collect_languages, GldfState};
pub use ui_language::{
    initial_ui_language, normalise_ui_language, ui_language_native_name, LangStore, NullLangStore,
    UiLanguageStore, SUPPORTED_UI_LANGUAGES,
};
#[cfg(feature = "xsd")]
pub use validation::{validation_report, ValidationReport, ValidationRow};
pub use wizard::{
    apply_wizard, text as wizard_text, WizardDraft, WizardLightSourceKind, WizardStep,
};
