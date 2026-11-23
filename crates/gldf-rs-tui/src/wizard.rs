//! New GLDF wizard for the TUI. Owns its own focus state and draws as
//! a centered modal over the main sidebar/detail layout. Builds the
//! same [`WizardDraft`] the CLI / Leptos / Python frontends produce
//! and feeds it to the same [`apply_wizard`] reducer.
//!
//! The overlay is intentionally minimal: line-edit textfields and
//! radio-style choice rows. The model layer (`gldf_rs_editor::wizard`)
//! does all the GLDF logic — this file is purely view + key handling.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use gldf_rs_editor::{
    apply_wizard, wizard_text as txt, EmitterDraft, GldfState, LightEmitterKind, WizardDraft,
    WizardLightSourceKind, WizardStep,
};

use crate::file_picker::{FilePicker, PickerOutcome};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Outcome of an `on_key` tick — `None` while the wizard is still
/// running, `Some(...)` once the user finishes or cancels.
// Transient UI value, one at a time; boxing would change the public enum.
#[allow(clippy::large_enum_variant)]
pub enum WizardOutcome {
    Continue,
    Finished(WizardDraft),
    Cancelled,
}

/// Live overlay state. Owns one text buffer per field; the active
/// field is identified by `step` + `focus`.
pub struct WizardOverlay {
    pub step: WizardStep,
    pub focus: usize,
    pub draft: WizardDraft,
    pub include_emitter: bool,
    pub message: Option<String>,
    /// When the photometry step pops the file picker (F4), it lives here
    /// and absorbs all keys until the user picks a file or cancels.
    pub picker: Option<FilePicker>,
}

impl Default for WizardOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl WizardOverlay {
    pub fn new() -> Self {
        let draft = WizardDraft {
            header_created_with: "gldf-edit (TUI)".to_string(),
            header_default_language: "en".to_string(),
            ..Default::default()
        };
        Self {
            step: WizardStep::Header,
            focus: 0,
            draft,
            include_emitter: true,
            message: None,
            picker: None,
        }
    }

    /// Key entry point used by `App`. Forwards to the file picker when
    /// it's open; otherwise dispatches to the active step's handler.
    pub fn on_key(&mut self, key: KeyEvent) -> WizardOutcome {
        if self.picker.is_some() {
            return self.on_key_with_picker(key);
        }
        let code = key.code;
        let mods = key.modifiers;
        // Esc cancels at any time.
        if matches!(code, KeyCode::Esc) {
            return WizardOutcome::Cancelled;
        }
        // F4 in the Photometry step opens the file picker.
        if matches!(code, KeyCode::F(4)) && matches!(self.step, WizardStep::Photometry) {
            match FilePicker::new("Pick a photometry file") {
                Ok(p) => self.picker = Some(p),
                Err(e) => self.message = Some(format!("file picker: {}", e)),
            }
            return WizardOutcome::Continue;
        }

        match self.step {
            WizardStep::Header => self.on_key_header(code, mods),
            WizardStep::Photometry => self.on_key_photometry(code, mods),
            WizardStep::LightSource => self.on_key_lightsource(code, mods),
            WizardStep::Emitter => self.on_key_emitter(code, mods),
            WizardStep::Geometry => self.on_key_geometry(code, mods),
            // Mounting + Driver steps are skip-only in the TUI for
            // now — Tab/Enter advances to the next step. The lib's
            // WizardDraft fields default to empty strings, which means
            // the apply_wizard reducer writes no `<Mountings>` or
            // ControlGear/Equipment block. Users who need either can
            // configure them from the editor's Variants / ControlGear
            // pages after Finish.
            WizardStep::Sensor
            | WizardStep::Emergency
            | WizardStep::Mounting
            | WizardStep::Driver => self.advance_step(code, mods),
            WizardStep::Variant => self.on_key_variant(code, mods),
            WizardStep::Review => self.on_key_review(code, mods),
        }
    }

    fn advance_step(&mut self, code: KeyCode, _mods: KeyModifiers) -> WizardOutcome {
        use crossterm::event::KeyCode::{Enter, Tab};
        if matches!(code, Tab | Enter) {
            self.step = match self.step {
                WizardStep::Sensor => WizardStep::Emergency,
                WizardStep::Emergency => WizardStep::LightSource,
                WizardStep::Mounting => WizardStep::Driver,
                WizardStep::Driver => WizardStep::Variant,
                other => other,
            };
        }
        WizardOutcome::Continue
    }

