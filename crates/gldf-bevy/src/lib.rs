//! GLDF Bevy 3D Scene Viewer Library
//!
//! **GitHub:** <https://github.com/holg/gldf-rs>
//!
//! This module provides a Bevy-based 3D viewer for GLDF files that displays
//! L3D 3D models with LDT photometric lighting in various scene types.
//! Also supports IFC geometry viewing from imported IFC files.

#[cfg(not(target_arch = "wasm32"))]
pub mod command_bridge;
pub mod ifc_loader;
pub mod l3d_loader;
pub mod orientation;
pub mod scene;
#[cfg(not(target_arch = "wasm32"))]
pub mod scene_bundle;

pub use ifc_loader::{IfcGeometry, IfcLoaderPlugin, IfcSceneData};

use bevy::prelude::*;
use eulumdat_bevy::viewer::{EulumdatViewerPlugin, FirstPersonCamera, SceneType};

/// Milliseconds clock shared by the animation cells' `started_at_ms` and
/// the apply systems' elapsed-time math. WASM uses wall-clock
/// `js_sys::Date::now()`; natively a monotonic since-start clock (both are
/// only ever used as *deltas*, so the epoch difference is irrelevant).
#[inline]
fn anim_now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::OnceLock;
        use std::time::Instant;
        static START: OnceLock<Instant> = OnceLock::new();
        START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
    }
}
pub use eulumdat_bevy::SceneSettings;

/// Resource to store the current GLDF scene data
#[derive(Resource, Default)]
pub struct GldfSceneData {
    /// The L3D model data (raw bytes)
    pub l3d_data: Option<Vec<u8>>,
    /// The LDT photometric data (parsed)
    pub ldt_data: Option<eulumdat::Eulumdat>,
    /// Scene type
    pub scene_type: SceneType,
    /// Per-emitter configuration from GLDF
    pub emitter_config: Vec<EmitterConfig>,
    /// Mounting information for positioning
    pub mounting: MountingConfig,
    /// Optional housing-color override (sRGB 0..=1). When `Some`, the
    /// renderer applies it to the luminaire body instead of the
    /// default light grey. Driven by `HousingColor.@ral` on the
    /// active variant — the editor resolves the RAL number to sRGB
    /// via `gldf_rs::ral::RalEntry` and pushes the triple into
    /// localStorage before starting the Bevy viewer.
    ///
    /// This is a **renderer-side override** — the underlying L3D
    /// bytes aren't mutated, so the same L3D archive can preview
    /// different variants in different colours without rewriting
    /// the MTL each time.
    pub housing_color_rgb: Option<[f32; 3]>,
}

/// Mounting configuration for luminaire positioning (serializable for WASM)
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct MountingConfig {
    /// Mounting type: "ceiling", "wall", "ground", "working_plane"
    pub mounting_type: Option<String>,
    /// Recessed depth in mm
    pub recessed_depth_mm: Option<i32>,
    /// Pendant length in mm (converted from f64)
    pub pendant_length_mm: Option<i32>,
    /// Wall mounting height in mm
    pub mounting_height_mm: Option<i32>,
    /// Pole height in mm
    pub pole_height_mm: Option<i32>,
    /// Is surface mounted
    pub is_surface_mounted: bool,
    /// Is free-standing
    pub is_free_standing: bool,
}

impl MountingConfig {
    /// Get the Y position in meters for the luminaire based on mounting type
    /// Returns (y_position, is_ceiling_mounted)
    pub fn get_y_position(&self, room_height: f32) -> (f32, bool) {
        match self.mounting_type.as_deref() {
            Some("ceiling") => {
                if let Some(pendant_mm) = self.pendant_length_mm {
                    // Pendant: hang down from ceiling
                    (room_height - (pendant_mm as f32 / 1000.0), true)
                } else if let Some(depth_mm) = self.recessed_depth_mm {
                    // Recessed: the luminaire body is BURIED inside
                    // the ceiling cavity, depth `depth_mm` deep. We
                    // place the LEO emission plane *above* the
                    // ceiling line by `depth/1000` so the luminaire's
                    // own body (which extends DOWNWARD from the LEOs
                    // in the L3D) only protrudes by
                    //   body_height − depth
                    // For SLV Tria 2: body ~55 mm − 50 mm recessed =
                    // ~5–20 mm protrusion. The cone we draw from each
                    // LEO still starts at the LEO position and flares
                    // downward, so the rendered effect reads as
                    // "light coming out of a hole in the ceiling."
                    let depth_m = (depth_mm as f32 / 1000.0).max(0.0);
                    (room_height + depth_m, true)
                } else {
                    // Surface mounted: slightly below ceiling
                    (room_height - 0.05, true)
                }
            }
            Some("wall") => {
                // Wall: use mounting height or default to 2m
                let height_m = self.mounting_height_mm.unwrap_or(2000) as f32 / 1000.0;
                (height_m, false)
            }
            Some("ground") => {
                if let Some(pole_mm) = self.pole_height_mm {
                    // Pole mounted: at pole height
                    (pole_mm as f32 / 1000.0, false)
                } else if self.is_free_standing {
                    // Free-standing on ground
                    (0.0, false)
                } else {
                    // Default ground level
                    (0.0, false)
                }
            }
            Some("working_plane") => {
                // Working plane: typical desk height ~0.75m
                (0.75, false)
            }
            _ => {
                // Default: assume ceiling mounted
                (room_height - 0.1, true)
            }
        }
    }
}

/// Resource to track localStorage timestamp for hot-reload (WASM)
#[derive(Resource, Default)]
pub struct GldfTimestamp(pub String);

/// Storage keys for WASM localStorage
#[cfg(target_arch = "wasm32")]
pub const L3D_STORAGE_KEY: &str = "gldf_current_l3d";
#[cfg(target_arch = "wasm32")]
pub const LDT_STORAGE_KEY: &str = "gldf_current_ldt";
#[cfg(target_arch = "wasm32")]
pub const EMITTER_CONFIG_KEY: &str = "gldf_emitter_config";
#[cfg(target_arch = "wasm32")]
pub const MOUNTING_CONFIG_KEY: &str = "gldf_mounting_config";
#[cfg(target_arch = "wasm32")]
pub const HOUSING_COLOR_KEY: &str = "gldf_housing_color_rgb";
#[cfg(target_arch = "wasm32")]
pub const GLDF_TIMESTAMP_KEY: &str = "gldf_timestamp";

/// Per-emitter configuration from GLDF
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct EmitterConfig {
    /// LEO name from L3D structure.xml. Kept for backwards compatibility
    /// with the original "one config per LEO" wiring; new flows should
    /// prefer `external_name` so SensorObject partNames can be targeted
    /// just like LEO partNames.
    pub leo_name: String,
    /// Optional override — the L3D partName this config targets. When
    /// present, the loader matches against ANY partName in the
    /// structure tree (LEOs OR SensorObjects), not just LEOs. Lets
    /// callers spawn a tinted light at a SensorObject (e.g. a centre
    /// emergency LED) without inventing a synthetic LEO. Backwards
    /// compatible: `None` → fall back to `leo_name`.
    pub external_name: Option<String>,
    /// Luminous flux in lumens
    pub luminous_flux: Option<i32>,
    /// Color temperature in Kelvin
    pub color_temperature: Option<i32>,
    /// Emergency behavior
    pub emergency_behavior: Option<String>,
}

impl EmitterConfig {
    /// The partName this config targets. Prefers `external_name` when
    /// set (the newer surface that can pin against a SensorObject);
    /// falls back to `leo_name` for older configs that only know about
    /// LEOs.
    pub fn target_name(&self) -> &str {
        match self.external_name.as_deref() {
            Some(name) if !name.is_empty() => name,
            _ => self.leo_name.as_str(),
        }
    }
}

/// Log to browser console (WASM only)
#[cfg(target_arch = "wasm32")]
fn log(msg: &str) {
    web_sys::console::log_1(&msg.into());
}

#[cfg(not(target_arch = "wasm32"))]
fn log(msg: &str) {
    println!("{}", msg);
}

/// Load L3D from localStorage (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn load_l3d_from_storage() -> Option<Vec<u8>> {
    use base64::Engine;
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let l3d_b64 = storage.get_item(L3D_STORAGE_KEY).ok()??;
    log(&format!(
        "[Bevy] Loading L3D from storage, base64 len: {}",
        l3d_b64.len()
    ));
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&l3d_b64)
        .ok();
    if let Some(ref data) = decoded {
        log(&format!("[Bevy] L3D decoded, {} bytes", data.len()));
    }
    decoded
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_l3d_from_storage() -> Option<Vec<u8>> {
    None
}

/// Load LDT from localStorage (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn load_ldt_from_storage() -> Option<eulumdat::Eulumdat> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let ldt_string = storage.get_item(LDT_STORAGE_KEY).ok()??;
    log(&format!(
        "[Bevy] Loading LDT from storage, len: {}",
        ldt_string.len()
    ));
    let parsed = eulumdat::Eulumdat::parse(&ldt_string).ok();
    if parsed.is_some() {
        log("[Bevy] LDT parsed successfully");
    }
    parsed
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_ldt_from_storage() -> Option<eulumdat::Eulumdat> {
    None
}

/// Load emitter config from localStorage (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn load_emitter_config_from_storage() -> Vec<EmitterConfig> {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return Vec::new(),
    };
    let storage = match window.local_storage().ok().flatten() {
        Some(s) => s,
        None => return Vec::new(),
    };
    let config_json = match storage.get_item(EMITTER_CONFIG_KEY).ok().flatten() {
        Some(j) => j,
        None => return Vec::new(),
    };
    log(&format!("[Bevy] Loading emitter config: {}", config_json));
    serde_json::from_str(&config_json).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_emitter_config_from_storage() -> Vec<EmitterConfig> {
    Vec::new()
}

