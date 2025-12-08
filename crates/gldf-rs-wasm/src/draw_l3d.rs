//! L3D 3D renderer using three-d
//!
//! This module provides WebGL-based 3D rendering for L3D files.

use gloo::console::log;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
#[allow(unused_imports)]
use std::sync::atomic::{AtomicU32, Ordering};
#[allow(unused_imports)]
use std::sync::Arc;
use three_d::*;
use wasm_bindgen::prelude::*;

/// Global counter for unique renderer IDs
#[allow(dead_code)]
static RENDERER_COUNTER: AtomicU32 = AtomicU32::new(0);

/// L3D renderer state - uses Context directly for WASM to support multiple instances
pub struct DrawL3d {
    id: u32,
    context: Context,
    camera: Camera,
    control: OrbitControl,
    light0: DirectionalLight,
    light1: DirectionalLight,
    ambient: AmbientLight,
    models: Vec<Model<PhysicalMaterial>>,
    frame_count: u32,
}

impl DrawL3d {
    /// Create a new L3D renderer attached to a canvas element
    #[allow(unused_variables)]
    pub fn create(canvas: web_sys::HtmlCanvasElement) -> Result<Self, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let id = RENDERER_COUNTER.fetch_add(1, Ordering::SeqCst);

            // Get actual canvas dimensions
            let width = canvas.width();
            let height = canvas.height();

            log!(format!(
                "[Renderer-{}] Creating with canvas size: {}x{}",
                id, width, height
            ));

            // Create WebGL2 context directly from canvas (no winit needed)
            let webgl_context = canvas
                .get_context("webgl2")
                .map_err(|e| format!("Failed to get WebGL2 context: {:?}", e))?
                .ok_or("WebGL2 not supported")?
                .dyn_into::<web_sys::WebGl2RenderingContext>()
                .map_err(|_| "Failed to cast to WebGl2RenderingContext")?;

            // Enable required extensions
            let _ = webgl_context.get_extension("EXT_color_buffer_float");
            let _ = webgl_context.get_extension("OES_texture_float_linear");
            let _ = webgl_context.get_extension("OES_texture_half_float_linear");

            // Create three-d context from WebGL2 context
            let context = Context::from_gl_context(Arc::new(
                three_d::context::Context::from_webgl2_context(webgl_context),
            ))
            .map_err(|e| format!("Failed to create three-d context: {:?}", e))?;

            // Initialize camera with actual canvas dimensions
            let camera = Camera::new_perspective(
                Viewport::new_at_origo(width, height),
                vec3(4.0, 4.0, 5.0),
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
                degrees(45.0),
                0.1,
                1000.0,
            );
            let control = OrbitControl::new(camera.target().clone(), 1.0, 100.0);

            let light0 = DirectionalLight::new(&context, 2.0, Srgba::WHITE, vec3(-1.0, -1.0, -1.0));
            let light1 = DirectionalLight::new(&context, 0.2, Srgba::WHITE, vec3(-0.1, 0.5, 0.5));
            let ambient = AmbientLight::new(&context, 0.4, Srgba::WHITE);

            log!(format!("[Renderer-{}] Created successfully", id));

