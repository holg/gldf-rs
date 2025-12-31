//! JavaScript bindings for the GLDF WASM editor
//!
//! This module centralizes all wasm_bindgen extern "C" declarations to avoid
//! duplicate symbol errors when both lib.rs and main.rs use the same bindings.

use wasm_bindgen::prelude::*;

// =============================================================================
// Typst PDF Compilation
// =============================================================================

#[wasm_bindgen]
extern "C" {
    /// Compile Typst source to PDF (loaded from typst-loader.js)
    #[wasm_bindgen(js_name = compileTypstToPdf, catch)]
    pub async fn compile_typst_to_pdf_js(source: &str) -> Result<JsValue, JsValue>;
}

// =============================================================================
// Star Sky / Bevy Integration
// =============================================================================

#[wasm_bindgen]
extern "C" {
    /// Save star sky JSON to localStorage for Bevy 3D viewer
    #[wasm_bindgen(js_name = saveStarSkyForBevy)]
    pub fn save_star_sky_for_bevy(json_data: &str);
}

// =============================================================================
// Embedded WASM Viewer Registration
// =============================================================================

#[wasm_bindgen]
extern "C" {
    /// Register embedded WASM viewer from GLDF file
    #[wasm_bindgen(js_name = registerEmbeddedViewer)]
    pub fn register_embedded_viewer(
        viewer_type: &str,
        manifest_json: &str,
        js_content: &str,
        wasm_bytes: &[u8],
    );

    /// Check if embedded viewer is available
    #[wasm_bindgen(js_name = hasEmbeddedViewer)]
    pub fn has_embedded_viewer(viewer_type: &str) -> bool;
}

// =============================================================================
// Plugin System
// =============================================================================

#[wasm_bindgen]
extern "C" {
    /// Check if a plugin is loaded
    #[wasm_bindgen(js_name = hasPlugin)]
    pub fn has_plugin(plugin_id: &str) -> bool;

    /// Get plugin capabilities as JSON
    #[wasm_bindgen(js_name = getPluginCapabilities)]
    pub fn get_plugin_capabilities_js(plugin_id: &str) -> Option<String>;

    /// Call a function on a plugin (returns JsValue)
    #[wasm_bindgen(js_name = callPluginFunction, catch)]
    pub fn call_plugin_function_js(
        plugin_id: &str,
        function_name: &str,
    ) -> Result<JsValue, JsValue>;

    /// Call a function with 1 string arg
    #[wasm_bindgen(js_name = callPluginFunction1, catch)]
    pub fn call_plugin_function_1(
        plugin_id: &str,
        function_name: &str,
        arg1: &str,
    ) -> Result<JsValue, JsValue>;

    /// Call a function with 2 string args
    #[wasm_bindgen(js_name = callPluginFunction2, catch)]
    pub fn call_plugin_function_2(
        plugin_id: &str,
        function_name: &str,
        arg1: &str,
        arg2: &str,
    ) -> Result<JsValue, JsValue>;

    /// Call a function with numeric args (for plot_function, benchmark, etc.)
    #[wasm_bindgen(js_name = callPluginFunctionNums, catch)]
    pub fn call_plugin_function_nums(
        plugin_id: &str,
        function_name: &str,
        arg1: &str,
        arg2: f64,
        arg3: f64,
        arg4: f64,
        arg5: f64,
        arg6: usize,
    ) -> Result<JsValue, JsValue>;

    /// List all loaded plugins as JSON
    #[wasm_bindgen(js_name = listPlugins)]
    pub fn list_plugins_js() -> JsValue;

    /// Register a WASM plugin with its manifest and files
    #[wasm_bindgen(js_name = registerWasmPlugin, catch)]
    pub async fn register_wasm_plugin_js(
        plugin_id: &str,
        manifest_json: &str,
        js_content: &str,
        wasm_bytes: &[u8],
    ) -> Result<JsValue, JsValue>;
}
