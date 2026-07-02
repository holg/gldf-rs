//! Inline edit modal for the post-wizard sidebar/detail view. The
//! modal itself is generic — it edits a single string field and
//! dispatches a caller-supplied `GldfAction` on commit. Each section's
//! "what fields can I edit" lives in [`section_fields`].

use crossterm::event::KeyCode;
use gldf_rs_editor::{wizard_text as txt, GldfAction, GldfState};
use gldf_rs::gldf::header::{Locale, LocaleFoo};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::Section;

/// Outcome of an `on_key` tick.
pub enum EditOutcome {
    Continue,
    Cancelled,
    Committed(GldfAction),
}

/// Live text-buffer + the closure that turns the committed value into
/// an action. Boxed so the modal can hold any kind of dispatch logic
/// without leaking the section it came from.
pub struct FieldEdit {
    pub label: String,
    pub buffer: String,
    pub commit: Box<dyn FnOnce(String) -> GldfAction + Send>,
}

impl FieldEdit {
    pub fn on_key(self, code: KeyCode) -> (Option<Self>, EditOutcome) {
        let Self {
            label,
            mut buffer,
            commit,
        } = self;
        match code {
            KeyCode::Esc => (None, EditOutcome::Cancelled),
            KeyCode::Enter => (None, EditOutcome::Committed(commit(buffer))),
            KeyCode::Backspace => {
                buffer.pop();
                (
                    Some(Self {
                        label,
                        buffer,
                        commit,
                    }),
                    EditOutcome::Continue,
                )
            }
            KeyCode::Char(c) => {
                buffer.push(c);
                (
                    Some(Self {
                        label,
                        buffer,
                        commit,
                    }),
                    EditOutcome::Continue,
                )
            }
            _ => (
                Some(Self {
                    label,
                    buffer,
                    commit,
                }),
                EditOutcome::Continue,
            ),
        }
    }
}

/// One editable field listed in the per-section action menu.
pub struct FieldSpec {
    /// Short shortcut letter, e.g. 'a' for Author. Shown next to the label.
    pub key: char,
    pub label: String,
    /// Build a populated `FieldEdit` from the current state. Boxed so
    /// the table can hold heterogeneous closures.
    pub make: Box<dyn Fn(&GldfState, usize) -> FieldEdit>,
}

/// Build the list of editable fields for a given section + row index.
/// Returns an empty Vec when nothing is editable in this cut.
pub fn section_fields(section: Section, row: usize, state: &GldfState) -> Vec<FieldSpec> {
    match section {
        Section::Header | Section::Overview => header_fields(),
        Section::LightSources => fixed_light_source_fields(row, state),
        Section::Emitters => emitter_fields(row, state),
        Section::Variants => variant_fields(row, state),
        Section::Files => file_fields(row, state),
        _ => Vec::new(),
    }
}

// ============================================================
// Header
// ============================================================

fn header_fields() -> Vec<FieldSpec> {
    vec![
        FieldSpec {
            key: 'a',
            label: txt::HEADER_AUTHOR_LABEL.to_string(),
            make: Box::new(|s, _| FieldEdit {
                label: txt::HEADER_AUTHOR_LABEL.to_string(),
                buffer: s.product.header.author.clone(),
                commit: Box::new(GldfAction::SetAuthor),
            }),
        },
        FieldSpec {
            key: 'm',
            label: txt::HEADER_MANUFACTURER_LABEL.to_string(),
            make: Box::new(|s, _| FieldEdit {
                label: txt::HEADER_MANUFACTURER_LABEL.to_string(),
                buffer: s.product.header.manufacturer.clone(),
                commit: Box::new(GldfAction::SetManufacturer),
            }),
        },
        FieldSpec {
            key: 'c',
            label: txt::HEADER_CREATED_WITH_LABEL.to_string(),
            make: Box::new(|s, _| FieldEdit {
                label: txt::HEADER_CREATED_WITH_LABEL.to_string(),
                buffer: s.product.header.created_with_application.clone(),
                commit: Box::new(GldfAction::SetCreatedWithApplication),
            }),
        },
        FieldSpec {
            key: 'l',
            label: txt::HEADER_DEFAULT_LANGUAGE_LABEL.to_string(),
            make: Box::new(|s, _| FieldEdit {
                label: txt::HEADER_DEFAULT_LANGUAGE_LABEL.to_string(),
                buffer: s
                    .product
                    .header
                    .default_language
                    .clone()
                    .unwrap_or_default(),
                commit: Box::new(|v: String| {
                    GldfAction::SetDefaultLanguage(if v.is_empty() { None } else { Some(v) })
                }),
            }),
        },
    ]
}

