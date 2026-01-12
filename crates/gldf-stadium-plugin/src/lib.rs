//! Stadium Floodlight Visualization Plugin for GLDF
//!
//! This plugin renders a 3D stadium with floodlights positioned at the corners
//! using the eulumdat-bevy photometric light system.
//!
//! ## Features
//! - 4 or 6 tower configurations
//! - Adjustable floodlight tilt (C/V keys)
//! - Light intensity control (+/- keys)
//! - Camera navigation (WASD, arrows, scroll)
//! - Photometric data visualization from LDT files

pub mod stadium;

use bevy::prelude::*;
use eulumdat::Eulumdat;
use eulumdat_bevy::photometric::PhotometricPlugin;
use eulumdat_bevy::viewer::CameraPlugin;
pub use stadium::{StadiumPlugin, StadiumSettings, TowerLayout};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Storage keys for WASM localStorage
#[cfg(target_arch = "wasm32")]
pub const STADIUM_LDT_KEY: &str = "gldf_stadium_ldt";
#[cfg(target_arch = "wasm32")]
pub const STADIUM_CONFIG_KEY: &str = "gldf_stadium_config";

/// Resource to store GLDF scene data for the stadium
#[derive(Resource, Default)]
pub struct StadiumGldfData {
    /// The LDT photometric data (parsed)
    pub ldt_data: Option<eulumdat::Eulumdat>,
}

/// Log to console
#[cfg(target_arch = "wasm32")]
pub fn log(msg: &str) {
    web_sys::console::log_1(&msg.into());
}

#[cfg(not(target_arch = "wasm32"))]
pub fn log(msg: &str) {
    println!("{}", msg);
}

/// Load LDT from localStorage (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn load_ldt_from_storage() -> Option<eulumdat::Eulumdat> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let ldt_string = storage.get_item(STADIUM_LDT_KEY).ok()??;
    log(&format!(
        "[Stadium] Loading LDT from storage, len: {}",
        ldt_string.len()
    ));
    let parsed = eulumdat::Eulumdat::parse(&ldt_string).ok();
    if let Some(ref ldt) = parsed {
        log(&format!(
            "[Stadium] LDT parsed: {}, {} lm",
            ldt.luminaire_name,
            ldt.total_luminous_flux()
        ));
    }
    parsed
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_ldt_from_storage() -> Option<eulumdat::Eulumdat> {
    None
}

/// Load stadium config from localStorage (WASM only)
#[cfg(target_arch = "wasm32")]
pub fn load_config_from_storage() -> Option<StadiumSettings> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let config_json = storage.get_item(STADIUM_CONFIG_KEY).ok()??;
    log(&format!(
        "[Stadium] Loading config from storage: {}",
        &config_json[..config_json.len().min(100)]
    ));
    serde_json::from_str(&config_json).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_config_from_storage() -> Option<StadiumSettings> {
    None
}

/// Run the stadium viewer with a specific LDT file
#[cfg(not(target_arch = "wasm32"))]
pub fn run_native_with_ldt(ldt_path: Option<&str>) {
    let ldt_data = ldt_path.and_then(|path| {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                match eulumdat::Eulumdat::parse(&content) {
                    Ok(ldt) => {
                        println!("[Stadium] Loaded LDT: {}", path);
                        println!("[Stadium] Luminaire: {}", ldt.luminaire_name);
                        println!("[Stadium] Luminous flux: {} lm", ldt.total_luminous_flux());
                        if let Some(lamp) = ldt.lamp_sets.first() {
                            println!("[Stadium] Color temp: {}", lamp.color_appearance);
                        }
                        Some(ldt)
                    }
                    Err(e) => {
                        eprintln!("[Stadium] Failed to parse LDT: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("[Stadium] Failed to read LDT file: {}", e);
                None
            }
        }
    });

    let gldf_data = StadiumGldfData { ldt_data };
    let stadium_settings = StadiumSettings::default();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "GLDF Stadium Viewer".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(gldf_data)
        .insert_resource(stadium_settings)
        // Camera for navigation
        .add_plugins(CameraPlugin)
        // Photometric plugin for LDC visualization
        .add_plugins(PhotometricPlugin::<Eulumdat>::default())
        // Add our stadium plugin
        .add_plugins(StadiumPlugin)
        .run();
}

/// Run the stadium viewer as a native window (desktop only)
#[cfg(not(target_arch = "wasm32"))]
pub fn run_native() {
    run_native_with_ldt(None);
}

/// Run the stadium viewer on a specific canvas (WASM)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_on_canvas(canvas_selector: &str) {
    console_error_panic_hook::set_once();
    log(&format!("[Stadium] Starting on canvas: {}", canvas_selector));

    // Load data from localStorage
    let ldt_data = load_ldt_from_storage();
    let stadium_settings = load_config_from_storage().unwrap_or_default();

    log(&format!(
        "[Stadium] Initial load - LDT: {}, Layout: {:?}",
        ldt_data.is_some(),
        stadium_settings.tower_layout
    ));

    let gldf_data = StadiumGldfData { ldt_data };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "GLDF Stadium Viewer".to_string(),
                canvas: Some(canvas_selector.to_string()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(gldf_data)
        .insert_resource(stadium_settings)
        // Camera for navigation
        .add_plugins(CameraPlugin)
        // Photometric plugin for LDC visualization
        .add_plugins(PhotometricPlugin::<Eulumdat>::default())
        // Add our stadium plugin
        .add_plugins(StadiumPlugin)
        .run();
}

#[cfg(target_arch = "wasm32")]
pub fn run_native() {
    run_on_canvas("#stadium-canvas");
}

#[cfg(target_arch = "wasm32")]
pub fn run_native_with_ldt(_ldt_path: Option<&str>) {
    run_on_canvas("#stadium-canvas");
}

/// WASM entry point - exported for manual calling
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_start() {
    log("[Stadium] wasm_start called");
    run_native();
}

/// Store LDT data in localStorage for WASM use
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn store_ldt_data(ldt_content: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("No window")?;
    let storage = window.local_storage()?.ok_or("No localStorage")?;
    storage.set_item(STADIUM_LDT_KEY, ldt_content)?;
    log(&format!("[Stadium] Stored LDT data, {} bytes", ldt_content.len()));
    Ok(())
}

/// Store stadium config in localStorage for WASM use
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn store_config(config_json: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("No window")?;
    let storage = window.local_storage()?.ok_or("No localStorage")?;
    storage.set_item(STADIUM_CONFIG_KEY, config_json)?;
    log(&format!("[Stadium] Stored config, {} bytes", config_json.len()));
    Ok(())
}
