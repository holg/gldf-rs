//! TUI application: owns the editor state, the navigation cursor, and
//! the crossterm/ratatui terminal session.

use anyhow::{Context, Result};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use gldf_rs::validation::{GldfSchemaVersion, DEFAULT_VALIDATION_SCHEMA};
use gldf_rs::EditableGldf;
use gldf_rs_editor::{validation_report, ValidationReport};
use gldf_rs_editor::{GldfAction, GldfState, LangStore};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::edit_modal::{ActionMenu, EditOutcome, FieldEdit, MenuOutcome};
use crate::fs_lang_store::FsLangStore;
use crate::l3d_view::OrbitAngles;
use crate::ui;
use crate::wizard::{self, WizardOutcome, WizardOverlay};

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
    Validation,
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
        Self::Validation,
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
            Self::Validation => "Validation",
        }
    }
}

/// What modal is on top right now (or `None` for the regular
/// sidebar/detail view).
// Transient UI value, one at a time; boxing would change the public enum.
#[allow(clippy::large_enum_variant)]
pub enum Modal {
    Wizard(WizardOverlay),
    SavePrompt {
        buffer: String,
        /// If `true`, quit after a successful save (Save-and-quit flow).
        quit_after: bool,
    },
    /// "Unsaved changes — Save / Discard / Cancel?" confirmation.
    QuitConfirm {
        selected: usize,
    },
    /// Per-section "what do you want to edit?" picker.
    Menu(ActionMenu),
    /// Live text editor for a single field. Dispatches a `GldfAction`
    /// on commit, closes on Esc.
    Field(FieldEdit),
}

