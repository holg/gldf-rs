//! GLDF Bevy 3D Viewer
//!
//! A cross-platform 3D viewer for GLDF files with L3D models and LDT lighting.
//! Works on desktop (Windows, macOS, Linux) and web (WASM).
//!
//! Note: For WASM builds, use the library's `wasm_start()` function directly
//! from JavaScript. This binary is only for native desktop builds.

use gldf_bevy::run_native;

/// Native entry point
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run_native();
}

/// WASM entry point - empty main, use library's wasm_start instead
#[cfg(target_arch = "wasm32")]
fn main() {
    // On WASM, the library exports wasm_start() directly
    // This binary isn't used for WASM - the library is loaded via wasm-bindgen
}
