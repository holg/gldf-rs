//! Filesystem-backed [`UiLanguageStore`] for the TUI.
//!
//! Persists the user's UI language pick under
//! `$XDG_CONFIG_HOME/gldf-edit/lang` (typically
//! `~/.config/gldf-edit/lang`). Falls back to the home directory if
//! `dirs::config_dir` is unavailable. Read/write errors are silent —
//! the app degrades to "no persistence" rather than crashing.

use gldf_rs_editor::UiLanguageStore;
use std::path::PathBuf;

const LANG_FILE_NAME: &str = "lang";

pub struct FsLangStore {
    path: PathBuf,
}

impl Default for FsLangStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FsLangStore {
    pub fn new() -> Self {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")));
        let path = dir.join("gldf-edit").join(LANG_FILE_NAME);
        Self { path }
    }
}

impl UiLanguageStore for FsLangStore {
    fn load(&self) -> Option<String> {
        std::fs::read_to_string(&self.path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }
    fn store(&self, lang: &str) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&self.path, lang);
    }
}
