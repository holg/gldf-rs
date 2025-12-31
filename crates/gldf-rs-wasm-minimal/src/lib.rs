//! Minimal GLDF Viewer Library
//!
//! This is a thin plugin host that delegates all heavy functionality to
//! embedded WASM plugins within the GLDF file. The viewer itself only handles:
//! - GLDF container parsing (ZIP)
//! - Plugin discovery and loading
//! - Basic UI shell
//!
//! All photometric analysis, 3D rendering, etc. comes from plugins.

use wasm_bindgen::prelude::*;

pub mod js_bindings;
pub mod plugins;

// Re-export plugin system
pub use plugins::*;