/// Load mounting config from localStorage (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn load_mounting_config_from_storage() -> MountingConfig {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return MountingConfig::default(),
    };
    let storage = match window.local_storage().ok().flatten() {
        Some(s) => s,
        None => return MountingConfig::default(),
    };
    let config_json = match storage.get_item(MOUNTING_CONFIG_KEY).ok().flatten() {
        Some(j) => j,
        None => return MountingConfig::default(),
    };
    log(&format!("[Bevy] Loading mounting config: {}", config_json));
    serde_json::from_str(&config_json).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_mounting_config_from_storage() -> MountingConfig {
    MountingConfig::default()
}

/// Load the variant's housing-color override from localStorage.
///
/// Format: a JSON array of three floats in sRGB 0..=1, e.g.
/// `[0.733, 0.118, 0.063]` for RAL 3020 Traffic red. Absent / unparseable
/// → `None`, which falls back to the default light-grey luminaire material.
#[cfg(target_arch = "wasm32")]
pub fn load_housing_color_from_storage() -> Option<[f32; 3]> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let json = storage.get_item(HOUSING_COLOR_KEY).ok()??;
    log(&format!("[Bevy] Loading housing color: {}", json));
    let triple: [f32; 3] = serde_json::from_str(&json).ok()?;
    Some(triple)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_housing_color_from_storage() -> Option<[f32; 3]> {
    None
}

/// Get timestamp from localStorage
#[cfg(target_arch = "wasm32")]
pub fn get_gldf_timestamp() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(GLDF_TIMESTAMP_KEY).ok()?
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_gldf_timestamp() -> Option<String> {
    None
}

/// Poll localStorage for changes
#[allow(unused_mut, unused_variables)]
pub fn poll_gldf_changes(
    mut scene_data: ResMut<GldfSceneData>,
    mut settings: ResMut<SceneSettings>,
    mut last_timestamp: ResMut<GldfTimestamp>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(new_timestamp) = get_gldf_timestamp() {
            if new_timestamp != last_timestamp.0 {
                log(&format!(
                    "[Bevy] Timestamp changed: {} -> {}",
                    last_timestamp.0, new_timestamp
                ));
                // Timestamp changed - reload data
                if let Some(l3d_bytes) = load_l3d_from_storage() {
                    log(&format!("[Bevy] Loaded L3D: {} bytes", l3d_bytes.len()));
                    scene_data.l3d_data = Some(l3d_bytes);
                } else {
                    log("[Bevy] No L3D data in storage");
                }
                if let Some(ldt) = load_ldt_from_storage() {
                    log("[Bevy] Loaded LDT, updating settings");
                    scene_data.ldt_data = Some(ldt.clone());
                    // eulumdat-bevy renamed `ldt_data` → `ldc_data` (LDC =
                    // light distribution curve, the format-agnostic name).
                    settings.ldc_data = Some(ldt);
                } else {
                    log("[Bevy] No LDT data in storage");
                }
                // Load emitter configuration
                let emitter_config = load_emitter_config_from_storage();
                if !emitter_config.is_empty() {
                    log(&format!(
                        "[Bevy] Loaded {} emitter configs",
                        emitter_config.len()
                    ));
                    scene_data.emitter_config = emitter_config;
                }
                // Load mounting configuration
                let mounting_config = load_mounting_config_from_storage();
                if mounting_config.mounting_type.is_some() {
                    log(&format!(
                        "[Bevy] Loaded mounting config: {:?}",
                        mounting_config.mounting_type
                    ));
                    scene_data.mounting = mounting_config;
                }
                // Load housing-color override. We pick it up
                // unconditionally — `None` means "drop back to the
                // grey default" so a variant without HousingColor
                // resets properly when the user flips back to it.
                scene_data.housing_color_rgb = load_housing_color_from_storage();
                log(&format!(
                    "[Bevy] Housing color override: {:?}",
                    scene_data.housing_color_rgb
                ));
                last_timestamp.0 = new_timestamp;
            }
        }
    }
}

/// Keyboard controls for the viewer
pub fn ui_controls_system(
    mut settings: ResMut<SceneSettings>,
    mut scene_data: ResMut<GldfSceneData>,
    mut show_cones: ResMut<l3d_loader::ShowLightCones>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Toggle photometric solid with P key
    if keyboard.just_pressed(KeyCode::KeyP) {
        settings.show_photometric_solid = !settings.show_photometric_solid;
    }

    // Toggle luminaire with L key
    if keyboard.just_pressed(KeyCode::KeyL) {
        settings.show_luminaire = !settings.show_luminaire;
    }

    // Toggle shadows with H key
    if keyboard.just_pressed(KeyCode::KeyH) {
        settings.show_shadows = !settings.show_shadows;
    }

    // Toggle debug light cones with K key — translucent emissive cones
    // along each spotlight's emission axis. Off by default because they
    // dominate dark scenes; turn on with K to verify the (LUMINAIRE-TILT
    // …) Lisp primitive is moving the right joint.
    if keyboard.just_pressed(KeyCode::KeyK) {
        show_cones.0 = !show_cones.0;
    }

    // Cycle scene types with 1-4 keys
    if keyboard.just_pressed(KeyCode::Digit1) {
        settings.scene_type = SceneType::Room;
        scene_data.scene_type = SceneType::Room;
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        settings.scene_type = SceneType::Road;
        scene_data.scene_type = SceneType::Road;
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        settings.scene_type = SceneType::Parking;
        scene_data.scene_type = SceneType::Parking;
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        settings.scene_type = SceneType::Outdoor;
        scene_data.scene_type = SceneType::Outdoor;
    }
}

/// Frame the camera on the L3D model the FIRST time one spawns. After
/// that the user's camera (orbited via mouse / WASD / arrows or set
/// via the JS bridge) is preserved across variant swaps — so they can
/// pick a vantage on variant A and compare it side-by-side with
/// variant B without having to re-orbit each time. Pressing R still
/// re-frames manually (handled by eulumdat-bevy's camera_reset
/// system). Without this the user lost their angle every time they
/// clicked a different variant chip.
fn frame_camera_on_l3d(
    _timestamp: Res<GldfTimestamp>,
    settings: Res<SceneSettings>,
    mut already_framed: Local<bool>,
    l3d_models: Query<&Transform, (With<l3d_loader::L3dModel>, Without<FirstPersonCamera>)>,
    l3d_lights: Query<&Transform, (With<l3d_loader::L3dLight>, Without<FirstPersonCamera>)>,
    mut camera_q: Query<(&mut Transform, &mut FirstPersonCamera)>,
) {
    if l3d_models.is_empty() {
        return; // No model spawned yet; wait.
    }
    if *already_framed {
        return; // Honour the user's current camera; only auto-frame once per session.
    }
    // Centroid of the L3D's actual light-emitting objects (LEOs).
    // Those are the user-visible lamp positions; the eulumdat / Bevy
    // loader places them at the spot/point lights it spawns alongside
    // each LEO in the L3D structure.xml. Falls back to the part-
    // translation AABB if no LEO lights are present (rare — only when
    // the L3D has no <LightEmittingObject> entries at all).
    let (target, source_label) = if !l3d_lights.is_empty() {
        let mut sum = Vec3::ZERO;
        let mut n = 0u32;
        for t in l3d_lights.iter() {
            sum += t.translation;
            n += 1;
        }
        (sum / n as f32, format!("{} LEO lights", n))
    } else {
        let mut sum = Vec3::ZERO;
        let mut n = 0u32;
        for t in l3d_models.iter() {
            sum += t.translation;
            n += 1;
        }
        (sum / n as f32, format!("{} model parts", n))
    };

    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for t in l3d_models.iter() {
        min = min.min(t.translation);
        max = max.max(t.translation);
    }
    let extent = (max - min).abs();
    let diag = extent.length().max(0.2);

    // Camera offset baked from a hand-tuned vantage that frames the
    // SLV TRIA 2 nicely with both heads + housing colour visible:
    // pressed `P` in the live viewer with the lamp well-framed and
    // logged cam-pos=(2.561, 2.093, 4.558) relative to LEO centroid
    // (2.000, 2.493, 2.500). That gives this offset:
    //
    //   target → cam = (+0.56, -0.40, +2.06)   ~2.1 m behind, 40 cm down
    //   yaw   = -6.142  ≈ -0.14 rad after mod-2π (looks down -Z, slight -X)
    //   pitch = -0.041  (~ -2.3°, near-level)
    //
    // The 3/4 angle and near-horizontal pitch matter — straight-on
    // shots of a recessed lamp at the ceiling are claustrophobic;
    // this gives breathing room and shows two LEOs distinctly.
    let cam_pos = target + Vec3::new(0.561, -0.400, 2.058);
    // Preserve the hand-tuned yaw/pitch verbatim instead of
    // recomputing via looking_at(target), so the slight -X drift
    // stays intact (looking_at would dead-centre the target and lose
    // the 3/4 angle).
    let yaw = -6.142_f32;
    let pitch = -0.041_f32;

    for (mut transform, mut camera) in camera_q.iter_mut() {
        transform.translation = cam_pos;
        camera.yaw = yaw;
        camera.pitch = pitch;
        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    }
    *already_framed = true;

    // Fat coord dump. The e2e camera_probe.py captures console logs and
    // writes them to tmp/e2e-out/bevy-coords.log so the user can sanity-
    // check what the scene actually looks like + tell us where the
    // camera *should* go.
    log("[CAMERA-PROBE] ── L3D framing dump ──");
    log(&format!(
        "[CAMERA-PROBE] target source: {}  centroid=({:.3}, {:.3}, {:.3})",
        source_label, target.x, target.y, target.z
    ));
    for (i, t) in l3d_lights.iter().enumerate() {
        log(&format!(
            "[CAMERA-PROBE]   LEO light #{}: ({:.3}, {:.3}, {:.3})",
            i, t.translation.x, t.translation.y, t.translation.z
        ));
    }
    for (i, t) in l3d_models.iter().enumerate() {
        log(&format!(
            "[CAMERA-PROBE]   mesh part #{}: ({:.3}, {:.3}, {:.3})",
            i, t.translation.x, t.translation.y, t.translation.z
        ));
    }
    log(&format!(
        "[CAMERA-PROBE] aabb-min=({:.3}, {:.3}, {:.3}) aabb-max=({:.3}, {:.3}, {:.3})",
        min.x, min.y, min.z, max.x, max.y, max.z
    ));
    log(&format!(
        "[CAMERA-PROBE] extent=({:.3}, {:.3}, {:.3}) diag={:.3}",
        extent.x, extent.y, extent.z, diag
    ));
    log(&format!(
        "[CAMERA-PROBE] room w×h×l = {:.2} × {:.2} × {:.2} (ceiling y={:.2})",
        settings.room_width, settings.room_height, settings.room_length, settings.room_height
    ));
    log(&format!(
        "[CAMERA-PROBE] cam-pos=({:.3}, {:.3}, {:.3}) yaw={:.3} pitch={:.3}",
        cam_pos.x, cam_pos.y, cam_pos.z, yaw, pitch
    ));
    let offset_from_target = cam_pos - target;
    log(&format!(
        "[CAMERA-PROBE] cam-offset-from-target=({:.3}, {:.3}, {:.3})",
        offset_from_target.x, offset_from_target.y, offset_from_target.z
    ));
    log("[CAMERA-PROBE] ── end ──");
}

