//! Polar intensity diagram painter for `anyrender` scenes.
//!
//! Renderer-agnostic: records into an [`anyrender::Scene`], which any
//! anyrender backend can rasterize — Vello (GPU, what blitz-shell uses),
//! vello_cpu (software, used by our PNG test), or vello_hybrid.
//!
//! Geometry comes entirely from `eulumdat::diagram::PolarDiagram` — the same
//! code path as the SVG/egui/Slint frontends in eulumdat-rs, so all symmetry
//! expansion, interpolation and scale logic stays in ONE place upstream.

use anyrender::PaintScene;
use eulumdat::diagram::{DiagramScale, PolarCurve, PolarDiagram};
use eulumdat::Eulumdat;
use peniko::color::AlphaColor;
use peniko::kurbo::{Affine, BezPath, Circle, Line, Point, Stroke};
use peniko::{Color, Fill};

/// Visual style for the polar diagram. Defaults follow the classic
/// datasheet look: solid blue C0–C180, dashed red C90–C270.
#[derive(Debug, Clone)]
pub struct PolarStyle {
    pub background: Option<Color>,
    pub grid: Color,
    pub axis: Color,
    pub c0_c180: Color,
    pub c90_c270: Color,
    pub curve_width: f64,
    pub grid_width: f64,
    pub fill_alpha: f32,
}

impl Default for PolarStyle {
    fn default() -> Self {
        Self {
            background: None, // transparent: let CSS background show through
            grid: Color::from_rgb8(0xc8, 0xcd, 0xd4),
            axis: Color::from_rgb8(0x8a, 0x92, 0x9c),
            c0_c180: Color::from_rgb8(0x1f, 0x77, 0xb4),
            c90_c270: Color::from_rgb8(0xd6, 0x27, 0x28),
            curve_width: 2.0,
            grid_width: 1.0,
            fill_alpha: 0.12,
        }
    }
}

impl PolarStyle {
    /// Dark-theme variant (matches gpui-component / shadcn dark palettes
    /// closely enough for a demo).
    pub fn dark() -> Self {
        Self {
            background: None,
            grid: Color::from_rgb8(0x3a, 0x40, 0x4a),
            axis: Color::from_rgb8(0x5c, 0x64, 0x70),
            c0_c180: Color::from_rgb8(0x60, 0xa5, 0xfa),
            c90_c270: Color::from_rgb8(0xf8, 0x71, 0x71),
            ..Self::default()
        }
    }
}

/// Paint a complete polar diagram into `scene`, fitted to `width` x `height`
/// device pixels (`scale` = HiDPI factor, same meaning as
/// `blitz Widget::paint`).
pub fn paint_polar(
    scene: &mut impl PaintScene,
    diagram: &PolarDiagram,
    width: f64,
    height: f64,
    hidpi_scale: f64,
) {
    paint_polar_styled(scene, diagram, width, height, hidpi_scale, &PolarStyle::default())
}

pub fn paint_polar_styled(
    scene: &mut impl PaintScene,
    diagram: &PolarDiagram,
    width: f64,
    height: f64,
    hidpi_scale: f64,
    style: &PolarStyle,
) {
    let w = width;
    let h = height;

    if let Some(bg) = style.background {
        let rect = peniko::kurbo::Rect::new(0.0, 0.0, w, h);
        scene.fill(Fill::NonZero, Affine::IDENTITY, bg, None, &rect);
    }

    // Layout: the polar origin (luminaire position) sits above center because
    // the main lobe points down (gamma 0 = nadir). Reserve a margin for the
    // outer ring. eulumdat's PolarPoint y grows *upward* for upward light,
    // and screen y grows downward, so we mirror y when mapping.
    let margin = 12.0 * hidpi_scale;
    let radius = ((w.min(h)) / 2.0) - margin;
    let cx = w / 2.0;
    // place origin so the downward hemisphere (the usual data) fills nicely
    let cy = h / 2.0;

    // intensity units per pixel — PolarCurve::screen_points divides by this
    let units_per_px = diagram.scale.scale_max / radius;

    paint_grid(scene, &diagram.scale, cx, cy, radius, style, hidpi_scale);

    // C0-C180: solid, filled
    paint_curve(
        scene,
        &diagram.c0_c180_curve,
        cx,
        cy,
        units_per_px,
        style.c0_c180,
        style.curve_width * hidpi_scale,
        style.fill_alpha,
        None,
    );

    // C90-C270: dashed, no fill (classic convention)
    if diagram.show_c90_c270() {
        paint_curve(
            scene,
            &diagram.c90_c270_curve,
            cx,
            cy,
            units_per_px,
            style.c90_c270,
            style.curve_width * hidpi_scale,
            0.0,
            Some(&[6.0 * hidpi_scale, 4.0 * hidpi_scale]),
        );
    }
}

