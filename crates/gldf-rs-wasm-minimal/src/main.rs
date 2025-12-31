#![recursion_limit = "256"]
//! GLDF Viewer - Plugin-based Architecture
//!
//! A viewer that loads functionality from embedded WASM plugins within GLDF files.
//! The GLDF becomes self-contained with both data AND the tools to visualize it.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use gldf_rs::gldf::GldfProduct;
use gldf_rs::{BufFile, FileBufGldf};
use gloo::console;
use gloo::file::callbacks::FileReader;
use gloo::file::File;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{FileList, HtmlInputElement};
use yew::prelude::*;

use gldf_rs_wasm_minimal::js_bindings::{
    call_plugin_function_1, call_plugin_function_3, has_plugin, register_wasm_plugin_js,
};

/// WASM wrapper for GldfProduct loading
struct WasmGldfProduct;

impl WasmGldfProduct {
    pub fn load_gldf_from_buf_all(buf: Vec<u8>) -> anyhow::Result<FileBufGldf> {
        GldfProduct::load_gldf_from_buf_all(buf)
    }
}

/// Plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginManifest {
    id: String,
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: String,
    js: String,
    wasm: String,
    #[serde(default)]
    constructor: Option<String>,
    #[serde(default)]
    capabilities: PluginCapabilities,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PluginCapabilities {
    #[serde(default)]
    functions: HashMap<String, FunctionInfo>,
    #[serde(default, rename = "inputFormats")]
    input_formats: Vec<String>,
    #[serde(default, rename = "outputFormats")]
    output_formats: Vec<String>,
    #[serde(default)]
    diagrams: Vec<String>,
    #[serde(default)]
    ui: PluginUiConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct FunctionInfo {
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    returns: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PluginUiConfig {
    #[serde(default)]
    icon: String,
    #[serde(default)]
    color: String,
    #[serde(default, rename = "primaryAction")]
    primary_action: String,
}

/// Loaded plugin info
#[derive(Clone, PartialEq, Debug)]
struct LoadedPlugin {
    id: String,
    name: String,
    icon: String,
    color: String,
    diagrams: Vec<String>,
    functions: Vec<String>,
}

/// Extracted file from GLDF
#[derive(Clone, PartialEq, Debug)]
struct ExtractedFile {
    name: String,
    path: String,
    content: String,
    file_type: String,
}

/// Product info extracted from GLDF
#[derive(Clone, PartialEq, Debug, Default)]
struct ProductInfo {
    name: String,
    manufacturer: String,
    description: String,
    image_data_url: Option<String>,
}

/// Application state
#[derive(Clone, PartialEq, Debug)]
struct AppState {
    file_name: Option<String>,
    product_info: ProductInfo,
    plugins: Vec<LoadedPlugin>,
    extracted_files: Vec<ExtractedFile>,
    active_plugin: Option<String>,
    active_view: String,
    active_file: Option<String>,
    svg_output: Option<String>,
    json_output: Option<String>,
    error: Option<String>,
    loading: bool,
    show_info: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            file_name: None,
            product_info: ProductInfo::default(),
            plugins: vec![],
            extracted_files: vec![],
            active_plugin: None,
            active_view: "polar".to_string(),
            active_file: None,
            svg_output: None,
            json_output: None,
            error: None,
            loading: false,
            show_info: true,
        }
    }
}

fn find_file_content(files: &[BufFile], path: &str) -> Option<Vec<u8>> {
    files
        .iter()
        .find(|f| f.path.as_ref().map(|p| p == path).unwrap_or(false))
        .and_then(|f| f.content.clone())
}

fn find_file_string(files: &[BufFile], path: &str) -> Option<String> {
    find_file_content(files, path).and_then(|c| String::from_utf8(c).ok())
}

