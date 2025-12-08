//! GLDF Viewer - Native entry point
//!
//! Cross-platform desktop application for viewing GLDF files.
//! Built with egui/eframe.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

fn main() -> eframe::Result<()> {
    // Log to stderr (if you run with `RUST_LOG=debug`).
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "GLDF Viewer",
        native_options,
        Box::new(|cc| Ok(Box::new(gldf_egui::GldfViewerApp::new(cc)))),
    )
}

/// Load the application icon
fn load_icon() -> egui::IconData {
    // For now, return a simple icon. In production, embed a real icon.
    // let icon_data = include_bytes!("../assets/icon.png");
    // let image = image::load_from_memory(icon_data).unwrap().to_rgba8();
    // egui::IconData {
    //     rgba: image.into_raw(),
    //     width: image.width(),
    //     height: image.height(),
    // }

    // Default empty icon
    egui::IconData::default()
}