// ─── JS ↔ Bevy camera bridge ─────────────────────────────────────────
//
// Two `#[wasm_bindgen]` functions exposed to JS:
//
//   window.gldf_bevy_set_camera(x, y, z, yaw, pitch)   // write request
//   window.gldf_bevy_get_camera()  -> {x,y,z,yaw,pitch,fwd:[..]}
//
// Mechanism: a pair of `Mutex<Option<…>>` static cells. `set_camera`
// just stores a `CamRequest`. A Bevy system reads + clears that on
// every frame and applies it to the FirstPersonCamera. `get_camera`
// reads the most recent snapshot the same system writes back each
// frame.
//
// Why not crossbeam channels / Bevy Events? The JS side needs to be
// callable at any time (DevTools console, Playwright `page.evaluate`)
// without holding a Bevy World handle. A `OnceLock<Mutex<…>>` static is
// the smallest thing that works.

#[derive(Clone, Copy, Debug)]
struct CamRequest {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
}

// The fields are read by the wasm32 JS bridges only.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Clone, Copy, Debug, Default)]
struct CamSnapshot {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
    forward: Vec3,
}

static CAM_REQUEST: std::sync::OnceLock<std::sync::Mutex<Option<CamRequest>>> =
    std::sync::OnceLock::new();

static CAM_SNAPSHOT: std::sync::OnceLock<std::sync::Mutex<CamSnapshot>> =
    std::sync::OnceLock::new();

fn cam_request_cell() -> &'static std::sync::Mutex<Option<CamRequest>> {
    CAM_REQUEST.get_or_init(|| std::sync::Mutex::new(None))
}

fn cam_snapshot_cell() -> &'static std::sync::Mutex<CamSnapshot> {
    CAM_SNAPSHOT.get_or_init(|| std::sync::Mutex::new(CamSnapshot::default()))
}

/// JS entry: set the camera position + orientation. Yaw / pitch in
/// radians, matching the `[CAMERA-PROBE-LIVE]` log output, so you can
/// paste log values straight back in:
///
///     window.gldf_bevy_set_camera(2.561, 2.093, 4.558, -6.142, -0.041)
///
/// Applied on the next Bevy frame.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_set_camera)]
pub fn set_camera_from_js(x: f32, y: f32, z: f32, yaw: f32, pitch: f32) {
    if let Ok(mut slot) = cam_request_cell().lock() {
        *slot = Some(CamRequest {
            pos: Vec3::new(x, y, z),
            yaw,
            pitch,
        });
    }
}

/// JS entry: return the current camera state as a JSON object:
///
///     { x, y, z, yaw, pitch, fx, fy, fz }
///
/// `fx/fy/fz` is the unit forward vector (handy for sanity-checking
/// what the camera is pointed at). Returns the last snapshot written
/// by the apply system — could be one frame stale, never more.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_get_camera)]
pub fn get_camera_from_js() -> wasm_bindgen::JsValue {
    let snap = cam_snapshot_cell()
        .lock()
        .ok()
        .map(|s| *s)
        .unwrap_or_default();
    let json = format!(
        "{{\"x\":{:.6},\"y\":{:.6},\"z\":{:.6},\"yaw\":{:.6},\"pitch\":{:.6},\"fx\":{:.6},\"fy\":{:.6},\"fz\":{:.6}}}",
        snap.pos.x, snap.pos.y, snap.pos.z, snap.yaw, snap.pitch,
        snap.forward.x, snap.forward.y, snap.forward.z,
    );
    // Parse-and-return: JsValue from a Rust serde-less JSON is a tiny
    // bit of plumbing, but `js_sys::JSON::parse` on the string keeps
    // this dep-free and gives the caller a real JS object, not a
    // string they have to JSON.parse themselves.
    js_sys::JSON::parse(&json).unwrap_or(wasm_bindgen::JsValue::NULL)
}

/// Bevy system: consume a pending `CamRequest` (if any) and apply it
/// to the FirstPersonCamera, then write the current camera state back
/// to `CAM_SNAPSHOT` so `get_camera_from_js` can read it.
fn js_camera_bridge_apply(mut camera_q: Query<(&mut Transform, &mut FirstPersonCamera)>) {
    // Take any pending request.
    let req = cam_request_cell().lock().ok().and_then(|mut s| s.take());
    for (mut transform, mut camera) in camera_q.iter_mut() {
        if let Some(r) = req {
            transform.translation = r.pos;
            camera.yaw = r.yaw;
            camera.pitch = r.pitch.clamp(-1.5, 1.5);
            transform.rotation = Quat::from_euler(EulerRot::YXZ, camera.yaw, camera.pitch, 0.0);
        }
        // Refresh snapshot every frame (cheap; one Mutex lock).
        if let Ok(mut snap) = cam_snapshot_cell().lock() {
            let fwd = transform.forward();
            *snap = CamSnapshot {
                pos: transform.translation,
                yaw: camera.yaw,
                pitch: camera.pitch,
                forward: Vec3::new(fwd.x, fwd.y, fwd.z),
            };
        }
    }
}

// ---------- JS → Bevy joint-tilt bridge ----------
//
// Companion to the camera bridge above. JS callers (or the WASM Lisp
// REPL panel via the `gldf-rs-lisp` engine) request a tilt:
//
//     window.gldf_bevy_luminaire_tilt("LEO0", 20.0, 0.0, 0.0)
//
// The joint name is the L3D `partName` (e.g. "LEO0", "Sensor"). The
// `gldf-rs-lisp` engine addresses these as `"l3d://leo0"`; the panel
// strips the `l3d://` prefix before forwarding. Angles are Euler XYZ
// in degrees.
//
// Mechanism mirrors the camera bridge: a `Mutex<HashMap<…>>` of
// pending tilt requests keyed by joint name. A Bevy system drains the
// map every frame, then walks every entity carrying
// `LightEmitterJoint(name)` + `LightEmitterBaseRotation(base)` and
// sets `transform.rotation = base * tilt`. Re-applying overwrites
// the previous tilt cleanly — no compounding across script runs.

static JOINT_TILTS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, [f32; 3]>>,
> = std::sync::OnceLock::new();

fn joint_tilts_cell() -> &'static std::sync::Mutex<std::collections::HashMap<String, [f32; 3]>> {
    JOINT_TILTS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// JS entry: request a tilt on one L3D joint, in degrees (Euler XYZ).
///
///     window.gldf_bevy_luminaire_tilt("LEO0", 20.0, 0.0, 0.0)
///
/// Subsequent calls to the same joint replace the previous tilt — the
/// apply system multiplies `base * tilt` against the captured base
/// rotation, so this is set-not-add semantics.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_luminaire_tilt)]
pub fn luminaire_tilt_from_js(joint: &str, deg_x: f32, deg_y: f32, deg_z: f32) {
    if let Ok(mut map) = joint_tilts_cell().lock() {
        map.insert(joint.to_string(), [deg_x, deg_y, deg_z]);
    }
}

/// JS entry: clear ALL pending tilts AND active animations on every
/// joint. Use this before starting a fresh choreography so a previous
/// session's leftover tilt can't compound on top of the new starting
/// pose. Also called automatically whenever a new L3D loads — the apex
/// pivot needs a known-zero starting state to look right.
///
///     window.gldf_bevy_reset_tilts()
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_reset_tilts)]
pub fn reset_tilts_from_js() {
    if let Ok(mut m) = joint_tilts_cell().lock() {
        m.clear();
    }
    if let Ok(mut m) = joint_anims_cell().lock() {
        m.clear();
    }
}

// ---------- Room-tweak request cells + JS bridges ----------
//
// Lisp scripts adjust the surrounding room via four primitives:
// `(SET-AMBIENT b)`, `(SET-ROOM-SIZE w l h)`,
// `(SET-{WALLS|FLOOR|CEILING}-COLOR r g b)`, and
// `(SET-{WALLS|FLOOR|CEILING}-REFLECTANCE v)`. Each gets its own
// `Mutex<Option<…>>` request cell. JS bridges drop a request in;
// Bevy systems below drain the cells every frame and mutate the
// appropriate resource / entity material.
//
// Surface targets ride as a string ("walls" / "floor" / "ceiling")
// because we don't want gldf-bevy to depend on gldf-rs-lisp's enum.

#[derive(Clone, Copy, Debug)]
struct RoomSizeRequest {
    width: f32,
    length: f32,
    height: f32,
}

#[derive(Clone, Debug)]
struct SurfaceColorRequest {
    surface: String,
    r: f32,
    g: f32,
    b: f32,
}

#[derive(Clone, Debug)]
struct SurfaceEmissiveRequest {
    surface: String,
    r: f32,
    g: f32,
    b: f32,
}

#[derive(Clone, Debug)]
struct SurfaceReflectanceRequest {
    surface: String,
    value: f32,
}

