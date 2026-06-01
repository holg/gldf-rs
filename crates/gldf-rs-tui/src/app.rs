//! TUI application: owns the editor state, the navigation cursor, and
//! the crossterm/ratatui terminal session.

use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use gldf_rs::EditableGldf;
use gldf_rs_editor::{GldfAction, GldfState, LangStore};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::fs_lang_store::FsLangStore;
use crate::ui;

/// Editor sections that appear in the left-nav sidebar. Mirrors the
/// WASM left-nav in `crates/gldf-rs-wasm/src/components/pages/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Overview,
    Header,
    Photometry,
    Spectra,
    LightSources,
    Emitters,
    Variants,
    Files,
    Linkage,
}

impl Section {
    pub const ALL: &'static [Self] = &[
        Self::Overview,
        Self::Header,
        Self::Photometry,
        Self::Spectra,
        Self::LightSources,
        Self::Emitters,
        Self::Variants,
        Self::Files,
        Self::Linkage,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Header => "Header",
            Self::Photometry => "Photometry",
            Self::Spectra => "Spectra",
            Self::LightSources => "Light Sources",
            Self::Emitters => "Emitters",
            Self::Variants => "Variants",
            Self::Files => "Files",
            Self::Linkage => "Linkage Map",
        }
    }
}

/// Top-level application state. Held by the main loop; the renderer
/// (`ui::draw`) reads it immutably; the event handler mutates it.
pub struct App {
    pub state: GldfState,
    /// Currently selected section in the sidebar (index into `Section::ALL`).
    pub selected: usize,
    /// Path the GLDF was loaded from (`None` if started blank).
    pub source_path: Option<PathBuf>,
    /// `false` until the user presses `q` to quit.
    pub running: bool,
    /// Last status line message (info, warning, error). Cleared on the
    /// next event tick.
    pub status: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let lang_store: LangStore = Arc::new(FsLangStore::new());
        Self {
            state: GldfState::with_lang_store(lang_store),
            selected: 0,
            source_path: None,
            running: true,
            status: None,
        }
    }

    /// Load a .gldf file into the editor state. Bytes go through
    /// `EditableGldf::from_gldf` which is the same path the WASM editor
    /// uses — every frontend goes through the same loader.
    pub fn load_file(&mut self, path: &Path) -> Result<()> {
        let editable = EditableGldf::from_gldf(
            path.to_str().context("non-UTF-8 path")?,
        )
        .with_context(|| format!("parse {}", path.display()))?;
        self.state.apply(GldfAction::LoadWithFiles {
            product: editable.product,
            embedded_files: editable.embedded_files,
        });
        self.source_path = Some(path.to_path_buf());
        self.status = Some(format!("Loaded {}", path.display()));
        Ok(())
    }

    /// Save the current state to disk via `EditableGldf::save_to_buf`.
    /// Used by the Ctrl-S binding once an edit slice exists.
    pub fn save(&mut self, path: &Path) -> Result<()> {
        let bytes = self.state.save_to_buf()?;
        std::fs::write(path, bytes).with_context(|| format!("write {}", path.display()))?;
        self.status = Some(format!("Saved {}", path.display()));
        Ok(())
    }

    /// Convenience: cycle the sidebar selection up/down by `delta`,
    /// wrapping around the section list.
    pub fn move_selected(&mut self, delta: isize) {
        let n = Section::ALL.len() as isize;
        let next = (self.selected as isize + delta).rem_euclid(n);
        self.selected = next as usize;
    }

    pub fn current_section(&self) -> Section {
        Section::ALL[self.selected.min(Section::ALL.len() - 1)]
    }

    /// Run the TUI event loop until the user quits.
    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode().context("enable raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).context("enter alt screen")?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).context("create terminal")?;

        let result = self.run_loop(&mut terminal);

        // Always restore the terminal, even on error.
        disable_raw_mode().ok();
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .ok();
        terminal.show_cursor().ok();

        result
    }

    fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        while self.running {
            terminal.draw(|frame| ui::draw(frame, self)).context("draw")?;
            if event::poll(Duration::from_millis(250)).context("poll")? {
                match event::read().context("read")? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key(key.code),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            KeyCode::Down | KeyCode::Char('j') => self.move_selected(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selected(-1),
            _ => {}
        }
    }
}