// ============================================================
// LightSources (fixed only in this cut)
// ============================================================

fn fixed_light_source_fields(row: usize, state: &GldfState) -> Vec<FieldSpec> {
    let ls = match state
        .product
        .general_definitions
        .light_sources
        .as_ref()
        .and_then(|l| l.fixed_light_source.get(row))
    {
        Some(ls) => ls,
        None => return Vec::new(),
    };
    let id = ls.id.clone();
    let id_for_rename = id.clone();
    let id_for_name = id.clone();
    let row_idx = row;
    vec![
        FieldSpec {
            key: 'r',
            label: format!("Rename id ({})", id),
            make: Box::new(move |_, _| FieldEdit {
                label: "New id".to_string(),
                buffer: id_for_rename.clone(),
                commit: {
                    let old = id_for_rename.clone();
                    Box::new(move |new_id| GldfAction::RenameFixedLightSource {
                        old_id: old.clone(),
                        new_id,
                    })
                },
            }),
        },
        FieldSpec {
            key: 'n',
            label: "Display name (en)".to_string(),
            make: Box::new(move |s, _| {
                let current = s
                    .product
                    .general_definitions
                    .light_sources
                    .as_ref()
                    .and_then(|l| l.fixed_light_source.get(row_idx))
                    .map(|ls| first_locale_value(Some(&ls.name)))
                    .unwrap_or_default();
                let _ = &id_for_name;
                FieldEdit {
                    label: "Display name".to_string(),
                    buffer: current,
                    commit: Box::new(move |v| {
                        GldfAction::SetFixedLightSourceName(row_idx, single_locale("en", &v))
                    }),
                }
            }),
        },
    ]
}

// ============================================================
// Emitters
// ============================================================

fn emitter_fields(row: usize, state: &GldfState) -> Vec<FieldSpec> {
    let emitters = state
        .product
        .general_definitions
        .emitters
        .as_ref()
        .map(|e| &e.emitter[..])
        .unwrap_or(&[]);
    let em = match emitters.get(row) {
        Some(e) => e,
        None => return Vec::new(),
    };
    let id = em.id.clone();
    let id_for_rename = id.clone();
    vec![FieldSpec {
        key: 'r',
        label: format!("Rename id ({})", id),
        make: Box::new(move |_, _| FieldEdit {
            label: "New id".to_string(),
            buffer: id_for_rename.clone(),
            commit: {
                let old = id_for_rename.clone();
                Box::new(move |new_id| GldfAction::RenameEmitter {
                    old_id: old.clone(),
                    new_id,
                })
            },
        }),
    }]
}

// ============================================================
// Variants
// ============================================================

fn variant_fields(row: usize, state: &GldfState) -> Vec<FieldSpec> {
    let variants = state
        .product
        .product_definitions
        .variants
        .as_ref()
        .map(|v| &v.variant[..])
        .unwrap_or(&[]);
    let v = match variants.get(row) {
        Some(v) => v,
        None => return Vec::new(),
    };
    let id = v.id.clone();
    let id_for_rename = id.clone();
    let row_idx = row;
    vec![
        FieldSpec {
            key: 'r',
            label: format!("Rename id ({})", id),
            make: Box::new(move |_, _| FieldEdit {
                label: "New id".to_string(),
                buffer: id_for_rename.clone(),
                commit: {
                    let old = id_for_rename.clone();
                    Box::new(move |new_id| GldfAction::RenameVariant {
                        old_id: old.clone(),
                        new_id,
                    })
                },
            }),
        },
        FieldSpec {
            key: 'n',
            label: "Display name (en)".to_string(),
            make: Box::new(move |s, _| {
                let current = s
                    .product
                    .product_definitions
                    .variants
                    .as_ref()
                    .and_then(|v| v.variant.get(row_idx))
                    .and_then(|v| v.name.as_ref())
                    .map(|lf| first_locale_value(Some(lf)))
                    .unwrap_or_default();
                FieldEdit {
                    label: "Variant name".to_string(),
                    buffer: current,
                    commit: Box::new(move |v| {
                        GldfAction::SetVariantName(row_idx, single_locale("en", &v))
                    }),
                }
            }),
        },
    ]
}