static AMBIENT_REQUEST: std::sync::OnceLock<std::sync::Mutex<Option<f32>>> =
    std::sync::OnceLock::new();
static ROOM_SIZE_REQUEST: std::sync::OnceLock<std::sync::Mutex<Option<RoomSizeRequest>>> =
    std::sync::OnceLock::new();
static SURFACE_COLOR_REQUEST: std::sync::OnceLock<std::sync::Mutex<Vec<SurfaceColorRequest>>> =
    std::sync::OnceLock::new();
static SURFACE_REFLECTANCE_REQUEST: std::sync::OnceLock<
    std::sync::Mutex<Vec<SurfaceReflectanceRequest>>,
> = std::sync::OnceLock::new();
static SURFACE_EMISSIVE_REQUEST: std::sync::OnceLock<
    std::sync::Mutex<Vec<SurfaceEmissiveRequest>>,
> = std::sync::OnceLock::new();

fn ambient_request_cell() -> &'static std::sync::Mutex<Option<f32>> {
    AMBIENT_REQUEST.get_or_init(|| std::sync::Mutex::new(None))
}
fn room_size_request_cell() -> &'static std::sync::Mutex<Option<RoomSizeRequest>> {
    ROOM_SIZE_REQUEST.get_or_init(|| std::sync::Mutex::new(None))
}
fn surface_color_request_cell() -> &'static std::sync::Mutex<Vec<SurfaceColorRequest>> {
    SURFACE_COLOR_REQUEST.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}
fn surface_reflectance_request_cell() -> &'static std::sync::Mutex<Vec<SurfaceReflectanceRequest>> {
    SURFACE_REFLECTANCE_REQUEST.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}
fn surface_emissive_request_cell() -> &'static std::sync::Mutex<Vec<SurfaceEmissiveRequest>> {
    SURFACE_EMISSIVE_REQUEST.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// JS entry: set the global ambient brightness. Picked up by
/// `apply_ambient_request` on the next frame.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_set_ambient)]
pub fn set_ambient_from_js(brightness: f32) {
    if let Ok(mut slot) = ambient_request_cell().lock() {
        *slot = Some(brightness.max(0.0));
    }
}

/// JS entry: resize the surrounding room. Each plane's transform is
/// rebuilt by `apply_room_size_request` on the next frame.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_set_room_size)]
pub fn set_room_size_from_js(width: f32, length: f32, height: f32) {
    if width <= 0.0 || length <= 0.0 || height <= 0.0 {
        return;
    }
    if let Ok(mut slot) = room_size_request_cell().lock() {
        *slot = Some(RoomSizeRequest {
            width,
            length,
            height,
        });
    }
}

/// JS entry: set a room surface's base colour. `surface` is one of
/// `"walls"`, `"floor"`, `"ceiling"`. Unknown surfaces are ignored.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_set_surface_color)]
pub fn set_surface_color_from_js(surface: String, r: f32, g: f32, b: f32) {
    if let Ok(mut v) = surface_color_request_cell().lock() {
        v.push(SurfaceColorRequest { surface, r, g, b });
    }
}

/// JS entry: set a room surface's reflectance (0 = matte, 1 = mirror).
/// `surface` is one of `"walls"`, `"floor"`, `"ceiling"`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_set_surface_reflectance)]
pub fn set_surface_reflectance_from_js(surface: String, value: f32) {
    if let Ok(mut v) = surface_reflectance_request_cell().lock() {
        v.push(SurfaceReflectanceRequest {
            surface,
            value: value.clamp(0.0, 1.0),
        });
    }
}

/// JS entry: set a room surface's emissive colour (light emitted by the
/// surface itself, independent of incident light). 0–1 per channel.
/// `surface` is one of `"walls"`, `"floor"`, `"ceiling"`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_set_surface_emissive)]
pub fn set_surface_emissive_from_js(surface: String, r: f32, g: f32, b: f32) {
    if let Ok(mut v) = surface_emissive_request_cell().lock() {
        v.push(SurfaceEmissiveRequest {
            surface,
            r: r.max(0.0),
            g: g.max(0.0),
            b: b.max(0.0),
        });
    }
}

/// Bevy system: drain `ambient_request_cell` and write into
/// `GlobalAmbientLight`. Runs every frame; no-op when the cell is None.
fn apply_ambient_request(mut ambient: ResMut<bevy::light::GlobalAmbientLight>) {
    let Some(brightness) = ambient_request_cell()
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
    else {
        return;
    };
    ambient.brightness = brightness;
}

/// Query filters that keep the three room-surface queries disjoint.
type OnlyFloor = (
    With<eulumdat_bevy::viewer::FloorSurface>,
    Without<eulumdat_bevy::viewer::CeilingSurface>,
    Without<eulumdat_bevy::viewer::WallSurface>,
);
type OnlyCeiling = (
    With<eulumdat_bevy::viewer::CeilingSurface>,
    Without<eulumdat_bevy::viewer::FloorSurface>,
    Without<eulumdat_bevy::viewer::WallSurface>,
);
type OnlyWalls = (
    With<eulumdat_bevy::viewer::WallSurface>,
    Without<eulumdat_bevy::viewer::FloorSurface>,
    Without<eulumdat_bevy::viewer::CeilingSurface>,
);

/// Bevy system: drain `room_size_request_cell` and rebuild the
/// floor / ceiling / wall transforms in place. The plane MESH sizes
/// are baked at spawn time and not easy to swap mid-run, so we
/// repurpose `Transform.scale` to make the planes fit the new
/// dimensions. Each plane was spawned at unit size 1×1; we scale
/// so its post-transform extent matches `w × l` (floor/ceiling) or
/// `w × h` / `l × h` (walls).
///
/// Note: the existing builder spawns planes with `mesh().size(w, l)`,
/// not unit size — so we'd really need a mesh swap to be exact. For
/// the demo we set Transform.scale so the visible extent scales
/// proportionally to the requested vs the original dimensions. The
/// original room defaults are read from `SceneSettings`.
fn apply_room_size_request(
    settings: Res<SceneSettings>,
    mut floor_q: Query<&mut Transform, OnlyFloor>,
    mut ceiling_q: Query<&mut Transform, OnlyCeiling>,
    mut wall_q: Query<&mut Transform, OnlyWalls>,
) {
    let Some(req) = room_size_request_cell()
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
    else {
        return;
    };
    let orig_w = settings.room_width;
    let orig_l = settings.room_length;
    let orig_h = settings.room_height;
    if orig_w <= 0.0 || orig_l <= 0.0 || orig_h <= 0.0 {
        return;
    }
    let sx = req.width / orig_w;
    let sy = req.height / orig_h;
    let sz = req.length / orig_l;

    // Floor: spawned at (w/2, 0, l/2). Scale X by sx (width), Z by sz
    // (length); translate proportionally so it stays centred at the
    // luminaire's origin-side corner.
    for mut t in floor_q.iter_mut() {
        t.translation.x = req.width / 2.0;
        t.translation.z = req.length / 2.0;
        t.scale.x = sx;
        t.scale.z = sz;
    }
    // Ceiling: same XZ scaling; Y translates to the new height.
    for mut t in ceiling_q.iter_mut() {
        t.translation.x = req.width / 2.0;
        t.translation.y = req.height;
        t.translation.z = req.length / 2.0;
        t.scale.x = sx;
        t.scale.z = sz;
    }
    // Walls: each wall is rotated to face inward. We can't easily
    // know which wall is which without another marker, but uniform
    // scale along the rotated axes gets us close: scale X by sx, Y
    // by sy, Z by sz. The translations on each wall (originally
    // (w/2, h/2, 0) / (w/2, h/2, l) / (0, h/2, l/2) / (w, h/2, l/2))
    // need their respective axes multiplied so the walls move with
    // the new room. We do that by checking which translation axis
    // was originally at the room boundary vs. centre.
    for mut t in wall_q.iter_mut() {
        // X axis: was at 0, w, or w/2.
        if (t.translation.x - orig_w / 2.0).abs() < 0.001 {
            t.translation.x = req.width / 2.0;
        } else if (t.translation.x - orig_w).abs() < 0.001 {
            t.translation.x = req.width;
        } // x == 0 stays at 0
        if (t.translation.y - orig_h / 2.0).abs() < 0.001 {
            t.translation.y = req.height / 2.0;
        }
        if (t.translation.z - orig_l / 2.0).abs() < 0.001 {
            t.translation.z = req.length / 2.0;
        } else if (t.translation.z - orig_l).abs() < 0.001 {
            t.translation.z = req.length;
        }
        t.scale.x = sx;
        t.scale.y = sy;
        t.scale.z = sz;
    }
}

/// Bevy system: drain pending surface-color requests and mutate the
/// `StandardMaterial` of every entity bearing the matching surface
/// marker. Mutates the shared material handle so all four walls
/// recolour in unison.
fn apply_surface_color_requests(
    mut materials: ResMut<Assets<StandardMaterial>>,
    floor_q: Query<&MeshMaterial3d<StandardMaterial>, With<eulumdat_bevy::viewer::FloorSurface>>,
    ceiling_q: Query<
        &MeshMaterial3d<StandardMaterial>,
        With<eulumdat_bevy::viewer::CeilingSurface>,
    >,
    wall_q: Query<&MeshMaterial3d<StandardMaterial>, With<eulumdat_bevy::viewer::WallSurface>>,
) {
    let requests: Vec<SurfaceColorRequest> = match surface_color_request_cell().lock() {
        Ok(mut v) => std::mem::take(&mut *v),
        Err(_) => return,
    };
    for req in requests {
        let color = Color::srgb(req.r, req.g, req.b);
        match req.surface.as_str() {
            "floor" => {
                for mat_handle in floor_q.iter() {
                    if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                        mat.base_color = color;
                    }
                }
            }
            "ceiling" => {
                for mat_handle in ceiling_q.iter() {
                    if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                        mat.base_color = color;
                    }
                }
            }
            "walls" => {
                for mat_handle in wall_q.iter() {
                    if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                        mat.base_color = color;
                    }
                }
            }
            _ => {} // unknown surface label — ignore
        }
    }
}