/// Extract product info from GLDF
fn extract_product_info(gldf: &FileBufGldf, files: &[BufFile]) -> ProductInfo {
    let mut info = ProductInfo::default();

    // Get manufacturer from header
    let header = &gldf.gldf.header;
    if !header.manufacturer.is_empty() {
        info.manufacturer = header.manufacturer.clone();
    }

    // Try to get product name from ProductDefinitions
    let product_defs = &gldf.gldf.product_definitions;
    if let Some(product_meta) = &product_defs.product_meta_data {
        if let Some(name) = &product_meta.name {
            // LocaleFoo has a locale Vec<Locale>
            if let Some(first_locale) = name.locale.first() {
                if !first_locale.value.is_empty() {
                    info.name = first_locale.value.clone();
                }
            }
        }
    }

    // Find product image
    for file in files {
        if let Some(path) = &file.path {
            let lower = path.to_lowercase();
            if (lower.contains("image/") || lower.contains("product"))
                && (lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".png"))
            {
                if let Some(content) = &file.content {
                    let mime = if lower.ends_with(".png") {
                        "image/png"
                    } else {
                        "image/jpeg"
                    };
                    let b64 = BASE64.encode(content);
                    info.image_data_url = Some(format!("data:{};base64,{}", mime, b64));
                    break;
                }
            }
        }
    }

    info
}

/// Extract and register embedded plugins from GLDF
async fn extract_and_register_plugins(files: &[BufFile]) -> Vec<LoadedPlugin> {
    let mut loaded = vec![];

    let plugin_dirs: Vec<String> = files
        .iter()
        .filter_map(|f| {
            let path = f.path.as_ref()?;
            if path.starts_with("other/viewer/") && path.ends_with("/manifest.json") {
                let parts: Vec<&str> = path.split('/').collect();
                if parts.len() >= 3 {
                    return Some(parts[2].to_string());
                }
            }
            None
        })
        .collect();

    for plugin_id in plugin_dirs {
        let manifest_path = format!("other/viewer/{}/manifest.json", plugin_id);

        let manifest_json = match find_file_string(files, &manifest_path) {
            Some(json) => json,
            None => continue,
        };

        let manifest: PluginManifest = match serde_json::from_str(&manifest_json) {
            Ok(m) => m,
            Err(e) => {
                console::error!(format!("Failed to parse manifest for {}: {:?}", plugin_id, e));
                continue;
            }
        };

        let js_files = js_sys::Object::new();
        let base_path = format!("other/viewer/{}/", plugin_id);

        for file in files {
            if let Some(path) = &file.path {
                if path.starts_with(&base_path) {
                    let filename = path.strip_prefix(&base_path).unwrap_or(path);
                    if let Some(content) = &file.content {
                        let arr = js_sys::Uint8Array::from(content.as_slice());
                        js_sys::Reflect::set(&js_files, &JsValue::from_str(filename), &arr).ok();
                    }
                }
            }
        }

        console::log!(format!("Loading plugin: {} ({})", manifest.name, plugin_id));
        match register_wasm_plugin_js(&plugin_id, &manifest_json, js_files.into()).await {
            Ok(_) => {
                console::log!(format!("Plugin {} loaded", plugin_id));
                loaded.push(LoadedPlugin {
                    id: manifest.id.clone(),
                    name: manifest.name.clone(),
                    icon: manifest.capabilities.ui.icon.clone(),
                    color: manifest.capabilities.ui.color.clone(),
                    diagrams: manifest.capabilities.diagrams.clone(),
                    functions: manifest.capabilities.functions.keys().cloned().collect(),
                });
            }
            Err(e) => {
                console::error!(format!("Failed to load plugin {}: {:?}", plugin_id, e));
            }
        }
    }

    loaded
}

/// Extract LDT/IES files from GLDF
fn extract_photometry_files(files: &[BufFile]) -> Vec<ExtractedFile> {
    let mut extracted = vec![];

    for file in files {
        if let (Some(path), Some(content)) = (&file.path, &file.content) {
            let lower = path.to_lowercase();
            if lower.ends_with(".ldt") || lower.ends_with(".ies") {
                if let Ok(text) = String::from_utf8(content.clone()) {
                    let file_type = if lower.ends_with(".ldt") { "ldt" } else { "ies" };
                    let name = path.split('/').last().unwrap_or(path).to_string();
                    extracted.push(ExtractedFile {
                        name,
                        path: path.clone(),
                        content: text,
                        file_type: file_type.to_string(),
                    });
                }
            }
        }
    }

    extracted
}

