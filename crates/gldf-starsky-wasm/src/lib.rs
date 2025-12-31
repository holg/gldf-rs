//! GLDF Star Sky WASM - Lightweight 2D Star Visualization
//!
//! A tribute to Astrophysics and the astronomy community.
//!
//! This crate provides a minimal WASM module for rendering a 2D polar projection
//! star map on an HTML canvas. It's designed to be embedded inside GLDF files
//! for self-contained star sky demonstrations.
//!
//! Features:
//! - Polar azimuth-altitude projection
//! - Temperature-based star coloring (blackbody approximation)
//! - Magnitude-based star sizing
//! - Cardinal directions and altitude circles
//! - Glow effects for stars

use serde::{Deserialize, Serialize};
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

/// Star data structure matching the JSON format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Star {
    pub name: String,
    pub ra: f64,
    pub dec: f64,
    pub mag: f64,
    pub temp: f64,
    pub spectral: String,
    pub alt: f64,
    pub az: f64,
}

/// Sky data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkyData {
    pub location: Location,
    /// Time of observation (ISO 8601 format)
    #[serde(alias = "observation_time")]
    pub time: String,
    pub stars: Vec<Star>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub lat: f64,
    pub lng: f64,
}

/// Star Sky Renderer
#[wasm_bindgen]
pub struct StarSkyRenderer {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    stars: Vec<Star>,
    location_name: String,
}