/// Bevy system: drain pending surface-reflectance requests and set
/// `base_color` to `(value, value, value)` on the matching surface.
///
/// **Semantic note**: in lighting design "reflectance" means *how much
/// of the incident light a surface throws back overall* — for diffuse
/// surfaces that's the albedo (the base colour's luminance), NOT how
/// scattered the highlight is (which is what `perceptual_roughness`
/// controls in PBR). 0.7 plaster bounces ~70% of the light because it
/// reads as a light-grey, regardless of whether the highlight is sharp
/// or matte. So `(SET-WALLS-REFLECTANCE 0.7)` sets the wall to a
/// neutral grey of luminance 0.7.
///
/// `perceptual_roughness` is left untouched. If you also want a glossy
/// floor, that's a separate concern — would need a `SET-*-GLOSS`
/// primitive (not implemented yet).
fn apply_surface_reflectance_requests(
    mut materials: ResMut<Assets<StandardMaterial>>,
    floor_q: Query<&MeshMaterial3d<StandardMaterial>, With<eulumdat_bevy::viewer::FloorSurface>>,
    ceiling_q: Query<
        &MeshMaterial3d<StandardMaterial>,
        With<eulumdat_bevy::viewer::CeilingSurface>,
    >,
    wall_q: Query<&MeshMaterial3d<StandardMaterial>, With<eulumdat_bevy::viewer::WallSurface>>,
) {
    let requests: Vec<SurfaceReflectanceRequest> = match surface_reflectance_request_cell().lock() {
        Ok(mut v) => std::mem::take(&mut *v),
        Err(_) => return,
    };
    for req in requests {
        let v = req.value.clamp(0.0, 1.0);
        let neutral = Color::srgb(v, v, v);
        match req.surface.as_str() {
            "floor" => {
                for mat_handle in floor_q.iter() {
                    if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                        mat.base_color = neutral;
                    }
                }
            }
            "ceiling" => {
                for mat_handle in ceiling_q.iter() {
                    if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                        mat.base_color = neutral;
                    }
                }
            }
            "walls" => {
                for mat_handle in wall_q.iter() {
                    if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                        mat.base_color = neutral;
                    }
                }
            }
            _ => {}
        }
    }
}

/// Bevy system: drain pending surface-emissive requests and set the
/// `emissive` field of the matching surface's material. Emissive light
/// bypasses the lighting equation — the surface renders at full colour
/// regardless of incident light, useful for backlit signage panels or
/// "the wall IS the colour" hero shots. `(0, 0, 0)` disables emission.
///
/// Bevy expects emissive as a `LinearRgba`; we treat the script's r/g/b
/// values as linear sRGB to match the SET-*-COLOR conventions.
fn apply_surface_emissive_requests(
    mut materials: ResMut<Assets<StandardMaterial>>,
    floor_q: Query<&MeshMaterial3d<StandardMaterial>, With<eulumdat_bevy::viewer::FloorSurface>>,
    ceiling_q: Query<
        &MeshMaterial3d<StandardMaterial>,
        With<eulumdat_bevy::viewer::CeilingSurface>,
    >,
    wall_q: Query<&MeshMaterial3d<StandardMaterial>, With<eulumdat_bevy::viewer::WallSurface>>,
) {
    let requests: Vec<SurfaceEmissiveRequest> = match surface_emissive_request_cell().lock() {
        Ok(mut v) => std::mem::take(&mut *v),
        Err(_) => return,
    };
    for req in requests {
        let emissive = bevy::color::LinearRgba::new(req.r, req.g, req.b, 1.0);
        match req.surface.as_str() {
            "floor" => {
                for mat_handle in floor_q.iter() {
                    if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                        mat.emissive = emissive;
                    }
                }
            }
            "ceiling" => {
                for mat_handle in ceiling_q.iter() {
                    if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                        mat.emissive = emissive;
                    }
                }
            }
            "walls" => {
                for mat_handle in wall_q.iter() {
                    if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                        mat.emissive = emissive;
                    }
                }
            }
            _ => {}
        }
    }
}

// ---------- Room-state snapshots (the read side) ----------
//
// Every frame the systems below copy the live room state into static
// snapshot cells. The `gldf_bevy_get_*` JS bridges read those cells
// and return JSON; the Lisp `(GET-AMBIENT)` / `(GET-ROOM-SIZE)` /
// `(GET-{WALLS,FLOOR,CEILING}-{COLOR,REFLECTANCE})` primitives call
// those bridges via `window.gldf_bevy_get_*` and convert into Expr.

// The fields are read by the wasm32 JS bridges only.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Clone, Copy, Debug, Default)]
struct RoomSizeSnapshot {
    width: f32,
    length: f32,
    height: f32,
}

// The fields are read by the wasm32 JS bridges only.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Clone, Copy, Debug, Default)]
struct SurfaceMaterialSnapshot {
    r: f32,
    g: f32,
    b: f32,
    reflectance: f32, // ≈ luminance of base_color (R+G+B)/3
    emissive_r: f32,
    emissive_g: f32,
    emissive_b: f32,
}

static AMBIENT_SNAPSHOT: std::sync::OnceLock<std::sync::Mutex<f32>> = std::sync::OnceLock::new();
static ROOM_SIZE_SNAPSHOT: std::sync::OnceLock<std::sync::Mutex<RoomSizeSnapshot>> =
    std::sync::OnceLock::new();
static WALLS_SNAPSHOT: std::sync::OnceLock<std::sync::Mutex<SurfaceMaterialSnapshot>> =
    std::sync::OnceLock::new();
static FLOOR_SNAPSHOT: std::sync::OnceLock<std::sync::Mutex<SurfaceMaterialSnapshot>> =
    std::sync::OnceLock::new();
static CEILING_SNAPSHOT: std::sync::OnceLock<std::sync::Mutex<SurfaceMaterialSnapshot>> =
    std::sync::OnceLock::new();

fn ambient_snapshot_cell() -> &'static std::sync::Mutex<f32> {
    AMBIENT_SNAPSHOT.get_or_init(|| std::sync::Mutex::new(0.0))
}
fn room_size_snapshot_cell() -> &'static std::sync::Mutex<RoomSizeSnapshot> {
    ROOM_SIZE_SNAPSHOT.get_or_init(|| std::sync::Mutex::new(RoomSizeSnapshot::default()))
}
fn walls_snapshot_cell() -> &'static std::sync::Mutex<SurfaceMaterialSnapshot> {
    WALLS_SNAPSHOT.get_or_init(|| std::sync::Mutex::new(SurfaceMaterialSnapshot::default()))
}
fn floor_snapshot_cell() -> &'static std::sync::Mutex<SurfaceMaterialSnapshot> {
    FLOOR_SNAPSHOT.get_or_init(|| std::sync::Mutex::new(SurfaceMaterialSnapshot::default()))
}
fn ceiling_snapshot_cell() -> &'static std::sync::Mutex<SurfaceMaterialSnapshot> {
    CEILING_SNAPSHOT.get_or_init(|| std::sync::Mutex::new(SurfaceMaterialSnapshot::default()))
}

/// Bevy system: every frame, snapshot the room state into the static
/// cells that the JS getters read. Cheap — three queries + four lock
/// writes per frame, all backed by RwLocks we already poke for the
/// setters.
fn snapshot_room_state(
    ambient: Res<bevy::light::GlobalAmbientLight>,
    floor_q: Query<
        (&MeshMaterial3d<StandardMaterial>, &Transform),
        With<eulumdat_bevy::viewer::FloorSurface>,
    >,
    ceiling_q: Query<
        (&MeshMaterial3d<StandardMaterial>, &Transform),
        With<eulumdat_bevy::viewer::CeilingSurface>,
    >,
    wall_q: Query<&MeshMaterial3d<StandardMaterial>, With<eulumdat_bevy::viewer::WallSurface>>,
    materials: Res<Assets<StandardMaterial>>,
    settings: Res<SceneSettings>,
) {
    // Ambient.
    if let Ok(mut slot) = ambient_snapshot_cell().lock() {
        *slot = ambient.brightness;
    }

    // Room size: floor X-extent ≈ width, Z-extent ≈ length;
    // ceiling Y position ≈ height. The floor was spawned at the
    // builder's settings size; current size = settings * Transform.scale.
    let orig_w = settings.room_width;
    let orig_l = settings.room_length;
    let orig_h = settings.room_height;
    let (width, length, height) = if let Some((_, t)) = floor_q.iter().next() {
        let scale = t.scale;
        let h = ceiling_q
            .iter()
            .next()
            .map(|(_, ct)| ct.translation.y)
            .unwrap_or(orig_h);
        (orig_w * scale.x, orig_l * scale.z, h)
    } else {
        (orig_w, orig_l, orig_h)
    };
    if let Ok(mut slot) = room_size_snapshot_cell().lock() {
        *slot = RoomSizeSnapshot {
            width,
            length,
            height,
        };
    }

    // Per-surface colour + reflectance. Read the first matching
    // entity's material — all entities of one surface share a
    // material handle.
    fn read_material(
        materials: &Assets<StandardMaterial>,
        mat_handle: &MeshMaterial3d<StandardMaterial>,
    ) -> SurfaceMaterialSnapshot {
        if let Some(mat) = materials.get(&mat_handle.0) {
            let srgba = mat.base_color.to_srgba();
            // Reflectance ≈ luminance of the base colour. Quick mean
            // is good enough for the "what was my SET-…-REFLECTANCE
            // last value?" save/restore case; for accuracy you'd use
            // the Rec. 709 weighting (0.2126·R + 0.7152·G + 0.0722·B),
            // but the room surfaces are typically near-neutral so the
            // simple mean is within a percent.
            let reflectance = ((srgba.red + srgba.green + srgba.blue) / 3.0).clamp(0.0, 1.0);
            // `mat.emissive` is already a `LinearRgba` in current Bevy.
            SurfaceMaterialSnapshot {
                r: srgba.red,
                g: srgba.green,
                b: srgba.blue,
                reflectance,
                emissive_r: mat.emissive.red,
                emissive_g: mat.emissive.green,
                emissive_b: mat.emissive.blue,
            }
        } else {
            SurfaceMaterialSnapshot::default()
        }
    }
    if let Some((mat_handle, _)) = floor_q.iter().next() {
        if let Ok(mut slot) = floor_snapshot_cell().lock() {
            *slot = read_material(&materials, mat_handle);
        }
    }
    if let Some((mat_handle, _)) = ceiling_q.iter().next() {
        if let Ok(mut slot) = ceiling_snapshot_cell().lock() {
            *slot = read_material(&materials, mat_handle);
        }
    }
    if let Some(mat_handle) = wall_q.iter().next() {
        if let Ok(mut slot) = walls_snapshot_cell().lock() {
            *slot = read_material(&materials, mat_handle);
        }
    }
}

