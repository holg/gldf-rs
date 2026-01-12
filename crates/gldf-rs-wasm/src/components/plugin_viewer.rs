//! Plugin Viewer Component
//!
//! Dynamically loads and renders WASM plugins embedded in GLDF files.

// Allow dead code - these types and functions are used at runtime via JS interop
// but appear unused to static analysis
#![allow(dead_code)]

use gldf_rs::plugin::{Plugin, PluginManager};
use gldf_rs::BufFile;
use gloo::console::log;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, Url};
use yew::prelude::*;

/// External JS functions for plugin loading
#[wasm_bindgen]
extern "C" {
    /// Store data in localStorage
    #[wasm_bindgen(js_namespace = localStorage, js_name = setItem)]
    fn local_storage_set(key: &str, value: &str);

    /// Remove data from localStorage
    #[wasm_bindgen(js_namespace = localStorage, js_name = removeItem)]
    fn local_storage_remove(key: &str);
}

#[derive(Clone, PartialEq)]
pub enum PluginState {
    NotLoaded,
    Loading,
    Loaded,
    Error(String),
}

#[derive(Properties, PartialEq)]
pub struct PluginViewerProps {
    /// All files from the GLDF
    pub files: Vec<BufFile>,
    /// Specific plugin type to load (if None, shows plugin selector)
    #[prop_or_default]
    pub plugin_type: Option<String>,
    /// Canvas width
    #[prop_or(800)]
    pub width: u32,
    /// Canvas height
    #[prop_or(600)]
    pub height: u32,
}

#[function_component(PluginViewer)]
pub fn plugin_viewer(props: &PluginViewerProps) -> Html {
    let plugin_state = use_state(|| PluginState::NotLoaded);
    let selected_plugin = use_state(|| props.plugin_type.clone());
    let canvas_ref = use_node_ref();

    // Discover available plugins
    let plugins = PluginManager::discover_plugins(&props.files);

    log!(format!("[PluginViewer] Found {} plugins", plugins.len()));

    // Effect to load plugin when selected
    {
        let plugin_state = plugin_state.clone();
        let selected_plugin = selected_plugin.clone();
        let plugins = plugins.clone();
        let canvas_ref = canvas_ref.clone();

        use_effect_with(
            (selected_plugin.clone(), plugins.clone()),
            move |(selected, plugins)| {
                if let Some(plugin_type) = selected.as_ref() {
                    if let Some(plugin) = plugins
                        .iter()
                        .find(|p| &p.manifest.plugin_type == plugin_type)
                    {
                        plugin_state.set(PluginState::Loading);

                        // Clone what we need for the async block
                        let plugin = plugin.clone();
                        let plugin_state = plugin_state.clone();
                        let canvas_ref = canvas_ref.clone();

                        wasm_bindgen_futures::spawn_local(async move {
                            match load_and_render_plugin(&plugin, &canvas_ref).await {
                                Ok(_) => {
                                    plugin_state.set(PluginState::Loaded);
                                }
                                Err(e) => {
                                    plugin_state.set(PluginState::Error(e));
                                }
                            }
                        });
                    }
                }
                || ()
            },
        );
    }

    // Plugin selector if multiple plugins available and none specified
    let plugin_selector = if plugins.len() > 1 && props.plugin_type.is_none() {
        let on_select = {
            let selected_plugin = selected_plugin.clone();
            Callback::from(move |e: Event| {
                let target = e.target().unwrap();
                let select = target.dyn_into::<web_sys::HtmlSelectElement>().unwrap();
                let value = select.value();
                if value.is_empty() {
                    selected_plugin.set(None);
                } else {
                    selected_plugin.set(Some(value));
                }
            })
        };

        html! {
            <div class="plugin-selector">
                <label>{"Select Plugin: "}</label>
                <select onchange={on_select}>
                    <option value="">{"-- Choose --"}</option>
                    { for plugins.iter().map(|p| {
                        html! {
                            <option value={p.manifest.plugin_type.clone()}>
                                { &p.manifest.name }
                            </option>
                        }
                    })}
                </select>
            </div>
        }
    } else {
        html! {}
    };

    // Auto-select single plugin
    if plugins.len() == 1 && selected_plugin.is_none() {
        selected_plugin.set(Some(plugins[0].manifest.plugin_type.clone()));
    }

    // Status indicator
    let status = match &*plugin_state {
        PluginState::NotLoaded => {
            html! { <div class="plugin-status">{"Select a plugin to load"}</div> }
        }
        PluginState::Loading => {
            html! { <div class="plugin-status loading">{"Loading plugin..."}</div> }
        }
        PluginState::Loaded => html! {},
        PluginState::Error(e) => {
            html! { <div class="plugin-status error">{format!("Error: {}", e)}</div> }
        }
    };

    // Plugin info
    let plugin_info = selected_plugin
        .as_ref()
        .and_then(|pt| plugins.iter().find(|p| &p.manifest.plugin_type == pt))
        .map(|p| {
            html! {
                <div class="plugin-info">
                    <h4>{ &p.manifest.name }</h4>
                    <p>{ &p.manifest.description }</p>
                    <span class="version">{ format!("v{}", p.manifest.version) }</span>
                </div>
            }
        })
        .unwrap_or_default();

    let canvas_id = selected_plugin
        .as_ref()
        .and_then(|pt| plugins.iter().find(|p| &p.manifest.plugin_type == pt))
        .and_then(|p| p.manifest.canvas_id.clone())
        .unwrap_or_else(|| "plugin-canvas".to_string());

    html! {
        <div class="plugin-viewer">
            { plugin_selector }
            { plugin_info }
            { status }
            <div class="plugin-canvas-container" style={format!("width: {}px; height: {}px;", props.width, props.height)}>
                <canvas
                    ref={canvas_ref}
                    id={canvas_id}
                    width={props.width.to_string()}
                    height={props.height.to_string()}
                />
            </div>
        </div>
    }
}