            Ok(DrawL3d {
                id,
                context,
                camera,
                control,
                light0,
                light1,
                ambient,
                models: Vec::new(),
                frame_count: 0,
            })
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Err("L3D renderer only supported in WASM".to_string())
        }
    }

    /// Load an L3D model for rendering
    pub fn set_model(&mut self, l3d: &l3d_rs::L3d) {
        log!(format!("[Renderer-{}] set_model called", self.id));
        log!(format!(
            "[Renderer-{}] L3D model parts: {}",
            self.id,
            l3d.model.parts.len()
        ));
        log!(format!(
            "[Renderer-{}] L3D file assets: {}",
            self.id,
            l3d.file.assets.len()
        ));

        // Log all available asset names
        let asset_names: Vec<_> = l3d.file.assets.iter().map(|a| a.name.as_str()).collect();
        log!(format!(
            "[Renderer-{}] Available assets: {:?}",
            self.id, asset_names
        ));

        // Log asset details
        for (i, asset) in l3d.file.assets.iter().enumerate() {
            log!(format!(
                "[Renderer-{}] Asset {}: name='{}', content={} bytes",
                self.id,
                i,
                asset.name,
                asset.content.len()
            ));
        }

        // Log part info
        for (i, part) in l3d.model.parts.iter().enumerate() {
            log!(format!(
                "[Renderer-{}] Part {}: path='{}', looking for asset match",
                self.id, i, part.path
            ));
            // Check if asset exists with this exact name
            let asset_exists = l3d.file.assets.iter().any(|a| a.name == part.path);
            log!(format!(
                "[Renderer-{}] Part {} asset match: {}",
                self.id, i, asset_exists
            ));
        }

        if l3d.model.parts.is_empty() {
            log!(format!(
                "[Renderer-{}] WARNING: No model parts found!",
                self.id
            ));
            return;
        }

        let mut loaded = load_assets(l3d);
        log!(format!(
            "[Renderer-{}] RawAssets created, starting model loading...",
            self.id
        ));

        // Rotation matrix to align model's Z axis to screen's Y axis
        let rotation_matrix = Mat4::from_angle_x(Deg(-90.0));

        let mut cpu_models = std::collections::HashMap::new();
        let mut models = Vec::<Model<PhysicalMaterial>>::new();

        // Track bounding box
        let mut min_bound = vec3(f32::MAX, f32::MAX, f32::MAX);
        let mut max_bound = vec3(f32::MIN, f32::MIN, f32::MIN);

        log!(format!(
            "[Renderer-{}] Loading {} model parts",
            self.id,
            l3d.model.parts.len()
        ));

        for (part_idx, part) in l3d.model.parts.iter().enumerate() {
            log!(format!(
                "[Renderer-{}] Processing part {}: path='{}'",
                self.id, part_idx, part.path
            ));

            let cpu_mdl = match cpu_models.get(&part.path) {
                Some(m) => {
                    log!(format!(
                        "[Renderer-{}] Using cached CpuModel for '{}'",
                        self.id, part.path
                    ));
                    m
                }
                None => {
                    log!(format!(
                        "[Renderer-{}] Deserializing CpuModel for '{}'...",
                        self.id, part.path
                    ));
                    match loaded.deserialize::<CpuModel>(&part.path) {
                        Ok(m) => {
                            log!(format!(
                                "[Renderer-{}] Successfully deserialized '{}'",
                                self.id, part.path
                            ));
                            cpu_models.insert(&part.path, m);
                            cpu_models.get(&part.path).unwrap()
                        }
                        Err(e) => {
                            log!(format!(
                                "[Renderer-{}] FAILED to deserialize model '{}': {:?}",
                                self.id, &part.path, e
                            ));
                            continue;
                        }
                    }
                }
            };

            log!(format!(
                "[Renderer-{}] Creating GPU model for '{}'...",
                self.id, part.path
            ));
            match Model::<PhysicalMaterial>::new(&self.context, cpu_mdl) {
                Ok(mut mdl) => {
                    log!(format!(
                        "[Renderer-{}] GPU model created successfully for '{}'",
                        self.id, part.path
                    ));
                    // Convert [f32; 16] to Mat4 (column-major order)
                    let part_mat = Mat4::from_cols(
                        vec4(part.mat[0], part.mat[1], part.mat[2], part.mat[3]),
                        vec4(part.mat[4], part.mat[5], part.mat[6], part.mat[7]),
                        vec4(part.mat[8], part.mat[9], part.mat[10], part.mat[11]),
                        vec4(part.mat[12], part.mat[13], part.mat[14], part.mat[15]),
                    );
                    let mat = rotation_matrix * part_mat;
                    mdl.iter_mut().for_each(|m| {
                        m.set_transformation(mat);
                        // Update bounding box from AABB
                        let aabb = m.aabb();
                        min_bound.x = min_bound.x.min(aabb.min().x);
                        min_bound.y = min_bound.y.min(aabb.min().y);
                        min_bound.z = min_bound.z.min(aabb.min().z);
                        max_bound.x = max_bound.x.max(aabb.max().x);
                        max_bound.y = max_bound.y.max(aabb.max().y);
                        max_bound.z = max_bound.z.max(aabb.max().z);
                    });
                    models.push(mdl);
                }
                Err(e) => {
                    log!(format!("Failed to create GPU model: {:?}", e));
                }
            }
        }

        log!(format!(
            "[Renderer-{}] set_model complete: loaded {} GPU models from {} parts",
            self.id,
            models.len(),
            l3d.model.parts.len()
        ));
        self.models = models;

        // Auto-fit camera to model if we have valid bounds
        if min_bound.x < max_bound.x {
            let center = vec3(
                (min_bound.x + max_bound.x) / 2.0,
                (min_bound.y + max_bound.y) / 2.0,
                (min_bound.z + max_bound.z) / 2.0,
            );
            let size = vec3(
                max_bound.x - min_bound.x,
                max_bound.y - min_bound.y,
                max_bound.z - min_bound.z,
            );
            let max_dim = size.x.max(size.y).max(size.z);

            // Position camera at a distance that fits the model
            // Using FOV of 45 degrees, distance = size / (2 * tan(FOV/2))
            let distance = max_dim / (2.0 * (22.5_f32.to_radians()).tan());
            let camera_pos = center + vec3(distance * 0.7, distance * 0.7, distance);

            log!(format!(
                "Model bounds: {:?} to {:?}, center: {:?}, distance: {}",
                min_bound, max_bound, center, distance
            ));

            // Update camera
            self.camera = Camera::new_perspective(
                self.camera.viewport(),
                camera_pos,
                center,
                vec3(0.0, 1.0, 0.0),
                degrees(45.0),
                0.01,
                distance * 10.0,
            );

            // Update orbit control with new target and appropriate zoom range
            self.control = OrbitControl::new(center, distance * 0.1, distance * 5.0);
        }
    }

    /// Handle orbit rotation
    pub fn orbit(&mut self, delta: (f32, f32)) {
        let event = Event::MouseMotion {
            button: Some(MouseButton::Left),
            delta,
            position: PhysicalPoint { x: 0.0, y: 0.0 },
            modifiers: Modifiers {
                alt: false,
                ctrl: false,
                shift: false,
                command: false,
            },
            handled: false,
        };
        let mut events = [event];
        self.control.handle_events(&mut self.camera, &mut events);
    }

    /// Handle zoom
    pub fn zoom(&mut self, delta: (f32, f32)) {
        let event = Event::MouseWheel {
            delta,
            position: PhysicalPoint { x: 0.0, y: 0.0 },
            modifiers: Modifiers {
                alt: false,
                ctrl: false,
                shift: false,
                command: false,
            },
            handled: false,
        };
        let mut events = [event];
        self.control.handle_events(&mut self.camera, &mut events);
    }

    /// Render a single frame
    pub fn render(&mut self) {
        self.frame_count += 1;

        // Log every 60 frames (roughly once per second)
        if self.frame_count == 1 || self.frame_count.is_multiple_of(60) {
            log!(format!(
                "[Renderer-{}] render() frame {}, models: {}",
                self.id,
                self.frame_count,
                self.models.len()
            ));
        }

        let viewport = self.camera.viewport();
        let screen = RenderTarget::screen(&self.context, viewport.width, viewport.height);
        screen.clear(ClearState::color_and_depth(0.8, 0.8, 0.8, 1.0, 1.0));

        for model in &self.models {
            screen.render(
                &self.camera,
                model.into_iter(),
                &[&self.light0, &self.light1, &self.ambient],
            );
        }
        // No swap_buffers needed for WebGL - browser handles this automatically
    }
}

