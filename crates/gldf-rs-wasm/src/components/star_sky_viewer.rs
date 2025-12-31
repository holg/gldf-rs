//! Star Sky 3D Viewer Component
//!
//! Displays a 3D star sky visualization using the Bevy engine.
//! Loads star position data from JSON and renders them on a celestial dome.

use gloo::console::log;
use wasm_bindgen::prelude::*;
use yew::prelude::*;

use super::bevy_scene::BevyLoadState;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadBevyViewer)]
    fn load_bevy_viewer() -> js_sys::Promise;

    #[wasm_bindgen(js_name = isBevyLoaded)]
    fn is_bevy_loaded() -> bool;

    #[wasm_bindgen(js_name = isBevyLoading)]
    fn is_bevy_loading() -> bool;

    #[wasm_bindgen(js_name = saveStarSkyForBevy)]
    fn save_star_sky_for_bevy(json_data: &str);

    #[wasm_bindgen(js_name = clearStarSkyData)]
    fn clear_star_sky_data();

    #[wasm_bindgen(js_name = highlightStarInViewer)]
    fn highlight_star_in_viewer(star_name: &str) -> bool;
}

#[derive(Properties, PartialEq)]
pub struct StarSkyViewerProps {
    /// Star sky JSON data (full JSON string)
    pub star_json: String,
    /// Location name for display
    #[prop_or_default]
    pub location_name: String,
    /// Star name to highlight (optional)
    #[prop_or_default]
    pub highlight_star: Option<String>,
    #[prop_or(800)]
    pub width: u32,
    #[prop_or(600)]
    pub height: u32,
}

#[function_component(StarSkyViewer)]
pub fn star_sky_viewer(props: &StarSkyViewerProps) -> Html {
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

    // Save star JSON to localStorage when component mounts
    {
        let star_json = props.star_json.clone();
        let location_name = props.location_name.clone();

        use_effect_with(star_json.clone(), move |_| {
            if !star_json.is_empty() {
                log!(format!(
                    "[StarSky] Saving star data for: {}, {} chars",
                    location_name,
                    star_json.len()
                ));
                save_star_sky_for_bevy(&star_json);
            }
            || {
                // Cleanup: clear star data when component unmounts
                clear_star_sky_data();
            }
        });
    }

    // Highlight star when prop changes
    {
        let highlight_star = props.highlight_star.clone();
        use_effect_with(highlight_star.clone(), move |_| {
            if let Some(ref star_name) = highlight_star {
                log!(format!("[StarSky] Highlighting star: {}", star_name));
                // Small delay to ensure viewer is loaded
                let star_name = star_name.clone();
                gloo::timers::callback::Timeout::new(500, move || {
                    highlight_star_in_viewer(&star_name);
                }).forget();
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
                        log!(format!("[StarSky] Load error: {}", msg));
                        // Check if it's the fake "control flow" error
                        if msg.contains("Using exceptions for control flow")
                            || msg.contains("don't mind me")
                        {
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
        <div class="star-sky-container" style={format!("width: {}px; height: {}px; position: relative;", props.width, props.height)}>
            // Canvas for Bevy to render into
            <canvas
                id="bevy-canvas"
                width={props.width.to_string()}
                height={props.height.to_string()}
                style="width: 100%; height: 100%; display: block; touch-action: none; background: #000;"
            />

            // Loading overlay
            {
                match *load_state {
                    BevyLoadState::NotLoaded => html! {
                        <div class="bevy-overlay star-sky" onclick={start_loading.clone()}>
                            <div class="bevy-overlay-content">
                                <div class="bevy-icon">{"⭐"}</div>
                                <div class="bevy-title">{"Star Sky Viewer"}</div>
                                <div class="bevy-subtitle">{format!("View night sky from {}", props.location_name)}</div>
                                <button class="btn btn-primary" onclick={start_loading}>
                                    {"Load Star Sky"}
                                </button>
                            </div>
                        </div>
                    },
                    BevyLoadState::Loading => html! {
                        <div class="bevy-overlay star-sky">
                            <div class="bevy-overlay-content">
                                <div class="bevy-spinner"></div>
                                <div class="bevy-title">{"Loading Star Sky..."}</div>
                                <div class="bevy-subtitle">{"Rendering thousands of stars"}</div>
                            </div>
                        </div>
                    },
                    BevyLoadState::Loaded => html! {
                        // Bevy is rendering, show controls hint
                        <div class="bevy-controls-hint star-sky">
                            {"WASD: Move | Q/E: Up/Down | Right-click+drag: Look | R: Reset"}
                        </div>
                    },
                    BevyLoadState::Error => html! {
                        <div class="bevy-overlay error star-sky">
                            <div class="bevy-overlay-content">
                                <div class="bevy-icon">{"❌"}</div>
                                <div class="bevy-title">{"Failed to load Star Sky"}</div>
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