/// Load and render a plugin
async fn load_and_render_plugin(plugin: &Plugin, _canvas_ref: &NodeRef) -> Result<(), String> {
    log!(format!(
        "[PluginViewer] Loading plugin: {}",
        plugin.manifest.name
    ));

    // 1. Store data in localStorage if plugin has data
    if let Some(data) = &plugin.data_content {
        // Use storage_keys.config if available, otherwise fall back to storage_key
        let storage_key = plugin
            .manifest
            .storage_keys
            .as_ref()
            .and_then(|sk| sk.config.as_deref())
            .or(plugin.manifest.storage_key.as_deref())
            .unwrap_or("gldf_plugin_data");

        let data_str = String::from_utf8(data.clone())
            .map_err(|e| format!("Invalid UTF-8 in data file: {}", e))?;

        local_storage_set(storage_key, &data_str);
        log!(format!(
            "[PluginViewer] Stored config data as '{}'",
            storage_key
        ));
    }

    // 1b. Store LDT data if plugin has it
    if let Some(ldt) = &plugin.ldt_content {
        let ldt_key = plugin
            .manifest
            .storage_keys
            .as_ref()
            .and_then(|sk| sk.ldt.as_deref())
            .unwrap_or("gldf_plugin_ldt");

        let ldt_str = String::from_utf8(ldt.clone())
            .map_err(|e| format!("Invalid UTF-8 in LDT file: {}", e))?;

        local_storage_set(ldt_key, &ldt_str);
        log!(format!(
            "[PluginViewer] Stored LDT data ({} bytes) as '{}'",
            ldt_str.len(),
            ldt_key
        ));
    }

    // 2. Create blob URLs for JS and WASM
    let wasm_blob = create_blob(&plugin.wasm_content, "application/wasm")?;
    let wasm_url = Url::create_object_url_with_blob(&wasm_blob)
        .map_err(|_| "Failed to create WASM blob URL")?;

    // 3. Modify JS to use our blob URL
    let js_str = plugin
        .js_as_string()
        .map_err(|e| format!("Invalid UTF-8 in JS file: {}", e))?;

    // Replace WASM URL patterns in the JS
    let modified_js = modify_wasm_urls(&js_str, &wasm_url);

    let js_blob = create_blob(modified_js.as_bytes(), "application/javascript")?;
    let js_url =
        Url::create_object_url_with_blob(&js_blob).map_err(|_| "Failed to create JS blob URL")?;

    log!(format!(
        "[PluginViewer] Created blob URLs - JS: {}, WASM: {}",
        js_url, wasm_url
    ));

    // 4. Dynamically import and initialize the module
    let module = import_module(&js_url).await?;

    // 5. Initialize WASM (call default export)
    init_wasm_module(&module).await?;

    // 6. Call the entry point
    let canvas_id = plugin
        .manifest
        .canvas_id
        .as_deref()
        .unwrap_or("plugin-canvas");

    let storage_key = plugin
        .manifest
        .storage_key
        .as_deref()
        .unwrap_or("gldf_plugin_data");

    let entry_point = plugin
        .manifest
        .entry_point
        .as_deref()
        .unwrap_or("render_from_storage");

    call_plugin_entry(&module, entry_point, canvas_id, storage_key)?;

    log!(format!(
        "[PluginViewer] Plugin {} rendered successfully",
        plugin.manifest.name
    ));

    // Cleanup blob URLs (optional - they'll be cleaned up on page unload anyway)
    // Url::revoke_object_url(&js_url).ok();
    // Url::revoke_object_url(&wasm_url).ok();

    Ok(())
}