    fn on_key_with_picker(&mut self, key: KeyEvent) -> WizardOutcome {
        let picker = self.picker.as_mut().unwrap();
        match picker.on_key(key) {
            PickerOutcome::Continue => {}
            PickerOutcome::Cancelled => {
                self.picker = None;
            }
            PickerOutcome::Picked(path) => {
                self.picker = None;
                // Populate the Photometry step from the picked file.
                self.draft.photometry_filename = path.to_string_lossy().into_owned();
                self.try_load_photometry();
            }
        }
        WizardOutcome::Continue
    }

    fn on_key_header(&mut self, code: KeyCode, _mods: KeyModifiers) -> WizardOutcome {
        // 4 fields: author, manufacturer, created-with, language.
        let n = 4;
        match code {
            KeyCode::Up => {
                if self.focus > 0 {
                    self.focus -= 1;
                }
                return WizardOutcome::Continue;
            }
            KeyCode::Down => {
                if self.focus + 1 < n {
                    self.focus += 1;
                }
                return WizardOutcome::Continue;
            }
            KeyCode::Tab => {
                self.step = WizardStep::Photometry;
                self.focus = 0;
                return WizardOutcome::Continue;
            }
            KeyCode::Enter => {
                if self.focus + 1 < n {
                    self.focus += 1;
                } else {
                    self.step = WizardStep::Photometry;
                    self.focus = 0;
                }
                return WizardOutcome::Continue;
            }
            _ => {}
        }
        let fields: [&mut String; 4] = [
            &mut self.draft.header_author,
            &mut self.draft.header_manufacturer,
            &mut self.draft.header_created_with,
            &mut self.draft.header_default_language,
        ];
        edit_text(fields[self.focus], code);
        WizardOutcome::Continue
    }

    fn on_key_photometry(&mut self, code: KeyCode, _mods: KeyModifiers) -> WizardOutcome {
        // Single field: photometry path (loaded via std::fs::read on Tab/Enter).
        if let KeyCode::BackTab = code {
            self.step = WizardStep::Header;
            self.focus = 0;
            return WizardOutcome::Continue;
        }
        if matches!(code, KeyCode::Tab | KeyCode::Enter) {
            self.try_load_photometry();
            self.step = WizardStep::LightSource;
            self.focus = 0;
            return WizardOutcome::Continue;
        }
        edit_text(&mut self.draft.photometry_filename, code);
        WizardOutcome::Continue
    }

    fn try_load_photometry(&mut self) {
        let raw = self.draft.photometry_filename.trim().to_string();
        if raw.is_empty() {
            self.draft.photometry_bytes = None;
            self.draft.photometry_filename.clear();
            return;
        }
        let path = std::path::PathBuf::from(&raw);
        match std::fs::read(&path) {
            Ok(bytes) => {
                self.draft.photometry_bytes = Some(bytes);
                self.draft.photometry_filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("photometry")
                    .to_string();
                self.message = Some(format!("Loaded photometry from {}", raw));
            }
            Err(e) => {
                self.draft.photometry_bytes = None;
                self.message = Some(format!("photometry: {} ({})", e, raw));
            }
        }
    }

    fn on_key_lightsource(&mut self, code: KeyCode, _mods: KeyModifiers) -> WizardOutcome {
        // 4 rows: kind picker, name, manufacturer, rated power.
        let n = 4;
        match code {
            KeyCode::BackTab => {
                self.step = WizardStep::Photometry;
                self.focus = 0;
                return WizardOutcome::Continue;
            }
            KeyCode::Tab => {
                self.step = WizardStep::Emitter;
                self.focus = 0;
                return WizardOutcome::Continue;
            }
            KeyCode::Up => {
                if self.focus > 0 {
                    self.focus -= 1;
                }
                return WizardOutcome::Continue;
            }
            KeyCode::Down => {
                if self.focus + 1 < n {
                    self.focus += 1;
                }
                return WizardOutcome::Continue;
            }
            KeyCode::Enter => {
                if self.focus + 1 < n {
                    self.focus += 1;
                } else {
                    self.step = WizardStep::Emitter;
                    self.focus = 0;
                }
                return WizardOutcome::Continue;
            }
            _ => {}
        }

        match self.focus {
            0 => self.cycle_ls_kind(code),
            1 => edit_text(&mut self.draft.ls_name, code),
            2 => edit_text(&mut self.draft.ls_manufacturer, code),
            3 => edit_text(&mut self.draft.ls_rated_power, code),
            _ => {}
        }
        WizardOutcome::Continue
    }

