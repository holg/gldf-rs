//! GLDF IFC Viewer - Simple IFC/STEP file visualization
//!
//! A lightweight WASM plugin for viewing IFC files exported from GLDF.
//! Focuses on light fixtures and their placement.
//!
//! Features:
//! - Parse IFC STEP format
//! - Extract light fixture entities
//! - 2D floor plan visualization
//! - Entity property display

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// Log to browser console
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format!($($t)*)))
}

// ============================================================================
// IFC Entity Types
// ============================================================================

/// Parsed IFC entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcEntity {
    pub id: u64,
    pub entity_type: String,
    pub attributes: Vec<String>,
}

/// Light fixture data extracted from IFC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightFixture {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub position: (f64, f64, f64),
    pub fixture_type: String,
    pub properties: HashMap<String, String>,
}

/// Building storey data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingStorey {
    pub id: u64,
    pub name: String,
    pub elevation: f64,
}

/// Parsed IFC file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcFile {
    pub schema: String,
    pub description: String,
    pub entities: Vec<IfcEntity>,
    pub light_fixtures: Vec<LightFixture>,
    pub storeys: Vec<BuildingStorey>,
    pub project_name: String,
    pub site_name: String,
    pub building_name: String,
}

impl Default for IfcFile {
    fn default() -> Self {
        Self {
            schema: String::new(),
            description: String::new(),
            entities: Vec::new(),
            light_fixtures: Vec::new(),
            storeys: Vec::new(),
            project_name: String::new(),
            site_name: String::new(),
            building_name: String::new(),
        }
    }
}

// ============================================================================
// STEP Parser
// ============================================================================

/// Parse an IFC STEP file
pub fn parse_ifc(content: &str) -> IfcFile {
    let mut ifc = IfcFile::default();
    let mut entities: HashMap<u64, IfcEntity> = HashMap::new();

    let mut in_data = false;

    for line in content.lines() {
        let line = line.trim();

        // Parse header
        if line.starts_with("FILE_SCHEMA") {
            if let Some(schema) = extract_string_param(line) {
                ifc.schema = schema;
            }
        } else if line.starts_with("FILE_DESCRIPTION") {
            if let Some(desc) = extract_string_param(line) {
                ifc.description = desc;
            }
        } else if line == "DATA;" {
            in_data = true;
        } else if line == "ENDSEC;" {
            in_data = false;
        } else if in_data && line.starts_with('#') {
            // Parse entity: #123=IFCENTITY(...)
            if let Some(entity) = parse_entity_line(line) {
                entities.insert(entity.id, entity);
            }
        }
    }

    // Extract meaningful data from entities
    for entity in entities.values() {
        match entity.entity_type.as_str() {
            "IFCPROJECT" => {
                if let Some(name) = entity.attributes.get(2) {
                    ifc.project_name = clean_string(name);
                }
            }
            "IFCSITE" => {
                if let Some(name) = entity.attributes.get(2) {
                    ifc.site_name = clean_string(name);
                }
            }
            "IFCBUILDING" => {
                if let Some(name) = entity.attributes.get(2) {
                    ifc.building_name = clean_string(name);
                }
            }
            "IFCBUILDINGSTOREY" => {
                let name = entity
                    .attributes
                    .get(2)
                    .map(|s| clean_string(s))
                    .unwrap_or_default();
                let elevation = entity
                    .attributes
                    .get(9)
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                ifc.storeys.push(BuildingStorey {
                    id: entity.id,
                    name,
                    elevation,
                });
            }
            "IFCLIGHTFIXTURE" => {
                let name = entity
                    .attributes
                    .get(2)
                    .map(|s| clean_string(s))
                    .unwrap_or_default();
                let description = entity
                    .attributes
                    .get(3)
                    .map(|s| clean_string(s))
                    .unwrap_or_default();

                // Try to find position from placement
                let position = if let Some(placement_ref) = entity.attributes.get(5) {
                    find_position(&entities, placement_ref)
                } else {
                    (0.0, 0.0, 0.0)
                };

                ifc.light_fixtures.push(LightFixture {
                    id: entity.id,
                    name,
                    description,
                    position,
                    fixture_type: "LIGHTFIXTURE".to_string(),
                    properties: HashMap::new(),
                });
            }
            _ => {}
        }
    }

    ifc.entities = entities.into_values().collect();
    ifc
}