/// Create a Blob from bytes
fn create_blob(data: &[u8], mime_type: &str) -> Result<Blob, String> {
    let uint8_array = js_sys::Uint8Array::from(data);
    let array = js_sys::Array::new();
    array.push(&uint8_array);

    let options = BlobPropertyBag::new();
    options.set_type(mime_type);

    Blob::new_with_u8_array_sequence_and_options(&array, &options)
        .map_err(|_| "Failed to create blob".to_string())
}

/// Modify JS to use our blob URL for WASM
fn modify_wasm_urls(js: &str, wasm_url: &str) -> String {
    // wasm-bindgen generates code like:
    // new URL('xxx_bg.wasm', import.meta.url)
    // We need to replace this with our blob URL

    let mut result = js.to_string();

    // Find and replace WASM URL patterns
    // Look for patterns like: new URL('something_bg.wasm', import.meta.url)
    // We do a simple search for _bg.wasm and replace the URL construction

    // Find the wasm filename in the JS
    if let Some(start) = result.find("_bg.wasm") {
        // Find the start of the filename (look backwards for quote)
        let search_start = start.saturating_sub(50);
        let prefix = &result[search_start..start];

        if let Some(quote_pos) = prefix.rfind(['\'', '"']) {
            let filename_start = search_start + quote_pos + 1;
            let filename_end = start + 8; // "_bg.wasm".len()
            let _old_filename = &result[filename_start..filename_end];

            // Find the full new URL(...) pattern and replace it
            // Look for: new URL('filename_bg.wasm', import.meta.url)
            if let Some(new_url_start) = result[..filename_start].rfind("new URL(") {
                if let Some(close_paren) = result[filename_end..].find(')') {
                    let pattern_end = filename_end + close_paren + 1;
                    let old_pattern = result[new_url_start..pattern_end].to_string();
                    result = result.replace(&old_pattern, &format!("\"{}\"", wasm_url));
                }
            }
        }
    }

    result
}

/// Dynamically import a JS module
async fn import_module(url: &str) -> Result<JsValue, String> {
    let promise = js_sys::eval(&format!("import('{}')", url))
        .map_err(|e| format!("Failed to create import: {:?}", e))?;

    let promise = js_sys::Promise::from(promise);

    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("Failed to import module: {:?}", e))
}

/// Initialize WASM module (call default export)
async fn init_wasm_module(module: &JsValue) -> Result<(), String> {
    let default_fn = js_sys::Reflect::get(module, &JsValue::from_str("default"))
        .map_err(|_| "Module has no default export")?;

    if default_fn.is_function() {
        let func = js_sys::Function::from(default_fn);
        let result = func
            .call0(&JsValue::NULL)
            .map_err(|e| format!("Failed to call default(): {:?}", e))?;

        // If it returns a promise, await it
        if result.is_instance_of::<js_sys::Promise>() {
            let promise = js_sys::Promise::from(result);
            wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map_err(|e| format!("WASM init failed: {:?}", e))?;
        }
    }

    Ok(())
}

