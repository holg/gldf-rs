//! GLDF Bevy 3D Scene Viewer Library
//!
//! **GitHub:** <https://github.com/holg/gldf-rs>
//!
//! This module provides a Bevy-based 3D viewer for GLDF files that displays
//! L3D 3D models with LDT photometric lighting in various scene types.
//! Also supports IFC geometry viewing from imported IFC files.

pub mod ifc_loader;
pub mod l3d_loader;
pub mod scene;

pub use ifc_loader::{IfcGeometry, IfcLoaderPlugin, IfcSceneData};

use bevy::prelude::*;
use eulumdat_bevy::viewer::{EulumdatViewerPlugin, FirstPersonCamera, SceneType};
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
                } else if self.recessed_depth_mm.is_some() {
                    // Recessed: flush with ceiling
                    (room_height, true)
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

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug)]
struct CamRequest {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, Default)]
struct CamSnapshot {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
    forward: Vec3,
}

#[cfg(target_arch = "wasm32")]
static CAM_REQUEST: std::sync::OnceLock<std::sync::Mutex<Option<CamRequest>>> =
    std::sync::OnceLock::new();

#[cfg(target_arch = "wasm32")]
static CAM_SNAPSHOT: std::sync::OnceLock<std::sync::Mutex<CamSnapshot>> =
    std::sync::OnceLock::new();

#[cfg(target_arch = "wasm32")]
fn cam_request_cell() -> &'static std::sync::Mutex<Option<CamRequest>> {
    CAM_REQUEST.get_or_init(|| std::sync::Mutex::new(None))
}

#[cfg(target_arch = "wasm32")]
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
#[cfg(target_arch = "wasm32")]
fn js_camera_bridge_apply(
    mut camera_q: Query<(&mut Transform, &mut FirstPersonCamera)>,
) {
    // Take any pending request.
    let req = cam_request_cell().lock().ok().and_then(|mut s| s.take());
    for (mut transform, mut camera) in camera_q.iter_mut() {
        if let Some(r) = req {
            transform.translation = r.pos;
            camera.yaw = r.yaw;
            camera.pitch = r.pitch.clamp(-1.5, 1.5);
            transform.rotation =
                Quat::from_euler(EulerRot::YXZ, camera.yaw, camera.pitch, 0.0);
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
    app.add_systems(Update, js_camera_bridge_apply);

    app.run();
}

/// Run the 3D viewer in a native window (desktop)
#[cfg(not(target_arch = "wasm32"))]
pub fn run_on_canvas(_canvas_selector: &str) {
    run_native();
}

/// Run the 3D viewer as a native window (desktop only)
#[cfg(not(target_arch = "wasm32"))]
pub fn run_native() {
    // Initialize with default settings
    let settings = SceneSettings::default();
    let scene_data = GldfSceneData::default();

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