/// Parse a single entity line
fn parse_entity_line(line: &str) -> Option<IfcEntity> {
    // Format: #123=IFCENTITY(attr1,attr2,...);
    let line = line.trim_end_matches(';');

    let eq_pos = line.find('=')?;
    let id_str = line[1..eq_pos].trim();
    let id: u64 = id_str.parse().ok()?;

    let rest = &line[eq_pos + 1..];
    let paren_pos = rest.find('(')?;
    let entity_type = rest[..paren_pos].trim().to_string();

    let attrs_str = &rest[paren_pos + 1..rest.len() - 1];
    let attributes = parse_attributes(attrs_str);

    Some(IfcEntity {
        id,
        entity_type,
        attributes,
    })
}

/// Parse comma-separated attributes (handling nested parentheses)
fn parse_attributes(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_string = false;

    for c in s.chars() {
        match c {
            '\'' => {
                in_string = !in_string;
                current.push(c);
            }
            '(' if !in_string => {
                depth += 1;
                current.push(c);
            }
            ')' if !in_string => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 && !in_string => {
                result.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(c),
        }
    }

    if !current.is_empty() {
        result.push(current.trim().to_string());
    }

    result
}

/// Extract string parameter from IFC line
fn extract_string_param(line: &str) -> Option<String> {
    let start = line.find('\'')?;
    let end = line.rfind('\'')?;
    if start < end {
        Some(line[start + 1..end].to_string())
    } else {
        None
    }
}

/// Clean a string value (remove quotes, handle $)
fn clean_string(s: &str) -> String {
    if s == "$" {
        return String::new();
    }
    s.trim_matches('\'').to_string()
}

/// Find position from placement reference
fn find_position(entities: &HashMap<u64, IfcEntity>, placement_ref: &str) -> (f64, f64, f64) {
    // Try to extract ID from reference like #123
    let id = placement_ref
        .trim_start_matches('#')
        .parse::<u64>()
        .ok();

    if let Some(id) = id {
        if let Some(entity) = entities.get(&id) {
            // IFCLOCALPLACEMENT has relative placement at index 1
            if entity.entity_type == "IFCLOCALPLACEMENT" {
                if let Some(rel_ref) = entity.attributes.get(1) {
                    return find_position_from_axis(entities, rel_ref);
                }
            }
        }
    }

    (0.0, 0.0, 0.0)
}

/// Extract position from axis placement
fn find_position_from_axis(entities: &HashMap<u64, IfcEntity>, axis_ref: &str) -> (f64, f64, f64) {
    let id = axis_ref.trim_start_matches('#').parse::<u64>().ok();

    if let Some(id) = id {
        if let Some(entity) = entities.get(&id) {
            // IFCAXIS2PLACEMENT3D has location at index 0
            if entity.entity_type == "IFCAXIS2PLACEMENT3D" {
                if let Some(point_ref) = entity.attributes.first() {
                    return find_cartesian_point(entities, point_ref);
                }
            }
        }
    }

    (0.0, 0.0, 0.0)
}

/// Extract coordinates from cartesian point
fn find_cartesian_point(entities: &HashMap<u64, IfcEntity>, point_ref: &str) -> (f64, f64, f64) {
    let id = point_ref.trim_start_matches('#').parse::<u64>().ok();

    if let Some(id) = id {
        if let Some(entity) = entities.get(&id) {
            if entity.entity_type == "IFCCARTESIANPOINT" {
                // Coordinates are in format: (x.,y.,z.)
                if let Some(coords_str) = entity.attributes.first() {
                    let coords = coords_str
                        .trim_start_matches('(')
                        .trim_end_matches(')')
                        .split(',')
                        .filter_map(|s| s.trim().trim_end_matches('.').parse::<f64>().ok())
                        .collect::<Vec<_>>();

                    if coords.len() >= 3 {
                        return (coords[0], coords[1], coords[2]);
                    }
                }
            }
        }
    }

    (0.0, 0.0, 0.0)
}

// ============================================================================
// WASM Renderer
// ============================================================================

/// IFC Viewer - renders IFC data on a canvas
#[wasm_bindgen]
pub struct IfcViewer {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    ifc: Option<IfcFile>,
    view_mode: ViewMode,
    zoom: f64,
    pan_x: f64,
    pan_y: f64,
}

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    FloorPlan,
    EntityList,
}

