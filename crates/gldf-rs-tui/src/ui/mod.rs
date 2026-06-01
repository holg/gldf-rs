//! Ratatui rendering. Splits the screen into a sidebar (section list)
//! and a detail pane (per-section content).

mod detail;
mod sidebar;

use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    draw_main(frame, app, outer[0]);
    draw_statusbar(frame, app, outer[1]);
}

fn draw_main(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(1)])
        .split(area);
    sidebar::draw(frame, app, cols[0]);
    detail::draw(frame, app, cols[1]);
}

fn draw_statusbar(frame: &mut Frame, app: &App, area: Rect) {
    let text = app
        .status
        .as_deref()
        .unwrap_or("q quit · ↑↓ navigate · gldf-edit Cut 1 (viewer)");
    let para = Paragraph::new(text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(para, area);
}
