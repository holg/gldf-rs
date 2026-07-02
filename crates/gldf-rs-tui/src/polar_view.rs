//! Polar photometric diagram renderer for the TUI's Photometry pane.
//! Lifted from bimifc-viewer-tui's viewport; depends only on
//! `eulumdat::diagram::PolarDiagram` so any frontend that has the
//! bytes can render this.

use eulumdat::{diagram::PolarDiagram, Eulumdat};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    symbols::Marker,
    text::Line,
    widgets::{
        canvas::{Canvas, Line as CanvasLine},
        Block, Borders,
    },
    Frame,
};

/// Try parsing `bytes` as EULUMDAT text (UTF-8 first, Latin-1 fallback)
/// and return a renderable PolarDiagram. Returns the luminaire name as
/// the second element for use in the panel title.
pub fn parse_eulumdat(bytes: &[u8]) -> Option<(Eulumdat, PolarDiagram)> {
    let text = std::str::from_utf8(bytes)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect());
    let ldt = Eulumdat::parse(&text).ok()?;
    let polar = PolarDiagram::from_eulumdat(&ldt);
    Some((ldt, polar))
}

/// Render a polar diagram in `area`. If `bytes` is None or unparseable,
/// renders a placeholder block so the caller doesn't have to.
pub fn draw(frame: &mut Frame, area: Rect, bytes: Option<&[u8]>, title_prefix: &str) {
    let (ldt, polar) = match bytes.and_then(parse_eulumdat) {
        Some(x) => x,
        None => {
            let title = if bytes.is_some() {
                format!(" {} (could not parse) ", title_prefix)
            } else {
                format!(" {} (no LDT/IES data) ", title_prefix)
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title);
            frame.render_widget(block, area);
            return;
        }
    };

    let scale_max = polar.scale.scale_max;
    let bound = scale_max * 1.15;
    let max_intensity = polar.scale.max_intensity;
    let luminaire_name = truncate(&ldt.luminaire_name, 30);
    let title = format!(
        " {} · {} ({:.0} cd max) ",
        title_prefix, luminaire_name, max_intensity
    );

    // Pre-collect points (paint closure must own its data).
    let c0_points: Vec<(f64, f64)> = polar
        .c0_c180_curve
        .points
        .iter()
        .map(|p| (p.x, -p.y))
        .collect();
    let c90_points: Vec<(f64, f64)> = polar
        .c90_c270_curve
        .points
        .iter()
        .map(|p| (p.x, -p.y))
        .collect();
    let show_c90 = polar.show_c90_c270();
    let grid_values = polar.scale.grid_values.clone();

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title),
        )
        .marker(Marker::Braille)
        .x_bounds([-bound, bound])
        .y_bounds([-bound, bound])
        .paint(move |ctx| {
            // Grid circles
            for &grid_val in &grid_values {
                let steps = 72;
                for i in 0..steps {
                    let a0 = i as f64 * std::f64::consts::TAU / steps as f64;
                    let a1 = (i + 1) as f64 * std::f64::consts::TAU / steps as f64;
                    ctx.draw(&CanvasLine {
                        x1: grid_val * a0.cos(),
                        y1: grid_val * a0.sin(),
                        x2: grid_val * a1.cos(),
                        y2: grid_val * a1.sin(),
                        color: Color::Rgb(40, 40, 55),
                    });
                }
            }

            // Axes
            ctx.draw(&CanvasLine {
                x1: -scale_max,
                y1: 0.0,
                x2: scale_max,
                y2: 0.0,
                color: Color::Rgb(60, 60, 80),
            });
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: -scale_max,
                x2: 0.0,
                y2: scale_max,
                color: Color::Rgb(60, 60, 80),
            });

            // C0-C180 (blue)
            for w in c0_points.windows(2) {
                ctx.draw(&CanvasLine {
                    x1: w[0].0,
                    y1: w[0].1,
                    x2: w[1].0,
                    y2: w[1].1,
                    color: Color::Rgb(80, 140, 255),
                });
            }
            // C90-C270 (red)
            if show_c90 {
                for w in c90_points.windows(2) {
                    ctx.draw(&CanvasLine {
                        x1: w[0].0,
                        y1: w[0].1,
                        x2: w[1].0,
                        y2: w[1].1,
                        color: Color::Rgb(255, 100, 100),
                    });
                }
            }

            // Angle labels
            let dim = Style::default().fg(Color::DarkGray);
            ctx.print(scale_max * 0.05, bound * 0.9, Line::styled("0°", dim));
            ctx.print(scale_max * 0.05, -bound * 0.9, Line::styled("180°", dim));
            ctx.print(bound * 0.8, 0.0, Line::styled("90°", dim));

            // Legend
            ctx.print(
                -bound * 0.95,
                -bound * 0.85,
                Line::styled("C0-C180", Style::default().fg(Color::Rgb(80, 140, 255))),
            );
            if show_c90 {
                ctx.print(
                    -bound * 0.95,
                    -bound * 0.95,
                    Line::styled("C90-C270", Style::default().fg(Color::Rgb(255, 100, 100))),
                );
            }
        });

    frame.render_widget(canvas, area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}