/// JS entry: returns the current ambient brightness as a JS Number.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_get_ambient)]
pub fn get_ambient_from_js() -> f32 {
    ambient_snapshot_cell()
        .lock()
        .ok()
        .map(|s| *s)
        .unwrap_or(0.0)
}

/// JS entry: returns `{width, length, height}` as a JS object.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_get_room_size)]
pub fn get_room_size_from_js() -> wasm_bindgen::JsValue {
    let snap = room_size_snapshot_cell()
        .lock()
        .ok()
        .map(|s| *s)
        .unwrap_or_default();
    let json = format!(
        "{{\"width\":{:.6},\"length\":{:.6},\"height\":{:.6}}}",
        snap.width, snap.length, snap.height
    );
    js_sys::JSON::parse(&json).unwrap_or(wasm_bindgen::JsValue::NULL)
}

/// JS entry: returns `{r, g, b, reflectance}` for a named surface.
/// `surface` is one of `"walls"`, `"floor"`, `"ceiling"`. Unknown
/// surfaces return all zeros.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_get_surface)]
pub fn get_surface_from_js(surface: String) -> wasm_bindgen::JsValue {
    let cell = match surface.as_str() {
        "walls" => walls_snapshot_cell(),
        "floor" => floor_snapshot_cell(),
        "ceiling" => ceiling_snapshot_cell(),
        _ => return wasm_bindgen::JsValue::NULL,
    };
    let snap = cell.lock().ok().map(|s| *s).unwrap_or_default();
    let json = format!(
        "{{\"r\":{:.6},\"g\":{:.6},\"b\":{:.6},\"reflectance\":{:.6},\
         \"er\":{:.6},\"eg\":{:.6},\"eb\":{:.6}}}",
        snap.r, snap.g, snap.b, snap.reflectance, snap.emissive_r, snap.emissive_g, snap.emissive_b,
    );
    js_sys::JSON::parse(&json).unwrap_or(wasm_bindgen::JsValue::NULL)
}

/// Bevy system: walk the tilt map and apply it to every tagged entity.
///
/// Reads the entire map (cloning since we don't want to hold the lock
/// across a Query iteration). For each entity carrying
/// [`l3d_loader::LightEmitterJoint`] + [`l3d_loader::LightEmitterBaseRotation`]
/// whose joint name matches an entry, set
/// `transform.rotation = base * Quat::from_euler(XYZ, …)`.
///
/// Tilts persist across frames — no clearing.
fn js_joint_bridge_apply(
    mut q: Query<(
        &mut Transform,
        &l3d_loader::LightEmitterJoint,
        &l3d_loader::LightEmitterBaseRotation,
        Option<&l3d_loader::LightEmitterPivot>,
    )>,
) {
    let snapshot: std::collections::HashMap<String, [f32; 3]> = joint_tilts_cell()
        .lock()
        .ok()
        .map(|m| m.clone())
        .unwrap_or_default();
    if snapshot.is_empty() {
        return;
    }
    for (mut transform, joint, base, pivot_opt) in q.iter_mut() {
        // `joint.0` is a list of L3D partNames (one entity can be shared
        // by multiple LEOs that live in the same head OBJ). Pick the
        // FIRST tilt request that targets any of them — "first-LEO-wins"
        // on concurrent conflicting tilts. In practice two LEOs sharing
        // one head move together, so this is the right semantic.
        if let Some(deg) = joint.0.iter().find_map(|n| snapshot.get(n)) {
            // Tilt is in WORLD-space Euler XYZ degrees.
            let tilt = Quat::from_euler(
                EulerRot::XYZ,
                deg[0].to_radians(),
                deg[1].to_radians(),
                deg[2].to_radians(),
            );
            transform.rotation = tilt * base.0;
            // Orbit the entity around the pivot (the LEO position).
            // Without this, an entity whose base_translation differs
            // from the pivot rotates around its OWN centre and barely
            // moves — the cone, in particular, is offset half its
            // length below the LEO so rotating it in place is nearly
            // invisible.
            if let Some(p) = pivot_opt {
                let offset = p.base_translation - p.pivot;
                transform.translation = p.pivot + tilt * offset;
            }
        }
    }
}

// ---------- JS → Bevy animation bridges ----------
//
// Two more pairs of `(static cell + JS entry + Bevy system)` mirroring
// the snapshot bridges above. The snapshot ones (`set_camera`, the
// joint-tilt map) move state in one frame; these interpolate over a
// duration, so a single script call yields visible motion.
//
//     window.gldf_bevy_camera_animate(ex, ey, ez, tx, ty, tz, ms)
//     window.gldf_bevy_luminaire_animate(joint, dx, dy, dz, ms)
//
// Mechanism: each call snapshots the *current* state as the
// animation's "from" and writes a `*Anim` record. Each Bevy frame the
// apply system reads `js_sys::Date::now()`, computes t = elapsed/ms,
// lerps (or slerps) between `from` and `to`, and writes the result.
// When `t >= 1.0` the entry is dropped.
//
// Using JS Date.now() (instead of `Res<Time>`) keeps the apply system
// signature-symmetric with the snapshot one — no extra resource to
// plumb, and the timing is in lock-step with `gldf_bevy_*_animate`'s
// own JS-side clock.

#[derive(Clone, Copy, Debug)]
struct CamAnim {
    from_pos: Vec3,
    from_yaw: f32,
    from_pitch: f32,
    to_pos: Vec3,
    to_yaw: f32,
    to_pitch: f32,
    /// `js_sys::Date::now()` at start.
    started_at_ms: f64,
    duration_ms: f64,
}

static CAM_ANIM: std::sync::OnceLock<std::sync::Mutex<Option<CamAnim>>> =
    std::sync::OnceLock::new();

fn cam_anim_cell() -> &'static std::sync::Mutex<Option<CamAnim>> {
    CAM_ANIM.get_or_init(|| std::sync::Mutex::new(None))
}

/// JS entry: animate the camera to a target pose over `ms` milliseconds.
///
///     window.gldf_bevy_camera_animate(ex, ey, ez, tx, ty, tz, ms)
///
/// `(ex, ey, ez)` is the destination eye position. `(tx, ty, tz)` is
/// the look-at target. Both metres. The "from" pose is read from the
/// current snapshot at the moment of the call, so calling this twice
/// in a row produces a continuous sweep through all three poses.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_camera_animate)]
pub fn camera_animate_from_js(ex: f32, ey: f32, ez: f32, tx: f32, ty: f32, tz: f32, ms: f32) {
    let to_pos = Vec3::new(ex, ey, ez);
    let dir = Vec3::new(tx - ex, ty - ey, tz - ez);
    // Bevy's Camera3d looks down its local -Z, so for forward to point
    // from `eye` toward `target` we need:
    //   pitch = asin(n.y)
    //   yaw   = atan2(-n.x, -n.z)
    // Matches the CAMERA-SET derivation in lisp_panel.rs. The previous
    // formula `(n.x.atan2(n.z), (-n.y).asin())` assumed a +Z-forward
    // camera, flipping yaw by 180° so ANIMATE-CAMERA landed at the
    // right eye but looking the wrong way — script-callers saw
    // CAMERA-SET and ANIMATE-CAMERA disagree on the same six numbers.
    let (to_yaw, to_pitch) = if dir.length_squared() > 1e-6 {
        let n = dir.normalize();
        ((-n.x).atan2(-n.z), n.y.asin())
    } else {
        (0.0, 0.0)
    };
    let snap = cam_snapshot_cell()
        .lock()
        .ok()
        .map(|s| *s)
        .unwrap_or_default();
    if let Ok(mut slot) = cam_anim_cell().lock() {
        *slot = Some(CamAnim {
            from_pos: snap.pos,
            from_yaw: snap.yaw,
            from_pitch: snap.pitch,
            to_pos,
            to_yaw,
            to_pitch,
            started_at_ms: js_sys::Date::now(),
            duration_ms: ms.max(1.0) as f64,
        });
    }
}