#[wasm_bindgen]
impl StarSkyRenderer {
    /// Create a new renderer attached to a canvas element
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str) -> Result<StarSkyRenderer, JsValue> {
        console_log!("[StarSky] Initializing on canvas: {}", canvas_id);

        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;

        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas not found")?
            .dyn_into::<HtmlCanvasElement>()?;

        let ctx = canvas
            .get_context("2d")?
            .ok_or("Failed to get 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()?;

        Ok(StarSkyRenderer {
            canvas,
            ctx,
            stars: Vec::new(),
            location_name: String::from("Unknown"),
        })
    }

    /// Load star data from JSON string
    #[wasm_bindgen]
    pub fn load_json(&mut self, json: &str) -> Result<usize, JsValue> {
        console_log!("[StarSky] Loading JSON, {} bytes", json.len());

        let data: SkyData = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;

        self.stars = data.stars;
        self.location_name = data.location.name;

        console_log!(
            "[StarSky] Loaded {} stars from {}",
            self.stars.len(),
            self.location_name
        );

        Ok(self.stars.len())
    }

    /// Load star data from localStorage
    #[wasm_bindgen]
    pub fn load_from_storage(&mut self, key: &str) -> Result<usize, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let storage = window
            .local_storage()?
            .ok_or("No localStorage")?;

        if let Some(json) = storage.get_item(key)? {
            self.load_json(&json)
        } else {
            Err(JsValue::from_str("No data in storage"))
        }
    }

    /// Resize canvas to fit parent
    #[wasm_bindgen]
    pub fn resize(&self) {
        if let Some(parent) = self.canvas.parent_element() {
            let rect = parent.get_bounding_client_rect();
            self.canvas.set_width(rect.width() as u32);
            self.canvas.set_height(rect.height() as u32);
        }
    }

    /// Get star count
    #[wasm_bindgen]
    pub fn star_count(&self) -> usize {
        self.stars.len()
    }

    /// Get location name
    #[wasm_bindgen]
    pub fn location(&self) -> String {
        self.location_name.clone()
    }

    /// Render the star sky
    #[wasm_bindgen]
    pub fn render(&self) {
        let w = self.canvas.width() as f64;
        let h = self.canvas.height() as f64;
        let cx = w / 2.0;
        let cy = h / 2.0;
        let radius = (w.min(h) / 2.0) - 40.0;

        // Clear and draw background
        self.ctx.set_fill_style_str("#0a0a1a");
        self.ctx.fill_rect(0.0, 0.0, w, h);

        // Draw horizon circle
        self.ctx.set_stroke_style_str("#2a2a4e");
        self.ctx.set_line_width(2.0);
        self.ctx.begin_path();
        self.ctx.arc(cx, cy, radius, 0.0, PI * 2.0).unwrap();
        self.ctx.stroke();

        // Draw altitude circles (30°, 60°)
        self.ctx.set_stroke_style_str("#1a1a2e");
        self.ctx.set_line_width(1.0);
        for alt in [30.0, 60.0] {
            let r = radius * (1.0 - alt / 90.0);
            self.ctx.begin_path();
            self.ctx.arc(cx, cy, r, 0.0, PI * 2.0).unwrap();
            self.ctx.stroke();
        }

        // Draw cardinal directions
        self.ctx.set_fill_style_str("#666666");
        self.ctx.set_font("12px sans-serif");
        self.ctx.set_text_align("center");
        self.ctx.fill_text("N", cx, cy - radius - 10.0).unwrap();
        self.ctx.fill_text("S", cx, cy + radius + 15.0).unwrap();
        self.ctx.set_text_align("right");
        self.ctx.fill_text("E", cx - radius - 8.0, cy + 4.0).unwrap();
        self.ctx.set_text_align("left");
        self.ctx.fill_text("W", cx + radius + 8.0, cy + 4.0).unwrap();

        // Draw stars
        for star in &self.stars {
            self.draw_star(star, cx, cy, radius);
        }

        // Draw labels for bright stars (mag < 1.5)
        self.ctx.set_fill_style_str("#888888");
        self.ctx.set_font("10px sans-serif");
        self.ctx.set_text_align("left");

        for star in self.stars.iter().filter(|s| s.mag < 1.5) {
            let az_rad = (star.az - 180.0) * PI / 180.0;
            let r = radius * (1.0 - star.alt / 90.0);
            let x = cx + r * az_rad.sin() + 8.0;
            let y = cy - r * az_rad.cos() + 3.0;
            self.ctx.fill_text(&star.name, x, y).unwrap();
        }
    }

    /// Draw a single star with glow effect
    fn draw_star(&self, star: &Star, cx: f64, cy: f64, radius: f64) {
        // Convert alt/az to x/y (azimuth-altitude projection)
        let az_rad = (star.az - 180.0) * PI / 180.0; // N at top
        let r = radius * (1.0 - star.alt / 90.0);
        let x = cx + r * az_rad.sin();
        let y = cy - r * az_rad.cos();

        // Size based on magnitude (brighter = bigger)
        let size = (8.0 - star.mag * 1.5).max(2.0);

        // Color based on temperature
        let color = temp_to_color(star.temp);

        // Draw star with glow (radial gradient)
        if let Ok(gradient) = self.ctx.create_radial_gradient(x, y, 0.0, x, y, size * 2.0) {
            gradient.add_color_stop(0.0, &color).unwrap();
            gradient.add_color_stop(0.5, &format!("{}88", color)).unwrap();
            gradient.add_color_stop(1.0, "transparent").unwrap();

            self.ctx.set_fill_style_canvas_gradient(&gradient);
            self.ctx.begin_path();
            self.ctx.arc(x, y, size * 2.0, 0.0, PI * 2.0).unwrap();
            self.ctx.fill();
        }

        // Draw star core
        self.ctx.set_fill_style_str(&color);
        self.ctx.begin_path();
        self.ctx.arc(x, y, size / 2.0, 0.0, PI * 2.0).unwrap();
        self.ctx.fill();
    }

    /// Highlight a star by name (flash effect)
    #[wasm_bindgen]
    pub fn highlight_star(&self, name: &str) {
        if let Some(star) = self.stars.iter().find(|s| s.name == name) {
            let w = self.canvas.width() as f64;
            let h = self.canvas.height() as f64;
            let cx = w / 2.0;
            let cy = h / 2.0;
            let radius = (w.min(h) / 2.0) - 40.0;

            let az_rad = (star.az - 180.0) * PI / 180.0;
            let r = radius * (1.0 - star.alt / 90.0);
            let x = cx + r * az_rad.sin();
            let y = cy - r * az_rad.cos();

            // Draw highlight circle
            self.ctx.set_stroke_style_str("#ffd700");
            self.ctx.set_line_width(2.0);
            self.ctx.begin_path();
            self.ctx.arc(x, y, 20.0, 0.0, PI * 2.0).unwrap();
            self.ctx.stroke();
        }
    }

    /// Get stars as JSON for the info panel
    #[wasm_bindgen]
    pub fn get_stars_json(&self) -> String {
        serde_json::to_string(&self.stars).unwrap_or_else(|_| "[]".to_string())
    }
}

/// Convert temperature to CSS color (blackbody approximation)
fn temp_to_color(temp: f64) -> String {
    if temp > 10000.0 {
        "#aabbff".to_string() // Blue-white
    } else if temp > 7500.0 {
        "#caddff".to_string() // White
    } else if temp > 6000.0 {
        "#fff4e8".to_string() // Yellow-white
    } else if temp > 5000.0 {
        "#ffd700".to_string() // Yellow
    } else if temp > 4000.0 {
        "#ffaa00".to_string() // Orange
    } else {
        "#ff6644".to_string() // Red
    }
}

/// Initialize the star sky renderer and attach to window
#[wasm_bindgen(start)]
pub fn main() {
    console_log!("[StarSky] WASM module loaded - Tribute to Astrophysics");
}

/// Convenience function to render from storage
#[wasm_bindgen]
pub fn render_from_storage(canvas_id: &str, storage_key: &str) -> Result<StarSkyRenderer, JsValue> {
    let mut renderer = StarSkyRenderer::new(canvas_id)?;
    renderer.resize();
    renderer.load_from_storage(storage_key)?;
    renderer.render();
    Ok(renderer)
}
