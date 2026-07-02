//! Right detail pane. Each `Section` gets a small read-only renderer
//! that reads from `app.state.product`. Cut 1 is intentionally minimal
//! — enough to prove the shared state pipeline works end-to-end.

use crate::app::{App, Section};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let section = app.current_section();
    let title = format!(" {} ", section.label());
    let lines = render_section(app, section);
    let para = Paragraph::new(lines)
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn render_section(app: &App, section: Section) -> Vec<Line<'static>> {
    let product = &app.state.product;
    let row = app.selected_row;
    match section {
        Section::Overview => vec![
            kv("Author", &product.header.author),
            kv("Manufacturer", &product.header.manufacturer),
            kv("Created with", &product.header.created_with_application),
            kv("Creation time", &product.header.creation_time_code),
            kv("Format version", &format!("{:?}", product.header.format_version)),
            kv("Embedded files", &app.state.embedded_files.len().to_string()),
            kv(
                "Variants",
                &product
                    .product_definitions
                    .variants
                    .as_ref()
                    .map(|v| v.variant.len())
                    .unwrap_or(0)
                    .to_string(),
            ),
        ],
        Section::Header => vec![
            kv("Author", &product.header.author),
            kv("Manufacturer", &product.header.manufacturer),
            kv("Created with", &product.header.created_with_application),
            kv("Creation time code", &product.header.creation_time_code),
            kv(
                "Default language",
                product.header.default_language.as_deref().unwrap_or("—"),
            ),
        ],
        Section::Photometry => {
            let ps = product
                .general_definitions
                .photometries
                .as_ref()
                .map(|p| &p.photometry[..])
                .unwrap_or(&[]);
            if ps.is_empty() {
                vec![muted("(no Photometry blocks declared)")]
            } else {
                ps.iter()
                    .map(|p| {
                        let file = p
                            .photometry_file_reference
                            .as_ref()
                            .map(|r| r.file_id.as_str())
                            .unwrap_or("—");
                        Line::from(format!("· {}  → File {}", p.id, file))
                    })
                    .collect()
            }
        }
        Section::Spectra => {
            let ss = product
                .general_definitions
                .spectrums
                .as_ref()
                .map(|s| &s.spectrum[..])
                .unwrap_or(&[]);
            if ss.is_empty() {
                vec![muted("(no Spectrum blocks declared)")]
            } else {
                ss.iter()
                    .map(|s| {
                        let file = s
                            .spectrum_file_reference
                            .as_ref()
                            .map(|r| r.file_id.as_str())
                            .unwrap_or("(inline)");
                        Line::from(format!("· {}  → {}", s.id, file))
                    })
                    .collect()
            }
        }
        Section::LightSources => {
            let ls = product.general_definitions.light_sources.as_ref();
            let fixed = ls.map(|l| &l.fixed_light_source[..]).unwrap_or(&[]);
            let changeable = ls.map(|l| &l.changeable_light_source[..]).unwrap_or(&[]);
            let mc = ls.map(|l| &l.multi_channel_light_source[..]).unwrap_or(&[]);
            let mut out = vec![];
            // The selected_row index points into fixed_light_source for
            // editing purposes (matches edit_modal::fixed_light_source_fields).
            for (i, f) in fixed.iter().enumerate() {
                out.push(cursor_line(
                    i == row,
                    format!(
                        "Fixed       · {}  manuf={}",
                        f.id,
                        f.manufacturer.as_deref().unwrap_or("—")
                    ),
                ));
            }
            for c in changeable {
                out.push(Line::from(format!(
                    "  Changeable  · {}  manuf={}",
                    c.id,
                    c.manufacturer.as_deref().unwrap_or("—")
                )));
            }
            for m in mc {
                out.push(Line::from(format!(
                    "  MultiChannel· {}  channels={}",
                    m.id,
                    m.channels.channel.len()
                )));
            }
            if out.is_empty() {
                out.push(muted("(no LightSource blocks declared)"));
            }
            out
        }
        Section::Emitters => {
            let es = product
                .general_definitions
                .emitters
                .as_ref()
                .map(|e| &e.emitter[..])
                .unwrap_or(&[]);
            if es.is_empty() {
                vec![muted("(no Emitter blocks declared)")]
            } else {
                es.iter()
                    .enumerate()
                    .map(|(i, e)| {
                        cursor_line(
                            i == row,
                            format!(
                                "{}  fixed={} chg={} mc={} sensor={}",
                                e.id,
                                e.fixed_light_emitter.len(),
                                e.changeable_light_emitter.len(),
                                e.multi_channel_light_emitter.len(),
                                e.sensor_emitter.len()
                            ),
                        )
                    })
                    .collect()
            }
        }
        Section::Variants => {
            let vs = product
                .product_definitions
                .variants
                .as_ref()
                .map(|v| &v.variant[..])
                .unwrap_or(&[]);
            if vs.is_empty() {
                vec![muted("(no Variants declared)")]
            } else {
                vs.iter()
                    .enumerate()
                    .map(|(i, v)| cursor_line(i == row, v.id.to_string()))
                    .collect()
            }
        }
        Section::Files => {
            let files = &product.general_definitions.files.file;
            if files.is_empty() {
                vec![muted("(no embedded files declared)")]
            } else {
                files
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        cursor_line(
                            i == row,
                            format!("{}  [{}]  {}", f.id, f.content_type, f.file_name),
                        )
                    })
                    .collect()
            }
        }
        Section::Linkage => {
            // Minimal linkage summary; full per-section orphan-check
            // lands in a later cut.
            vec![
                Line::from(format!(
                    "Files declared: {}",
                    product.general_definitions.files.file.len()
                )),
                Line::from(format!("Embedded bytes: {}", app.state.embedded_files.len())),
                Line::from(format!(
                    "Photometries:    {}",
                    product
                        .general_definitions
                        .photometries
                        .as_ref()
                        .map(|p| p.photometry.len())
                        .unwrap_or(0)
                )),
                Line::from(format!(
                    "Spectra:         {}",
                    product
                        .general_definitions
                        .spectrums
                        .as_ref()
                        .map(|s| s.spectrum.len())
                        .unwrap_or(0)
                )),
                muted("(full linkage diagram in a follow-up cut)"),
            ]
        }
    }
}

fn kv(label: &str, value: &str) -> Line<'static> {
    let value = if value.is_empty() { "—".to_string() } else { value.to_string() };
    Line::from(vec![
        Span::styled(
            format!("{:18}", label),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(value),
    ])
}

fn cursor_line(selected: bool, text: String) -> Line<'static> {
    let marker = if selected { "▸ " } else { "  " };
    let style = if selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    Line::from(vec![
        Span::raw(marker.to_string()),
        Span::styled(text, style),
    ])
}

fn muted(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    ))
}
