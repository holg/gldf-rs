//! GLDF editor state — thin Leptos wrapper over [`gldf_rs_editor`].
//!
//! The pure state machine and actions live in the shared `gldf-rs-editor`
//! crate (so the TUI, CLI, and Python frontends consume the same API).
//! This module re-exports them under `crate::state::*` for unchanged
//! downstream imports, and adds the WASM-specific bits:
//!
//! - [`GldfContext`] — a Leptos `RwSignal<GldfState>` wrapper.
//! - [`provide_gldf_state`] / [`use_gldf`] — Leptos context plumbing.
//! - [`BrowserLangStore`] — a [`UiLanguageStore`] implementation backed
//!   by `window.localStorage`. Wired into [`provide_gldf_state`] so the
//!   user's last-chosen UI language persists across sessions.

pub use gldf_rs_editor::{
    collect_languages, normalise_ui_language, ui_language_native_name, GldfAction, GldfState,
    LangStore, LightEmitterKind, NullLangStore, Scope, UiLanguageStore, SUPPORTED_UI_LANGUAGES,
};

use leptos::prelude::*;
use std::sync::Arc;

/// Leptos context: a reactive signal wrapping the shared [`GldfState`].
/// Children read with `.get()` / `.with(...)` and dispatch with
/// `.update(|s| s.apply(action))`.
pub type GldfContext = RwSignal<GldfState>;

/// Install a fresh `GldfState` signal as Leptos context. Seeds the
/// state's `lang_store` with [`BrowserLangStore`] so `SetUiLanguage`
/// persists to `localStorage` automatically.
pub fn provide_gldf_state() -> GldfContext {
    let store: LangStore = Arc::new(BrowserLangStore);
    let state = GldfState::with_lang_store(store);
    let signal = RwSignal::new(state);
    provide_context(signal);
    signal
}

/// Look up the editor state signal from the current Leptos context.
pub fn use_gldf() -> GldfContext {
    expect_context::<GldfContext>()
}

/// `localStorage` key used by [`BrowserLangStore`]. Distinct from
/// `gldf.display_language` (which the LanguageBanner uses for the
/// document's own locales).
const UI_LANGUAGE_STORAGE_KEY: &str = "gldf.ui_language";

/// [`UiLanguageStore`] adapter writing to the browser's `localStorage`.
/// Persistence is best-effort — if `window` or `localStorage` is
/// unavailable (e.g. running headless), `load` returns `None` and
/// `store` silently drops the value.
pub struct BrowserLangStore;

impl UiLanguageStore for BrowserLangStore {
    fn load(&self) -> Option<String> {
        web_sys::window()?
            .local_storage()
            .ok()
            .flatten()?
            .get_item(UI_LANGUAGE_STORAGE_KEY)
            .ok()
            .flatten()
    }
    fn store(&self, lang: &str) {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(UI_LANGUAGE_STORAGE_KEY, lang);
            }
        }
    }
}