#[wasm_bindgen]
impl IfcViewer {
    /// Create a new IFC viewer
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str) -> Result<IfcViewer, JsValue> {
        console_log!("[IFC Viewer] Initializing on canvas: {}", canvas_id);

        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;

        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas not found")?
            .dyn_into::<HtmlCanvasElement>()?;

        let ctx = canvas
            .get_context("2d")?
            .ok_or("No 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()?;

        Ok(IfcViewer {
            canvas,
            ctx,
            ifc: None,
            view_mode: ViewMode::FloorPlan,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
        })
    }

    /// Load IFC content from string
    #[wasm_bindgen]
    pub fn load_ifc(&mut self, content: &str) -> Result<usize, JsValue> {
        console_log!("[IFC Viewer] Parsing IFC, {} bytes", content.len());

        let ifc = parse_ifc(content);

        console_log!(
            "[IFC Viewer] Parsed: {} entities, {} fixtures, {} storeys",
            ifc.entities.len(),
            ifc.light_fixtures.len(),
            ifc.storeys.len()
        );

        let fixture_count = ifc.light_fixtures.len();
        self.ifc = Some(ifc);

        Ok(fixture_count)
    }

    /// Load from localStorage
    #[wasm_bindgen]
    pub fn load_from_storage(&mut self, key: &str) -> Result<usize, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window.local_storage()?.ok_or("No localStorage")?;

        if let Some(content) = storage.get_item(key)? {
            self.load_ifc(&content)
        } else {
            Err(JsValue::from_str("No IFC data in storage"))
        }
    }

    /// Toggle view mode
    #[wasm_bindgen]
    pub fn toggle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::FloorPlan => ViewMode::EntityList,
            ViewMode::EntityList => ViewMode::FloorPlan,
        };
        self.render();
    }

    /// Zoom in
    #[wasm_bindgen]
    pub fn zoom_in(&mut self) {
        self.zoom *= 1.2;
        self.render();
    }

    /// Zoom out
    #[wasm_bindgen]
    pub fn zoom_out(&mut self) {
        self.zoom /= 1.2;
        self.render();
    }

    /// Render the view
    #[wasm_bindgen]
    pub fn render(&self) {
        let w = self.canvas.width() as f64;
        let h = self.canvas.height() as f64;

        // Clear canvas
        self.ctx.set_fill_style_str("#1a1a2e");
        self.ctx.fill_rect(0.0, 0.0, w, h);

        let Some(ref ifc) = self.ifc else {
            self.draw_no_data(w, h);
            return;
        };

        match self.view_mode {
            ViewMode::FloorPlan => self.draw_floor_plan(ifc, w, h),
            ViewMode::EntityList => self.draw_entity_list(ifc, w, h),
        }
    }

    /// Draw "no data" message
    fn draw_no_data(&self, w: f64, h: f64) {
        self.ctx.set_fill_style_str("#666666");
        self.ctx.set_font("16px sans-serif");
        self.ctx.set_text_align("center");
        self.ctx
            .fill_text("No IFC data loaded", w / 2.0, h / 2.0)
            .ok();
    }

    /// Draw floor plan view
    fn draw_floor_plan(&self, ifc: &IfcFile, w: f64, h: f64) {
        let cx = w / 2.0 + self.pan_x;
        let cy = h / 2.0 + self.pan_y;

        // Draw title
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_font("bold 14px sans-serif");
        self.ctx.set_text_align("left");
        let title = if !ifc.project_name.is_empty() {
            format!("Project: {}", ifc.project_name)
        } else {
            "IFC Floor Plan".to_string()
        };
        self.ctx.fill_text(&title, 10.0, 25.0).ok();

        // Draw grid
        self.draw_grid(w, h, cx, cy);

        // Draw light fixtures
        for fixture in &ifc.light_fixtures {
            self.draw_fixture(fixture, cx, cy);
        }

        // Draw legend
        self.draw_legend(w, h, ifc);
    }

    /// Draw grid
    fn draw_grid(&self, w: f64, h: f64, cx: f64, cy: f64) {
        let grid_size = 50.0 * self.zoom;

        self.ctx.set_stroke_style_str("#2a2a4e");
        self.ctx.set_line_width(0.5);

        // Vertical lines
        let mut x = cx % grid_size;
        while x < w {
            self.ctx.begin_path();
            self.ctx.move_to(x, 0.0);
            self.ctx.line_to(x, h);
            self.ctx.stroke();
            x += grid_size;
        }

        // Horizontal lines
        let mut y = cy % grid_size;
        while y < h {
            self.ctx.begin_path();
            self.ctx.move_to(0.0, y);
            self.ctx.line_to(w, y);
            self.ctx.stroke();
            y += grid_size;
        }

        // Draw axes
        self.ctx.set_stroke_style_str("#4a4a6e");
        self.ctx.set_line_width(1.0);

        // X axis
        self.ctx.begin_path();
        self.ctx.move_to(0.0, cy);
        self.ctx.line_to(w, cy);
        self.ctx.stroke();

        // Y axis
        self.ctx.begin_path();
        self.ctx.move_to(cx, 0.0);
        self.ctx.line_to(cx, h);
        self.ctx.stroke();
    }

    /// Draw a light fixture
    fn draw_fixture(&self, fixture: &LightFixture, cx: f64, cy: f64) {
        let scale = 10.0 * self.zoom; // 1 meter = 10 pixels at zoom 1.0
        let x = cx + fixture.position.0 * scale;
        let y = cy - fixture.position.1 * scale; // Flip Y for screen coords

        let size = 8.0 * self.zoom;

        // Draw fixture symbol (light bulb icon)
        // Glow effect
        if let Ok(gradient) = self.ctx.create_radial_gradient(x, y, 0.0, x, y, size * 3.0) {
            gradient.add_color_stop(0.0, "#ffdd44").ok();
            gradient.add_color_stop(0.5, "#ffdd4444").ok();
            gradient.add_color_stop(1.0, "transparent").ok();
            self.ctx.set_fill_style_canvas_gradient(&gradient);
            self.ctx.begin_path();
            self.ctx.arc(x, y, size * 3.0, 0.0, PI * 2.0).ok();
            self.ctx.fill();
        }

        // Fixture body
        self.ctx.set_fill_style_str("#ffdd44");
        self.ctx.begin_path();
        self.ctx.arc(x, y, size, 0.0, PI * 2.0).ok();
        self.ctx.fill();

        // Fixture outline
        self.ctx.set_stroke_style_str("#ffffff");
        self.ctx.set_line_width(1.5);
        self.ctx.begin_path();
        self.ctx.arc(x, y, size, 0.0, PI * 2.0).ok();
        self.ctx.stroke();

        // Label
        if !fixture.name.is_empty() {
            self.ctx.set_fill_style_str("#cccccc");
            self.ctx.set_font("10px sans-serif");
            self.ctx.set_text_align("center");
            self.ctx.fill_text(&fixture.name, x, y + size + 12.0).ok();
        }
    }

    /// Draw legend
    fn draw_legend(&self, w: f64, h: f64, ifc: &IfcFile) {
        let x = w - 150.0;
        let y = h - 80.0;

        // Background
        self.ctx.set_fill_style_str("#1a1a2e99");
        self.ctx.fill_rect(x - 10.0, y - 20.0, 150.0, 90.0);

        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_font("12px sans-serif");
        self.ctx.set_text_align("left");

        self.ctx.fill_text("Legend:", x, y).ok();

        // Fixture symbol
        self.ctx.set_fill_style_str("#ffdd44");
        self.ctx.begin_path();
        self.ctx.arc(x + 8.0, y + 20.0, 6.0, 0.0, PI * 2.0).ok();
        self.ctx.fill();

        self.ctx.set_fill_style_str("#cccccc");
        self.ctx
            .fill_text(&format!("Light Fixtures ({})", ifc.light_fixtures.len()), x + 20.0, y + 24.0)
            .ok();

        // Scale
        self.ctx.set_fill_style_str("#888888");
        self.ctx
            .fill_text(&format!("Zoom: {:.0}%", self.zoom * 100.0), x, y + 50.0)
            .ok();
    }

    /// Draw entity list view
    fn draw_entity_list(&self, ifc: &IfcFile, w: f64, h: f64) {
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.set_font("bold 14px sans-serif");
        self.ctx.set_text_align("left");
        self.ctx.fill_text("IFC Entity Summary", 10.0, 25.0).ok();

        self.ctx.set_font("12px monospace");
        let mut y = 50.0;
        let line_height = 18.0;

        // Schema
        self.ctx.set_fill_style_str("#88ccff");
        self.ctx
            .fill_text(&format!("Schema: {}", ifc.schema), 10.0, y)
            .ok();
        y += line_height;

        // Project info
        if !ifc.project_name.is_empty() {
            self.ctx.set_fill_style_str("#aaffaa");
            self.ctx
                .fill_text(&format!("Project: {}", ifc.project_name), 10.0, y)
                .ok();
            y += line_height;
        }

        if !ifc.building_name.is_empty() {
            self.ctx
                .fill_text(&format!("Building: {}", ifc.building_name), 10.0, y)
                .ok();
            y += line_height;
        }

        y += line_height / 2.0;

        // Entity counts
        self.ctx.set_fill_style_str("#cccccc");
        self.ctx
            .fill_text(&format!("Total Entities: {}", ifc.entities.len()), 10.0, y)
            .ok();
        y += line_height;

        self.ctx
            .fill_text(&format!("Building Storeys: {}", ifc.storeys.len()), 10.0, y)
            .ok();
        y += line_height;

        self.ctx.set_fill_style_str("#ffdd44");
        self.ctx
            .fill_text(&format!("Light Fixtures: {}", ifc.light_fixtures.len()), 10.0, y)
            .ok();
        y += line_height * 1.5;

        // List fixtures
        if !ifc.light_fixtures.is_empty() {
            self.ctx.set_fill_style_str("#ffffff");
            self.ctx.fill_text("Fixtures:", 10.0, y).ok();
            y += line_height;

            for (i, fixture) in ifc.light_fixtures.iter().enumerate() {
                if y > h - 20.0 {
                    self.ctx
                        .fill_text(&format!("... and {} more", ifc.light_fixtures.len() - i), 20.0, y)
                        .ok();
                    break;
                }

                self.ctx.set_fill_style_str("#aaaaaa");
                let pos = format!(
                    "({:.1}, {:.1}, {:.1})",
                    fixture.position.0, fixture.position.1, fixture.position.2
                );
                let name = if fixture.name.is_empty() {
                    format!("#{}", fixture.id)
                } else {
                    fixture.name.clone()
                };
                self.ctx
                    .fill_text(&format!("  {} @ {}", name, pos), 20.0, y)
                    .ok();
                y += line_height;
            }
        }
    }

    /// Get IFC info as JSON
    #[wasm_bindgen]
    pub fn get_info_json(&self) -> String {
        if let Some(ref ifc) = self.ifc {
            serde_json::json!({
                "schema": ifc.schema,
                "project": ifc.project_name,
                "building": ifc.building_name,
                "entities": ifc.entities.len(),
                "fixtures": ifc.light_fixtures.len(),
                "storeys": ifc.storeys.len()
            })
            .to_string()
        } else {
            "{}".to_string()
        }
    }
}

/// Convenience function to render from storage
#[wasm_bindgen]
pub fn render_ifc_from_storage(canvas_id: &str, storage_key: &str) -> Result<IfcViewer, JsValue> {
    let mut viewer = IfcViewer::new(canvas_id)?;
    viewer.load_from_storage(storage_key)?;
    viewer.render();
    Ok(viewer)
}

#[wasm_bindgen(start)]
pub fn main() {
    console_log!("[IFC Viewer] WASM module loaded");
}
