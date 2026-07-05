//! GLDF Bevy 3D Viewer
//!
//! A cross-platform 3D viewer for GLDF files with L3D models and LDT lighting.
//! Works on desktop (Windows, macOS, Linux) and web (WASM).
//!
//! Note: For WASM builds, use the library's `wasm_start()` function directly
//! from JavaScript. This binary is only for native desktop builds.

/// Native entry point.
///
/// Usage:
///   gldf-bevy-viewer                      # empty scene (legacy)
///   gldf-bevy-viewer <file.gldf>          # open variant 0
///   gldf-bevy-viewer <file.gldf> --variant N
///
/// The editor (`gldf-editor-native`) launches this with a temp `.gldf`
/// (its current product via `save_to_buf`) and the active variant index.
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let mut path: Option<std::path::PathBuf> = None;
    let mut variant: usize = 0;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--variant" => {
                if let Some(v) = args.next().and_then(|s| s.parse().ok()) {
                    variant = v;
                }
            }
            other if !other.starts_with("--") => {
                path = Some(std::path::PathBuf::from(other));
            }
            _ => {}
        }
    }

    match path {
        Some(p) => match gldf_bevy::scene_bundle::load_scene_from_gldf(&p, variant) {
            Ok(scene) => {
                eprintln!(
                    "[gldf-bevy] loaded {} variant {variant}: l3d={} ldt={} emitters={}",
                    p.display(),
                    scene.l3d_data.is_some(),
                    scene.ldt_data.is_some(),
                    scene.emitter_config.len(),
                );
                gldf_bevy::run_native_with_scene(scene);
            }
            Err(e) => {
                eprintln!("[gldf-bevy] failed to load {}: {e}", p.display());
                gldf_bevy::run_native();
            }
        },
        None => gldf_bevy::run_native(),
    }
}

/// WASM entry point - empty main, use library's wasm_start instead
#[cfg(target_arch = "wasm32")]
fn main() {
    // On WASM, the library exports wasm_start() directly
    // This binary isn't used for WASM - the library is loaded via wasm-bindgen
}
