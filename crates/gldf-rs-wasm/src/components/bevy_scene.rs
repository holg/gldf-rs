//! Bevy 3D Scene Viewer Component
//!
//! Lazy-loads the Bevy WASM module and displays L3D models with LDT lighting.

use gloo::console::log;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use yew::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadBevyViewer)]
    fn load_bevy_viewer() -> js_sys::Promise;

    #[wasm_bindgen(js_name = isBevyLoaded)]
    fn is_bevy_loaded() -> bool;

    #[wasm_bindgen(js_name = isBevyLoading)]
    fn is_bevy_loading() -> bool;

    #[wasm_bindgen(js_name = saveL3dForBevy)]
    fn save_l3d_for_bevy(l3d_data: &js_sys::Uint8Array, ldt_data: Option<&str>, emitter_json: Option<&str>);
}

/// Per-emitter rendering data (serializable for localStorage)
#[derive(Clone, Debug, PartialEq, Default, Serialize)]
pub struct EmitterConfig {
    /// LEO name from L3D
    pub leo_name: String,
    /// Luminous flux in lumens
    pub luminous_flux: Option<i32>,
    /// Color temperature in Kelvin
    pub color_temperature: Option<i32>,
    /// Emergency behavior
    pub emergency_behavior: Option<String>,
}

/// Loading state for the Bevy viewer
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BevyLoadState {
    NotLoaded,
    Loading,
    Loaded,
    Error,
}

#[derive(Properties, PartialEq)]
pub struct BevySceneViewerProps {
    /// L3D file data
    pub l3d_data: Vec<u8>,
    /// Optional LDT data for lighting
    #[prop_or_default]
    pub ldt_data: Option<Vec<u8>>,
    /// Per-emitter configuration (flux, color temp)
    #[prop_or_default]
    pub emitter_config: Vec<EmitterConfig>,
    /// Variant ID (used to trigger reload when switching variants)
    #[prop_or_default]
    pub variant_id: String,
    #[prop_or(800)]
    pub width: u32,
    #[prop_or(600)]
    pub height: u32,
}

#[function_component(BevySceneViewer)]
pub fn bevy_scene_viewer(props: &BevySceneViewerProps) -> Html {
    let load_state = use_state(|| {
        if is_bevy_loaded() {
            BevyLoadState::Loaded
        } else if is_bevy_loading() {
            BevyLoadState::Loading
        } else {
            BevyLoadState::NotLoaded
        }
    });
    let error_msg = use_state(|| None::<String>);

    // Save L3D data to localStorage when component mounts or variant changes
    {
        let l3d_data = props.l3d_data.clone();
        let ldt_data = props.ldt_data.clone();
        let emitter_config = props.emitter_config.clone();
        let variant_id = props.variant_id.clone();

        // Use variant_id as the key to ensure effect re-runs when switching variants
        use_effect_with(variant_id.clone(), move |_| {
            if !l3d_data.is_empty() {
                log!(format!("[BevyScene] Saving data for variant: {}", variant_id));
                let js_array = js_sys::Uint8Array::from(l3d_data.as_slice());
                let ldt_str = ldt_data.as_ref().and_then(|data| {
                    std::str::from_utf8(data).ok()
                });
                // Serialize emitter config to JSON
                let emitter_json = if !emitter_config.is_empty() {
                    serde_json::to_string(&emitter_config).ok()
                } else {
                    None
                };
                log!(format!("[BevyScene] Emitter config: {:?}", emitter_json));
                save_l3d_for_bevy(&js_array, ldt_str, emitter_json.as_deref());
            }
            || {}
        });
    }

    // Trigger loading
    let start_loading = {
        let load_state = load_state.clone();
        let error_msg = error_msg.clone();
        Callback::from(move |_| {
            if *load_state != BevyLoadState::NotLoaded {
                return;
            }

            load_state.set(BevyLoadState::Loading);

            let load_state = load_state.clone();
            let error_msg = error_msg.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let promise = load_bevy_viewer();
                let result = wasm_bindgen_futures::JsFuture::from(promise).await;

                match result {
                    Ok(_) => {
                        load_state.set(BevyLoadState::Loaded);
                    }
                    Err(e) => {
                        let msg = format!("{:?}", e);
                        log!(format!("[BevyScene] Load error: {}", msg));
                        // Check if it's the fake "control flow" error
                        if msg.contains("Using exceptions for control flow") ||
                           msg.contains("don't mind me") {
                            load_state.set(BevyLoadState::Loaded);
                        } else {
                            error_msg.set(Some(msg));
                            load_state.set(BevyLoadState::Error);
                        }
                    }
                }
            });
        })
    };

    // Retry loading
    let retry_loading = {
        let load_state = load_state.clone();
        let error_msg = error_msg.clone();
        Callback::from(move |_| {
            load_state.set(BevyLoadState::NotLoaded);
            error_msg.set(None);
        })
    };

    html! {
        <div class="bevy-scene-container" style={format!("width: {}px; height: {}px; position: relative;", props.width, props.height)}>
            // Canvas for Bevy to render into
            <canvas
                id="bevy-canvas"
                width={props.width.to_string()}
                height={props.height.to_string()}
                style="width: 100%; height: 100%; display: block; touch-action: none;"
            />

            // Loading overlay
            {
                match *load_state {
                    BevyLoadState::NotLoaded => html! {
                        <div class="bevy-overlay" onclick={start_loading.clone()}>
                            <div class="bevy-overlay-content">
                                <div class="bevy-icon">{"🏠"}</div>
                                <div class="bevy-title">{"3D Scene Viewer"}</div>
                                <div class="bevy-subtitle">{"View L3D model with photometric lighting"}</div>
                                <button class="btn btn-primary" onclick={start_loading}>
                                    {"Load 3D Viewer"}
                                </button>
                            </div>
                        </div>
                    },
                    BevyLoadState::Loading => html! {
                        <div class="bevy-overlay">
                            <div class="bevy-overlay-content">
                                <div class="bevy-spinner"></div>
                                <div class="bevy-title">{"Loading 3D Viewer..."}</div>
                                <div class="bevy-subtitle">{"This may take a moment"}</div>
                            </div>
                        </div>
                    },
                    BevyLoadState::Loaded => html! {
                        // Bevy is rendering, show controls hint
                        <div class="bevy-controls-hint">
                            {"WASD: Move | Q/E: Up/Down | Right-click+drag: Look | R: Reset | 1-4: Scene types"}
                        </div>
                    },
                    BevyLoadState::Error => html! {
                        <div class="bevy-overlay error">
                            <div class="bevy-overlay-content">
                                <div class="bevy-icon">{"❌"}</div>
                                <div class="bevy-title">{"Failed to load 3D Viewer"}</div>
                                <div class="bevy-error">{(*error_msg).clone().unwrap_or_default()}</div>
                                <button class="btn btn-primary" onclick={retry_loading}>
                                    {"Try Again"}
                                </button>
                            </div>
                        </div>
                    },
                }
            }
        </div>
    }
}
