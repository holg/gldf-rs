//! Ratatui rendering. Splits the screen into a sidebar (section list)
//! and a detail pane (per-section content). Modals (wizard, save
//! prompt) draw centered overlays on top via `Clear`.

mod detail;
mod sidebar;

use crate::app::{App, Modal, Section};
use crate::edit_modal;
use crate::l3d_view;
use crate::polar_view;
use crate::wizard;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    if app.has_content {
        draw_main(frame, app, outer[0]);
    } else {
        draw_welcome(frame, outer[0]);
    }
    draw_statusbar(frame, app, outer[1]);

    match &app.modal {
        Some(Modal::Wizard(overlay)) => wizard::draw(frame, area, overlay),
        Some(Modal::SavePrompt { buffer, .. }) => draw_save_prompt(frame, area, buffer),
        Some(Modal::QuitConfirm { selected }) => draw_quit_confirm(frame, area, *selected),
        Some(Modal::Menu(menu)) => edit_modal::draw_action_menu(frame, area, menu),
        Some(Modal::Field(field)) => edit_modal::draw_field_edit(frame, area, field),
        None => {}
    }
}

fn draw_main(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(1)])
        .split(area);
    sidebar::draw(frame, app, cols[0]);
    // Photometry and Files get a canvas-based visualizer on top of the
    // text detail view; everything else stays text-only.
    match app.current_section() {
        Section::Photometry => draw_photometry_with_polar(frame, app, cols[1]),
        Section::Files | Section::Linkage => draw_files_with_l3d(frame, app, cols[1]),
        _ => detail::draw(frame, app, cols[1]),
    }
}

fn draw_photometry_with_polar(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(8)])
        .split(area);
    detail::draw(frame, app, rows[0]);
    polar_view::draw(frame, rows[1], app.first_photometry_bytes(), "Polar");
}

fn draw_files_with_l3d(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(8)])
        .split(area);
    detail::draw(frame, app, rows[0]);
    l3d_view::draw(frame, rows[1], app.first_l3d_bytes(), &app.orbit, "L3D iso");
}

fn draw_welcome(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::raw(""),
        Line::styled(
            "  gldf-edit",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Line::raw("  Terminal frontend for the gldf-rs editor"),
        Line::raw(""),
        Line::raw("  No GLDF loaded yet."),
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                "  N",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw("       Run the New GLDF wizard"),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl-S",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  Save current state (after the wizard)"),
        ]),
        Line::from(vec![
            Span::styled(
                "  q / Esc",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Quit"),
        ]),
        Line::raw(""),
        Line::styled(
            "  Or pass --file <path.gldf> on the command line to open a file directly.",
            Style::default().fg(Color::DarkGray),
        ),
    ];
    let para = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
    frame.render_widget(para, area);
}

fn draw_save_prompt(frame: &mut Frame, area: Rect, buffer: &str) {
    let modal = centered_rect(60, 20, area);
    frame.render_widget(Clear, modal);
    let block = Block::default()
        .title(" Save GLDF ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(modal);
    frame.render_widget(block, modal);
    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Path:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if buffer.is_empty() {
                    "_".to_string()
                } else {
                    format!("{}_", buffer)
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

fn draw_quit_confirm(frame: &mut Frame, area: Rect, selected: usize) {
    let modal = centered_rect(60, 25, area);
    frame.render_widget(Clear, modal);
    let block = Block::default()
        .title(" Unsaved changes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let labels = [("S", "Save & quit"), ("D", "Discard & quit"), ("C", "Cancel")];
    let mut buttons: Vec<Span> = Vec::new();
    for (i, (key, label)) in labels.iter().enumerate() {
        let active = i == selected;
        let style = if active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        buttons.push(Span::styled(format!(" [{}] {} ", key, label), style));
        if i + 1 < labels.len() {
            buttons.push(Span::raw(" "));
        }
    }

    let lines = vec![
        Line::raw(""),
        Line::styled(
            "  You have unsaved changes from the wizard.",
            Style::default().fg(Color::White),
        ),
        Line::raw(""),
        Line::from(buttons),
        Line::raw(""),
        Line::styled(
            "  ←/→ pick · Enter confirm · S save · D discard · C / Esc cancel",
            Style::default().fg(Color::DarkGray),
        ),
    ];
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

fn draw_statusbar(frame: &mut Frame, app: &App, area: Rect) {
    let default_text = if app.has_content {
        match app.current_section() {
            Section::Header | Section::Overview => {
                "q quit · ↑↓ section · e edit · Ctrl-S save · N new GLDF"
            }
            Section::LightSources | Section::Emitters | Section::Variants => {
                "q quit · ↑↓ section · n/p row · e edit · d delete · Ctrl-S save"
            }
            Section::Files | Section::Linkage => {
                "q quit · ↑↓ section · n/p row · e rename · d delete · Shift-←↑↓→ orbit · Ctrl-S save"
            }
            _ => "q quit · ↑↓ section · Ctrl-S save · N new GLDF",
        }
    } else {
        "N: new GLDF wizard · q: quit"
    };
    let text = app.status.as_deref().unwrap_or(default_text);
    let para = Paragraph::new(text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(para, area);
}
