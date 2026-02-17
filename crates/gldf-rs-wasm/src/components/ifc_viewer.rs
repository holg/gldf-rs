//! IFC Viewer Component
//!
//! Main container for viewing imported IFC luminaire data with:
//! - 3D geometry view (via Bevy)
//! - Property tree
//! - Building context hierarchy

use gldf_rs::ifc::ImportedLuminaire;
use gloo::console::log;
use wasm_bindgen::prelude::*;
use yew::prelude::*;

use super::ifc_building_context::IfcBuildingContext;
use super::ifc_property_tree::IfcPropertyTree;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadBevyViewer)]
    fn load_bevy_viewer() -> js_sys::Promise;

    #[wasm_bindgen(js_name = isBevyLoaded)]
    fn is_bevy_loaded() -> bool;

    #[wasm_bindgen(js_name = isBevyLoading)]
    fn is_bevy_loading() -> bool;

    #[wasm_bindgen(js_name = saveIfcGeometryForBevy)]
    fn save_ifc_geometry_for_bevy(geometry_json: &str, variant_name: Option<&str>);

    #[wasm_bindgen(js_name = clearIfcGeometry)]
    fn clear_ifc_geometry();
}

/// Tab selection for IFC viewer
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum IfcViewerTab {
    ThreeD,
    Properties,
    BuildingContext,
}

#[derive(Properties, PartialEq, Clone)]
pub struct IfcViewerProps {
    pub luminaire: ImportedLuminaire,
    #[prop_or(0)]
    pub initial_variant: usize,
}

/// Loading state for the Bevy viewer
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BevyState {
    NotLoaded,
    Loading,
    Loaded,
    Error,
}

