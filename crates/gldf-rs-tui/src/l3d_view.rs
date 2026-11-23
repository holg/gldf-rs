//! Iso-3D wireframe viewer for L3D files. Ports the renderer shape from
//! bimifc-viewer-tui's `render_iso3d`: parses the .l3d archive via
//! `l3d_rs::from_buffer`, parses every referenced OBJ via `tobj`, applies
//! each part's transformation matrix, then projects triangle edges onto
//! the braille canvas with a configurable orbit camera.

use l3d_rs::from_buffer;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    symbols::Marker,
    widgets::{
        canvas::{Canvas, Line as CanvasLine},
        Block, Borders,
    },
    Frame,
};

/// 3D vector (no extra dep — we don't need glam for a wireframe).
#[derive(Clone, Copy, Default)]
struct V3 {
    x: f32,
    y: f32,
    z: f32,
}

impl V3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

/// Apply a column-major 4x4 matrix to a point (w = 1.0).
fn mat4_mul_point(m: &[f32; 16], p: V3) -> V3 {
    V3 {
        x: m[0] * p.x + m[4] * p.y + m[8] * p.z + m[12],
        y: m[1] * p.x + m[5] * p.y + m[9] * p.z + m[13],
        z: m[2] * p.x + m[6] * p.y + m[10] * p.z + m[14],
    }
}

/// Orbit-camera angles. Step controls how much one keypress rotates.
#[derive(Clone, Copy, Debug)]
pub struct OrbitAngles {
    pub azimuth: f64,
    pub elevation: f64,
}

impl Default for OrbitAngles {
    fn default() -> Self {
        Self {
            azimuth: std::f64::consts::FRAC_PI_6,
            elevation: std::f64::consts::FRAC_PI_6,
        }
    }
}

impl OrbitAngles {
    const STEP: f64 = 0.15;
    pub fn rotate_left(&mut self) {
        self.azimuth -= Self::STEP;
    }
    pub fn rotate_right(&mut self) {
        self.azimuth += Self::STEP;
    }
    pub fn rotate_up(&mut self) {
        self.elevation = (self.elevation + Self::STEP).min(std::f64::consts::FRAC_PI_2 - 0.05);
    }
    pub fn rotate_down(&mut self) {
        self.elevation = (self.elevation - Self::STEP).max(-std::f64::consts::FRAC_PI_2 + 0.05);
    }
}

/// One frame of geometry extracted from an L3D archive.
pub struct L3dScene {
    /// Triangle edges as `(p0, p1)` pairs in world space.
    edges: Vec<(V3, V3)>,
    bounds_min: V3,
    bounds_max: V3,
    part_count: usize,
}

impl L3dScene {
    /// Parse L3D bytes (a .l3d zip) and extract a wireframe scene.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let l3d = from_buffer(bytes);
        if l3d.model.parts.is_empty() || l3d.file.assets.is_empty() {
            return None;
        }
        let mut edges = Vec::new();
        let mut bmin = V3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut bmax = V3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

        for part in &l3d.model.parts {
            // Find the matching OBJ asset; tobj parses OBJ text + the
            // .l3d archive may also contain MTL files we can ignore.
            let Some(asset) = l3d
                .file
                .assets
                .iter()
                .find(|a| a.name == part.path && a.name.to_lowercase().ends_with(".obj"))
            else {
                continue;
            };
            // tobj wants a BufRead; wrap the bytes.
            let mut cursor = std::io::Cursor::new(&asset.content);
            let load_opts = tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ignore_points: true,
                ignore_lines: true,
            };
            let Ok((models, _)) = tobj::load_obj_buf(
                &mut cursor,
                &load_opts,
                |_| Err(tobj::LoadError::OpenFileFailed), // skip MTL refs
            ) else {
                continue;
            };

            for model in models {
                let positions = &model.mesh.positions;
                let indices = &model.mesh.indices;
                for tri in indices.as_chunks::<3>().0 {
                    let p: [V3; 3] = std::array::from_fn(|k| {
                        let i = tri[k] as usize * 3;
                        let local = V3::new(positions[i], positions[i + 1], positions[i + 2]);
                        mat4_mul_point(&part.mat, local)
                    });
                    for v in &p {
                        bmin = V3::new(bmin.x.min(v.x), bmin.y.min(v.y), bmin.z.min(v.z));
                        bmax = V3::new(bmax.x.max(v.x), bmax.y.max(v.y), bmax.z.max(v.z));
                    }
                    edges.push((p[0], p[1]));
                    edges.push((p[1], p[2]));
                    edges.push((p[2], p[0]));
                }
            }
        }

        if edges.is_empty() {
            return None;
        }
        Some(Self {
            edges,
            bounds_min: bmin,
            bounds_max: bmax,
            part_count: l3d.model.parts.len(),
        })
    }
}

