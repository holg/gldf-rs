//! Modal file picker wrapping `ratatui-explorer`. Pressing Enter on a
//! file returns the path; on a directory descends into it. Esc cancels.
//!
//! Used by:
//! - the wizard's Photometry step (F4 / Ctrl-O while focusing the path
//!   field opens this overlay)
//! - any future "add file" flow in the editor

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, widgets::Clear, Frame};
use ratatui_explorer::{FileExplorer, Input};
use std::path::PathBuf;

/// What the picker is doing right now.
pub enum PickerOutcome {
    /// Still active; keep absorbing keys.
    Continue,
    /// User picked a file. Path is absolute (the explorer guarantees this).
    Picked(PathBuf),
    /// User pressed Esc.
    Cancelled,
}

pub struct FilePicker {
    explorer: FileExplorer,
}

impl FilePicker {
    /// New picker rooted in the process's CWD. The `title` is currently
    /// ignored — ratatui-explorer 0.2's default theme draws its own
    /// title block. We can layer custom styling on top when we upgrade
    /// to the 0.3 line (which requires ratatui 0.30).
    pub fn new(_title: &str) -> Result<Self> {
        let explorer = FileExplorer::new()?;
        Ok(Self { explorer })
    }

    pub fn on_key(&mut self, key: KeyEvent) -> PickerOutcome {
        match key.code {
            KeyCode::Esc => PickerOutcome::Cancelled,
            KeyCode::Enter => {
                let current = self.explorer.current();
                if current.is_dir() {
                    // Descend; ratatui-explorer's `Right` handler does this.
                    let _ = self.explorer.handle(Input::Right);
                    PickerOutcome::Continue
                } else {
                    PickerOutcome::Picked(current.path().clone())
                }
            }
            // Forward navigation keys to the explorer.
            _ => {
                let input: Input = match (key.code, key.modifiers) {
                    (KeyCode::Up, _) => Input::Up,
                    (KeyCode::Down, _) => Input::Down,
                    (KeyCode::Left, _) | (KeyCode::Char('h'), KeyModifiers::NONE) => Input::Left,
                    (KeyCode::Right, _) | (KeyCode::Char('l'), KeyModifiers::NONE) => Input::Right,
                    (KeyCode::Char('j'), KeyModifiers::NONE) => Input::Down,
                    (KeyCode::Char('k'), KeyModifiers::NONE) => Input::Up,
                    (KeyCode::Home, _) => Input::Home,
                    (KeyCode::End, _) => Input::End,
                    (KeyCode::PageUp, _) => Input::PageUp,
                    (KeyCode::PageDown, _) => Input::PageDown,
                    _ => Input::None,
                };
                let _ = self.explorer.handle(input);
                PickerOutcome::Continue
            }
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        let modal = centered_rect(70, 70, area);
        frame.render_widget(Clear, modal);
        frame.render_widget_ref(self.explorer.widget(), modal);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    use ratatui::layout::{Constraint, Direction, Layout};
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