/// Main application component
#[function_component(App)]
fn app() -> Html {
    let state = use_state(AppState::default);
    let readers = use_mut_ref(|| HashMap::<String, FileReader>::new());

    let ondrop = {
        let state = state.clone();
        let readers = readers.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            if let Some(files) = e.data_transfer().and_then(|dt| dt.files()) {
                process_files(files, state.clone(), readers.clone());
            }
        })
    };

    let ondragover = Callback::from(|e: DragEvent| {
        e.prevent_default();
    });

    let onchange = {
        let state = state.clone();
        let readers = readers.clone();
        Callback::from(move |e: Event| {
            if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                if let Some(files) = input.files() {
                    process_files(files, state.clone(), readers.clone());
                }
            }
        })
    };

    let on_view_change = {
        let state = state.clone();
        Callback::from(move |view: String| {
            let mut new_state = (*state).clone();
            new_state.active_view = view;
            new_state.svg_output = None;
            state.set(new_state);
        })
    };

    let _toggle_info = {
        let state = state.clone();
        Callback::from(move |_: MouseEvent| {
            let mut new_state = (*state).clone();
            new_state.show_info = !new_state.show_info;
            state.set(new_state);
        })
    };

    // Generate diagram when view or file changes
    {
        let state_clone = state.clone();
        let active_view = state.active_view.clone();
        let active_plugin = state.active_plugin.clone();
        let active_file = state.active_file.clone();
        let extracted_files = state.extracted_files.clone();

        use_effect_with(
            (active_view.clone(), active_plugin.clone(), active_file.clone()),
            move |_| {
                if let (Some(plugin_id), Some(file_name)) = (&active_plugin, &active_file) {
                    if has_plugin(plugin_id) {
                        if let Some(file) = extracted_files.iter().find(|f| &f.name == file_name) {
                            let parse_fn = if file.file_type == "ldt" { "parse_ldt" } else { "parse_ies" };

                            match call_plugin_function_1(plugin_id, parse_fn, &file.content) {
                                Ok(_) => {
                                    let fn_name = format!("{}_svg", active_view);
                                    match call_plugin_function_3(plugin_id, &fn_name, 500.0, 400.0, "light") {
                                        Ok(svg) => {
                                            let mut new_state = (*state_clone).clone();
                                            new_state.svg_output = svg.as_string();
                                            state_clone.set(new_state);
                                        }
                                        Err(e) => {
                                            console::error!(format!("Failed to generate {}: {:?}", fn_name, e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    console::error!(format!("Failed to parse: {:?}", e));
                                }
                            }
                        }
                    }
                }
                || ()
            },
        );
    }

    html! {
        <div class="app" ondrop={ondrop} ondragover={ondragover}>
            <style>{CSS}</style>

            if state.loading {
                <div class="loading">
                    <div class="spinner"></div>
                    <p>{"Loading GLDF..."}</p>
                </div>
            } else if state.file_name.is_none() {
                // Landing page
                <div class="landing">
                    <div class="hero">
                        <h1>{"GLDF Viewer"}</h1>
                        <p class="subtitle">{"Self-contained lighting data visualization"}</p>
                    </div>
                    <div class="drop-zone">
                        <div class="drop-icon">{"📦"}</div>
                        <p>{"Drop a GLDF file here"}</p>
                        <p class="or">{"or"}</p>
                        <label class="file-button">
                            {"Choose File"}
                            <input type="file" accept=".gldf" onchange={onchange} />
                        </label>
                    </div>
                    <div class="features">
                        <div class="feature">
                            <span class="feature-icon">{"🔌"}</span>
                            <h3>{"Plugin Architecture"}</h3>
                            <p>{"GLDF files contain embedded viewers"}</p>
                        </div>
                        <div class="feature">
                            <span class="feature-icon">{"📊"}</span>
                            <h3>{"Photometry"}</h3>
                            <p>{"Polar, Cartesian, BUG diagrams"}</p>
                        </div>
                        <div class="feature">
                            <span class="feature-icon">{"🎨"}</span>
                            <h3>{"Self-Contained"}</h3>
                            <p>{"No external dependencies"}</p>
                        </div>
                    </div>
                </div>
            } else {
                // Main viewer
                <div class="viewer">
                    // Header
                    <header class="header">
                        <div class="header-left">
                            <button class="back-btn" onclick={
                                let state = state.clone();
                                Callback::from(move |_| state.set(AppState::default()))
                            }>{"← Back"}</button>
                            <h1 class="product-name">
                                {if state.product_info.name.is_empty() {
                                    state.file_name.clone().unwrap_or_default()
                                } else {
                                    state.product_info.name.clone()
                                }}
                            </h1>
                        </div>
                        <div class="header-right">
                            if !state.product_info.manufacturer.is_empty() {
                                <span class="manufacturer">{&state.product_info.manufacturer}</span>
                            }
                        </div>
                    </header>

                    <div class="main-content">
                        // Sidebar
                        <aside class="sidebar">
                            // Product image
                            if let Some(img_url) = &state.product_info.image_data_url {
                                <div class="product-image">
                                    <img src={img_url.clone()} alt="Product" />
                                </div>
                            }

                            // Plugins
                            <section class="sidebar-section">
                                <h3>{"Viewers"}</h3>
                                {for state.plugins.iter().map(|plugin| {
                                    let is_active = state.active_plugin.as_ref() == Some(&plugin.id);
                                    let plugin_id = plugin.id.clone();
                                    let state_clone = state.clone();
                                    let onclick = Callback::from(move |_| {
                                        let mut new_state = (*state_clone).clone();
                                        new_state.active_plugin = Some(plugin_id.clone());
                                        state_clone.set(new_state);
                                    });

                                    html! {
                                        <div class={classes!("plugin-item", is_active.then_some("active"))} onclick={onclick}>
                                            <span class="plugin-icon">{if plugin.icon.is_empty() { "🔌" } else { &plugin.icon }}</span>
                                            <span class="plugin-name">{&plugin.name}</span>
                                        </div>
                                    }
                                })}
                                if state.plugins.is_empty() {
                                    <p class="no-plugins">{"No embedded viewers found"}</p>
                                }
                            </section>

                            // Files
                            if !state.extracted_files.is_empty() {
                                <section class="sidebar-section">
                                    <h3>{"Photometry Files"}</h3>
                                    {for state.extracted_files.iter().map(|file| {
                                        let is_active = state.active_file.as_ref() == Some(&file.name);
                                        let file_name = file.name.clone();
                                        let state_clone = state.clone();
                                        let onclick = Callback::from(move |_| {
                                            let mut new_state = (*state_clone).clone();
                                            new_state.active_file = Some(file_name.clone());
                                            new_state.svg_output = None;
                                            state_clone.set(new_state);
                                        });

                                        html! {
                                            <div class={classes!("file-item", is_active.then_some("active"))} onclick={onclick}>
                                                <span class="file-icon">{if file.file_type == "ldt" { "📄" } else { "📋" }}</span>
                                                <span class="file-name">{&file.name}</span>
                                            </div>
                                        }
                                    })}
                                </section>
                            }
                        </aside>

                        // Main viewer area
                        <main class="content">
                            if state.active_plugin.is_some() && state.active_file.is_some() {
                                // Diagram tabs
                                {
                                    if let Some(plugin) = state.plugins.iter().find(|p| Some(&p.id) == state.active_plugin.as_ref()) {
                                        if !plugin.diagrams.is_empty() {
                                            html! {
                                                <div class="diagram-tabs">
                                                    {for plugin.diagrams.iter().map(|diagram| {
                                                        let is_active = &state.active_view == diagram;
                                                        let diagram_clone = diagram.clone();
                                                        let on_click = on_view_change.clone();
                                                        html! {
                                                            <button
                                                                class={classes!("tab", is_active.then_some("active"))}
                                                                onclick={Callback::from(move |_| on_click.emit(diagram_clone.clone()))}
                                                            >
                                                                {diagram}
                                                            </button>
                                                        }
                                                    })}
                                                </div>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    } else {
                                        html! {}
                                    }
                                }

                                // SVG output
                                <div class="diagram-container">
                                    if let Some(svg) = &state.svg_output {
                                        <SvgRenderer svg={svg.clone()} />
                                    } else {
                                        <div class="loading-diagram">
                                            <div class="spinner"></div>
                                        </div>
                                    }
                                </div>
                            } else if state.active_plugin.is_none() && !state.plugins.is_empty() {
                                <div class="placeholder">
                                    <p>{"Select a viewer from the sidebar"}</p>
                                </div>
                            } else if state.active_file.is_none() && !state.extracted_files.is_empty() {
                                <div class="placeholder">
                                    <p>{"Select a photometry file"}</p>
                                </div>
                            } else {
                                <div class="placeholder">
                                    <p>{"No viewable content found"}</p>
                                    <p class="hint">{"This GLDF may not contain embedded viewers or photometry data"}</p>
                                </div>
                            }
                        </main>
                    </div>
                </div>
            }

            if let Some(error) = &state.error {
                <div class="error-toast">{error}</div>
            }
        </div>
    }
}

/// SVG renderer component
#[derive(Properties, PartialEq)]
struct SvgRendererProps {
    svg: String,
}

#[function_component(SvgRenderer)]
fn svg_renderer(props: &SvgRendererProps) -> Html {
    let container_ref = use_node_ref();

    {
        let container_ref = container_ref.clone();
        let svg = props.svg.clone();
        use_effect_with(svg, move |svg| {
            if let Some(container) = container_ref.cast::<web_sys::HtmlElement>() {
                container.set_inner_html(svg);
            }
            || ()
        });
    }

    html! {
        <div ref={container_ref} class="svg-container" />
    }
}

fn process_files(
    files: FileList,
    state: UseStateHandle<AppState>,
    readers: Rc<std::cell::RefCell<HashMap<String, FileReader>>>,
) {
    for i in 0..files.length() {
        if let Some(file) = files.get(i) {
            let name = file.name();
            if !name.ends_with(".gldf") {
                continue;
            }

            let state = state.clone();
            let readers_for_callback = readers.clone();
            let readers_for_insert = readers.clone();
            let file = File::from(file);
            let file_name = name.clone();

            {
                let mut new_state = (*state).clone();
                new_state.loading = true;
                state.set(new_state);
            }

            let reader = gloo::file::callbacks::read_as_bytes(&file, move |result| {
                let bytes = match result {
                    Ok(b) => b,
                    Err(e) => {
                        let mut new_state = (*state).clone();
                        new_state.loading = false;
                        new_state.error = Some(format!("Failed to read file: {:?}", e));
                        state.set(new_state);
                        return;
                    }
                };

                match WasmGldfProduct::load_gldf_from_buf_all(bytes) {
                    Ok(gldf) => {
                        console::log!(format!("Loaded GLDF: {} files", gldf.files.len()));

                        let extracted_files = extract_photometry_files(&gldf.files);
                        let product_info = extract_product_info(&gldf, &gldf.files);
                        let files_for_plugins: Vec<BufFile> = gldf.files.clone();
                        let file_name_clone = file_name.clone();
                        let first_file = extracted_files.first().map(|f| f.name.clone());

                        spawn_local(async move {
                            let plugins = extract_and_register_plugins(&files_for_plugins).await;

                            let mut new_state = (*state).clone();
                            new_state.file_name = Some(file_name_clone);
                            new_state.product_info = product_info;
                            new_state.plugins = plugins.clone();
                            new_state.extracted_files = extracted_files;
                            new_state.loading = false;
                            new_state.error = None;

                            // Auto-select first plugin
                            if let Some(first) = plugins.first() {
                                new_state.active_plugin = Some(first.id.clone());
                                if let Some(first_diagram) = first.diagrams.first() {
                                    new_state.active_view = first_diagram.clone();
                                }
                            }

                            // Auto-select first file
                            new_state.active_file = first_file;

                            state.set(new_state);
                        });
                    }
                    Err(e) => {
                        let mut new_state = (*state).clone();
                        new_state.loading = false;
                        new_state.error = Some(format!("Failed to parse GLDF: {:?}", e));
                        state.set(new_state);
                    }
                }

                readers_for_callback.borrow_mut().remove(&file_name);
            });

            readers_for_insert.borrow_mut().insert(name, reader);
        }
    }
}

const CSS: &str = r#"
* { box-sizing: border-box; margin: 0; padding: 0; }

.app {
    min-height: 100vh;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #f5f7fa;
    color: #333;
}

/* Landing Page */
.landing {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
}

.hero { text-align: center; margin-bottom: 2rem; }
.hero h1 { font-size: 2.5rem; color: #1a1a2e; margin-bottom: 0.5rem; }
.hero .subtitle { color: #666; font-size: 1.1rem; }

.drop-zone {
    background: white;
    border: 3px dashed #ccc;
    border-radius: 16px;
    padding: 3rem 4rem;
    text-align: center;
    transition: all 0.3s;
    cursor: pointer;
}
.drop-zone:hover { border-color: #4a90d9; background: #f8faff; }
.drop-icon { font-size: 4rem; margin-bottom: 1rem; }
.drop-zone p { color: #666; margin: 0.5rem 0; }
.or { font-size: 0.9rem; }

.file-button {
    display: inline-block;
    padding: 12px 24px;
    background: #4a90d9;
    color: white;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 500;
    transition: background 0.2s;
}
.file-button:hover { background: #3a7bc8; }
.file-button input { display: none; }

.features {
    display: flex;
    gap: 2rem;
    margin-top: 3rem;
    flex-wrap: wrap;
    justify-content: center;
}
.feature {
    text-align: center;
    padding: 1.5rem;
    max-width: 200px;
}
.feature-icon { font-size: 2rem; display: block; margin-bottom: 0.5rem; }
.feature h3 { font-size: 1rem; margin-bottom: 0.5rem; }
.feature p { font-size: 0.85rem; color: #666; }

/* Loading */
.loading {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
}
.spinner {
    width: 40px; height: 40px;
    border: 3px solid #e0e0e0;
    border-top-color: #4a90d9;
    border-radius: 50%;
    animation: spin 1s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

/* Viewer */
.viewer { display: flex; flex-direction: column; min-height: 100vh; }

.header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.5rem;
    background: white;
    border-bottom: 1px solid #e0e0e0;
    box-shadow: 0 1px 3px rgba(0,0,0,0.05);
}
.header-left { display: flex; align-items: center; gap: 1rem; }
.back-btn {
    padding: 8px 16px;
    background: #f0f0f0;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
}
.back-btn:hover { background: #e0e0e0; }
.product-name { font-size: 1.25rem; font-weight: 600; }
.manufacturer { color: #666; font-size: 0.9rem; }

.main-content { display: flex; flex: 1; }

/* Sidebar */
.sidebar {
    width: 260px;
    background: white;
    border-right: 1px solid #e0e0e0;
    padding: 1rem;
    overflow-y: auto;
}

.product-image {
    margin-bottom: 1rem;
    border-radius: 8px;
    overflow: hidden;
    background: #f5f5f5;
}
.product-image img { width: 100%; display: block; }

.sidebar-section { margin-bottom: 1.5rem; }
.sidebar-section h3 {
    font-size: 0.75rem;
    text-transform: uppercase;
    color: #888;
    margin-bottom: 0.75rem;
    padding: 0 0.5rem;
}

.plugin-item, .file-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem;
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.2s;
    margin-bottom: 0.25rem;
}
.plugin-item:hover, .file-item:hover { background: #f5f5f5; }
.plugin-item.active, .file-item.active { background: #e3f2fd; }
.plugin-icon, .file-icon { font-size: 1.25rem; }
.plugin-name, .file-name { font-size: 0.9rem; }
.no-plugins { color: #999; font-style: italic; padding: 0.5rem; font-size: 0.85rem; }

/* Content */
.content {
    flex: 1;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
}

.diagram-tabs {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
}
.tab {
    padding: 8px 16px;
    background: #e8e8e8;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
    transition: all 0.2s;
}
.tab:hover { background: #d8d8d8; }
.tab.active { background: #4a90d9; color: white; }

.diagram-container {
    flex: 1;
    background: white;
    border-radius: 12px;
    padding: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 8px rgba(0,0,0,0.05);
}

.svg-container { display: flex; justify-content: center; }
.svg-container svg { max-width: 100%; height: auto; }

.loading-diagram { padding: 3rem; }

.placeholder {
    text-align: center;
    color: #888;
    padding: 3rem;
}
.placeholder .hint { font-size: 0.85rem; margin-top: 0.5rem; }

.error-toast {
    position: fixed;
    bottom: 1.5rem;
    right: 1.5rem;
    background: #e53935;
    color: white;
    padding: 1rem 1.5rem;
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.15);
    max-width: 400px;
}

@media (max-width: 768px) {
    .main-content { flex-direction: column; }
    .sidebar { width: 100%; border-right: none; border-bottom: 1px solid #e0e0e0; }
    .features { flex-direction: column; align-items: center; }
}
"#;

fn main() {
    yew::Renderer::<App>::new().render();
}
