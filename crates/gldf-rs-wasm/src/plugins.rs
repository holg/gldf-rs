//! WASM Plugin System
//!
//! Provides a dynamic interface to WASM plugins that self-describe their capabilities.
//! Plugins are loaded from embedded GLDF files or remote URLs and register themselves
//! with a manifest describing their functions, examples, output formats, and UI hints.

use gloo::console::log;
use serde::{Deserialize, Serialize};

// Re-export from centralized bindings
pub use crate::js_bindings::{
    call_plugin_function_1, call_plugin_function_2, call_plugin_function_js,
    call_plugin_function_nums, get_plugin_capabilities_js, has_plugin, list_plugins_js,
    register_wasm_plugin_js,
};

/// Function metadata from plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginFunction {
    pub name: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub returns: String,
    #[serde(default)]
    pub description: String,
}

/// UI configuration from plugin manifest
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginUi {
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub primary_action: String,
    #[serde(default)]
    pub show_examples: bool,
    #[serde(default)]
    pub editor_language: String,
}

/// Plugin capabilities parsed from manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapabilities {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub functions: Vec<PluginFunction>,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub output_formats: Vec<String>,
    #[serde(default)]
    pub ui: PluginUi,
}

/// Get capabilities of a loaded plugin
pub fn get_plugin_capabilities(plugin_id: &str) -> Option<PluginCapabilities> {
    let json = get_plugin_capabilities_js(plugin_id)?;
    match serde_json::from_str(&json) {
        Ok(caps) => Some(caps),
        Err(e) => {
            log!(format!(
                "[Plugin] Failed to parse capabilities for {}: {}",
                plugin_id, e
            ));
            None
        }
    }
}

/// Plugin handle for calling functions
pub struct Plugin {
    pub id: String,
    pub capabilities: PluginCapabilities,
}

impl Plugin {
    /// Get a plugin by ID (if loaded)
    pub fn get(plugin_id: &str) -> Option<Self> {
        if !has_plugin(plugin_id) {
            return None;
        }
        let capabilities = get_plugin_capabilities(plugin_id)?;
        Some(Plugin {
            id: plugin_id.to_string(),
            capabilities,
        })
    }

    /// Check if plugin has a specific function
    pub fn has_function(&self, name: &str) -> bool {
        self.capabilities.functions.iter().any(|f| f.name == name)
    }

    /// Get function info
    pub fn get_function(&self, name: &str) -> Option<&PluginFunction> {
        self.capabilities.functions.iter().find(|f| f.name == name)
    }

    /// Call a function with no arguments, returns string
    pub fn call(&self, function_name: &str) -> Result<String, String> {
        match call_plugin_function_js(&self.id, function_name) {
            Ok(val) => Ok(val.as_string().unwrap_or_default()),
            Err(e) => Err(format!("{:?}", e)),
        }
    }

    /// Call a function with 1 string argument
    pub fn call_1(&self, function_name: &str, arg1: &str) -> Result<String, String> {
        match call_plugin_function_1(&self.id, function_name, arg1) {
            Ok(val) => Ok(val.as_string().unwrap_or_default()),
            Err(e) => Err(format!("{:?}", e)),
        }
    }

    /// Call a function with 2 string arguments
    pub fn call_2(&self, function_name: &str, arg1: &str, arg2: &str) -> Result<String, String> {
        match call_plugin_function_2(&self.id, function_name, arg1, arg2) {
            Ok(val) => Ok(val.as_string().unwrap_or_default()),
            Err(e) => Err(format!("{:?}", e)),
        }
    }

    /// Convenience methods for common operations

    /// Evaluate code (for interpreter plugins like acadlisp)
    pub fn eval(&self, code: &str) -> Result<String, String> {
        self.call_1("eval", code)
    }

    /// Get SVG output
    pub fn get_svg(&self) -> Result<String, String> {
        // Try common function names
        if self.has_function("get_entities_svg") {
            self.call("get_entities_svg")
        } else if self.has_function("get_svg") {
            self.call("get_svg")
        } else {
            Err("No SVG function available".to_string())
        }
    }

    /// Get JSON output
    pub fn get_json(&self) -> Result<String, String> {
        if self.has_function("get_entities_json") {
            self.call("get_entities_json")
        } else if self.has_function("get_json") {
            self.call("get_json")
        } else {
            Err("No JSON function available".to_string())
        }
    }

    /// Get output/console buffer
    pub fn get_output(&self) -> Result<String, String> {
        if self.has_function("get_output") {
            self.call("get_output")
        } else {
            Ok(String::new())
        }
    }

    /// Clear state
    pub fn clear(&self) -> Result<(), String> {
        if self.has_function("clear") {
            self.call("clear")?;
        }
        Ok(())
    }

    /// Get entity count
    pub fn entity_count(&self) -> usize {
        if self.has_function("entity_count") {
            self.call("entity_count")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0)
        } else {
            0
        }
    }

    /// Get example code by name
    pub fn get_example(&self, name: &str) -> Option<String> {
        if self.has_function("get_example") {
            self.call_1("get_example", name).ok()
        } else {
            None
        }
    }

    /// Get KiCad symbol export (acadlisp specific)
    pub fn get_kicad_sym(&self, library_name: &str, symbol_name: &str) -> Result<String, String> {
        self.call_2("get_kicad_sym", library_name, symbol_name)
    }

    /// Get KiCad footprint export (acadlisp specific)
    pub fn get_kicad_mod(&self, footprint_name: &str) -> Result<String, String> {
        self.call_1("get_kicad_mod", footprint_name)
    }

    /// Set CAD type (acadlisp specific)
    pub fn set_cad_type(&self, cad_type: &str) -> Result<(), String> {
        if self.has_function("set_cad_type") {
            self.call_1("set_cad_type", cad_type)?;
        }
        Ok(())
    }

    /// Get engine info
    pub fn engine_info(&self) -> String {
        if self.has_function("engine_info") {
            self.call("engine_info").unwrap_or_default()
        } else {
            format!(
                r#"{{"name":"{}","version":"{}"}}"#,
                self.capabilities.name, self.capabilities.version
            )
        }
    }
}

/// List all loaded plugins
pub fn list_plugins() -> Vec<PluginCapabilities> {
    let js_val = list_plugins_js();
    if let Some(json) = js_val.as_string() {
        serde_json::from_str(&json).unwrap_or_default()
    } else {
        // Try to convert JsValue array
        Vec::new()
    }
}

/// Check if any plugin with a specific capability is loaded
pub fn has_plugin_with_function(function_name: &str) -> bool {
    // This would need JS support to efficiently query
    // For now, check known plugin IDs
    for plugin_id in &["acadlisp", "bevy", "typst", "starsky"] {
        if let Some(plugin) = Plugin::get(plugin_id) {
            if plugin.has_function(function_name) {
                return true;
            }
        }
    }
    false
}

/// Get first plugin that has a specific function
pub fn get_plugin_with_function(function_name: &str) -> Option<Plugin> {
    for plugin_id in &["acadlisp", "bevy", "typst", "starsky"] {
        if let Some(plugin) = Plugin::get(plugin_id) {
            if plugin.has_function(function_name) {
                return Some(plugin);
            }
        }
    }
    None
}