fn paint_grid(
    scene: &mut impl PaintScene,
    scale: &DiagramScale,
    cx: f64,
    cy: f64,
    radius: f64,
    style: &PolarStyle,
    hidpi: f64,
) {
    let grid_stroke = Stroke::new(style.grid_width * hidpi);
    let axis_stroke = Stroke::new(style.grid_width * hidpi);

    // Concentric intensity rings at the nice grid values
    for value in &scale.grid_values {
        let r = radius * (value / scale.scale_max);
        let circle = Circle::new(Point::new(cx, cy), r);
        scene.stroke(&grid_stroke, Affine::IDENTITY, style.grid, None, &circle);
    }

    // Gamma spokes every 30° (0..360), slightly stronger on the vertical axis
    for i in 0..12 {
        let ang = (i as f64) * 30.0_f64.to_radians();
        // gamma 0 points DOWN on screen
        let dx = ang.sin();
        let dy = ang.cos();
        let line = Line::new(
            Point::new(cx, cy),
            Point::new(cx + dx * radius, cy + dy * radius),
        );
        let color = if i % 6 == 0 { style.axis } else { style.grid };
        scene.stroke(&axis_stroke, Affine::IDENTITY, color, None, &line);
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_curve(
    scene: &mut impl PaintScene,
    curve: &PolarCurve,
    cx: f64,
    cy: f64,
    units_per_px: f64,
    color: Color,
    stroke_width: f64,
    fill_alpha: f32,
    dash: Option<&[f64]>,
) {
    if curve.is_empty() {
        return;
    }

    // eulumdat PolarPoint: +y is "up" in luminaire space; screen +y is down.
    // screen_points() adds y/scale directly, so mirror by negating here via
    // manual mapping instead of using screen_points (keeps orientation
    // explicit and matches the SVG renderer output).
    let mut path = BezPath::new();
    let mut first = true;
    for p in &curve.points {
        let sx = cx + p.x / units_per_px;
        let sy = cy - p.y / units_per_px;
        if first {
            path.move_to(Point::new(sx, sy));
            first = false;
        } else {
            path.line_to(Point::new(sx, sy));
        }
    }
    path.close_path();

    if fill_alpha > 0.0 {
        let fill_color: AlphaColor<_> = color.with_alpha(fill_alpha);
        scene.fill(Fill::NonZero, Affine::IDENTITY, fill_color, None, &path);
    }

    let mut stroke = Stroke::new(stroke_width);
    if let Some(dash) = dash {
        stroke = stroke.with_dashes(0.0, dash.iter().copied());
    }
    scene.stroke(&stroke, Affine::IDENTITY, color, None, &path);
}

/// Convenience: parse + default diagram + paint in one call.
pub fn paint_ldt_polar(
    scene: &mut impl PaintScene,
    ldt: &Eulumdat,
    c_plane: Option<f64>,
    width: f64,
    height: f64,
    hidpi_scale: f64,
    style: &PolarStyle,
) {
    let diagram = match c_plane {
        Some(c) => PolarDiagram::from_eulumdat_for_plane(ldt, c),
        None => PolarDiagram::from_eulumdat(ldt),
    };
    paint_polar_styled(scene, &diagram, width, height, hidpi_scale, style);
}
