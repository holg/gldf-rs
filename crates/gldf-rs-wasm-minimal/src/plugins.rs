//! Plugin system for loading and calling embedded WASM plugins
//!
//! Plugins are discovered from the GLDF file's other/viewer/ directory.
//! Each plugin has a manifest.json that describes its capabilities.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

pub use crate::js_bindings::{
    call_plugin_function_1, call_plugin_function_2, call_plugin_function_nums,
    get_plugin_capabilities_js, has_plugin, list_plugins_js, register_wasm_plugin_js,
};

/// Plugin manifest describing capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub js: String,
    pub wasm: String,
    #[serde(default)]
    pub init: String,
    #[serde(default)]
    pub constructor: Option<String>,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
    #[serde(default)]
    pub files: Vec<String>,
}

/// Plugin capabilities from manifest
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginCapabilities {
    #[serde(default)]
    pub functions: std::collections::HashMap<String, FunctionInfo>,
    #[serde(default, rename = "staticFunctions")]
    pub static_functions: std::collections::HashMap<String, FunctionInfo>,
    #[serde(default, rename = "inputFormats")]
    pub input_formats: Vec<String>,
    #[serde(default, rename = "outputFormats")]
    pub output_formats: Vec<String>,
    #[serde(default)]
    pub diagrams: Vec<String>,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub ui: PluginUiConfig,
}

/// Function metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub returns: String,
    #[serde(default)]
    pub description: String,
}

/// Plugin UI configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginUiConfig {
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub color: String,
    #[serde(default, rename = "primaryAction")]
    pub primary_action: String,
    #[serde(default, rename = "showExamples")]
    pub show_examples: bool,
    #[serde(default, rename = "editorLanguage")]
    pub editor_language: Option<String>,
}

/// Discovered plugin info (from list_plugins)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub functions: Vec<String>,
}

/// List all loaded plugins
pub fn list_plugins() -> Vec<PluginInfo> {
    let json = list_plugins_js();
    serde_json::from_str(&json).unwrap_or_default()
}

/// Get plugin capabilities
pub fn get_plugin_capabilities(plugin_id: &str) -> Option<PluginCapabilities> {
    let json = get_plugin_capabilities_js(plugin_id)?;
    serde_json::from_str(&json).ok()
}

/// Call a plugin function that takes content and returns a result
pub fn call_plugin_parse(plugin_id: &str, function_name: &str, content: &str) -> Result<String, String> {
    call_plugin_function_1(plugin_id, function_name, content)
        .map(|v| v.as_string().unwrap_or_default())
        .map_err(|e| e.as_string().unwrap_or_else(|| "Unknown error".to_string()))
}

/// Call a plugin function that generates an SVG diagram
pub fn call_plugin_svg(
    plugin_id: &str,
    function_name: &str,
    width: f64,
    height: f64,
    theme: &str,
) -> Result<String, String> {
    // For SVG functions, we need to use the numeric call with theme encoded
    // Most plugins expect (width, height, theme_str) but wasm_bindgen can't mix types easily
    // So we use call_plugin_function_2 with dimensions as first arg
    let dims = format!("{}x{}", width as u32, height as u32);
    call_plugin_function_2(plugin_id, function_name, &dims, theme)
        .map(|v| v.as_string().unwrap_or_default())
        .map_err(|e| e.as_string().unwrap_or_else(|| "Unknown error".to_string()))
}
