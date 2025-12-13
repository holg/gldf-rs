//! GLDF Bevy 3D Viewer
//!
//! A cross-platform 3D viewer for GLDF files with L3D models and LDT lighting.
//! Works on desktop (Windows, macOS, Linux) and web (WASM).

use gldf_bevy::run_native;

/// Native entry point
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run_native();
}

/// WASM entry point - runs automatically on WASM init
#[cfg(target_arch = "wasm32")]
fn main() {
    // On WASM, use wasm_start instead
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn wasm_start() {
        run_native();
    }
}