#[function_component(IfcViewer)]
pub fn ifc_viewer(props: &IfcViewerProps) -> Html {
    let active_tab = use_state(|| IfcViewerTab::ThreeD);
    let selected_variant = use_state(|| props.initial_variant);
    let bevy_state = use_state(|| {
        if is_bevy_loaded() {
            BevyState::Loaded
        } else if is_bevy_loading() {
            BevyState::Loading
        } else {
            BevyState::NotLoaded
        }
    });
    let error_msg = use_state(|| None::<String>);

    let luminaire = props.luminaire.clone();
    let variant_idx = *selected_variant;

    // Get current variant
    let current_variant = luminaire.variants.get(variant_idx);

    // Save IFC geometry to localStorage when variant changes
    {
        let luminaire = luminaire.clone();
        use_effect_with(variant_idx, move |_| {
            if let Some(variant) = luminaire.variants.get(variant_idx) {
                if let Some(ref geometry) = variant.geometry {
                    // Serialize geometry to JSON for localStorage
                    #[derive(serde::Serialize)]
                    struct GeometryJson {
                        vertices: Vec<(f64, f64, f64)>,
                        triangles: Vec<(u32, u32, u32)>,
                    }

                    let geom_json = GeometryJson {
                        vertices: geometry.vertices.clone(),
                        triangles: geometry.triangles.clone(),
                    };

                    if let Ok(json) = serde_json::to_string(&geom_json) {
                        log!(format!(
                            "[IfcViewer] Saving geometry for variant {}: {} vertices",
                            variant.name,
                            geometry.vertices.len()
                        ));
                        save_ifc_geometry_for_bevy(&json, Some(&variant.name));
                    }
                } else {
                    log!(format!(
                        "[IfcViewer] Variant {} has no geometry",
                        variant.name
                    ));
                    clear_ifc_geometry();
                }
            }
            || {}
        });
    }

    // Tab click handlers
    let on_tab_3d = {
        let active_tab = active_tab.clone();
        Callback::from(move |_| active_tab.set(IfcViewerTab::ThreeD))
    };
    let on_tab_props = {
        let active_tab = active_tab.clone();
        Callback::from(move |_| active_tab.set(IfcViewerTab::Properties))
    };
    let on_tab_context = {
        let active_tab = active_tab.clone();
        Callback::from(move |_| active_tab.set(IfcViewerTab::BuildingContext))
    };

    // Variant selection callback
    let on_variant_select = {
        let selected_variant = selected_variant.clone();
        Callback::from(move |idx: usize| selected_variant.set(idx))
    };

    // Start loading Bevy
    let start_bevy_loading = {
        let bevy_state = bevy_state.clone();
        let error_msg = error_msg.clone();

        Callback::from(move |_| {
            if *bevy_state != BevyState::NotLoaded {
                return;
            }

            bevy_state.set(BevyState::Loading);

            let bevy_state = bevy_state.clone();
            let error_msg = error_msg.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let promise = load_bevy_viewer();
                let result = wasm_bindgen_futures::JsFuture::from(promise).await;

                match result {
                    Ok(_) => {
                        bevy_state.set(BevyState::Loaded);
                    }
                    Err(e) => {
                        let msg = format!("{:?}", e);
                        // Check for fake control flow exception
                        if msg.contains("Using exceptions for control flow")
                            || msg.contains("don't mind me")
                        {
                            bevy_state.set(BevyState::Loaded);
                        } else {
                            log!(format!("[IfcViewer] Bevy load error: {}", msg));
                            error_msg.set(Some(msg));
                            bevy_state.set(BevyState::Error);
                        }
                    }
                }
            });
        })
    };

    // Retry loading
    let retry_loading = {
        let bevy_state = bevy_state.clone();
        let error_msg = error_msg.clone();
        Callback::from(move |_| {
            bevy_state.set(BevyState::NotLoaded);
            error_msg.set(None);
        })
    };

    // Check if current variant has geometry
    let has_geometry = current_variant
        .map(|v| v.geometry.is_some())
        .unwrap_or(false);

    html! {
        <div class="ifc-viewer">
            // Header with variant selector
            <div class="ifc-viewer-header">
                <h2 class="ifc-viewer-title">{"IFC Luminaire Viewer"}</h2>
                <div class="variant-selector">
                    <label>{"Variant: "}</label>
                    <select
                        value={variant_idx.to_string()}
                        onchange={
                            let selected = selected_variant.clone();
                            Callback::from(move |e: Event| {
                                if let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                                    if let Ok(idx) = target.value().parse::<usize>() {
                                        selected.set(idx);
                                    }
                                }
                            })
                        }
                    >
                        {for luminaire.variants.iter().enumerate().map(|(i, v)| {
                            html! {
                                <option value={i.to_string()} selected={i == variant_idx}>
                                    {&v.name}
                                    {v.geometry.as_ref().map(|_| " [3D]").unwrap_or("")}
                                </option>
                            }
                        })}
                    </select>
                </div>
            </div>

            // Tab bar
            <div class="ifc-viewer-tabs">
                <button
                    class={classes!("tab", (*active_tab == IfcViewerTab::ThreeD).then_some("active"))}
                    onclick={on_tab_3d}
                >
                    {"3D View"}
                    {if has_geometry {
                        html! { <span class="tab-badge">{"✓"}</span> }
                    } else {
                        html! {}
                    }}
                </button>
                <button
                    class={classes!("tab", (*active_tab == IfcViewerTab::Properties).then_some("active"))}
                    onclick={on_tab_props}
                >
                    {"Properties"}
                </button>
                <button
                    class={classes!("tab", (*active_tab == IfcViewerTab::BuildingContext).then_some("active"))}
                    onclick={on_tab_context}
                >
                    {"Building Context"}
                </button>
            </div>

            // Tab content
            <div class="ifc-viewer-content">
                {match *active_tab {
                    IfcViewerTab::ThreeD => {
                        if has_geometry {
                            html! {
                                <div class="ifc-3d-container">
                                    // Canvas for Bevy
                                    <canvas
                                        id="bevy-canvas"
                                        width="800"
                                        height="600"
                                        style="width: 100%; height: 100%; display: block; touch-action: none;"
                                    />

                                    // Loading overlay
                                    {match *bevy_state {
                                        BevyState::NotLoaded => html! {
                                            <div class="bevy-overlay" onclick={start_bevy_loading.clone()}>
                                                <div class="bevy-overlay-content">
                                                    <div class="bevy-icon">{"🏠"}</div>
                                                    <div class="bevy-title">{"IFC 3D Viewer"}</div>
                                                    <div class="bevy-subtitle">{"View imported IFC geometry"}</div>
                                                    <button class="btn btn-primary" onclick={start_bevy_loading}>
                                                        {"Load 3D Viewer"}
                                                    </button>
                                                </div>
                                            </div>
                                        },
                                        BevyState::Loading => html! {
                                            <div class="bevy-overlay">
                                                <div class="bevy-overlay-content">
                                                    <div class="bevy-spinner"></div>
                                                    <div class="bevy-title">{"Loading 3D Viewer..."}</div>
                                                    <div class="bevy-subtitle">{"This may take a moment"}</div>
                                                </div>
                                            </div>
                                        },
                                        BevyState::Loaded => html! {
                                            <div class="bevy-controls-hint">
                                                {"WASD: Move | Q/E: Up/Down | Right-click+drag: Look | R: Reset"}
                                            </div>
                                        },
                                        BevyState::Error => html! {
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
                                    }}
                                </div>
                            }
                        } else {
                            html! {
                                <div class="no-geometry-message">
                                    <div class="message-icon">{"📐"}</div>
                                    <div class="message-title">{"No 3D Geometry"}</div>
                                    <div class="message-text">
                                        {"This variant does not have tessellated geometry. "}
                                        {"Select a variant with [3D] tag or view Properties instead."}
                                    </div>
                                </div>
                            }
                        }
                    },
                    IfcViewerTab::Properties => html! {
                        <IfcPropertyTree
                            luminaire={luminaire.clone()}
                            selected_variant={variant_idx}
                        />
                    },
                    IfcViewerTab::BuildingContext => html! {
                        <IfcBuildingContext
                            luminaire={luminaire.clone()}
                            selected_variant={variant_idx}
                            on_variant_select={on_variant_select}
                        />
                    },
                }}
            </div>
        </div>
    }
}