/// Bevy system: drive the camera animation. Runs every frame; reads
/// the optional `CamAnim`, lerps pos / yaw / pitch by t, and writes the
/// result via `cam_request_cell` so the existing
/// `js_camera_bridge_apply` system picks it up. Drops the record when
/// `t >= 1.0`.
fn js_camera_anim_apply() {
    let now = anim_now_ms();
    let anim = match cam_anim_cell().lock() {
        Ok(g) => *g,
        Err(_) => return,
    };
    let Some(a) = anim else {
        return;
    };
    let t = ((now - a.started_at_ms) / a.duration_ms).clamp(0.0, 1.0) as f32;
    // Smoothstep for nicer ease-in/out.
    let s = t * t * (3.0 - 2.0 * t);
    let pos = a.from_pos.lerp(a.to_pos, s);
    let yaw = a.from_yaw + (a.to_yaw - a.from_yaw) * s;
    let pitch = a.from_pitch + (a.to_pitch - a.from_pitch) * s;
    if let Ok(mut slot) = cam_request_cell().lock() {
        *slot = Some(CamRequest { pos, yaw, pitch });
    }
    if t >= 1.0 {
        if let Ok(mut slot) = cam_anim_cell().lock() {
            *slot = None;
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct JointAnim {
    from_deg: [f32; 3],
    to_deg: [f32; 3],
    started_at_ms: f64,
    duration_ms: f64,
}

static JOINT_ANIMS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, JointAnim>>,
> = std::sync::OnceLock::new();

fn joint_anims_cell() -> &'static std::sync::Mutex<std::collections::HashMap<String, JointAnim>> {
    JOINT_ANIMS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// JS entry: animate one joint to a target Euler-degrees rotation.
///
///     window.gldf_bevy_luminaire_animate("LEO0", 0.0, 30.0, 0.0, 800.0)
///
/// "From" is whatever the joint's current tilt is in `JOINT_TILTS`
/// (defaults to zero if untouched). At animation completion the entry
/// is moved into the snapshot store so subsequent animations chain
/// smoothly.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = gldf_bevy_luminaire_animate)]
pub fn luminaire_animate_from_js(joint: &str, deg_x: f32, deg_y: f32, deg_z: f32, ms: f32) {
    let from = joint_tilts_cell()
        .lock()
        .ok()
        .and_then(|g| g.get(joint).copied())
        .unwrap_or([0.0, 0.0, 0.0]);
    if let Ok(mut map) = joint_anims_cell().lock() {
        map.insert(
            joint.to_string(),
            JointAnim {
                from_deg: from,
                to_deg: [deg_x, deg_y, deg_z],
                started_at_ms: js_sys::Date::now(),
                duration_ms: ms.max(1.0) as f64,
            },
        );
    }
}

/// Bevy system: tick every active joint animation. Writes interpolated
/// degrees into the existing `JOINT_TILTS` map so the existing
/// `js_joint_bridge_apply` system rotates the entities. Drops finished
/// records.
fn js_joint_anim_apply() {
    let now = anim_now_ms();
    let snapshot: std::collections::HashMap<String, JointAnim> = joint_anims_cell()
        .lock()
        .ok()
        .map(|m| m.clone())
        .unwrap_or_default();
    if snapshot.is_empty() {
        return;
    }
    let mut finished: Vec<String> = Vec::new();
    if let Ok(mut tilts) = joint_tilts_cell().lock() {
        for (name, a) in &snapshot {
            let t = ((now - a.started_at_ms) / a.duration_ms).clamp(0.0, 1.0) as f32;
            let s = t * t * (3.0 - 2.0 * t);
            let cur = [
                a.from_deg[0] + (a.to_deg[0] - a.from_deg[0]) * s,
                a.from_deg[1] + (a.to_deg[1] - a.from_deg[1]) * s,
                a.from_deg[2] + (a.to_deg[2] - a.from_deg[2]) * s,
            ];
            tilts.insert(name.clone(), cur);
            if t >= 1.0 {
                finished.push(name.clone());
            }
        }
    }
    if !finished.is_empty() {
        if let Ok(mut map) = joint_anims_cell().lock() {
            for n in finished {
                map.remove(&n);
            }
        }
    }
}

/// Press `P` to dump the current camera position + yaw + pitch to the
/// browser console as a `[CAMERA-PROBE-LIVE]` line. Pair with
/// `tests/e2e/camera_probe.py`: orbit the scene with mouse / arrow keys
/// until the luminaire is framed, hit P, then read the coords back from
/// `tmp/e2e-out/bevy-coords.log` to bake them into `frame_camera_on_l3d`.
fn print_camera_on_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    camera_q: Query<(&Transform, &FirstPersonCamera)>,
) {
    if !keyboard.just_pressed(KeyCode::KeyP) {
        return;
    }
    for (t, cam) in camera_q.iter() {
        let fwd = t.forward();
        log(&format!(
            "[CAMERA-PROBE-LIVE] cam-pos=({:.3}, {:.3}, {:.3}) yaw={:.3} pitch={:.3} forward=({:.3}, {:.3}, {:.3})",
            t.translation.x, t.translation.y, t.translation.z,
            cam.yaw, cam.pitch,
            fwd.x, fwd.y, fwd.z,
        ));
    }
}

/// Press `C` to append a `(CAMERA-SET ex ey ez tx ty tz)` line to the
/// Lisp REPL panel's textarea, using the current camera pose.
///
/// **Target = closest point on the look ray to the luminaire centroid.**
/// Why this matters: the line is meant to round-trip through
/// `(ANIMATE-CAMERA …)`, which interpolates the *target point itself*
/// (not just rotation). If we recorded a target that's just `eye + fwd
/// * 2` (a point in empty space along the look line), every script
/// that flies *to* this pose would interpolate the target through air,
/// staring at nothing mid-flight — the "void" bug.
///
/// By locking the recorded target onto the luminaire (specifically:
/// the foot of the perpendicular from the LEO centroid to the look
/// ray), every captured pose is animation-safe: ANIMATE-CAMERA lerps
/// the target toward a stable point ON the luminaire, so the
/// in-flight view always looks at the lamp.
///
/// The recorded yaw/pitch are unchanged — the target still lies on the
/// look ray, so the derived view direction matches what the user saw
/// when they pressed C.
///
/// Reads camera pose from `CAM_SNAPSHOT` (refreshed every frame by
/// `js_camera_bridge_apply`).
///
/// Workflow: orbit the scene with right-click-drag until you like the
/// framing → press C → switch to the Lisp panel → the CAMERA-SET line is
/// already there, ready to use in any `(ANIMATE-CAMERA …)`.
#[cfg(target_arch = "wasm32")]
fn append_camera_set_on_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    _camera_q: Query<(&Transform, &FirstPersonCamera)>,
    l3d_lights: Query<&Transform, (With<l3d_loader::L3dLight>, Without<FirstPersonCamera>)>,
    l3d_models: Query<&Transform, (With<l3d_loader::L3dModel>, Without<FirstPersonCamera>)>,
) {
    if !keyboard.just_pressed(KeyCode::KeyC) {
        return;
    }
    // Read from the snapshot maintained by js_camera_bridge_apply.
    let snap = match cam_snapshot_cell().lock() {
        Ok(g) => *g,
        Err(_) => return,
    };
    let eye = snap.pos;
    let fwd = snap.forward.normalize_or_zero();
    if fwd.length_squared() < 0.001 {
        return; // Degenerate look direction; refuse to emit a bogus line.
    }

    // Find the luminaire centroid — prefer LEO lights (the actual lamp
    // positions), fall back to the model parts' centroid if no lights
    // are spawned (e.g. mid-reload). Matches `frame_camera_on_l3d`.
    let centroid = if !l3d_lights.is_empty() {
        let mut sum = Vec3::ZERO;
        let mut n = 0u32;
        for t in l3d_lights.iter() {
            sum += t.translation;
            n += 1;
        }
        Some(sum / n as f32)
    } else if !l3d_models.is_empty() {
        let mut sum = Vec3::ZERO;
        let mut n = 0u32;
        for t in l3d_models.iter() {
            sum += t.translation;
            n += 1;
        }
        Some(sum / n as f32)
    } else {
        None
    };

    // Foot of the perpendicular from the centroid to the look ray.
    // Ray = eye + t * fwd. We solve t = dot(centroid - eye, fwd).
    // Clamp t so we don't pick a target *behind* the camera (which
    // would happen if the user is looking away from the luminaire).
    let target = match centroid {
        Some(c) => {
            let t_proj = (c - eye).dot(fwd);
            // If the luminaire is behind the camera, t_proj <= 0. In
            // that case we still want a sensible target — pick a point
            // 2 m in front so the recorded line at least round-trips
            // the user's chosen view direction.
            if t_proj > 0.05 {
                eye + fwd * t_proj
            } else {
                eye + fwd * 2.0
            }
        }
        None => eye + fwd * 2.0,
    };

    let line = format!(
        "(CAMERA-SET {:.3} {:.3} {:.3}  {:.3} {:.3} {:.3})",
        eye.x, eye.y, eye.z, target.x, target.y, target.z,
    );
    log(&format!(
        "[CAMERA-PROBE-LIVE] {} (appending to Lisp panel)",
        line
    ));

    use wasm_bindgen::JsCast as _;
    if let Some(win) = web_sys::window() {
        if let Ok(prop) = js_sys::Reflect::get(
            &win,
            &wasm_bindgen::JsValue::from_str("gldf_lisp_append_camera_set"),
        ) {
            if let Ok(func) = prop.dyn_into::<js_sys::Function>() {
                let arr = js_sys::Array::new();
                arr.push(&wasm_bindgen::JsValue::from_str(&line));
                let _ = func.apply(&wasm_bindgen::JsValue::NULL, &arr);
            }
        }
    }
}

/// Stub for native builds — the C-key Lisp-append needs the WASM
/// `window` bridge, so it's a no-op outside the browser.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn append_camera_set_on_key(
    _keyboard: Res<ButtonInput<KeyCode>>,
    _camera_q: Query<(&Transform, &FirstPersonCamera)>,
    _l3d_lights: Query<&Transform, (With<l3d_loader::L3dLight>, Without<FirstPersonCamera>)>,
    _l3d_models: Query<&Transform, (With<l3d_loader::L3dModel>, Without<FirstPersonCamera>)>,
) {
}

