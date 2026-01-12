//! GLDF Plugin System
//!
//! This module provides support for viewer plugins embedded in GLDF files.
//! Plugins are WASM modules stored in `other/viewer/<plugin-type>/` directories
//! with a `manifest.json` describing their capabilities.
//!
//! ## Plugin Structure in GLDF
//!
//! ```text
//! myfile.gldf (ZIP archive)
//! └── other/
//!     └── viewer/
//!         └── starsky/                    # Plugin directory
//!             ├── manifest.json           # Plugin metadata
//!             ├── plugin.js               # WASM JS bindings
//!             └── plugin_bg.wasm          # WASM binary
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use gldf_rs::plugin::{PluginManager, PluginManifest};
//!
//! let gldf = FileBufGldf::load("luminaire.gldf")?;
//! let plugins = PluginManager::discover_plugins(&gldf);
//!
//! for plugin in plugins {
//!     println!("Found plugin: {} ({})", plugin.manifest.name, plugin.manifest.plugin_type);
//! }
//! ```

use serde::{Deserialize, Serialize};

use crate::BufFile;

/// Plugin manifest describing a viewer plugin embedded in a GLDF file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin type identifier (e.g., "starsky", "acadlisp")
    #[serde(rename = "type")]
    pub plugin_type: String,

    /// Human-readable plugin name
    pub name: String,

    /// Plugin version (semver)
    pub version: String,

    /// Plugin description
    #[serde(default)]
    pub description: String,

    /// JavaScript file name (wasm-bindgen generated)
    pub js: String,

    /// WASM binary file name
    pub wasm: String,

    /// Data file this plugin consumes (path within GLDF, e.g., "other/sky_data.json")
    #[serde(default)]
    pub data_file: Option<String>,

    /// Canvas element ID the plugin renders to
    #[serde(default)]
    pub canvas_id: Option<String>,

    /// Entry point function name to call after loading
    #[serde(default)]
    pub entry_point: Option<String>,

    /// localStorage key for passing data to the plugin
    #[serde(default)]
    pub storage_key: Option<String>,

    /// LDT file this plugin uses (path within GLDF, e.g., "ldc/file.ldt")
    #[serde(default)]
    pub ldt_file: Option<String>,

    /// Multiple storage keys (for plugins needing config + LDT)
    #[serde(default)]
    pub storage_keys: Option<StorageKeys>,
}

/// Storage keys for plugins that need multiple data files
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StorageKeys {
    /// localStorage key for LDT data
    #[serde(default)]
    pub ldt: Option<String>,
    /// localStorage key for config data
    #[serde(default)]
    pub config: Option<String>,
}

/// A discovered plugin with its manifest and file contents.
#[derive(Debug, Clone, PartialEq)]
pub struct Plugin {
    /// Plugin manifest
    pub manifest: PluginManifest,

    /// Directory path within GLDF (e.g., "other/viewer/starsky")
    pub plugin_dir: String,

    /// JavaScript file content
    pub js_content: Vec<u8>,

    /// WASM binary content
    pub wasm_content: Vec<u8>,

    /// Optional data file content (if data_file is specified and exists)
    pub data_content: Option<Vec<u8>>,

    /// Optional LDT file content (if ldt_file is specified and exists)
    pub ldt_content: Option<Vec<u8>>,
}

impl Plugin {
    /// Get the JS content as a string
    pub fn js_as_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.js_content.clone())
    }

    /// Check if this plugin has associated data
    pub fn has_data(&self) -> bool {
        self.data_content.is_some()
    }

    /// Get the data content as a string (for JSON data)
    pub fn data_as_string(&self) -> Option<Result<String, std::string::FromUtf8Error>> {
        self.data_content
            .as_ref()
            .map(|d| String::from_utf8(d.clone()))
    }
}

/// Plugin discovery and management for GLDF files.
pub struct PluginManager;

impl PluginManager {
    /// Discover all plugins in a GLDF file's embedded files.
    ///
    /// Scans for `other/viewer/*/manifest.json` files and loads the associated
    /// plugin files (JS, WASM, and optional data).
    pub fn discover_plugins(files: &[BufFile]) -> Vec<Plugin> {
        let mut plugins = Vec::new();

        // Find all manifest.json files in other/viewer/*/
        let manifests: Vec<_> = files
            .iter()
            .filter(|f| {
                f.name
                    .as_ref()
                    .map(|n| n.starts_with("other/viewer/") && n.ends_with("/manifest.json"))
                    .unwrap_or(false)
            })
            .collect();

        for manifest_file in manifests {
            let manifest_path = match &manifest_file.name {
                Some(p) => p,
                None => continue,
            };

            let manifest_content = match &manifest_file.content {
                Some(c) => c,
                None => continue,
            };

            // Parse the manifest
            let manifest: PluginManifest = match serde_json::from_slice(manifest_content) {
                Ok(m) => m,
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "[Plugin] Failed to parse manifest {}: {}",
                        manifest_path, _e
                    );
                    continue;
                }
            };