/// Orbit-project a 3D point to 2D screen coords.
fn orbit_project(v: V3, angles: &OrbitAngles) -> (f64, f64) {
    let (x, y, z) = (v.x as f64, v.y as f64, v.z as f64);
    let (sa, ca) = angles.azimuth.sin_cos();
    let (se, ce) = angles.elevation.sin_cos();
    let rx = x * ca + z * sa;
    let rz = -x * sa + z * ca;
    let sx = rx;
    let sy = y * ce - rz * se;
    (sx, sy)
}

/// Render the L3D wireframe in `area`. Renders a placeholder block if
/// the bytes don't parse.
pub fn draw(
    frame: &mut Frame,
    area: Rect,
    bytes: Option<&[u8]>,
    orbit: &OrbitAngles,
    title_prefix: &str,
) {
    let scene = match bytes.and_then(L3dScene::from_bytes) {
        Some(s) => s,
        None => {
            let title = if bytes.is_some() {
                format!(" {} (could not parse) ", title_prefix)
            } else {
                format!(" {} (no L3D data) ", title_prefix)
            };
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title);
            frame.render_widget(block, area);
            return;
        }
    };

    let center = V3::new(
        (scene.bounds_min.x + scene.bounds_max.x) * 0.5,
        (scene.bounds_min.y + scene.bounds_max.y) * 0.5,
        (scene.bounds_min.z + scene.bounds_max.z) * 0.5,
    );
    let project = |v: V3| orbit_project(v.sub(center), orbit);

    // Compute projected bounds from the eight bbox corners.
    let corners = [
        V3::new(scene.bounds_min.x, scene.bounds_min.y, scene.bounds_min.z),
        V3::new(scene.bounds_max.x, scene.bounds_min.y, scene.bounds_min.z),
        V3::new(scene.bounds_min.x, scene.bounds_max.y, scene.bounds_min.z),
        V3::new(scene.bounds_max.x, scene.bounds_max.y, scene.bounds_min.z),
        V3::new(scene.bounds_min.x, scene.bounds_min.y, scene.bounds_max.z),
        V3::new(scene.bounds_max.x, scene.bounds_min.y, scene.bounds_max.z),
        V3::new(scene.bounds_min.x, scene.bounds_max.y, scene.bounds_max.z),
        V3::new(scene.bounds_max.x, scene.bounds_max.y, scene.bounds_max.z),
    ];
    let mut x_min = f64::MAX;
    let mut x_max = f64::MIN;
    let mut y_min = f64::MAX;
    let mut y_max = f64::MIN;
    for c in &corners {
        let (px, py) = project(*c);
        x_min = x_min.min(px);
        x_max = x_max.max(px);
        y_min = y_min.min(py);
        y_max = y_max.max(py);
    }
    let dx = (x_max - x_min).max(1.0) * 0.05;
    let dy = (y_max - y_min).max(1.0) * 0.05;
    x_min -= dx;
    x_max += dx;
    y_min -= dy;
    y_max += dy;

    let projected: Vec<(f64, f64, f64, f64)> = scene
        .edges
        .iter()
        .map(|(a, b)| {
            let (ax, ay) = project(*a);
            let (bx, by) = project(*b);
            (ax, ay, bx, by)
        })
        .collect();
    let edge_count = projected.len();
    let part_count = scene.part_count;
    let az_deg = orbit.azimuth.to_degrees();
    let el_deg = orbit.elevation.to_degrees();

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(
                    " {} · {} parts · {} edges · az:{:.0}° el:{:.0}° ",
                    title_prefix, part_count, edge_count, az_deg, el_deg
                )),
        )
        .marker(Marker::Braille)
        .x_bounds([x_min, x_max])
        .y_bounds([y_min, y_max])
        .paint(move |ctx| {
            for &(x0, y0, x1, y1) in &projected {
                ctx.draw(&CanvasLine {
                    x1: x0,
                    y1: y0,
                    x2: x1,
                    y2: y1,
                    color: Color::Rgb(180, 180, 200),
                });
            }
        });

    frame.render_widget(canvas, area);
}