/// Fallback lighting system - adds a bright light if no photometric light exists.
/// Waits a few frames to give the L3D loader time to spawn its lights first.
fn ensure_visible_scene(
    mut commands: Commands,
    _settings: Res<SceneSettings>,
    point_lights: Query<Entity, With<PointLight>>,
    spot_lights: Query<Entity, With<SpotLight>>,
    l3d_lights: Query<Entity, With<l3d_loader::L3dLight>>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    // Wait a few frames for the L3D loader to finish spawning lights
    if *frame_count < 10 {
        return;
    }
    // Only run the check once (on frame 10)
    if *frame_count > 10 {
        return;
    }
    // Add fallback light only if no lights exist from any source
    if point_lights.is_empty() && spot_lights.is_empty() && l3d_lights.is_empty() {
        log("[Bevy] Adding fallback light (no lights in scene after L3D load)");
        commands.spawn((
            PointLight {
                color: Color::srgb(1.0, 0.95, 0.9), // Warm white
                intensity: 100000.0,
                radius: 0.1,
                range: 50.0,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_xyz(2.0, 2.5, 2.5),
        ));
    }
}

/// Run the 3D viewer on a specific canvas element (WASM)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_on_canvas(canvas_selector: &str) {
    console_error_panic_hook::set_once();
    log(&format!("[Bevy] Starting on canvas: {}", canvas_selector));

    // Load data from localStorage BEFORE building the app
    let l3d_data = load_l3d_from_storage();
    let ldt_data = load_ldt_from_storage();
    let emitter_config = load_emitter_config_from_storage();
    let mounting_config = load_mounting_config_from_storage();
    let housing_color_rgb = load_housing_color_from_storage();

    log(&format!(
        "[Bevy] Initial load - L3D: {} bytes, LDT: {}, Emitters: {}, Mounting: {:?}, Housing: {:?}",
        l3d_data.as_ref().map(|d| d.len()).unwrap_or(0),
        ldt_data.is_some(),
        emitter_config.len(),
        mounting_config.mounting_type,
        housing_color_rgb
    ));

    // Create settings with LDT data
    // Disable show_luminaire since we have L3D model
    let has_l3d = l3d_data.is_some();
    let settings = SceneSettings {
        ldc_data: ldt_data.clone(),
        show_luminaire: !has_l3d, // Don't show fake luminaire if we have L3D
        ..default()
    };

    let scene_data = GldfSceneData {
        l3d_data,
        ldt_data,
        scene_type: SceneType::Room,
        emitter_config,
        mounting: mounting_config,
        housing_color_rgb,
    };

    // Build app with resources inserted FIRST, before ANY plugins
    let mut app = App::new();

    // Insert resources BEFORE any plugins
    app.insert_resource(settings);
    app.insert_resource(scene_data);
    app.insert_resource(GldfTimestamp::default());

    // Now add plugins
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "GLDF 3D Viewer".to_string(),
            canvas: Some(canvas_selector.to_string()),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: false,
            ..default()
        }),
        ..default()
    }));

    app.add_plugins((
        EulumdatViewerPlugin::default(),
        l3d_loader::L3dLoaderPlugin,
        ifc_loader::IfcLoaderPlugin,
    ));

    app.add_systems(Update, ui_controls_system);
    app.add_systems(Update, poll_gldf_changes);
    app.add_systems(Update, ensure_visible_scene);
    app.add_systems(Update, frame_camera_on_l3d);
    app.add_systems(Update, print_camera_on_key);
    app.add_systems(Update, append_camera_set_on_key);
    #[cfg(target_arch = "wasm32")]
    {
        app.add_systems(Update, apply_ambient_request);
        app.add_systems(Update, apply_room_size_request);
        app.add_systems(Update, apply_surface_color_requests);
        app.add_systems(Update, apply_surface_reflectance_requests);
        app.add_systems(Update, apply_surface_emissive_requests);
        app.add_systems(Update, snapshot_room_state);
    }
    // Animation drivers MUST run BEFORE the snapshot appliers — they
    // write into the snapshot cells (`cam_request_cell` /
    // `joint_tilts_cell`) that the appliers then read this same frame.
    app.add_systems(Update, js_camera_anim_apply);
    app.add_systems(Update, js_joint_anim_apply);
    app.add_systems(Update, js_camera_bridge_apply);
    app.add_systems(Update, js_joint_bridge_apply);

    app.run();
}

/// Run the 3D viewer in a native window (desktop)
#[cfg(not(target_arch = "wasm32"))]
pub fn run_on_canvas(_canvas_selector: &str) {
    run_native();
}

// ---------- Native command-bridge request helpers ----------
//
// Phase 2: the editor forwards Lisp `SceneCommand`s to this viewer over
// stdin (NDJSON); `command_bridge::apply` calls these to write into the
// SAME request cells the WASM `*_from_js` setters use, so the (now shared)
// apply/animation systems drive the scene identically on both targets.

/// Snap the camera so its eye is at `eye` looking toward `target`.
/// Derivation matches `camera_animate_from_js` / `CAMERA-SET`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn request_camera_look_at(eye: [f32; 3], target: [f32; 3]) {
    let eye = Vec3::from(eye);
    let dir = Vec3::from(target) - eye;
    let (yaw, pitch) = if dir.length_squared() > 1e-6 {
        let n = dir.normalize();
        ((-n.x).atan2(-n.z), n.y.asin())
    } else {
        (0.0, 0.0)
    };
    if let Ok(mut slot) = cam_request_cell().lock() {
        *slot = Some(CamRequest {
            pos: eye,
            yaw,
            pitch,
        });
    }
}

/// Animate the camera from its current pose to `eye` looking at `target`
/// over `duration_ms`. `now` is a monotonic ms clock (native `now_ms()`).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn request_camera_animate(eye: [f32; 3], target: [f32; 3], duration_ms: f32, now: f64) {
    let to_pos = Vec3::from(eye);
    let dir = Vec3::from(target) - to_pos;
    let (to_yaw, to_pitch) = if dir.length_squared() > 1e-6 {
        let n = dir.normalize();
        ((-n.x).atan2(-n.z), n.y.asin())
    } else {
        (0.0, 0.0)
    };
    let snap = cam_snapshot_cell()
        .lock()
        .ok()
        .map(|s| *s)
        .unwrap_or_default();
    if let Ok(mut slot) = cam_anim_cell().lock() {
        *slot = Some(CamAnim {
            from_pos: snap.pos,
            from_yaw: snap.yaw,
            from_pitch: snap.pitch,
            to_pos,
            to_yaw,
            to_pitch,
            started_at_ms: now,
            duration_ms: duration_ms as f64,
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn request_joint_tilt(joint: &str, euler_deg: [f32; 3]) {
    if let Ok(mut map) = joint_tilts_cell().lock() {
        map.insert(joint.to_string(), euler_deg);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn request_joint_animate(joint: &str, euler_deg: [f32; 3], duration_ms: f32, now: f64) {
    let from_deg = joint_tilts_cell()
        .lock()
        .ok()
        .and_then(|m| m.get(joint).copied())
        .unwrap_or([0.0, 0.0, 0.0]);
    if let Ok(mut map) = joint_anims_cell().lock() {
        map.insert(
            joint.to_string(),
            JointAnim {
                from_deg,
                to_deg: euler_deg,
                started_at_ms: now,
                duration_ms: duration_ms as f64,
            },
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn request_reset_tilts() {
    if let Ok(mut m) = joint_tilts_cell().lock() {
        m.clear();
    }
    if let Ok(mut m) = joint_anims_cell().lock() {
        m.clear();
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn request_ambient(brightness: f32) {
    if let Ok(mut slot) = ambient_request_cell().lock() {
        *slot = Some(brightness.max(0.0));
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn request_surface_color(surface: &str, r: f32, g: f32, b: f32) {
    if let Ok(mut v) = surface_color_request_cell().lock() {
        v.push(SurfaceColorRequest {
            surface: surface.to_string(),
            r,
            g,
            b,
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn request_surface_reflectance(surface: &str, value: f32) {
    if let Ok(mut v) = surface_reflectance_request_cell().lock() {
        v.push(SurfaceReflectanceRequest {
            surface: surface.to_string(),
            value: value.clamp(0.0, 1.0),
        });
    }
}

/// Run the 3D viewer as a native window (desktop only)
#[cfg(not(target_arch = "wasm32"))]
pub fn run_native() {
    run_native_with_scene(GldfSceneData::default());
}

/// Native entry point that starts with a pre-populated scene (loaded from
/// a `.gldf` on disk by [`scene_bundle::load_scene_from_gldf`]). The
/// argv-parsing `main` uses this so `gldf-bevy-viewer <file> --variant N`
/// opens the luminaire immediately instead of an empty room.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_native_with_scene(scene_data: GldfSceneData) {
    // Initialize with default settings
    let settings = SceneSettings::default();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "GLDF 3D Viewer".to_string(),
                resolution: (1280u32, 720u32).into(),
                ..default()
            }),
            ..default()
        }))
        // Insert resources BEFORE adding plugins
        .insert_resource(settings)
        .insert_resource(scene_data)
        .insert_resource(GldfTimestamp::default())
        .insert_resource(command_bridge::CommandRx(std::sync::Mutex::new(
            command_bridge::spawn_stdin_reader(),
        )))
        .add_plugins((
            EulumdatViewerPlugin::default(),
            l3d_loader::L3dLoaderPlugin,
            ifc_loader::IfcLoaderPlugin,
        ))
        .add_systems(Update, ui_controls_system)
        .add_systems(Update, poll_gldf_changes)
        .add_systems(Update, ensure_visible_scene)
        .add_systems(Update, frame_camera_on_l3d)
        .add_systems(Update, print_camera_on_key)
        .add_systems(Update, append_camera_set_on_key)
        // Room-state appliers (native + wasm now share these).
        .add_systems(Update, apply_ambient_request)
        .add_systems(Update, apply_room_size_request)
        .add_systems(Update, apply_surface_color_requests)
        .add_systems(Update, apply_surface_reflectance_requests)
        .add_systems(Update, apply_surface_emissive_requests)
        .add_systems(Update, snapshot_room_state)
        // Animation drivers BEFORE the snapshot appliers (they write into
        // the request cells the appliers read this same frame).
        .add_systems(Update, js_camera_anim_apply)
        .add_systems(Update, js_joint_anim_apply)
        .add_systems(Update, js_camera_bridge_apply)
        .add_systems(Update, js_joint_bridge_apply)
        // Native command bridge: drain SceneCommands (from the editor's
        // stdin, NDJSON) into the request cells above.
        .add_systems(Update, command_bridge::drain_command_channel)
        .run();
}

#[cfg(target_arch = "wasm32")]
pub fn run_native() {
    run_on_canvas("#bevy-canvas");
}

/// WASM entry point - exported for manual calling
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn wasm_start() {
    log("[Bevy] wasm_start called");
    run_native();
}