    fn cycle_ls_kind(&mut self, code: KeyCode) {
        match code {
            KeyCode::Left => {
                self.draft.ls_kind = match self.draft.ls_kind {
                    WizardLightSourceKind::Fixed => WizardLightSourceKind::MultiChannel,
                    WizardLightSourceKind::Changeable => WizardLightSourceKind::Fixed,
                    WizardLightSourceKind::MultiChannel => WizardLightSourceKind::Changeable,
                };
            }
            KeyCode::Right | KeyCode::Char(' ') => {
                self.draft.ls_kind = match self.draft.ls_kind {
                    WizardLightSourceKind::Fixed => WizardLightSourceKind::Changeable,
                    WizardLightSourceKind::Changeable => WizardLightSourceKind::MultiChannel,
                    WizardLightSourceKind::MultiChannel => WizardLightSourceKind::Fixed,
                };
            }
            _ => {}
        }
    }

    fn on_key_emitter(&mut self, code: KeyCode, _mods: KeyModifiers) -> WizardOutcome {
        // 2 rows: include-emitter toggle, child name.
        let n = 2;
        match code {
            KeyCode::BackTab => {
                self.step = WizardStep::LightSource;
                self.focus = 0;
                return WizardOutcome::Continue;
            }
            KeyCode::Tab => {
                self.step = WizardStep::Geometry;
                self.focus = 0;
                return WizardOutcome::Continue;
            }
            KeyCode::Up => {
                if self.focus > 0 {
                    self.focus -= 1;
                }
                return WizardOutcome::Continue;
            }
            KeyCode::Down => {
                if self.focus + 1 < n {
                    self.focus += 1;
                }
                return WizardOutcome::Continue;
            }
            KeyCode::Enter => {
                if self.focus + 1 < n {
                    self.focus += 1;
                } else {
                    self.step = WizardStep::Geometry;
                    self.focus = 0;
                }
                return WizardOutcome::Continue;
            }
            _ => {}
        }

        match self.focus {
            0 => {
                if matches!(code, KeyCode::Left | KeyCode::Right | KeyCode::Char(' ')) {
                    self.include_emitter = !self.include_emitter;
                }
            }
            1 if self.include_emitter => {
                let mut buf = self
                    .draft
                    .emitter_draft
                    .clone()
                    .unwrap_or_else(|| EmitterDraft {
                        child_name: "Primary".to_string(),
                        kind: self.emitter_kind_for_ls(),
                        ..EmitterDraft::default()
                    });
                buf.kind = self.emitter_kind_for_ls();
                edit_text(&mut buf.child_name, code);
                self.draft.emitter_draft = Some(buf);
            }
            _ => {}
        }
        WizardOutcome::Continue
    }

    fn emitter_kind_for_ls(&self) -> LightEmitterKind {
        match self.draft.ls_kind {
            WizardLightSourceKind::Fixed => LightEmitterKind::Fixed,
            WizardLightSourceKind::Changeable => LightEmitterKind::Changeable,
            WizardLightSourceKind::MultiChannel => LightEmitterKind::MultiChannel,
        }
    }

    fn on_key_review(&mut self, code: KeyCode, _mods: KeyModifiers) -> WizardOutcome {
        // Resolve emitter inclusion right at the boundary so toggling
        // it on/off after step 4 is honoured.
        if matches!(code, KeyCode::BackTab) {
            self.step = WizardStep::Variant;
            self.focus = 0;
            return WizardOutcome::Continue;
        }
        if matches!(
            code,
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y')
        ) {
            let mut draft = self.draft.clone();
            if !self.include_emitter {
                draft.emitter_draft = None;
            }
            return WizardOutcome::Finished(draft);
        }
        WizardOutcome::Continue
    }