/// Top-level application state. Held by the main loop; the renderer
/// (`ui::draw`) reads it immutably; the event handler mutates it.
pub struct App {
    pub state: GldfState,
    /// `true` until the user has done anything (loaded a file or
    /// finished the wizard). Drives the welcome banner.
    pub has_content: bool,
    /// Currently selected section in the sidebar (index into `Section::ALL`).
    pub selected: usize,
    /// Path the GLDF was loaded from (`None` if started blank).
    pub source_path: Option<PathBuf>,
    /// Active modal overlay, if any.
    pub modal: Option<Modal>,
    /// Row cursor inside the currently-selected section. Indexed into
    /// the section's items (light sources, emitters, variants, files).
    /// `0` for sections without rows.
    pub selected_row: usize,
    /// Orbit camera for the L3D wireframe view. Mutated via arrow keys
    /// while on the Files / Geometry section.
    pub orbit: OrbitAngles,
    /// `false` until the user presses `q` to quit.
    pub running: bool,
    /// Last status line message (info, warning, error). Cleared on the
    /// next event tick.
    pub status: Option<String>,
    /// Schema revision the Validation section checks against; `t`
    /// toggles rc-3 ↔ rc-4 while that section is active.
    pub validation_schema: GldfSchemaVersion,
    /// Report for the current state, refreshed after every key event
    /// while the Validation section is showing (`None` until then).
    pub validation: Option<ValidationReport>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let lang_store: LangStore = Arc::new(FsLangStore::new());
        Self {
            state: GldfState::with_lang_store(lang_store),
            has_content: false,
            selected: 0,
            source_path: None,
            modal: None,
            selected_row: 0,
            orbit: OrbitAngles::default(),
            running: true,
            status: None,
            validation_schema: DEFAULT_VALIDATION_SCHEMA,
            validation: None,
        }
    }

    /// Row count in the current section's item list. Used to clamp
    /// `selected_row` after section changes / item deletes.
    pub fn current_section_row_count(&self) -> usize {
        let p = &self.state.product;
        match self.current_section() {
            Section::LightSources => p
                .general_definitions
                .light_sources
                .as_ref()
                .map(|l| l.fixed_light_source.len())
                .unwrap_or(0),
            Section::Emitters => p
                .general_definitions
                .emitters
                .as_ref()
                .map(|e| e.emitter.len())
                .unwrap_or(0),
            Section::Variants => p
                .product_definitions
                .variants
                .as_ref()
                .map(|v| v.variant.len())
                .unwrap_or(0),
            Section::Files => p.general_definitions.files.file.len(),
            _ => 0,
        }
    }

    /// Reset `selected_row` to 0 whenever the section changes, so we
    /// don't carry a stale index that points past a smaller list.
    pub fn clamp_selected_row(&mut self) {
        let n = self.current_section_row_count();
        if n == 0 {
            self.selected_row = 0;
        } else if self.selected_row >= n {
            self.selected_row = n - 1;
        }
    }

    /// Load a .gldf file into the editor state. Bytes go through
    /// `EditableGldf::from_gldf` which is the same path the WASM editor
    /// uses — every frontend goes through the same loader.
    pub fn load_file(&mut self, path: &Path) -> Result<()> {
        let editable = EditableGldf::from_gldf(path.to_str().context("non-UTF-8 path")?)
            .with_context(|| format!("parse {}", path.display()))?;
        self.state.apply(GldfAction::LoadWithFiles {
            product: editable.product,
            embedded_files: editable.embedded_files,
        });
        self.source_path = Some(path.to_path_buf());
        self.has_content = true;
        self.status = Some(format!("Loaded {}", path.display()));
        Ok(())
    }

    /// Save the current state to disk via `EditableGldf::save_to_buf`.
    /// Used by the Ctrl-S binding.
    pub fn save(&mut self, path: &Path) -> Result<()> {
        let bytes = self.state.save_to_buf()?;
        std::fs::write(path, bytes).with_context(|| format!("write {}", path.display()))?;
        self.status = Some(format!("Saved {}", path.display()));
        Ok(())
    }

    /// Open the wizard overlay. Subsequent keys go to the overlay
    /// until it Finishes (populating `state`) or Cancels.
    pub fn open_wizard(&mut self) {
        self.modal = Some(Modal::Wizard(WizardOverlay::new()));
        self.status = Some("Wizard: ↑/↓ field · Tab next step · Esc cancel".to_string());
    }

    /// Open the save-path prompt. The buffer pre-fills with the source
    /// path (if any) or a sensible default. When `quit_after` is true,
    /// a successful save also exits the app (Save-and-quit flow).
    pub fn open_save_prompt(&mut self, quit_after: bool) {
        let buffer = self
            .source_path
            .as_ref()
            .and_then(|p| p.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "out.gldf".to_string());
        self.modal = Some(Modal::SavePrompt { buffer, quit_after });
    }

    /// Open the per-section action menu for the highlighted row.
    pub fn open_action_menu(&mut self) {
        let section = self.current_section();
        let row = self.selected_row;
        match ActionMenu::new(section, row, &self.state) {
            Some(menu) => self.modal = Some(Modal::Menu(menu)),
            None => {
                self.status = Some(format!("Nothing editable in {} yet", section.label()));
            }
        }
    }

    /// Handle a quit request from the main screen. If there are
    /// unsaved changes, open a confirm dialog instead of exiting
    /// straight away.
    pub fn request_quit(&mut self) {
        if self.has_content && self.state.is_modified {
            self.modal = Some(Modal::QuitConfirm { selected: 0 });
        } else {
            self.running = false;
        }
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

    /// Find the first embedded LDT/IES photometry file's bytes. Used by
    /// the Photometry detail pane's polar diagram.
    pub fn first_photometry_bytes(&self) -> Option<&[u8]> {
        let files = &self.state.product.general_definitions.files.file;
        for f in files {
            let lower = f.file_name.to_lowercase();
            if lower.ends_with(".ldt") || lower.ends_with(".ies") {
                if let Some(bytes) = self.state.embedded_files.get(&f.id) {
                    return Some(bytes.as_slice());
                }
            }
        }
        None
    }

    /// Find the first embedded L3D geometry file's bytes. Used by the
    /// Files detail pane's iso-3D wireframe.
    pub fn first_l3d_bytes(&self) -> Option<&[u8]> {
        let files = &self.state.product.general_definitions.files.file;
        for f in files {
            if f.file_name.to_lowercase().ends_with(".l3d") {
                if let Some(bytes) = self.state.embedded_files.get(&f.id) {
                    return Some(bytes.as_slice());
                }
            }
        }
        None
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

    fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        while self.running {
            terminal
                .draw(|frame| ui::draw(frame, self))
                .context("draw")?;
            if event::poll(Duration::from_millis(250)).context("poll")? {
                match event::read().context("read")? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.on_key(key.code, key.modifiers);
                        self.refresh_validation();
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// Recompute the validation report when the Validation section is
    /// showing. Runs after every key event: the XSD pass is a few ms,
    /// cheap next to a keypress, and keeps the view in sync with edits
    /// made through the modals without a separate dirty flag.
    fn refresh_validation(&mut self) {
        if !self.has_content || !matches!(self.current_section(), Section::Validation) {
            return;
        }
        let stale = self
            .validation
            .as_ref()
            .map(|r| r.schema != self.validation_schema)
            .unwrap_or(true)
            || self.modal.is_none();
        if stale {
            self.validation = Some(validation_report(
                &self.state.product,
                &self.state.embedded_files,
                self.validation_schema,
            ));
        }
    }

    fn on_key(&mut self, key: KeyCode, mods: KeyModifiers) {
        // Modals (Wizard / SavePrompt / QuitConfirm) absorb all keys.
        if self.modal.is_some() {
            self.handle_modal_key(key, mods);
            return;
        }

        // Global shortcuts available from any non-modal screen.
        match (key, mods) {
            (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
                self.request_quit();
                return;
            }
            (KeyCode::Char('n'), KeyModifiers::NONE) | (KeyCode::Char('N'), _) => {
                self.open_wizard();
                return;
            }
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                if self.has_content {
                    self.open_save_prompt(false);
                } else {
                    self.status =
                        Some("Nothing to save yet — press N to run the wizard.".to_string());
                }
                return;
            }
            _ => {}
        }

        if !self.has_content {
            // Welcome screen: only N / Ctrl-S / q are meaningful.
            return;
        }

        // `t` flips the XSD schema revision on the Validation section.
        if matches!(key, KeyCode::Char('t') | KeyCode::Char('T'))
            && matches!(self.current_section(), Section::Validation)
        {
            self.validation_schema = match self.validation_schema {
                GldfSchemaVersion::Rc3 => GldfSchemaVersion::Rc4,
                GldfSchemaVersion::Rc4 => GldfSchemaVersion::Rc3,
            };
            self.status = Some(format!(
                "Validating against {}",
                ValidationReport::schema_label(self.validation_schema)
            ));
            return;
        }

        // Shift-arrow orbits the L3D camera when the geometry view is
        // active. Plain arrows stay as sidebar navigation.
        if mods.contains(KeyModifiers::SHIFT)
            && matches!(self.current_section(), Section::Files | Section::Linkage)
        {
            match key {
                KeyCode::Left => {
                    self.orbit.rotate_left();
                    return;
                }
                KeyCode::Right => {
                    self.orbit.rotate_right();
                    return;
                }
                KeyCode::Up => {
                    self.orbit.rotate_up();
                    return;
                }
                KeyCode::Down => {
                    self.orbit.rotate_down();
                    return;
                }
                _ => {}
            }
        }

        match key {
            // Sidebar (section) navigation.
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selected(1);
                self.clamp_selected_row();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selected(-1);
                self.clamp_selected_row();
            }
            // Per-section row navigation.
            KeyCode::Char('n') => self.move_row(1),
            KeyCode::Char('p') => self.move_row(-1),
            // Open the edit menu for the highlighted row (or the
            // section's only-thing-editable for Header).
            KeyCode::Char('e') | KeyCode::Enter => self.open_action_menu(),
            // Delete the highlighted row.
            KeyCode::Char('d') => self.delete_selected_row(),
            _ => {}
        }
    }

    fn move_row(&mut self, delta: isize) {
        let n = self.current_section_row_count() as isize;
        if n == 0 {
            return;
        }
        let next = (self.selected_row as isize + delta).rem_euclid(n);
        self.selected_row = next as usize;
    }

    /// Dispatch a Remove action appropriate for the highlighted row.
    fn delete_selected_row(&mut self) {
        let row = self.selected_row;
        let p = &self.state.product;
        let action = match self.current_section() {
            Section::LightSources => p
                .general_definitions
                .light_sources
                .as_ref()
                .and_then(|l| l.fixed_light_source.get(row))
                .map(|ls| gldf_rs_editor::GldfAction::RemoveLightSource { id: ls.id.clone() }),
            Section::Emitters => p
                .general_definitions
                .emitters
                .as_ref()
                .and_then(|e| e.emitter.get(row))
                .map(|em| gldf_rs_editor::GldfAction::RemoveEmitter { id: em.id.clone() }),
            Section::Variants => p
                .product_definitions
                .variants
                .as_ref()
                .and_then(|v| v.variant.get(row))
                .map(|v| gldf_rs_editor::GldfAction::RemoveVariant { id: v.id.clone() }),
            Section::Files => p
                .general_definitions
                .files
                .file
                .get(row)
                .map(|f| gldf_rs_editor::GldfAction::RemoveFile(f.id.clone())),
            _ => None,
        };
        let Some(action) = action else {
            self.status = Some(format!(
                "Nothing to delete on {}",
                self.current_section().label()
            ));
            return;
        };
        self.state.apply(action);
        if let Some(err) = self.state.last_error.as_deref() {
            self.status = Some(format!("delete failed: {}", err));
        } else {
            self.status = Some("Deleted row".to_string());
            self.clamp_selected_row();
        }
    }

    fn handle_modal_key(&mut self, key: KeyCode, mods: KeyModifiers) {
        // Take ownership of the modal so we can replace it after
        // dispatch without fighting the borrow checker.
        let modal = self.modal.take().unwrap();
        match modal {
            Modal::Wizard(mut overlay) => {
                match overlay.on_key(crossterm::event::KeyEvent::new(key, mods)) {
                    WizardOutcome::Continue => {
                        self.modal = Some(Modal::Wizard(overlay));
                    }
                    WizardOutcome::Cancelled => {
                        self.status = Some("Wizard cancelled".to_string());
                    }
                    WizardOutcome::Finished(draft) => match wizard::finish(draft) {
                        Ok(state) => {
                            self.state = state;
                            self.has_content = true;
                            self.selected = 0;
                            self.status =
                                Some("Wizard finished. Ctrl-S to save, q to quit.".to_string());
                        }
                        Err(err) => {
                            overlay.message = Some(format!("apply_wizard failed: {}", err));
                            self.modal = Some(Modal::Wizard(overlay));
                        }
                    },
                }
            }
            Modal::SavePrompt {
                mut buffer,
                quit_after,
            } => match key {
                KeyCode::Esc => {
                    self.status = Some("Save cancelled".to_string());
                }
                KeyCode::Enter => {
                    let path = std::path::PathBuf::from(buffer.trim());
                    match self.save(&path) {
                        Ok(()) => {
                            self.source_path = Some(path);
                            if quit_after {
                                self.running = false;
                            }
                        }
                        Err(e) => {
                            self.status = Some(format!("save failed: {:#}", e));
                            // Keep the modal open so the user can fix
                            // the path or cancel.
                            self.modal = Some(Modal::SavePrompt { buffer, quit_after });
                        }
                    }
                }
                KeyCode::Backspace => {
                    buffer.pop();
                    self.modal = Some(Modal::SavePrompt { buffer, quit_after });
                }
                KeyCode::Char(c) => {
                    buffer.push(c);
                    self.modal = Some(Modal::SavePrompt { buffer, quit_after });
                }
                _ => {
                    self.modal = Some(Modal::SavePrompt { buffer, quit_after });
                }
            },
            Modal::Menu(mut menu) => match menu.on_key(key, &self.state) {
                MenuOutcome::Continue => {
                    self.modal = Some(Modal::Menu(menu));
                }
                MenuOutcome::Cancelled => {
                    self.status = Some("Edit cancelled".to_string());
                }
                MenuOutcome::Picked(field) => {
                    self.modal = Some(Modal::Field(field));
                }
            },
            Modal::Field(field) => match field.on_key(key) {
                (Some(f), EditOutcome::Continue) => {
                    self.modal = Some(Modal::Field(f));
                }
                (_, EditOutcome::Cancelled) => {
                    self.status = Some("Edit cancelled".to_string());
                }
                (_, EditOutcome::Committed(action)) => {
                    self.state.apply(action);
                    if let Some(err) = self.state.last_error.as_deref() {
                        self.status = Some(format!("edit failed: {}", err));
                    } else {
                        self.status = Some("Field updated".to_string());
                        self.clamp_selected_row();
                    }
                }
                _ => {}
            },
            Modal::QuitConfirm { mut selected } => {
                // Three buttons: 0=Save & quit, 1=Discard & quit, 2=Cancel.
                const N: usize = 3;
                match key {
                    KeyCode::Esc => {
                        self.status = Some("Quit cancelled".to_string());
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        selected = selected.saturating_sub(1);
                        self.modal = Some(Modal::QuitConfirm { selected });
                    }
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                        if selected + 1 < N {
                            selected += 1;
                        }
                        self.modal = Some(Modal::QuitConfirm { selected });
                    }
                    // Letter shortcuts for the three actions.
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        self.open_save_prompt(true);
                    }
                    KeyCode::Char('d') | KeyCode::Char('D') => {
                        self.running = false;
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        self.status = Some("Quit cancelled".to_string());
                    }
                    KeyCode::Enter => match selected {
                        0 => self.open_save_prompt(true),
                        1 => self.running = false,
                        _ => {
                            self.status = Some("Quit cancelled".to_string());
                        }
                    },
                    _ => {
                        self.modal = Some(Modal::QuitConfirm { selected });
                    }
                }
            }
        }
    }
}
