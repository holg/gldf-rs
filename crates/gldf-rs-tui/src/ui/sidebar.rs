//! Left sidebar — section list mirroring the WASM left-nav.

use crate::app::{App, Section};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = Section::ALL
        .iter()
        .enumerate()
        .map(|(idx, s)| {
            let marker = if idx == app.selected { "▸ " } else { "  " };
            ListItem::new(Line::from(format!("{}{}", marker, s.label())))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected));

    let list = List::new(items)
        .block(Block::default().title("Sections").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut list_state);
}