    fn on_key_variant(&mut self, code: KeyCode, _mods: KeyModifiers) -> WizardOutcome {
        // Single field — variant name.
        match code {
            KeyCode::BackTab => {
                self.step = WizardStep::Geometry;
                self.focus = 0;
                WizardOutcome::Continue
            }
            KeyCode::Tab | KeyCode::Enter => {
                self.step = WizardStep::Review;
                self.focus = 0;
                WizardOutcome::Continue
            }
            _ => {
                edit_text(&mut self.draft.variant_name, code);
                WizardOutcome::Continue
            }
        }
    }

    fn on_key_geometry(&mut self, code: KeyCode, _mods: KeyModifiers) -> WizardOutcome {
        // Single field — geometry path. The actual file load happens on
        // Tab/Enter via try_load_geometry().
        match code {
            KeyCode::BackTab => {
                self.step = WizardStep::Emitter;
                self.focus = 0;
                WizardOutcome::Continue
            }
            KeyCode::Tab | KeyCode::Enter => {
                self.try_load_geometry();
                self.step = WizardStep::Variant;
                self.focus = 0;
                WizardOutcome::Continue
            }
            _ => {
                edit_text(&mut self.draft.geometry_filename, code);
                WizardOutcome::Continue
            }
        }
    }

    /// Best-effort: if `geometry_filename` looks like a file path,
    /// read the bytes into `geometry_bytes`. Mirrors `try_load_photometry`.
    fn try_load_geometry(&mut self) {
        let raw = self.draft.geometry_filename.trim().to_string();
        if raw.is_empty() {
            self.draft.geometry_bytes = None;
            self.draft.geometry_filename.clear();
            return;
        }
        let path = std::path::PathBuf::from(&raw);
        match std::fs::read(&path) {
            Ok(bytes) => {
                self.draft.geometry_bytes = Some(bytes);
                self.draft.geometry_filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("luminaire.l3d")
                    .to_string();
                self.message = Some(format!("Loaded geometry from {}", raw));
            }
            Err(e) => {
                self.draft.geometry_bytes = None;
                self.message = Some(format!("geometry: {} ({})", e, raw));
            }
        }
    }
}

/// Minimal text-edit: append printable chars, backspace removes last.
fn edit_text(buf: &mut String, code: KeyCode) {
    match code {
        KeyCode::Char(c) => buf.push(c),
        KeyCode::Backspace => {
            buf.pop();
        }
        _ => {}
    }
}

/// Render the wizard overlay centered over the main UI. The caller
/// already drew the underlying layout; we clear a centered rect and
/// paint the modal on top.
pub fn draw(frame: &mut Frame, area: Rect, w: &WizardOverlay) {
    let modal = centered_rect(70, 80, area);
    frame.render_widget(Clear, modal);

    let block = Block::default()
        .title(" New GLDF wizard ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(inner);

    draw_step_indicator(frame, rows[0], w.step);
    draw_step_body(frame, rows[1], w);
    draw_footer(frame, rows[2], w);

    // File picker layers over the wizard when active.
    if let Some(picker) = &w.picker {
        picker.draw(frame, area);
    }
}

fn draw_step_indicator(frame: &mut Frame, area: Rect, current: WizardStep) {
    let mut spans = vec![];
    let total = WizardStep::ALL.len();
    for (i, step) in WizardStep::ALL.iter().enumerate() {
        let active = *step == current;
        let style = if active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(
            format!(" {} {} ", step.number(), step.label()),
            style,
        ));
        if i + 1 < total {
            spans.push(Span::raw(" → "));
        }
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_step_body(frame: &mut Frame, area: Rect, w: &WizardOverlay) {
    let lines = match w.step {
        WizardStep::Header => header_body(w),
        WizardStep::Photometry => photometry_body(w),
        WizardStep::LightSource => lightsource_body(w),
        WizardStep::Emitter => emitter_body(w),
        WizardStep::Geometry => geometry_body(w),
        WizardStep::Sensor | WizardStep::Emergency | WizardStep::Mounting | WizardStep::Driver => {
            skip_step_body(w)
        }
        WizardStep::Variant => variant_body(w),
        WizardStep::Review => review_body(w),
    };
    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn header_body(w: &WizardOverlay) -> Vec<Line<'static>> {
    let labels = [
        (txt::HEADER_AUTHOR_LABEL, w.draft.header_author.as_str()),
        (
            txt::HEADER_MANUFACTURER_LABEL,
            w.draft.header_manufacturer.as_str(),
        ),
        (
            txt::HEADER_CREATED_WITH_LABEL,
            w.draft.header_created_with.as_str(),
        ),
        (
            txt::HEADER_DEFAULT_LANGUAGE_LABEL,
            w.draft.header_default_language.as_str(),
        ),
    ];
    let mut out = field_rows(&labels, w.focus);
    out.push(Line::raw(""));
    out.push(Line::styled(
        WizardStep::Header.blurb(),
        Style::default().fg(Color::DarkGray),
    ));
    out
}

fn photometry_body(w: &WizardOverlay) -> Vec<Line<'static>> {
    let path_line = if w.draft.photometry_bytes.is_some() {
        format!("{} (loaded)", w.draft.photometry_filename)
    } else {
        w.draft.photometry_filename.clone()
    };
    let mut out = field_rows(&[(txt::PHOTOMETRY_DROP_LABEL, path_line.as_str())], 0);
    out.push(Line::raw(""));
    out.push(Line::styled(
        txt::PHOTOMETRY_SKIP_HINT,
        Style::default().fg(Color::DarkGray),
    ));
    out
}

fn lightsource_body(w: &WizardOverlay) -> Vec<Line<'static>> {
    let kind = format!("{}  (←/→/Space to change)", w.draft.ls_kind.long_label());
    let labels = [
        (txt::LS_KIND_LABEL, kind.as_str()),
        (txt::LS_NAME_LABEL, w.draft.ls_name.as_str()),
        (txt::LS_MANUFACTURER_LABEL, w.draft.ls_manufacturer.as_str()),
        (txt::LS_POWER_LABEL, w.draft.ls_rated_power.as_str()),
    ];
    let mut out = field_rows(&labels, w.focus);
    out.push(Line::raw(""));
    out.push(Line::styled(
        WizardStep::LightSource.blurb(),
        Style::default().fg(Color::DarkGray),
    ));
    out
}

