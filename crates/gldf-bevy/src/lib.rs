//! GLDF Bevy 3D Scene Viewer Library
//!
//! **GitHub:** <https://github.com/holg/gldf-rs>
//!
//! This module provides a Bevy-based 3D viewer for GLDF files that displays
//! L3D 3D models with LDT photometric lighting in various scene types.

pub mod l3d_loader;
pub mod scene;
pub mod star_sky;

use bevy::prelude::*;
use eulumdat_bevy::photometric::PhotometricPlugin;
use eulumdat_bevy::viewer::{CameraPlugin, ScenePlugin, SceneType};
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
pub const GLDF_TIMESTAMP_KEY: &str = "gldf_timestamp";
#[cfg(target_arch = "wasm32")]
pub const STAR_SKY_STORAGE_KEY: &str = "gldf_star_sky_json";

/// Per-emitter configuration from GLDF
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct EmitterConfig {
    /// LEO name from L3D structure.xml
    pub leo_name: String,
    /// Luminous flux in lumens
    pub luminous_flux: Option<i32>,
    /// Color temperature in Kelvin
    pub color_temperature: Option<i32>,
    /// Emergency behavior
    pub emergency_behavior: Option<String>,
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

/// Check if star sky data exists in localStorage
#[cfg(target_arch = "wasm32")]
pub fn has_star_sky_data() -> bool {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };
    let storage = match window.local_storage().ok().flatten() {
        Some(s) => s,
        None => return false,
    };
    storage.get_item(STAR_SKY_STORAGE_KEY).ok().flatten().is_some()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn has_star_sky_data() -> bool {
    false
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
                    settings.ldt_data = Some(ldt);
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

/// Fallback lighting system - adds a bright light if no photometric light exists
fn ensure_visible_scene(
    mut commands: Commands,
    settings: Res<SceneSettings>,
    lights: Query<Entity, With<PointLight>>,
) {
    // Only add fallback light if no LDT data and no existing lights
    if settings.ldt_data.is_none() && lights.is_empty() {
        log("[Bevy] Adding fallback light (no LDT data)");
        // Add a bright point light at the center of the room
        commands.spawn((
            PointLight {
                color: Color::srgb(1.0, 0.95, 0.9), // Warm white
                intensity: 100000.0,                // Bright enough to see
                radius: 0.1,
                range: 50.0,
                shadows_enabled: true,
                ..default()
            },
            Transform::from_xyz(2.0, 2.5, 2.5),
        ));
    }
}

/// Run the 3D viewer on a specific canvas element (WASM)
#[cfg(all(target_arch = "wasm32", feature = "standalone"))]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_on_canvas(canvas_selector: &str) {
    console_error_panic_hook::set_once();
    log(&format!("[Bevy] Starting on canvas: {}", canvas_selector));

    // Load data from localStorage BEFORE building the app
    let l3d_data = load_l3d_from_storage();
    let ldt_data = load_ldt_from_storage();
    let emitter_config = load_emitter_config_from_storage();

    log(&format!(
        "[Bevy] Initial load - L3D: {} bytes, LDT: {}, Emitters: {}",
        l3d_data.as_ref().map(|d| d.len()).unwrap_or(0),
        ldt_data.is_some(),
        emitter_config.len()
    ));

    // Create settings with LDT data
    // Disable show_luminaire and pendulum since we have L3D model
    let has_l3d = l3d_data.is_some();
    let settings = SceneSettings {
        ldt_data: ldt_data.clone(),
        show_luminaire: !has_l3d, // Don't show fake luminaire if we have L3D
        pendulum_length: if has_l3d { 0.0 } else { 0.3 }, // No pendulum with L3D
        ..default()
    };

    // Check if we have star sky data (for astral sky demo)
    let has_star_sky = has_star_sky_data();

    // Use Outdoor scene for star sky (no room walls), Room for luminaires
    let scene_type = if has_star_sky && !has_l3d {
        log("[Bevy] Star sky mode - using Outdoor scene");
        SceneType::Outdoor
    } else {
        SceneType::Room
    };

    let scene_data = GldfSceneData {
        l3d_data,
        ldt_data,
        scene_type,
        emitter_config,
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

    app.add_plugins((CameraPlugin, ScenePlugin, l3d_loader::L3dLoaderPlugin));

    // Only add PhotometricPlugin if we don't have L3D with emitters
    // (L3D loader handles its own lights from LEO positions)
    if !has_l3d {
        app.add_plugins(PhotometricPlugin::<eulumdat::Eulumdat>::default());
    }

    // Add StarSkyPlugin for rendering star sky
    app.add_plugins(star_sky::StarSkyPlugin);

    app.add_systems(Update, ui_controls_system);
    app.add_systems(Update, poll_gldf_changes);
    app.add_systems(Update, ensure_visible_scene);

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
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        // Insert resources BEFORE adding plugins
        .insert_resource(settings)
        .insert_resource(scene_data)
        .insert_resource(GldfTimestamp::default())
        .add_plugins((
            CameraPlugin,
            ScenePlugin,
            PhotometricPlugin::<eulumdat::Eulumdat>::default(),
            l3d_loader::L3dLoaderPlugin,
        ))
        .add_systems(Update, ui_controls_system)
        .add_systems(Update, poll_gldf_changes)
        .add_systems(Update, ensure_visible_scene)
        .run();
}

/// WASM entry point with standalone feature
#[cfg(all(target_arch = "wasm32", feature = "standalone"))]
pub fn run_native() {
    run_on_canvas("#bevy-canvas");
}

/// WASM stub without standalone feature
#[cfg(all(target_arch = "wasm32", not(feature = "standalone")))]
pub fn run_native() {
    // No-op when standalone is not enabled
    log("[Bevy] run_native called but standalone feature not enabled");
}

/// WASM entry point - exported for manual calling from JavaScript
/// Use this when embedding gldf-bevy in another WASM app
#[cfg(all(target_arch = "wasm32", feature = "standalone"))]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn start_gldf_viewer() {
    log("[Bevy] start_gldf_viewer called");
    run_native();
}