// ============================================================
// Files
// ============================================================

fn file_fields(row: usize, state: &GldfState) -> Vec<FieldSpec> {
    let files = &state.product.general_definitions.files.file;
    let f = match files.get(row) {
        Some(f) => f,
        None => return Vec::new(),
    };
    let id = f.id.clone();
    let id_for_rename = id.clone();
    vec![FieldSpec {
        key: 'r',
        label: format!("Rename id ({})", id),
        make: Box::new(move |_, _| FieldEdit {
            label: "New id".to_string(),
            buffer: id_for_rename.clone(),
            commit: {
                let old = id_for_rename.clone();
                Box::new(move |new_id| GldfAction::RenameFile {
                    old_id: old.clone(),
                    new_id,
                })
            },
        }),
    }]
}

// ============================================================
// LocaleFoo helpers
// ============================================================

fn first_locale_value(locales: Option<&LocaleFoo>) -> String {
    locales
        .and_then(|l| l.locale.first())
        .map(|loc| loc.value.clone())
        .unwrap_or_default()
}

fn single_locale(lang: &str, value: &str) -> LocaleFoo {
    LocaleFoo {
        locale: vec![Locale {
            language: lang.to_string(),
            value: value.to_string(),
        }],
    }
}

// ============================================================
// Action-menu modal — shown after a row is picked so the user can
// see which field to edit.
// ============================================================

pub struct ActionMenu {
    pub section: Section,
    pub row: usize,
    pub fields: Vec<FieldSpec>,
    pub selected: usize,
}

impl ActionMenu {
    pub fn new(section: Section, row: usize, state: &GldfState) -> Option<Self> {
        let fields = section_fields(section, row, state);
        if fields.is_empty() {
            None
        } else {
            Some(Self {
                section,
                row,
                fields,
                selected: 0,
            })
        }
    }

    /// `MenuOutcome` — same shape as `EditOutcome` but for menu navigation.
    pub fn on_key(&mut self, code: KeyCode, state: &GldfState) -> MenuOutcome {
        match code {
            KeyCode::Esc => MenuOutcome::Cancelled,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                MenuOutcome::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.fields.len() {
                    self.selected += 1;
                }
                MenuOutcome::Continue
            }
            KeyCode::Enter => {
                let f = &self.fields[self.selected];
                MenuOutcome::Picked((f.make)(state, self.row))
            }
            KeyCode::Char(c) => {
                // Letter shortcut: 'a' for Author etc.
                if let Some(f) = self.fields.iter().find(|f| f.key == c) {
                    MenuOutcome::Picked((f.make)(state, self.row))
                } else {
                    MenuOutcome::Continue
                }
            }
            _ => MenuOutcome::Continue,
        }
    }
}

pub enum MenuOutcome {
    Continue,
    Cancelled,
    Picked(FieldEdit),
}

// ============================================================
// Drawing
// ============================================================

pub fn draw_field_edit(frame: &mut Frame, area: Rect, edit: &FieldEdit) {
    let modal = centered_rect(60, 25, area);
    frame.render_widget(Clear, modal);
    let block = Block::default()
        .title(format!(" Edit · {} ", edit.label))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                format!("  {:18}", edit.label),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                if edit.buffer.is_empty() {
                    "_".to_string()
                } else {
                    format!("{}_", edit.buffer)
                },
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::styled(
            "  Enter: save · Esc: cancel",
            Style::default().fg(Color::DarkGray),
        ),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

pub fn draw_action_menu(frame: &mut Frame, area: Rect, menu: &ActionMenu) {
    let modal = centered_rect(50, 40, area);
    frame.render_widget(Clear, modal);
    let title = format!(" Edit · {} · row {} ", menu.section.label(), menu.row + 1);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let mut lines = vec![Line::raw("")];
    for (i, f) in menu.fields.iter().enumerate() {
        let active = i == menu.selected;
        let style = if active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(vec![Span::styled(
            format!("  [{}] {}", f.key, f.label),
            style,
        )]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "  ↑/↓ pick · Enter or letter · Esc cancel",
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
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