fn emitter_body(w: &WizardOverlay) -> Vec<Line<'static>> {
    let toggle = if w.include_emitter {
        "Yes — create one Emitter linked to the LightSource"
    } else {
        "No — skip emitter (advanced)"
    };
    let name = w
        .draft
        .emitter_draft
        .as_ref()
        .map(|e| e.child_name.as_str())
        .unwrap_or("");
    let mut out = field_rows(
        &[
            (txt::EMITTER_INCLUDE_LABEL, toggle),
            (txt::EMITTER_NAME_LABEL, name),
        ],
        w.focus,
    );
    out.push(Line::raw(""));
    out.push(Line::styled(
        WizardStep::Emitter.blurb(),
        Style::default().fg(Color::DarkGray),
    ));
    out
}

fn skip_step_body(_w: &WizardOverlay) -> Vec<Line<'static>> {
    vec![
        Line::raw(""),
        Line::styled(
            "TUI: this step is not yet implemented. Tab/Enter to skip.",
            Style::default().fg(Color::DarkGray),
        ),
        Line::raw(""),
        Line::styled(
            "Edit Mountings + Driver from the WASM editor or post-Finish via the sidebar pages.",
            Style::default().fg(Color::DarkGray),
        ),
    ]
}

fn variant_body(w: &WizardOverlay) -> Vec<Line<'static>> {
    let mut out = field_rows(
        &[(txt::VARIANT_NAME_LABEL, w.draft.variant_name.as_str())],
        0,
    );
    out.push(Line::raw(""));
    out.push(Line::styled(
        txt::VARIANT_HINT,
        Style::default().fg(Color::DarkGray),
    ));
    out.push(Line::raw(""));
    out.push(Line::styled(
        WizardStep::Variant.blurb(),
        Style::default().fg(Color::DarkGray),
    ));
    out
}

fn geometry_body(w: &WizardOverlay) -> Vec<Line<'static>> {
    let path_line = if w.draft.geometry_bytes.is_some() {
        format!("{} (loaded)", w.draft.geometry_filename)
    } else {
        w.draft.geometry_filename.clone()
    };
    let mut out = field_rows(&[("Path to .l3d", path_line.as_str())], 0);
    out.push(Line::raw(""));
    out.push(Line::styled(
        "Leave blank to skip — Enter loads the file and advances. \
         Without an L3D the wizard writes a bare EmitterReference \
         (no 3D model); with one it writes a ModelGeometryReference.",
        Style::default().fg(Color::DarkGray),
    ));
    out.push(Line::raw(""));
    out.push(Line::styled(
        WizardStep::Geometry.blurb(),
        Style::default().fg(Color::DarkGray),
    ));
    out
}

