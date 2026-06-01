//! Reducer tests — pure state-machine tests, no UI.
//!
//! Each test instantiates a `GldfState`, dispatches a sequence of
//! `GldfAction`s, and asserts the resulting `GldfProduct` shape. This
//! is the focused integration-test surface for the editor's 146-action
//! reducer.

use gldf_rs_editor::{GldfAction, GldfState};

#[test]
fn set_author_marks_modified() {
    let mut state = GldfState::default();
    assert!(!state.is_modified);
    state.apply(GldfAction::SetAuthor("Acme".to_string()));
    assert_eq!(state.product.header.author, "Acme");
    assert!(state.is_modified);
    assert!(state.last_error.is_none());
}

#[test]
fn set_manufacturer_then_author_keeps_both() {
    let mut state = GldfState::default();
    state.apply(GldfAction::SetManufacturer("Acme".to_string()));
    state.apply(GldfAction::SetAuthor("Engineer".to_string()));
    assert_eq!(state.product.header.manufacturer, "Acme");
    assert_eq!(state.product.header.author, "Engineer");
}

#[test]
fn add_fixed_light_source_grows_catalog() {
    use gldf_rs::gldf::general_definitions::lightsources::FixedLightSource;
    let mut state = GldfState::default();
    state.apply(GldfAction::AddFixedLightSource {
        payload: Box::new(FixedLightSource::default()),
    });
    let ls = state
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .expect("light_sources created");
    assert_eq!(ls.fixed_light_source.len(), 1);
    // Auto-generated id from `generate_unique_id("lightsource")`.
    assert!(ls.fixed_light_source[0].id.starts_with("lightsource_"));
}

#[test]
fn ui_language_persists_via_lang_store() {
    use gldf_rs_editor::{LangStore, UiLanguageStore};
    use std::sync::{Arc, Mutex};

    // Custom in-memory lang store that records what apply() wrote.
    struct RecordingStore {
        last: Arc<Mutex<Option<String>>>,
    }
    impl UiLanguageStore for RecordingStore {
        fn load(&self) -> Option<String> {
            self.last.lock().unwrap().clone()
        }
        fn store(&self, lang: &str) {
            *self.last.lock().unwrap() = Some(lang.to_string());
        }
    }

    let last = Arc::new(Mutex::new(None));
    let store: LangStore = Arc::new(RecordingStore {
        last: last.clone(),
    });
    let mut state = GldfState::with_lang_store(store);
    state.apply(GldfAction::SetUiLanguage("de".to_string()));
    assert_eq!(state.ui_language, "de");
    assert_eq!(last.lock().unwrap().as_deref(), Some("de"));
}
