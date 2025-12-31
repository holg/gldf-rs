//! Centralized JavaScript bindings for WASM interop
//!
//! All wasm_bindgen extern declarations go here to avoid duplicate symbol errors.

use wasm_bindgen::prelude::*;

// =============================================================================
// Plugin System Bindings (wasm-plugin-loader.js)
// =============================================================================

#[wasm_bindgen]
extern "C" {
    /// Register a WASM plugin from embedded files
    #[wasm_bindgen(js_name = registerWasmPlugin, catch)]
    pub async fn register_wasm_plugin_js(
        plugin_id: &str,
        manifest_json: &str,
        files: JsValue,
    ) -> Result<JsValue, JsValue>;

    /// Check if a plugin is loaded
    #[wasm_bindgen(js_name = hasPlugin)]
    pub fn has_plugin(plugin_id: &str) -> bool;

    /// List all loaded plugins as JSON
    #[wasm_bindgen(js_name = listPlugins)]
    pub fn list_plugins_js() -> String;

    /// Get plugin capabilities as JSON
    #[wasm_bindgen(js_name = getPluginCapabilities)]
    pub fn get_plugin_capabilities_js(plugin_id: &str) -> Option<String>;

    /// Call a plugin function with 1 string argument
    #[wasm_bindgen(js_name = callPluginFunction1, catch)]
    pub fn call_plugin_function_1(
        plugin_id: &str,
        function_name: &str,
        arg1: &str,
    ) -> Result<JsValue, JsValue>;

    /// Call a plugin function with 2 string arguments
    #[wasm_bindgen(js_name = callPluginFunction2, catch)]
    pub fn call_plugin_function_2(
        plugin_id: &str,
        function_name: &str,
        arg1: &str,
        arg2: &str,
    ) -> Result<JsValue, JsValue>;

    /// Call a plugin function with 3 arguments (f64, f64, string)
    /// Used for SVG diagram generation: (width, height, theme)
    #[wasm_bindgen(js_name = callPluginFunction, catch)]
    pub fn call_plugin_function_3(
        plugin_id: &str,
        function_name: &str,
        arg1: f64,
        arg2: f64,
        arg3: &str,
    ) -> Result<JsValue, JsValue>;

    /// Call a plugin function with numeric arguments (for diagrams)
    #[wasm_bindgen(js_name = callPluginFunctionNums, catch)]
    pub fn call_plugin_function_nums(
        plugin_id: &str,
        function_name: &str,
        arg1: f64,
        arg2: f64,
        arg3: f64,
        arg4: f64,
        arg5: f64,
        arg6: f64,
    ) -> Result<JsValue, JsValue>;
}