fn review_body(w: &WizardOverlay) -> Vec<Line<'static>> {
    let photometry_summary = if w.draft.photometry_bytes.is_some() {
        w.draft.photometry_filename.clone()
    } else {
        txt::REVIEW_PHOTOMETRY_SKIPPED.to_string()
    };
    let emitter_summary = if w.include_emitter {
        w.draft
            .emitter_draft
            .as_ref()
            .map(|e| format!("{} \"{}\"", emitter_kind_label(e.kind), e.child_name))
            .unwrap_or_else(|| "Primary".to_string())
    } else {
        txt::REVIEW_EMITTER_SKIPPED.to_string()
    };
    vec![
        kv(txt::HEADER_AUTHOR_LABEL, &w.draft.header_author),
        kv(txt::HEADER_MANUFACTURER_LABEL, &w.draft.header_manufacturer),
        kv(txt::HEADER_CREATED_WITH_LABEL, &w.draft.header_created_with),
        kv(
            txt::HEADER_DEFAULT_LANGUAGE_LABEL,
            &w.draft.header_default_language,
        ),
        kv(WizardStep::Photometry.label(), &photometry_summary),
        kv(
            WizardStep::LightSource.label(),
            &format!("{} \"{}\"", w.draft.ls_kind.short_label(), w.draft.ls_name),
        ),
        kv(WizardStep::Emitter.label(), &emitter_summary),
        kv(
            WizardStep::Variant.label(),
            if w.draft.variant_name.trim().is_empty() {
                "Standard"
            } else {
                w.draft.variant_name.as_str()
            },
        ),
        Line::raw(""),
        Line::styled(txt::REVIEW_FOOTNOTE, Style::default().fg(Color::DarkGray)),
    ]
}

fn emitter_kind_label(kind: LightEmitterKind) -> &'static str {
    match kind {
        LightEmitterKind::Fixed => "Fixed",
        LightEmitterKind::Changeable => "Changeable",
        LightEmitterKind::MultiChannel => "MultiChannel",
        LightEmitterKind::Sensor => "Sensor",
    }
}

fn field_rows(rows: &[(&str, &str)], focus: usize) -> Vec<Line<'static>> {
    rows.iter()
        .enumerate()
        .flat_map(|(i, (label, value))| {
            let active = i == focus;
            let marker = if active { "▸ " } else { "  " };
            let value_style = if active {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let label_style = if active {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let display = if value.is_empty() && active {
                "_".to_string()
            } else if value.is_empty() {
                "—".to_string()
            } else {
                (*value).to_string()
            };
            vec![
                Line::from(vec![
                    Span::raw(marker.to_string()),
                    Span::styled(format!("{:18}", label), label_style),
                    Span::styled(format!(" {}", display), value_style),
                ]),
                Line::raw(""),
            ]
        })
        .collect()
}

fn kv(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{:18}", label),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(if value.is_empty() {
            "—".to_string()
        } else {
            value.to_string()
        }),
    ])
}

fn draw_footer(frame: &mut Frame, area: Rect, w: &WizardOverlay) {
    let hint = match w.step {
        WizardStep::Review => "Enter/Y build · Shift-Tab back · Esc cancel",
        WizardStep::Photometry => {
            "↑/↓ field · F4 file picker · Tab next step · Shift-Tab prev · Esc cancel"
        }
        _ => "↑/↓ field · Tab next step · Shift-Tab prev · Esc cancel",
    };
    let mut lines = vec![Line::styled(
        hint.to_string(),
        Style::default().fg(Color::DarkGray),
    )];
    if let Some(msg) = &w.message {
        lines.push(Line::styled(
            msg.clone(),
            Style::default().fg(Color::Yellow),
        ));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(v[1])[1]
}

/// Finalize a draft by running `apply_wizard`. Returns the loaded
/// state on success or an `Err(msg)` if the reducer flagged
/// `last_error`. Used by `App` so the wizard module owns the model
/// glue, not the app shell.
pub fn finish(draft: WizardDraft) -> Result<GldfState, String> {
    let mut state = GldfState::default();
    apply_wizard(&mut state, &draft);
    if let Some(err) = state.last_error.clone() {
        return Err(err);
    }
    Ok(state)
}
