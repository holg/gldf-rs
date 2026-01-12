//! Stadium Floodlight Viewer - Native Entry Point
//!
//! Run the stadium visualization in a native desktop window.
//!
//! ## Usage
//!
//! ```bash
//! gldf-stadium-viewer [LDT_FILE]
//! ```
//!
//! ## Controls
//!
//! - **Arrow Keys**: Rotate camera around stadium (Left/Right) or change height (Up/Down)
//! - **O**: Hold to auto-orbit the camera
//! - **H**: Toggle shadows
//! - **N**: Toggle night/day mode
//! - **+/-**: Increase/decrease light intensity

use gldf_stadium_plugin::run_native_with_ldt;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    println!("GLDF Stadium Floodlight Viewer");
    println!("==============================");
    println!();

    let ldt_path = args.get(1).map(|s| s.as_str());

    if let Some(path) = ldt_path {
        println!("Loading LDT file: {}", path);
    } else {
        println!("No LDT file specified, using default lighting.");
        println!("Usage: gldf-stadium-viewer <path/to/file.ldt>");
    }

    println!();
    println!("Controls:");
    println!("  Arrow Left/Right: Orbit camera around stadium");
    println!("  Arrow Up/Down: Move camera up/down");
    println!("  W/S: Move camera forward/backward");
    println!("  A/D: Move camera left/right");
    println!("  Q/E: Zoom in/out");
    println!("  R: Reset view");
    println!("  T: Toggle tower layout (4 corners / 6 with middle)");
    println!("  C/V: Decrease/Increase floodlight tilt");
    println!("  +/-: Adjust light intensity");
    println!();

    run_native_with_ldt(ldt_path);
}