impl fmt::Debug for DrawL3d {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrawL3d")
            .field("models_count", &self.models.len())
            .finish()
    }
}

/// Animation frame callback type
#[wasm_bindgen]
extern "C" {
    fn requestAnimationFrame(closure: &Closure<dyn FnMut()>) -> i32;
    fn cancelAnimationFrame(id: i32);
}

/// Start the render loop for an L3D renderer
#[allow(clippy::type_complexity)]
pub fn start_render_loop(renderer: Rc<RefCell<DrawL3d>>) {
    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::new(move || {
        renderer.borrow_mut().render();

        // Schedule next frame
        if let Some(ref closure) = *f.borrow() {
            requestAnimationFrame(closure);
        }
    }));

    // Start the loop - use a block to ensure the borrow is dropped before function ends
    {
        let borrowed = g.borrow();
        if let Some(ref closure) = *borrowed {
            requestAnimationFrame(closure);
        }
    }

    // Keep the closure alive by leaking it (it will run forever anyway)
    std::mem::forget(g);
}

/// Load assets from L3D file into three_d RawAssets
/// Also adds stub MTL files for any missing material references in OBJ files
pub fn load_assets(l3d: &l3d_rs::L3d) -> three_d_asset::io::RawAssets {
    let mut raw_assets = three_d_asset::io::RawAssets::new();

    // First pass: collect all assets and find MTL references in OBJ files
    let mut mtl_refs: Vec<(String, String)> = Vec::new(); // (base_path, mtl_name)

    for asset in &l3d.file.assets {
        raw_assets.insert(&asset.name, asset.content.clone());

        // If this is an OBJ file, parse it to find mtllib references
        if asset.name.to_lowercase().ends_with(".obj") {
            if let Ok(content) = std::str::from_utf8(&asset.content) {
                // Extract base directory from asset path
                let base_path = if let Some(pos) = asset.name.rfind('/') {
                    &asset.name[..=pos]
                } else {
                    ""
                };

                // Find mtllib directives
                for line in content.lines() {
                    let line = line.trim();
                    if let Some(mtl_name) = line.strip_prefix("mtllib ") {
                        let mtl_name = mtl_name.trim();
                        log!(format!(
                            "[load_assets] OBJ '{}' references mtllib '{}'",
                            asset.name, mtl_name
                        ));
                        mtl_refs.push((base_path.to_string(), mtl_name.to_string()));
                    }
                }
            }
        }
    }

    // Second pass: add stub MTL files for missing references
    for (base_path, mtl_name) in mtl_refs {
        // Try various path combinations
        let paths_to_try = vec![
            format!("{}{}", base_path, mtl_name),
            format!("{}./{}", base_path, mtl_name),
            mtl_name.clone(),
        ];

        for mtl_path in &paths_to_try {
            // Check if this MTL already exists
            let exists = l3d.file.assets.iter().any(|a| a.name == *mtl_path);
            if !exists {
                // Add a minimal stub MTL file with default material
                // MTL format requires specific order: Ns before Ka
                let stub_mtl = b"# Stub material file\nnewmtl default\nNs 100.0\nKa 0.2 0.2 0.2\nKd 0.8 0.8 0.8\nKs 0.1 0.1 0.1\nd 1.0\nillum 2\n";
                log!(format!("[load_assets] Adding stub MTL for '{}'", mtl_path));
                raw_assets.insert(mtl_path, stub_mtl.to_vec());
            }
        }
    }

    raw_assets
}