            // Get the plugin directory
            let plugin_dir = manifest_path
                .strip_suffix("/manifest.json")
                .unwrap_or(manifest_path)
                .to_string();

            // Find JS file
            let js_path = format!("{}/{}", plugin_dir, manifest.js);
            let js_content = files
                .iter()
                .find(|f| f.name.as_ref() == Some(&js_path))
                .and_then(|f| f.content.clone());

            // Find WASM file
            let wasm_path = format!("{}/{}", plugin_dir, manifest.wasm);
            let wasm_content = files
                .iter()
                .find(|f| f.name.as_ref() == Some(&wasm_path))
                .and_then(|f| f.content.clone());

            // Find data file if specified
            let data_content = manifest.data_file.as_ref().and_then(|data_path| {
                files
                    .iter()
                    .find(|f| f.name.as_ref() == Some(data_path))
                    .and_then(|f| f.content.clone())
            });

            // Find LDT file if specified
            let ldt_content = manifest.ldt_file.as_ref().and_then(|ldt_path| {
                files
                    .iter()
                    .find(|f| f.name.as_ref() == Some(ldt_path))
                    .and_then(|f| f.content.clone())
            });

            // Only add if we have both JS and WASM
            if let (Some(js), Some(wasm)) = (js_content, wasm_content) {
                plugins.push(Plugin {
                    manifest,
                    plugin_dir,
                    js_content: js,
                    wasm_content: wasm,
                    data_content,
                    ldt_content,
                });
            } else {
                #[cfg(debug_assertions)]
                eprintln!("[Plugin] {} missing JS or WASM files", manifest.name);
            }
        }

        plugins
    }

    /// Check if a GLDF file contains any plugins.
    pub fn has_plugins(files: &[BufFile]) -> bool {
        files.iter().any(|f| {
            f.name
                .as_ref()
                .map(|n| n.starts_with("other/viewer/") && n.ends_with("/manifest.json"))
                .unwrap_or(false)
        })
    }

    /// Get plugin types available in a GLDF file.
    pub fn get_plugin_types(files: &[BufFile]) -> Vec<String> {
        Self::discover_plugins(files)
            .iter()
            .map(|p| p.manifest.plugin_type.clone())
            .collect()
    }

    /// Find a specific plugin by type.
    pub fn find_plugin(files: &[BufFile], plugin_type: &str) -> Option<Plugin> {
        Self::discover_plugins(files)
            .into_iter()
            .find(|p| p.manifest.plugin_type == plugin_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_files() -> Vec<BufFile> {
        let manifest = r#"{
            "type": "test-plugin",
            "name": "Test Plugin",
            "version": "1.0.0",
            "description": "A test plugin",
            "js": "test.js",
            "wasm": "test_bg.wasm",
            "data_file": "other/test_data.json"
        }"#;

        vec![
            BufFile {
                name: Some("other/viewer/test-plugin/manifest.json".to_string()),
                content: Some(manifest.as_bytes().to_vec()),
                file_id: None,
                path: None,
            },
            BufFile {
                name: Some("other/viewer/test-plugin/test.js".to_string()),
                content: Some(b"// test JS".to_vec()),
                file_id: None,
                path: None,
            },
            BufFile {
                name: Some("other/viewer/test-plugin/test_bg.wasm".to_string()),
                content: Some(b"\0asm\x01\0\0\0".to_vec()), // WASM magic header
                file_id: None,
                path: None,
            },
            BufFile {
                name: Some("other/test_data.json".to_string()),
                content: Some(b"{\"test\": true}".to_vec()),
                file_id: None,
                path: None,
            },
        ]
    }

    #[test]
    fn test_discover_plugins() {
        let files = create_test_files();
        let plugins = PluginManager::discover_plugins(&files);

        assert_eq!(plugins.len(), 1);
        let plugin = &plugins[0];
        assert_eq!(plugin.manifest.plugin_type, "test-plugin");
        assert_eq!(plugin.manifest.name, "Test Plugin");
        assert!(plugin.has_data());
    }

    #[test]
    fn test_has_plugins() {
        let files = create_test_files();
        assert!(PluginManager::has_plugins(&files));

        let empty_files: Vec<BufFile> = vec![];
        assert!(!PluginManager::has_plugins(&empty_files));
    }

    #[test]
    fn test_find_plugin() {
        let files = create_test_files();

        let found = PluginManager::find_plugin(&files, "test-plugin");
        assert!(found.is_some());

        let not_found = PluginManager::find_plugin(&files, "nonexistent");
        assert!(not_found.is_none());
    }
}