/// Call the plugin's entry point function
fn call_plugin_entry(
    module: &JsValue,
    entry_point: &str,
    canvas_id: &str,
    storage_key: &str,
) -> Result<(), String> {
    // Try to get the entry point function
    let func = js_sys::Reflect::get(module, &JsValue::from_str(entry_point))
        .map_err(|_| format!("Entry point '{}' not found", entry_point))?;

    if func.is_function() {
        let func = js_sys::Function::from(func);
        // For Bevy plugins (run_on_canvas), use CSS selector with just one arg
        // For Canvas 2D plugins (render_from_storage), use canvas ID and storage key
        let result = if entry_point == "run_on_canvas" {
            // Bevy expects CSS selector like "#canvas-id"
            func.call1(
                &JsValue::NULL,
                &JsValue::from_str(&format!("#{}", canvas_id)),
            )
        } else {
            // Canvas 2D plugins use ID and storage key
            func.call2(
                &JsValue::NULL,
                &JsValue::from_str(canvas_id),
                &JsValue::from_str(storage_key),
            )
        };

        match result {
            Ok(_) => return Ok(()),
            Err(e) => {
                let error_str = format!("{:?}", e);
                // Bevy uses exceptions for control flow - ignore these
                if error_str.contains("Using exceptions for control flow")
                    || error_str.contains("don't mind me")
                {
                    log!(format!(
                        "[PluginViewer] Ignoring control flow exception (Bevy is running)"
                    ));
                    return Ok(()); // Bevy is running, not an error
                }
                return Err(format!("Entry point failed: {:?}", e));
            }
        }
    }

    // Fallback: try StarSkyRenderer constructor (for starsky plugin)
    let renderer_class = js_sys::Reflect::get(module, &JsValue::from_str("StarSkyRenderer"));
    if let Ok(class) = renderer_class {
        if class.is_function() {
            let constructor = js_sys::Function::from(class);
            let renderer = js_sys::Reflect::construct(
                &constructor,
                &js_sys::Array::of1(&JsValue::from_str(canvas_id)),
            )
            .map_err(|e| format!("Failed to construct renderer: {:?}", e))?;

            // Call load_from_storage
            if let Ok(load_fn) =
                js_sys::Reflect::get(&renderer, &JsValue::from_str("load_from_storage"))
            {
                if load_fn.is_function() {
                    let func = js_sys::Function::from(load_fn);
                    func.call1(&renderer, &JsValue::from_str(storage_key)).ok();
                }
            }

            // Call render
            if let Ok(render_fn) = js_sys::Reflect::get(&renderer, &JsValue::from_str("render")) {
                if render_fn.is_function() {
                    let func = js_sys::Function::from(render_fn);
                    func.call0(&renderer).ok();
                }
            }

            return Ok(());
        }
    }

    Err(format!(
        "Could not find entry point '{}' or fallback renderer",
        entry_point
    ))
}

/// Component to list available plugins in a GLDF
#[derive(Properties, PartialEq)]
pub struct PluginListProps {
    pub files: Vec<BufFile>,
    #[prop_or_default]
    pub on_select: Callback<String>,
}

#[function_component(PluginList)]
pub fn plugin_list(props: &PluginListProps) -> Html {
    let plugins = PluginManager::discover_plugins(&props.files);

    if plugins.is_empty() {
        return html! {
            <div class="plugin-list empty">
                <p>{"No plugins found in this GLDF file."}</p>
            </div>
        };
    }

    html! {
        <div class="plugin-list">
            <h4>{"Available Plugins"}</h4>
            <ul>
                { for plugins.iter().map(|p| {
                    let plugin_type = p.manifest.plugin_type.clone();
                    let on_click = {
                        let on_select = props.on_select.clone();
                        let plugin_type = plugin_type.clone();
                        Callback::from(move |_| {
                            on_select.emit(plugin_type.clone());
                        })
                    };

                    html! {
                        <li onclick={on_click} class="plugin-item">
                            <span class="plugin-name">{ &p.manifest.name }</span>
                            <span class="plugin-version">{ format!("v{}", p.manifest.version) }</span>
                            <p class="plugin-description">{ &p.manifest.description }</p>
                        </li>
                    }
                })}
            </ul>
        </div>
    }
}
